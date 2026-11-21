//! Test de integración 30AA-1: Facebook como entidad, versiones de texto,
//! variantes de foto y el manifiesto de exportación que consume el escritorio.
//!
//! Cubre las tres invariantes que sostienen la función:
//!  1. `inmuebles.marketplace_id` es espejo del aviso de la cuenta principal
//!     (la versión más baja desempata dentro de ella), y el borrador encuentra
//!     la ficha aunque el aviso lo guarde otra cuenta.
//!  2. Editar o regenerar una variante deja entrada en el historial.
//!  3. El manifiesto separa v1 (con la mejorada en lugar de su original) y v2
//!     (solo las variantes IA), que es lo que separa las carpetas en disco.
//!
//! BD desechables en 127.0.0.1 (`glory_versiones_a/b`); nunca `PROD_BASE`.
//! Ejecutar: `node scripts/run-with-db.mjs test --test versiones_facebook`

use std::str::FromStr;

use glory_backend::models::{CreateInmuebleRequest, EditarVersionRequest, FacebookRequest};
use glory_backend::repositories::{InmuebleRepository, VersionRepository};
use glory_backend::services::{ExportService, FacebookService, InmuebleService, VersionService};
use serde_json::json;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{Connection, PgConnection, PgPool};
use uuid::Uuid;

fn admin_opts() -> PgConnectOptions {
    let url = std::env::var("DATABASE_URL")
        .expect("versiones_facebook: falta DATABASE_URL (BD de pruebas con permiso CREATE DATABASE)");
    PgConnectOptions::from_str(&url).expect("versiones_facebook: DATABASE_URL no es válida")
}

