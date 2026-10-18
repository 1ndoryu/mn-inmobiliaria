//! Test de integración F2 y F3 de `plan-resolver-diferencias-sync-2026-10-14` (D17).
//! Aplica el plan de resolución entre dos BD desechables: `glory_sync_ap_local`
//! (origen) y `glory_sync_ap_remoto` (servidor de prueba en 127.0.0.1). Nunca usa
//! `PROD_BASE`. Requiere `DATABASE_URL` con permiso para crear BD; sin ella falla
//! (no se omite).
//! Ejecutar: `node scripts/run-cargo.mjs test --test sync_aplicar`
//! Con éxito borra sus BD y su carpeta temporal; con fallo las deja para inspección.

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use glory_backend::config::AppConfig;
use glory_backend::errors::AppError;
use glory_backend::models::{CreateInmuebleRequest, RecetaPublicidad, RegisterRequest, UpdateInmuebleRequest};

use glory_backend::repositories::InmuebleRepository;
use glory_backend::services::sync::aplicar::aplicar;
use glory_backend::services::sync::comparacion::comparar::{comparar_todo, Base, Comparacion, Estado, Lado};
use glory_backend::services::sync::comparacion::detalle::detalle_local;
use glory_backend::services::sync::comparacion::resolver::{Accion, Eleccion};
use glory_backend::services::sync::operacion::Turno;
use glory_backend::services::sync::plan::plan_de;
use glory_backend::services::sync::publicar::publicar;
use glory_backend::services::sync::remoto::{self, Credenciales};
use glory_backend::services::{AuthService, InmuebleService};
use serde_json::json;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{Connection, PgConnection, PgPool};
use uuid::Uuid;

const SECRETO: &str = "ap-secreto-prueba-12345678";
const EMAIL: &str = "ap@example.com";
const PASSWORD: &str = "ap-prueba-12345";
const BD_LOCAL: &str = "glory_sync_ap_local";
const BD_REMOTO: &str = "glory_sync_ap_remoto";

fn admin_opts() -> PgConnectOptions {
    let url = std::env::var("DATABASE_URL")
        .expect("sync_aplicar: falta DATABASE_URL (BD de pruebas con permiso CREATE DATABASE)");
    PgConnectOptions::from_str(&url).expect("sync_aplicar: DATABASE_URL no es válida")
}

/// Recrea una BD vacía y la migra. `nombre` es una constante del test (no entrada externa).
async fn recrear_bd(admin: &PgConnectOptions, nombre: &str) -> PgPool {
    let mut conn = PgConnection::connect_with(&admin.clone().database("postgres"))
        .await
        .expect("sync_aplicar: no conecta a la BD postgres");
    sqlx::query(&format!("DROP DATABASE IF EXISTS {nombre} WITH (FORCE)"))
        .execute(&mut conn)
        .await
        .unwrap();
    sqlx::query(&format!("CREATE DATABASE {nombre}"))
        .execute(&mut conn)
        .await
        .unwrap();
    conn.close().await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(admin.clone().database(nombre))
        .await
        .unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    pool
}

/// Servidor HTTP del remoto de prueba. `database_url` va vacía a propósito: el router
/// recibe su pool y nada debe reconectar por URL.
async fn servidor_remoto(pool: &PgPool, upload: &Path) -> String {
    let config = AppConfig {
        database_url: String::new(),
        jwt_secret: SECRETO.to_string(),
        host: "127.0.0.1".to_string(),
        port: 0,
        upload_dir: upload.display().to_string(),
        static_dir: None,
        cors_origins: Vec::new(),
    };
    let app = glory_backend::handlers::create_router(pool.clone(), config);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    base
}

/// Credenciales del remoto de prueba vía archivo temporal (`SYNC_REMOTO_ENV`).
fn credenciales_remoto(dir: &Path, base: &str) -> Credenciales {
    let ruta = dir.join("remoto.env");
    std::fs::write(
        &ruta,
        format!("PROD_BASE={base}\nPROD_EMAIL={EMAIL}\nPROD_PASSWORD={PASSWORD}\n"),
    )
    .unwrap();
    std::env::set_var("SYNC_REMOTO_ENV", &ruta);
    Credenciales::cargar().expect("sync_aplicar: no carga las credenciales de prueba")
}

/// Bytes con cabecera JPEG válida; `semilla` distingue el contenido (y el sha256).
fn jpeg(semilla: u8) -> Vec<u8> {
    let mut bytes = vec![0xFF, 0xD8, 0xFF, 0xE0];
    bytes.extend(std::iter::repeat(semilla).take(256));
    bytes
}

