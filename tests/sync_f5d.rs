//! Test de integración F5d de `plan-resolver-diferencias-sync-2026-10-14` (D17).
//! Publica y trae entre dos BD desechables: `glory_sync_f5d_local` (origen) y
//! `glory_sync_f5d_remoto` (servidor de prueba en 127.0.0.1). Nunca usa `PROD_BASE`.
//! Requiere `DATABASE_URL` con permiso para crear BD; sin ella falla (no se omite).
//! Ejecutar: `node scripts/run-cargo.mjs test --test sync_f5d`
//! Con éxito borra sus BD y su carpeta temporal; con fallo las deja para inspección.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use glory_backend::config::AppConfig;
use glory_backend::errors::AppError;
use glory_backend::models::{
    CreateInmuebleRequest, RecetaPublicidad, RegisterRequest, UpdateInmuebleRequest,
};
use glory_backend::repositories::InmuebleRepository;
use glory_backend::services::sync::comparacion::comparar::{comparar_todo, Comparacion, Estado};
use glory_backend::services::sync::comparacion::detalle::detalle_local;
use glory_backend::services::sync::publicar::publicar;
use glory_backend::services::sync::remoto::{self, Credenciales};
use glory_backend::services::sync::traer::traer;
use glory_backend::services::{AuthService, InmuebleService};
use serde_json::json;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{Connection, PgConnection, PgPool};
use uuid::Uuid;

const SECRETO: &str = "f5d-secreto-prueba-12345678";
const EMAIL: &str = "f5d@example.com";
const PASSWORD: &str = "f5d-prueba-12345";

fn admin_opts() -> PgConnectOptions {
    let url = std::env::var("DATABASE_URL")
        .expect("sync_f5d: falta DATABASE_URL (BD de pruebas con permiso CREATE DATABASE)");
    PgConnectOptions::from_str(&url).expect("sync_f5d: DATABASE_URL no es válida")
}

/// Recrea una BD vacía y la migra. `nombre` es una constante del test (no entrada externa).
async fn recrear_bd(admin: &PgConnectOptions, nombre: &str) -> PgPool {
    let mut conn = PgConnection::connect_with(&admin.clone().database("postgres"))
        .await
        .expect("sync_f5d: no conecta a la BD postgres");
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
    Credenciales::cargar().expect("sync_f5d: no carga las credenciales de prueba")
}