/// Recrea una BD vacía y la migra. `nombre` es constante del test (no entrada externa).
async fn recrear_bd(admin: &PgConnectOptions, nombre: &str) -> PgPool {
    /* Las migraciones se empotran al compilar (sqlx::migrate!): borrar la BD no
     * basta si el binario es de antes de una migración nueva, y el test falla
     * con un «ColumnNotFound» que engaña. Se avisa claro. */
    let migraciones = sqlx::migrate!("./migrations");
    assert!(
        migraciones.iter().any(|m| m.version == 20_261_117_000_054),
        "binario de pruebas viejo: recompila con la migración 20261117000054"
    );
    let mut conn = PgConnection::connect_with(&admin.clone().database("postgres"))
        .await
        .expect("versiones_facebook: no conecta a la BD postgres");
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

async fn borrar_bd(admin: &PgConnectOptions, nombre: &str) {
    let mut conn = PgConnection::connect_with(&admin.clone().database("postgres"))
        .await
        .unwrap();
    sqlx::query(&format!("DROP DATABASE IF EXISTS {nombre} WITH (FORCE)"))
        .execute(&mut conn)
        .await
        .unwrap();
    conn.close().await.unwrap();
}

/// Inmueble con dos originales y una mejorada hermana de la primera.
async fn sembrar(pool: &PgPool) -> Uuid {
    let alta: CreateInmuebleRequest = serde_json::from_value(json!({
        "titulo": "Apartamento de 3 habitaciones en Residencias Kama",
        "descripcion": "90 m² con dos baños y un puesto de estacionamiento.",
        "ubicacion": "Zona de prueba",
        "precio": 250_000.0,
        "habitaciones": 3,
        "banos": 2,
        "metros": 90.0,
        "metros_terreno": 0.0,
        "puestos": 1,
    }))
    .expect("alta de prueba válida");
    let inmueble = InmuebleService::create(pool, alta)
        .await
        .expect("alta de prueba");
    for (clave, orden) in [("sala.jpg", 0), ("cocina.jpg", 1)] {
        InmuebleRepository::add_foto(pool, inmueble.id, clave, Some(orden), "original")
            .await
            .expect("foto de prueba");
    }
    InmuebleRepository::add_foto(pool, inmueble.id, "sala-mejorada.jpg", Some(0), "mejorada")
        .await
        .expect("mejorada de prueba");
    inmueble.id
}

fn cuenta(nombre: &str, principal: bool) -> FacebookRequest {
    FacebookRequest {
        nombre: nombre.to_string(),
        url: String::new(),
        notas: String::new(),
        principal,
        version: None,
    }
}

fn cuenta_en(nombre: &str, principal: bool, version: i16) -> FacebookRequest {
    FacebookRequest {
        version: Some(version),
        ..cuenta(nombre, principal)
    }
}

#[tokio::test]
async fn el_aviso_espeja_la_cuenta_principal() {
    let admin = admin_opts();
    let pool = recrear_bd(&admin, "glory_versiones_a").await;
    let id = sembrar(&pool).await;

    /* [GW-14] Caso real del 2026-11-21: ninguna cuenta es v1 (Wan v2 y
     * Maye-v0 v0), y con la regla vieja (`version = 1 AND principal`) el
     * espejo se quedaba vacío en TODOS los inmuebles. */
    let wan = FacebookService::crear(&pool, cuenta_en("Wan", true, 2))
        .await
        .expect("cuenta principal v2");
    let maye_v0 = FacebookService::crear(&pool, cuenta_en("Maye-v0", false, 0))
        .await
        .expect("cuenta v0");

    FacebookService::vincular(&pool, id, wan.id, 1, Some("1111111111"))
        .await
        .expect("aviso en la principal");
    let espejo = espejo_de(&pool, id).await;
    assert_eq!(
        espejo.as_deref(),
        Some("1111111111"),
        "sin cuenta v1 el espejo igual se llena: si no, el borrador va sin ficha"
    );

    /* [GW-17] La v0 de una cuenta secundaria NO pisa el aviso de la
     * principal. Caso real: Florida Country y Arivana espejaban el aviso de
     * `Maye-v0` y el panel de sync ofrecía reemplazar en producción el bueno. */
    FacebookService::vincular(&pool, id, maye_v0.id, 1, Some("2222222222"))
        .await
        .expect("aviso en la v0");
    assert_eq!(
        espejo_de(&pool, id).await.as_deref(),
        Some("1111111111"),
        "manda la cuenta principal, no la versión más baja"
    );

    /* Al desvincular la principal toma el relevo la otra cuenta: el espejo no
     * queda colgado mientras haya algún vínculo. */
    FacebookService::desvincular(&pool, id, wan.id, 1)
        .await
        .expect("desvincular la principal");
    assert_eq!(espejo_de(&pool, id).await.as_deref(), Some("2222222222"));

    /* Sin ningún vínculo, espejo NULL. */
    FacebookService::desvincular(&pool, id, maye_v0.id, 1)
        .await
        .expect("desvincular la v0");
    assert_eq!(espejo_de(&pool, id).await, None);

    borrar_bd(&admin, "glory_versiones_a").await;
}

#[tokio::test]
async fn el_espejo_desempata_por_la_principal() {
    let admin = admin_opts();
    let pool = recrear_bd(&admin, "glory_versiones_e").await;
    let id = sembrar(&pool).await;

    /* Dos cuentas sin versión declarada: mismas condiciones, gana la principal. */
    let principal = FacebookService::crear(&pool, cuenta("Principal", true))
        .await
        .expect("principal");
    let otra = FacebookService::crear(&pool, cuenta("Otra", false))
        .await
        .expect("segunda");

    FacebookService::vincular(&pool, id, otra.id, 1, Some("3333333333"))
        .await
        .expect("aviso en la no principal");
    FacebookService::vincular(&pool, id, principal.id, 1, Some("4444444444"))
        .await
        .expect("aviso en la principal");
    assert_eq!(espejo_de(&pool, id).await.as_deref(), Some("4444444444"));

    /* Desvincular la principal deja a la otra como representante. */
    FacebookService::desvincular(&pool, id, principal.id, 1)
        .await
        .expect("desvincular la principal");
    assert_eq!(espejo_de(&pool, id).await.as_deref(), Some("3333333333"));

    borrar_bd(&admin, "glory_versiones_e").await;
}

#[tokio::test]
async fn el_aviso_de_otra_cuenta_resuelve_la_ficha() {
    let admin = admin_opts();
    let pool = recrear_bd(&admin, "glory_versiones_f").await;
    let id = sembrar(&pool).await;

    let principal = FacebookService::crear(&pool, cuenta_en("Principal", true, 2))
        .await
        .expect("principal v2");
    let v0 = FacebookService::crear(&pool, cuenta_en("Maye-v0", false, 0))
        .await
        .expect("v0");

    /* El espejo guarda UN aviso; el otro vive solo en `inmueble_facebook`.
     * [GW-14] El respaldo tiene que encontrarlo: es el mismo ID exacto.
     * [GW-17] El espejo es el de la principal; el de la v0 necesita respaldo. */
    FacebookService::vincular(&pool, id, principal.id, 1, Some("5555555555"))
        .await
        .expect("aviso de la principal");
    FacebookService::vincular(&pool, id, v0.id, 1, Some("6666666666"))
        .await
        .expect("aviso de la v0");
    assert_eq!(espejo_de(&pool, id).await.as_deref(), Some("5555555555"));

    let por_espejo = InmuebleRepository::find_by_marketplace_id(&pool, "6666666666")
        .await
        .unwrap();
    assert!(por_espejo.is_none(), "el espejo solo guarda el de la principal");

    let por_vinculo = InmuebleRepository::find_by_aviso_vinculado(&pool, "6666666666")
        .await
        .unwrap();
    assert_eq!(
        por_vinculo.map(|f| f.id),
        Some(id),
        "el respaldo por `inmueble_facebook` sí encuentra la ficha"
    );

    /* Un ID que no existe en ningún sitio sigue sin ficha (nunca por título). */
    assert!(
        InmuebleRepository::find_by_aviso_vinculado(&pool, "9999999999")
            .await
            .unwrap()
            .is_none()
    );

    borrar_bd(&admin, "glory_versiones_f").await;
}

/// Espejo `inmuebles.marketplace_id` de un inmueble.
async fn espejo_de(pool: &PgPool, id: Uuid) -> Option<String> {
    sqlx::query_scalar("SELECT marketplace_id FROM inmuebles WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn editar_una_variante_deja_historial() {
    let admin = admin_opts();
    let pool = recrear_bd(&admin, "glory_versiones_b").await;
    let id = sembrar(&pool).await;

    /* [30AA-16] La configuración sube a 5 mientras dura el lote, así que
     * fijar el valor exacto hacía fallar el test por un cambio de datos. Lo
     * que importa es que esté dentro de rango. */
    let configuradas = VersionRepository::configuradas(&pool).await.unwrap();
    assert!(
        (1..=5).contains(&configuradas),
        "versiones configuradas fuera de rango: {configuradas}"
    );

    VersionRepository::upsert(
        &pool,
        id,
        2,
        "Apartamento con 3 habitaciones en Residencias Kama",
        "Dos baños, 90 m² y un puesto de estacionamiento.",
        "prueba",
        "prompt de prueba",
    )
    .await
    .expect("variante de prueba");

    /* Editar los dos campos a mano: dos entradas, origen `manual`. */
    let req = EditarVersionRequest {
        titulo: Some("En Residencias Kama, apartamento de 3 habitaciones".into()),
        descripcion: Some("Un puesto de estacionamiento, 90 m² y dos baños.".into()),
        estado: Some("lista".into()),
    };
    let guardada = VersionService::editar(&pool, id, 2, &req)
        .await
        .expect("edición válida");
    assert_eq!(guardada.estado, "lista");

    let historial = VersionRepository::historial(&pool, id).await.unwrap();
    assert_eq!(historial.len(), 2, "un renglón por campo cambiado");
    assert!(historial.iter().all(|h| h.origen == "manual"));
    assert!(historial.iter().any(|h| h.campo == "titulo"));

    /* Sin cambio real no se ensucia el historial. */
    VersionService::editar(&pool, id, 2, &req).await.expect("sin cambios");
    assert_eq!(
        VersionRepository::historial(&pool, id).await.unwrap().len(),
        2,
        "repetir el mismo texto no anota nada"
    );

    borrar_bd(&admin, "glory_versiones_b").await;
}

#[tokio::test]
async fn el_manifiesto_separa_v1_y_v2() {
    let admin = admin_opts();
    let pool = recrear_bd(&admin, "glory_versiones_c").await;
    let id = sembrar(&pool).await;
    /* [30AA-13] Cada versión tiene UN dueño: v1 = Maye, v2 = Wan. Maye ya la
     * siembra la migración de Facebook, con la v1 asignada. */
    let maye = FacebookService::listar(&pool)
        .await
        .expect("cuentas")
        .into_iter()
        .find(|c| c.principal)
        .expect("Maye sembrada");
    assert_eq!(maye.version, Some(1), "la principal nace con la v1");
    let wan = FacebookService::crear(&pool, cuenta_en("Wan", false, 2))
        .await
        .expect("cuenta de la v2");
    FacebookService::vincular(&pool, id, maye.id, 1, None)
        .await
        .expect("vínculo");

    /* v2: una variante de foto (orden 0) y su texto. */
    VersionRepository::upsert_foto(
        &pool,
        id,
        0,
        2,
        "variantes/sala-v2.jpg",
        "prueba",
        "prompt",
    )
    .await
    .expect("variante de foto");
    VersionRepository::upsert(
        &pool,
        id,
        2,
        "Apartamento con 3 habitaciones en Residencias Kama",
        "Dos baños, 90 m² y un puesto de estacionamiento.",
        "prueba",
        "prompt",
    )
    .await
    .expect("variante de texto");

    let m = ExportService::manifiesto(&pool, None, None)
        .await
        .expect("manifiesto");
    assert_eq!(m.cuenta, maye.id);
    assert_eq!(m.grupos.len(), 2, "v1 y v2 (todavía sin texto de v0)");

    let v1 = &m.grupos[0];
    assert_eq!(v1.version, 1);
    let inmueble = &v1.inmuebles[0];
    assert_eq!(inmueble.fotos.len(), 2, "la mejorada sustituye a su original");
    assert!(inmueble.fotos.iter().any(|f| f.origen == "mejorada"));
    assert!(inmueble.fotos.windows(2).all(|p| p[0].orden <= p[1].orden));
    /* [30AA-12] Vinculado sin aviso (ID vacío) todavía NO está publicado:
     * el aviso no existe en Marketplace hasta que tiene número. */
    assert!(!inmueble.publicado, "vínculo sin ID = pendiente");

    let v2 = &m.grupos[1];
    assert_eq!(v2.version, 2);
    let variante = &v2.inmuebles[0];
    assert_eq!(variante.fotos.len(), 1);
    assert_eq!(variante.fotos[0].origen, "variante");
    assert!(variante.fotos[0].url.starts_with("/uploads/"));
    assert!(!variante.publicado);

    /* [30AA-13] La v2 se mide en SU cuenta (Wan), no en Maye: un aviso de la
     * v2 en Maye no publica nada, y el mismo aviso en Wan sí. */
    FacebookService::vincular(&pool, id, maye.id, 2, Some("999888777"))
        .await
        .expect("aviso de la v2 en la cuenta equivocada");
    let m = ExportService::manifiesto(&pool, None, None)
        .await
        .expect("manifiesto con aviso ajeno");
    assert!(!m.grupos[0].inmuebles[0].publicado, "la v1 sigue pendiente");
    assert!(!m.grupos[1].inmuebles[0].publicado, "la v2 apuntada en Maye no cuenta");

    FacebookService::vincular(&pool, id, wan.id, 2, Some("999888777"))
        .await
        .expect("aviso de la v2 en su cuenta");
    let m = ExportService::manifiesto(&pool, None, None)
        .await
        .expect("manifiesto con aviso");
    assert!(!m.grupos[0].inmuebles[0].publicado, "la v1 sigue pendiente");
    assert!(m.grupos[1].inmuebles[0].publicado, "la v2 ya está publicada en Wan");

    /* Pedir solo v2 no devuelve el grupo de v1. */
    let solo_v2 = ExportService::manifiesto(&pool, None, Some(2))
        .await
        .expect("manifiesto v2");
    assert_eq!(solo_v2.grupos.len(), 1);

    /* [30AA-18] La v0: texto propio y las fotos ORIGINALES, sin las mejoradas
     * (aquí no vale `preferir_mejoradas`). No exige cuenta dueña y su aviso
     * cuenta en cualquier cuenta, porque no tiene una asignada. */
    VersionRepository::upsert(
        &pool,
        id,
        0,
        "En Residencias Kama, apartamento de 3 habitaciones",
        "Dos baños, 90 m² y un puesto de estacionamiento.",
        "prueba",
        "prompt",
    )
    .await
    .expect("texto de la v0");
    let m = ExportService::manifiesto(&pool, None, None)
        .await
        .expect("manifiesto con v0");
    assert_eq!(m.grupos.len(), 3, "v0, v1 y v2");
    let v0 = &m.grupos[0];
    assert_eq!(v0.version, 0, "la v0 va la primera");
    assert_eq!(v0.inmuebles[0].titulo, "En Residencias Kama, apartamento de 3 habitaciones");
    assert_eq!(
        v0.inmuebles[0].fotos.len(),
        2,
        "las originales: las dos fotos, ninguna mejorada"
    );
    assert!(v0.inmuebles[0].fotos.iter().all(|f| f.origen == "original"));
    assert!(!v0.inmuebles[0].publicado, "nadie la ha publicado");

    FacebookService::vincular(&pool, id, wan.id, 0, Some("555444333"))
        .await
        .expect("aviso de la v0 en una cuenta cualquiera");
    let m = ExportService::manifiesto(&pool, None, Some(0))
        .await
        .expect("manifiesto solo v0");
    assert_eq!(m.grupos.len(), 1);
    assert!(m.grupos[0].inmuebles[0].publicado, "el aviso de v0 cuenta en Maye o en Wan");
    assert!(
        !FacebookService::faltan_avisos(&pool, id, 5)
            .await
            .unwrap()
            .versiones
            .contains(&0),
        "con el aviso puesto, la v0 ya no se echa en falta"
    );

    /* [30AA-15] Un inmueble SIN ninguna cuenta no desaparece: sale en la v1 de
     * la principal como pendiente. Antes lo filtraba `por_facebook` y el
     * escritorio no tenía dónde ponerlo (`version 1/pendientes` vacía con
     * cuatro fichas sin aviso). */
    sembrar(&pool).await;
    let m = ExportService::manifiesto(&pool, None, Some(1))
        .await
        .expect("manifiesto con inmueble sin cuenta");
    assert_eq!(
        m.grupos[0].inmuebles.len(),
        2,
        "el inmueble sin cuenta no desaparece del manifiesto"
    );
    assert!(
        m.grupos[0].inmuebles.iter().any(|i| !i.publicado),
        "sale como pendiente: no tiene aviso"
    );

    borrar_bd(&admin, "glory_versiones_c").await;
}

#[tokio::test]
async fn las_fotos_variante_cuentan_como_version_propia() {
    let admin = admin_opts();
    let pool = recrear_bd(&admin, "glory_versiones_d").await;
    let id = sembrar(&pool).await;
    let maye = FacebookService::listar(&pool)
        .await
        .expect("cuentas")
        .into_iter()
        .find(|c| c.principal)
        .expect("Maye sembrada");

    /* [30AA-17] Con la v1 vinculada y SIN texto de v2, unas fotos variante ya
     * hacen que la ficha deba un aviso de v2. Antes el badge decía «al día»:
     * solo miraba `inmueble_versiones`, y el lote de variantes no escribe
     * texto. */
    VersionRepository::upsert_foto(&pool, id, 0, 2, "variantes/sala-v2.jpg", "prueba", "prompt")
        .await
        .expect("variante de foto");
    assert_eq!(
        FacebookService::faltan_avisos(&pool, id, 5).await.unwrap().versiones,
        vec![1, 2],
        "sin vínculos faltan la v1 y la v2 de fotos"
    );

    FacebookService::vincular(&pool, id, maye.id, 1, None)
        .await
        .expect("vínculo de la v1");
    assert_eq!(
        FacebookService::faltan_avisos(&pool, id, 5).await.unwrap().versiones,
        vec![2],
        "la v2 sale pendiente aunque no tenga texto"
    );

    /* Una v3 de fotos sin vínculo también se echa en falta. */
    VersionRepository::upsert_foto(&pool, id, 1, 3, "variantes/cocina-v3.jpg", "prueba", "prompt")
        .await
        .expect("variante v3");
    assert_eq!(
        FacebookService::faltan_avisos(&pool, id, 5).await.unwrap().versiones,
        vec![2, 3]
    );

    borrar_bd(&admin, "glory_versiones_d").await;
}