fn receta() -> RecetaPublicidad {
    RecetaPublicidad {
        fondo_idx: 0,
        circular_grande_idx: 1,
        circular_mediano_idx: 2,
        formato: "post-4-5".into(),
        con_precio: true,
        titulo1: String::new(),
        titulo2: String::new(),
    }
}

/// Inmueble completo (copy, receta, ficha, dos fotos, publicado y reservado).
/// Devuelve `(id, slug)`.
async fn sembrar(pool: &PgPool, upload: &Path, titulo: &str) -> (Uuid, String) {
    let alta: CreateInmuebleRequest = serde_json::from_value(json!({
        "titulo": titulo,
        "descripcion": "Inmueble de prueba del test de aplicar",
        "ubicacion": "Zona de prueba",
        "puestos": 2,
        "residencia": "Residencia de prueba",
        "precio": 150000.0,
        "habitaciones": 2,
        "banos": 1,
        "metros": 80.0,
        "metros_terreno": 0.0,
        "copy": {
            "corta": "Copy corta de prueba",
            "larga": "Copy larga de prueba",
            "modelo": "ap",
            "actualizada_en": "2026-10-16T10:00:00Z"
        }
    }))
    .expect("alta de prueba válida");
    let inmueble = InmuebleService::create(pool, alta)
        .await
        .expect("alta de prueba");
    InmuebleService::update(
        pool,
        inmueble.id,
        UpdateInmuebleRequest {
            receta: Some(receta()),
            ..Default::default()
        },
    )
    .await
    .expect("receta de prueba");
    InmuebleRepository::set_ficha(
        pool,
        inmueble.id,
        sqlx::types::Json(json!({"contacto": "ap", "nota": "ficha de prueba"})),
        Some(120_000.0),
    )
    .await
    .expect("ficha de prueba")
    .expect("ficha: el inmueble existe");
    InmuebleService::subir_foto(
        pool,
        upload,
        inmueble.id,
        "original.jpg",
        Some("original"),
        Some(1),
        &jpeg(1),
    )
    .await
    .expect("foto original");
    InmuebleService::subir_foto(
        pool,
        upload,
        inmueble.id,
        "mejorada.jpg",
        Some("mejorada"),
        Some(2),
        &jpeg(2),
    )
    .await
    .expect("foto mejorada");
    InmuebleService::set_publicado(pool, inmueble.id, true)
        .await
        .expect("publicado de prueba");
    InmuebleService::set_estado(pool, inmueble.id, "reservado")
        .await
        .expect("estado de prueba");
    (inmueble.id, inmueble.slug)
}

/// Anota la base acordada, como hace la vigilancia cuando los dos lados coinciden.
/// Sin ella toda diferencia es `Indeterminado` y el plan decide por `updated_at`,
/// no por quién cambió cada campo. Va por SQL directo porque `repositories::sync`
/// es privado del crate; el JSON es el mismo que escribe `guardar_bases`.
async fn acordar_base(pool: &PgPool, slug: &str) {
    let huella = detalle_local(pool, slug)
        .await
        .expect("detalle local")
        .expect("el inmueble existe en local")
        .huella;
    let base = Base::de(&huella);
    sqlx::query(
        "INSERT INTO sync_base (slug, campos, fotos, acordado_en) VALUES ($1, $2, $3, now()) \
         ON CONFLICT (slug) DO UPDATE SET campos = EXCLUDED.campos, \
         fotos = EXCLUDED.fotos, acordado_en = now()",
    )
    .bind(slug)
    .bind(sqlx::types::Json(serde_json::to_value(&base.campos).unwrap()))
    .bind(sqlx::types::Json(serde_json::to_value(&base.fotos).unwrap()))
    .execute(pool)
    .await
    .expect("base acordada");
}

/// Siembra en local, publica, anota la base y devuelve `(id local, id remoto, slug)`.
async fn sembrar_publicado(
    local: &PgPool,
    upload: &Path,
    cred: &Credenciales,
    titulo: &str,
) -> (Uuid, Uuid, String) {
    let (id, slug) = sembrar(local, upload, titulo).await;
    let informe = publicar(local, upload, cred, &slug).await.expect("publicar");
    assert!(informe.fallo.is_none(), "publicar falló: {:?}", informe.fallo);
    let id_remoto = remoto::detalle(cred, &slug)
        .await
        .expect("detalle remoto")
        .expect("el inmueble está en el servidor")
        .huella
        .id;
    acordar_base(local, &slug).await;
    (id, id_remoto, slug)
}