/// Bytes con cabecera JPEG válida; `semilla` distingue el contenido (y el sha256).
fn jpeg(semilla: u8) -> Vec<u8> {
    let mut bytes = vec![0xFF, 0xD8, 0xFF, 0xE0];
    bytes.extend(std::iter::repeat_n(semilla, 256));
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
        "descripcion": "Inmueble de prueba del test F5d",
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
            "modelo": "f5d",
            "actualizada_en": "2026-10-14T10:00:00Z"
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
        sqlx::types::Json(json!({"contacto": "f5d", "nota": "ficha de prueba"})),
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

#[tokio::test]
async fn publicar_y_traer_quedan_iguales_en_ambos_lados() {
    let admin = admin_opts();
    // Temporal del entorno donde corre el test: `C:\tmp` en el portátil, `/var/tmp`
    // en la 89 (AGENTS §1). Con `C:\tmp` en Linux sería una carpeta relativa dentro
    // del proyecto, y la réplica de Mutagen la borraría a media prueba.
    let base = if cfg!(windows) {
        PathBuf::from(r"C:\tmp")
    } else {
        PathBuf::from("/var/tmp")
    };
    let raiz = base.join(format!("sync-f5d-{}", Uuid::new_v4()));
    let dir_local = raiz.join("local");
    let dir_remoto = raiz.join("remoto");
    std::fs::create_dir_all(&dir_local).unwrap();
    std::fs::create_dir_all(&dir_remoto).unwrap();

    let pool_local = recrear_bd(&admin, "glory_sync_f5d_local").await;
    let pool_remoto = recrear_bd(&admin, "glory_sync_f5d_remoto").await;

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

    // 1. Publicar con dos fotos, receta, ficha, copy, publicado y reservado → Igual.
    let (_, slug1) = sembrar(&pool_local, &dir_local, "Casa F5d publicar").await;
    let informe = publicar(&pool_local, &dir_local, &cred, &slug1)
        .await
        .expect("publicar");
    assert!(
        informe.fallo.is_none(),
        "publicar falló: {:?}",
        informe.fallo
    );
    assert!(
        informe.hecho.iter().any(|h| h == "2 fotos"),
        "hecho: {:?}",
        informe.hecho
    );
    assert_eq!(
        estado_entre(&pool_local, &cred, &slug1).await,
        Estado::Igual,
        "tras publicar"
    );

    // 2. Traer un inmueble que solo existe en el remoto (otro slug) → Igual.
    let (_, slug2) = sembrar(&pool_remoto, &dir_remoto, "Piso F5d traer").await;
    let informe = traer(&pool_local, &dir_local, &cred, &slug2)
        .await
        .expect("traer");
    assert!(informe.fallo.is_none(), "traer falló: {:?}", informe.fallo);
    assert_eq!(
        estado_entre(&pool_local, &cred, &slug2).await,
        Estado::Igual,
        "tras traer"
    );

    // 3. Slug ocupado en el remoto: aborta antes de escribir.
    let (_, slug3) = sembrar(&pool_remoto, &dir_remoto, "Local F5d conflicto").await;
    let (_, slug3_local) = sembrar(&pool_local, &dir_local, "Local F5d conflicto").await;
    assert_eq!(slug3, slug3_local, "mismo título, mismo slug");
    let antes = conteos(&pool_remoto, &slug3).await;
    assert_eq!(antes.0, 1, "una sola fila remota con ese slug");
    let err = publicar(&pool_local, &dir_local, &cred, &slug3)
        .await
        .expect_err("slug ocupado debe abortar");
    assert!(matches!(err, AppError::Conflict(_)), "error: {err:?}");
    assert_eq!(
        conteos(&pool_remoto, &slug3).await,
        antes,
        "el servidor no cambia"
    );

    // 4. Fallo de una foto a mitad: informe del paso y ninguna fila local borrada.
    let (id4, slug4) = sembrar(&pool_local, &dir_local, "Galeria F5d fallo").await;
    let clave4: String =
        sqlx::query_scalar("SELECT storage_key FROM fotos WHERE inmueble_id = $1 AND orden = 2")
            .bind(id4)
            .fetch_one(&pool_local)
            .await
            .unwrap();
    std::fs::write(
        InmuebleService::ruta_archivo(&dir_local, &clave4),
        b"no soy una imagen",
    )
    .unwrap();
    let informe = publicar(&pool_local, &dir_local, &cred, &slug4)
        .await
        .expect("el fallo de una foto va al informe, no a Err");
    let fallo = informe.fallo.clone().expect("debe haber un fallo");
    assert!(fallo.contains("foto 2"), "fallo: {fallo}");
    assert!(
        informe.hecho.iter().any(|h| h.starts_with("Alta")),
        "hecho: {:?}",
        informe.hecho
    );
    assert!(
        informe.hecho.iter().any(|h| h == "Receta publicitaria"),
        "hecho: {:?}",
        informe.hecho
    );
    assert!(
        informe.hecho.iter().any(|h| h == "Ficha privada (/ask)"),
        "hecho: {:?}",
        informe.hecho
    );
    assert!(
        !informe
            .hecho
            .iter()
            .any(|h| h.contains("fotos") || h.contains("Publicado")),
        "no debe declarar fotos ni visibilidad: {:?}",
        informe.hecho
    );
    assert_eq!(
        conteos(&pool_local, &slug4).await,
        (1, 2),
        "filas locales intactas"
    );
    let publicado: bool = sqlx::query_scalar("SELECT publicado FROM inmuebles WHERE slug = $1")
        .bind(&slug4)
        .fetch_one(&pool_local)
        .await
        .unwrap();
    assert!(publicado, "la visibilidad local no cambia");

    // Limpieza: solo tras éxito. Las BD son desechables y se borran con su conexión.
    pool_local.close().await;
    pool_remoto.close().await;
    let mut conn = PgConnection::connect_with(&admin.clone().database("postgres"))
        .await
        .unwrap();
    for nombre in ["glory_sync_f5d_local", "glory_sync_f5d_remoto"] {
        sqlx::query(&format!("DROP DATABASE IF EXISTS {nombre} WITH (FORCE)"))
            .execute(&mut conn)
            .await
            .unwrap();
    }
    conn.close().await.unwrap();
    // Temporal propio de C:\tmp; si no se puede borrar, lo barre GloryTmpSweep.
    let _ = std::fs::remove_dir_all(&raiz);
}