/// Filas de inmuebles y fotos con ese slug en la BD dada.
async fn conteos(pool: &PgPool, slug: &str) -> (i64, i64) {
    let inmuebles: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM inmuebles WHERE slug = $1")
        .bind(slug)
        .fetch_one(pool)
        .await
        .unwrap();
    let fotos: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM fotos f JOIN inmuebles i ON i.id = f.inmueble_id WHERE i.slug = $1",
    )
    .bind(slug)
    .fetch_one(pool)
    .await
    .unwrap();
    (inmuebles, fotos)
}

/// Compara la huella local con la del servidor y devuelve el estado del inmueble.
async fn estado_entre(pool: &PgPool, cred: &Credenciales, slug: &str) -> Estado {
    let local = detalle_local(pool, slug)
        .await
        .expect("detalle local")
        .expect("el inmueble existe en local");
    let remoto = remoto::detalle(cred, slug)
        .await
        .expect("detalle remoto")
        .expect("el inmueble existe en el remoto");
    let cmp: Comparacion = comparar_todo(&[local.huella], &[remoto.huella], &HashMap::new());
    cmp.inmuebles
        .iter()
        .find(|e| e.slug == slug)
        .expect("slug presente en la comparación")
        .resultado
        .estado
}

/// Campo de texto de un inmueble de la BD, por slug (lo que quedó escrito de verdad).
async fn texto(pool: &PgPool, slug: &str, campo: &str) -> String {
    let sql = format!("SELECT {campo} FROM inmuebles WHERE slug = $1");
    sqlx::query_scalar(&sql)
        .bind(slug)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn booleano(pool: &PgPool, slug: &str, campo: &str) -> bool {
    let sql = format!("SELECT {campo} FROM inmuebles WHERE slug = $1");
    sqlx::query_scalar(&sql)
        .bind(slug)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// Vínculo con Marketplace de un inmueble de la BD, por slug (None = sin vincular).
async fn vinculo(pool: &PgPool, slug: &str) -> Option<String> {
    sqlx::query_scalar("SELECT marketplace_id FROM inmuebles WHERE slug = $1")
        .bind(slug)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// Sin elecciones: todo lo decide el plan por defecto.
fn sin_conservar() -> BTreeSet<String> {
    BTreeSet::new()
}

/// Temporales del entorno donde corre el test: `C:\tmp` en el portátil, `/var/tmp`
/// en la 89 (AGENTS §1). Absoluta siempre: el test no depende del directorio actual.
fn raiz_temporal(nombre: &str) -> PathBuf {
    let base = if cfg!(windows) {
        PathBuf::from(r"C:\tmp")
    } else {
        PathBuf::from("/var/tmp")
    };
    base.join(format!("{nombre}-{}", Uuid::new_v4()))
}

#[tokio::test]
async fn aplicar_escribe_en_los_dos_sentidos_y_verifica() {
    let admin = admin_opts();
    let raiz = raiz_temporal("sync-aplicar");
    let dir_local = raiz.join("local");
    let dir_remoto = raiz.join("remoto");
    std::fs::create_dir_all(&dir_local).unwrap();
    std::fs::create_dir_all(&dir_remoto).unwrap();

    let pool_local = recrear_bd(&admin, BD_LOCAL).await;
    let pool_remoto = recrear_bd(&admin, BD_REMOTO).await;

    AuthService::register(
        &pool_remoto,
        RegisterRequest {
            email: EMAIL.into(),
            password: PASSWORD.into(),
        },
        SECRETO,
    )
    .await
    .expect("owner del remoto de prueba");
    let base = servidor_remoto(&pool_remoto, &dir_remoto).await;
    let cred = credenciales_remoto(&raiz, &base);

    /* 1. Un campo cambió en cada lado: cada uno se escribe en el contrario y los
     * dos quedan iguales, sin residuo. */
    let (id1, id_remoto1, slug1) = sembrar_publicado(
        &pool_local,
        &dir_local,
        &cred,
        "Casa aplicar dos sentidos",
    )
    .await;
    InmuebleService::update(
        &pool_local,
        id1,
        UpdateInmuebleRequest {
            titulo: Some("Casa aplicar dos sentidos (local)".into()),
            ..Default::default()
        },
    )
    .await
    .expect("título local");
    remoto::actualizar_inmueble(&cred, id_remoto1, &json!({"precio": 175_000.0}))
        .await
        .expect("precio remoto");

    let plan = plan_de(&pool_local, &cred, &slug1, &sin_conservar())
        .await
        .expect("plan");
    let volcado = plan
        .filas
        .iter()
        .map(|f| format!("{}: {:?}/{:?} → {:?}", f.clave, f.lado, f.elegido, f.accion))
        .collect::<Vec<_>>()
        .join(" | ");
    assert_eq!(
        (plan.en_local, plan.en_servidor),
        (1, 1),
        "una operación por lado; filas: {volcado}"
    );
    let de = |clave: &str| plan.filas.iter().find(|f| f.clave == clave).expect(clave);
    assert_eq!(de("titulo").elegido, Eleccion::Local);
    assert_eq!(de("precio").elegido, Eleccion::Servidor);

    let informe = aplicar(&pool_local, &dir_local, &cred, &slug1, &plan.plan_id, &sin_conservar())
        .await
        .expect("aplicar");
    assert!(informe.fallo.is_none(), "fallo: {:?}", informe.fallo);
    assert!(informe.residuo.is_empty(), "residuo: {:?}", informe.residuo);
    assert_eq!(informe.en_local.len(), 1, "hecho: {:?}", informe.en_local);
    assert_eq!(informe.en_servidor.len(), 1, "hecho: {:?}", informe.en_servidor);
    assert!(informe.en_local[0].contains("precio"), "hecho: {:?}", informe.en_local);
    assert!(informe.en_servidor[0].contains("titulo"), "hecho: {:?}", informe.en_servidor);
    assert_eq!(texto(&pool_local, &slug1, "titulo").await, "Casa aplicar dos sentidos (local)");
    assert_eq!(
        texto(&pool_remoto, &slug1, "titulo").await,
        "Casa aplicar dos sentidos (local)",
        "el título viaja al servidor"
    );
    assert_eq!(estado_entre(&pool_local, &cred, &slug1).await, Estado::Igual, "tras aplicar");

    /* 2. `plan_id` viejo: el servidor cambió desde que se mostró el plan → 409 y
     * no se escribe nada (D3). */
    let (id2, id_remoto2, slug2) = sembrar_publicado(
        &pool_local,
        &dir_local,
        &cred,
        "Casa aplicar plan viejo",
    )
    .await;
    let plan2 = plan_de(&pool_local, &cred, &slug2, &sin_conservar())
        .await
        .expect("plan");
    remoto::actualizar_inmueble(&cred, id_remoto2, &json!({"precio": 199_000.0}))
        .await
        .expect("precio remoto");
    let err = aplicar(&pool_local, &dir_local, &cred, &slug2, &plan2.plan_id, &sin_conservar())
        .await
        .expect_err("un plan viejo debe dar 409");
    assert!(matches!(err, AppError::Conflict(_)), "error: {err:?}");
    assert_eq!(
        texto(&pool_local, &slug2, "titulo").await,
        "Casa aplicar plan viejo",
        "el local no cambia"
    );
    assert_eq!(id2, id2, "el local sigue siendo el mismo inmueble");
    let plan2bis = plan_de(&pool_local, &cred, &slug2, &sin_conservar())
        .await
        .expect("plan");
    assert_ne!(plan2.plan_id, plan2bis.plan_id, "el plan cambió de verdad");

    /* 3. F3: las fotos viajan por bytes en los dos sentidos. Una `original` que
     * solo está en el local se sube al servidor; una que el servidor perdió se
     * borra del local. En los dos casos el residuo queda vacío, que es la prueba
     * de que los bytes coinciden (el `original` compara sha256). */
    let (id3, id_remoto3, slug3) = sembrar_publicado(
        &pool_local,
        &dir_local,
        &cred,
        "Casa aplicar fotos",
    )
    .await;
    InmuebleService::subir_foto(
        &pool_local,
        &dir_local,
        id3,
        "extra.jpg",
        Some("original"),
        Some(3),
        &jpeg(3),
    )
    .await
    .expect("foto extra en local");
    let plan3 = plan_de(&pool_local, &cred, &slug3, &sin_conservar())
        .await
        .expect("plan");
    let fila3 = plan3
        .filas
        .iter()
        .find(|f| f.clave == "foto:original:3")
        .expect("la foto nueva está en el plan");
    assert_eq!(
        (fila3.elegido, fila3.accion),
        (Eleccion::Local, Accion::Escribir),
        "la foto la tiene el local: se sube"
    );
    assert_eq!(plan3.en_servidor, 1, "el contador la cuenta: se puede aplicar");
    assert_eq!(conteos(&pool_remoto, &slug3).await.1, 2, "el servidor aún no la tiene");

    let informe3 = aplicar(&pool_local, &dir_local, &cred, &slug3, &plan3.plan_id, &sin_conservar())
        .await
        .expect("aplicar");
    assert!(informe3.fallo.is_none(), "fallo: {:?}", informe3.fallo);
    assert!(informe3.pendientes.is_empty(), "pendientes: {:?}", informe3.pendientes);
    assert!(
        informe3.en_servidor.iter().any(|h| h.contains("foto:original:3")),
        "hecho: {:?}",
        informe3.en_servidor
    );
    assert_eq!(conteos(&pool_remoto, &slug3).await.1, 3, "la foto subió");
    assert_eq!(
        estado_entre(&pool_local, &cred, &slug3).await,
        Estado::Igual,
        "los bytes de la foto son los mismos en los dos lados"
    );
    assert!(informe3.residuo.is_empty(), "residuo: {:?}", informe3.residuo);

    // El servidor pierde esa foto: ahora gana él y se borra del local. La base se
    // refresca antes, como hace la vigilancia tras aplicar: si no, la foto nueva
    // sigue siendo «un cambio del local» y el plan la volvería a subir.
    acordar_base(&pool_local, &slug3).await;
    let remoto3 = remoto::inmueble(&cred, id_remoto3)
        .await
        .expect("inmueble remoto")
        .expect("existe en el servidor");
    let id_foto_remota = remoto3
        .fotos
        .iter()
        .find(|f| f.origen == "original" && f.orden == 3)
        .expect("la foto está en el servidor")
        .id;
    remoto::borrar_foto(&cred, id_foto_remota)
        .await
        .expect("borrar la foto en el servidor");
    let plan3bis = plan_de(&pool_local, &cred, &slug3, &sin_conservar())
        .await
        .expect("plan");
    let fila3bis = plan3bis
        .filas
        .iter()
        .find(|f| f.clave == "foto:original:3")
        .expect("la fila sigue en el plan");
    assert_eq!(
        (fila3bis.elegido, fila3bis.accion),
        (Eleccion::Servidor, Accion::Quitar),
        "gana el servidor, que ya no la tiene"
    );
    let informe3bis = aplicar(
        &pool_local,
        &dir_local,
        &cred,
        &slug3,
        &plan3bis.plan_id,
        &sin_conservar(),
    )
    .await
    .expect("aplicar");
    assert!(informe3bis.fallo.is_none(), "fallo: {:?}", informe3bis.fallo);
    assert!(
        informe3bis.en_local.iter().any(|h| h.contains("foto:original:3")),
        "hecho: {:?}",
        informe3bis.en_local
    );
    assert_eq!(conteos(&pool_local, &slug3).await.1, 2, "la foto se borró del local");
    assert_eq!(
        estado_entre(&pool_local, &cred, &slug3).await,
        Estado::Igual,
        "otra vez iguales"
    );
    assert!(informe3bis.residuo.is_empty(), "residuo: {:?}", informe3bis.residuo);

    /* 3b. La otra dirección de F3: una foto que solo tiene el servidor se baja al
     * local. Es el caso del álbum que creció en producción, y el que la subida no
     * cubre: aquí el viaje es una descarga más un alta local. */
    acordar_base(&pool_local, &slug3).await;
    remoto::subir_foto(&cred, id_remoto3, "nueva.jpg", "original", 4, jpeg(4))
        .await
        .expect("foto nueva en el servidor");
    let plan3c = plan_de(&pool_local, &cred, &slug3, &sin_conservar())
        .await
        .expect("plan");
    let fila3c = plan3c
        .filas
        .iter()
        .find(|f| f.clave == "foto:original:4")
        .expect("la foto del servidor está en el plan");
    assert_eq!(
        (fila3c.elegido, fila3c.accion),
        (Eleccion::Servidor, Accion::Escribir),
        "la foto la tiene el servidor: se baja"
    );
    assert_eq!(plan3c.en_local, 1, "el contador la cuenta: se puede aplicar");
    assert_eq!(conteos(&pool_local, &slug3).await.1, 2, "el local aún no la tiene");
    let informe3c = aplicar(
        &pool_local,
        &dir_local,
        &cred,
        &slug3,
        &plan3c.plan_id,
        &sin_conservar(),
    )
    .await
    .expect("aplicar");
    assert!(informe3c.fallo.is_none(), "fallo: {:?}", informe3c.fallo);
    assert!(informe3c.pendientes.is_empty(), "pendientes: {:?}", informe3c.pendientes);
    assert!(
        informe3c.en_local.iter().any(|h| h.contains("foto:original:4")),
        "hecho: {:?}",
        informe3c.en_local
    );
    assert_eq!(conteos(&pool_local, &slug3).await.1, 3, "la foto bajó");
    assert_eq!(
        estado_entre(&pool_local, &cred, &slug3).await,
        Estado::Igual,
        "los bytes bajaron enteros"
    );
    assert!(informe3c.residuo.is_empty(), "residuo: {:?}", informe3c.residuo);

    /* 3c. El destino ya tiene esa clave con **otra** foto, no con los mismos
     * bytes: al local se le cambia el archivo en su sitio, conservando la fila y
     * el `orden`. Borrarla y volver a subirla renumeraría el álbum. */
    let (id3d, slug3d) = sembrar(&pool_local, &dir_local, "Casa aplicar reemplazo").await;
    // Un álbum de una sola foto: así el borrado en el servidor no renumera nada
    // y la sustituta cae en el mismo `orden`.
    let mejorada3d: Uuid =
        sqlx::query_scalar("SELECT id FROM fotos WHERE inmueble_id = $1 AND origen = 'mejorada'")
            .bind(id3d)
            .fetch_one(&pool_local)
            .await
            .unwrap();
    InmuebleService::delete_foto(&pool_local, &dir_local, mejorada3d)
        .await
        .expect("quitar la mejorada de prueba");
    let informe3d = publicar(&pool_local, &dir_local, &cred, &slug3d)
        .await
        .expect("publicar");
    assert!(informe3d.fallo.is_none(), "publicar falló: {:?}", informe3d.fallo);
    acordar_base(&pool_local, &slug3d).await;
    let id_remoto3d = remoto::detalle(&cred, &slug3d)
        .await
        .expect("detalle remoto")
        .expect("el inmueble está en el servidor")
        .huella
        .id;
    let foto_local3d: Uuid =
        sqlx::query_scalar("SELECT id FROM fotos WHERE inmueble_id = $1 AND origen = 'original'")
            .bind(id3d)
            .fetch_one(&pool_local)
            .await
            .unwrap();
    // El servidor cambia su única foto por otra distinta, en el mismo `orden`.
    let id_foto_remota3d = remoto::inmueble(&cred, id_remoto3d)
        .await
        .expect("inmueble remoto")
        .expect("existe en el servidor")
        .fotos
        .iter()
        .find(|f| f.origen == "original")
        .expect("la original está en el servidor")
        .id;
    remoto::borrar_foto(&cred, id_foto_remota3d)
        .await
        .expect("borrar la foto del servidor");
    remoto::subir_foto(&cred, id_remoto3d, "otra.jpg", "original", 1, jpeg(9))
        .await
        .expect("subir la sustituta");
    let plan3d = plan_de(&pool_local, &cred, &slug3d, &sin_conservar())
        .await
        .expect("plan");
    let fila3d = plan3d
        .filas
        .iter()
        .find(|f| f.clave == "foto:original:1")
        .expect("la foto está en el plan");
    assert_eq!(
        (fila3d.elegido, fila3d.accion),
        (Eleccion::Servidor, Accion::Escribir),
        "otra foto en el mismo orden: gana el servidor"
    );
    assert_eq!(plan3d.en_local, 1, "el contador la cuenta: se puede aplicar");
    let informe3d = aplicar(
        &pool_local,
        &dir_local,
        &cred,
        &slug3d,
        &plan3d.plan_id,
        &sin_conservar(),
    )
    .await
    .expect("aplicar");
    assert!(informe3d.fallo.is_none(), "fallo: {:?}", informe3d.fallo);
    assert!(informe3d.pendientes.is_empty(), "pendientes: {:?}", informe3d.pendientes);
    assert!(
        informe3d.en_local.iter().any(|h| h.contains("reemplazada")),
        "hecho: {:?}",
        informe3d.en_local
    );
    let foto_local3d_despues: Uuid =
        sqlx::query_scalar("SELECT id FROM fotos WHERE inmueble_id = $1 AND origen = 'original'")
            .bind(id3d)
            .fetch_one(&pool_local)
            .await
            .unwrap();
    assert_eq!(
        foto_local3d_despues, foto_local3d,
        "se cambió el archivo, no la fila"
    );
    assert_eq!(conteos(&pool_local, &slug3d).await.1, 1, "ni una fila de más");
    assert_eq!(
        estado_entre(&pool_local, &cred, &slug3d).await,
        Estado::Igual,
        "los bytes nuevos coinciden"
    );
    assert!(informe3d.residuo.is_empty(), "residuo: {:?}", informe3d.residuo);

    /* 4. El estado va por su propio camino: «vendido» en el servidor despublica
     * también en el local. */
    let (_, id_remoto4, slug4) = sembrar_publicado(
        &pool_local,
        &dir_local,
        &cred,
        "Casa aplicar estado",
    )
    .await;
    remoto::poner_estado(&cred, id_remoto4, "vendido")
        .await
        .expect("estado en el servidor");
    let plan4 = plan_de(&pool_local, &cred, &slug4, &sin_conservar())
        .await
        .expect("plan");
    let informe4 = aplicar(&pool_local, &dir_local, &cred, &slug4, &plan4.plan_id, &sin_conservar())
        .await
        .expect("aplicar");
    assert!(informe4.fallo.is_none(), "fallo: {:?}", informe4.fallo);
    assert_eq!(texto(&pool_local, &slug4, "estado").await, "vendido");
    assert!(!booleano(&pool_local, &slug4, "publicado").await, "vendido despublica");
    assert!(
        informe4.residuo.is_empty(),
        "residuo: {:?}",
        informe4.residuo
    );

    /* 5. El lado ganador no tiene receta y la API no la borra: queda pendiente en
     * vez de darse por aplicada. */
    let (id5, _, slug5) = sembrar_publicado(
        &pool_local,
        &dir_local,
        &cred,
        "Casa aplicar receta",
    )
    .await;
    sqlx::query(
        "UPDATE inmuebles SET receta = NULL, updated_at = NOW() + interval '1 hour' WHERE id = $1",
    )
    .bind(id5)
    .execute(&pool_local)
    .await
    .expect("receta nula en local");
    let plan5 = plan_de(&pool_local, &cred, &slug5, &sin_conservar())
        .await
        .expect("plan");
    let fila5 = plan5.filas.iter().find(|f| f.clave == "receta").expect("fila receta");
    assert_eq!(fila5.elegido, Eleccion::Local, "gana el local, que no tiene receta");
    let informe5 = aplicar(&pool_local, &dir_local, &cred, &slug5, &plan5.plan_id, &sin_conservar())
        .await
        .expect("aplicar");
    assert!(informe5.fallo.is_none(), "fallo: {:?}", informe5.fallo);
    assert!(
        informe5.pendientes.iter().any(|p| p.starts_with("receta")),
        "pendientes: {:?}",
        informe5.pendientes
    );
    let receta_remota: Option<serde_json::Value> =
        sqlx::query_scalar("SELECT receta FROM inmuebles WHERE slug = $1")
            .bind(&slug5)
            .fetch_one(&pool_remoto)
            .await
            .unwrap();
    assert!(receta_remota.is_some(), "la receta del servidor no se toca");

    /* 6. Una operación en curso sobre el mismo inmueble: 409 sin escribir. */
    let (_, _, slug6) = sembrar_publicado(
        &pool_local,
        &dir_local,
        &cred,
        "Casa aplicar turno",
    )
    .await;
    InmuebleService::update(
        &pool_local,
        sqlx::query_scalar("SELECT id FROM inmuebles WHERE slug = $1")
            .bind(&slug6)
            .fetch_one(&pool_local)
            .await
            .unwrap(),
        UpdateInmuebleRequest {
            titulo: Some("Casa aplicar turno (local)".into()),
            ..Default::default()
        },
    )
    .await
    .expect("título local");
    let plan6 = plan_de(&pool_local, &cred, &slug6, &sin_conservar())
        .await
        .expect("plan");
    let turno = Turno::tomar(&slug6).expect("turno libre");
    let err = aplicar(&pool_local, &dir_local, &cred, &slug6, &plan6.plan_id, &sin_conservar())
        .await
        .expect_err("con el turno tomado debe dar 409");
    assert!(matches!(err, AppError::Conflict(_)), "error: {err:?}");
    drop(turno);
    assert_eq!(
        texto(&pool_remoto, &slug6, "titulo").await,
        "Casa aplicar turno",
        "el servidor no cambia"
    );

    /* 7. [18AA-3] El vínculo con Marketplace se resuelve desde el panel como
     * cualquier campo: si el local lo tiene y el servidor no, se escribe allí, y
     * si el local lo suelta, se borra allí (el PUT del servidor es tri-estado y
     * el plan manda el valor canónico vacío, que para el vínculo es «sin
     * vincular»). */
    let (id7, _, slug7) = sembrar_publicado(
        &pool_local,
        &dir_local,
        &cred,
        "Casa aplicar vínculo",
    )
    .await;
    InmuebleService::update(
        &pool_local,
        id7,
        UpdateInmuebleRequest {
            marketplace_id: Some(Some("7000000001".into())),
            ..Default::default()
        },
    )
    .await
    .expect("vínculo en local");
    assert_eq!(vinculo(&pool_remoto, &slug7).await, None, "el servidor no lo tiene");
    let plan7 = plan_de(&pool_local, &cred, &slug7, &sin_conservar())
        .await
        .expect("plan");
    let fila7 = plan7
        .filas
        .iter()
        .find(|f| f.clave == "marketplace_id")
        .expect("el vínculo está en el plan");
    assert_eq!(fila7.lado, Lado::Local, "lo cambió el local");
    assert_eq!(
        (fila7.elegido, fila7.accion),
        (Eleccion::Local, Accion::Escribir),
        "gana el local, que lo tiene"
    );
    assert_eq!((plan7.en_local, plan7.en_servidor), (0, 1), "se escribe en el servidor");

    let informe7 = aplicar(&pool_local, &dir_local, &cred, &slug7, &plan7.plan_id, &sin_conservar())
        .await
        .expect("aplicar");
    assert!(informe7.fallo.is_none(), "fallo: {:?}", informe7.fallo);
    assert!(informe7.pendientes.is_empty(), "pendientes: {:?}", informe7.pendientes);
    assert_eq!(
        vinculo(&pool_remoto, &slug7).await.as_deref(),
        Some("7000000001"),
        "el vínculo viajó al servidor"
    );
    assert_eq!(
        estado_entre(&pool_local, &cred, &slug7).await,
        Estado::Igual,
        "ya coinciden"
    );

    // Y al revés: el local suelta el vínculo y el servidor lo suelta con él.
    acordar_base(&pool_local, &slug7).await;
    InmuebleService::update(
        &pool_local,
        id7,
        UpdateInmuebleRequest {
            marketplace_id: Some(None),
            ..Default::default()
        },
    )
    .await
    .expect("desvincular en local");
    let plan7bis = plan_de(&pool_local, &cred, &slug7, &sin_conservar())
        .await
        .expect("plan");
    let fila7bis = plan7bis
        .filas
        .iter()
        .find(|f| f.clave == "marketplace_id")
        .expect("la fila sigue en el plan");
    assert_eq!(
        (fila7bis.elegido, fila7bis.accion),
        (Eleccion::Local, Accion::Escribir),
        "gana el local, que ya no lo tiene: se escribe el vacío"
    );
    let informe7bis = aplicar(
        &pool_local,
        &dir_local,
        &cred,
        &slug7,
        &plan7bis.plan_id,
        &sin_conservar(),
    )
    .await
    .expect("aplicar");
    assert!(informe7bis.fallo.is_none(), "fallo: {:?}", informe7bis.fallo);
    assert_eq!(
        vinculo(&pool_remoto, &slug7).await,
        None,
        "el servidor se desvincula"
    );
    assert_eq!(
        estado_entre(&pool_local, &cred, &slug7).await,
        Estado::Igual,
        "otra vez iguales"
    );

    // Limpieza: solo tras éxito. Las BD son desechables y se borran con su conexión.
    pool_local.close().await;
    pool_remoto.close().await;
    let mut conn = PgConnection::connect_with(&admin.clone().database("postgres"))
        .await
        .unwrap();
    for nombre in [BD_LOCAL, BD_REMOTO] {
        sqlx::query(&format!("DROP DATABASE IF EXISTS {nombre} WITH (FORCE)"))
            .execute(&mut conn)
            .await
            .unwrap();
    }
    conn.close().await.unwrap();
    // Temporal propio de C:\tmp; si no se puede borrar, lo barre GloryTmpSweep.
    let _ = std::fs::remove_dir_all(&raiz);
}
