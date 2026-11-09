//! Test de integración 30AA-1: Facebook como entidad, versiones de texto,
//! variantes de foto y el manifiesto de exportación que consume el escritorio.
//!
//! Cubre las tres invariantes que sostienen la función:
//!  1. `inmuebles.marketplace_id` es espejo del aviso de la cuenta principal.
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
        "precio": 250000.0,
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
    }
}

#[tokio::test]
async fn el_aviso_espeja_la_cuenta_principal() {
    let admin = admin_opts();
    let pool = recrear_bd(&admin, "glory_versiones_a").await;
    let id = sembrar(&pool).await;

    let maye = FacebookService::crear(&pool, cuenta("Maye", true))
        .await
        .expect("cuenta principal");
    let nueva = FacebookService::crear(&pool, cuenta("Nueva", false))
        .await
        .expect("segunda cuenta");

    /* Vincular la NO principal no toca el espejo; la principal sí. */
    FacebookService::vincular(&pool, id, nueva.id, None)
        .await
        .expect("vínculo con la nueva");
    let espejo: Option<String> =
        sqlx::query_scalar("SELECT marketplace_id FROM inmuebles WHERE id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(espejo, None, "la cuenta no principal no escribe el espejo");

    FacebookService::vincular(&pool, id, maye.id, Some("1234567890"))
        .await
        .expect("vínculo con la principal");
    let espejo: Option<String> =
        sqlx::query_scalar("SELECT marketplace_id FROM inmuebles WHERE id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(espejo.as_deref(), Some("1234567890"));

    let vinculos = FacebookService::vinculos(&pool, id).await.unwrap();
    assert_eq!(vinculos.len(), 2, "el mismo inmueble va en dos cuentas");
    assert!(vinculos.iter().any(|v| v.principal && v.marketplace_id.is_some()));

    /* Desvincular la principal deja el espejo vacío: no queda un aviso colgado. */
    FacebookService::desvincular(&pool, id, maye.id)
        .await
        .expect("desvincular");
    let espejo: Option<String> =
        sqlx::query_scalar("SELECT marketplace_id FROM inmuebles WHERE id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(espejo, None);

    borrar_bd(&admin, "glory_versiones_a").await;
}

#[tokio::test]
async fn editar_una_variante_deja_historial() {
    let admin = admin_opts();
    let pool = recrear_bd(&admin, "glory_versiones_b").await;
    let id = sembrar(&pool).await;

    assert_eq!(
        VersionRepository::configuradas(&pool).await.unwrap(),
        2,
        "la migración siembra 2 versiones"
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
    let maye = FacebookService::crear(&pool, cuenta("Maye", true))
        .await
        .expect("cuenta principal");
    FacebookService::vincular(&pool, id, maye.id, None)
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
    assert_eq!(m.grupos.len(), 2, "v1 y v2");

    let v1 = &m.grupos[0];
    assert_eq!(v1.version, 1);
    let inmueble = &v1.inmuebles[0];
    assert_eq!(inmueble.fotos.len(), 2, "la mejorada sustituye a su original");
    assert!(inmueble.fotos.iter().any(|f| f.origen == "mejorada"));
    assert!(inmueble.fotos.windows(2).all(|p| p[0].orden <= p[1].orden));

    let v2 = &m.grupos[1];
    assert_eq!(v2.version, 2);
    let variante = &v2.inmuebles[0];
    assert_eq!(variante.fotos.len(), 1);
    assert_eq!(variante.fotos[0].origen, "variante");
    assert!(variante.fotos[0].url.starts_with("/uploads/"));

    /* Pedir solo v2 no devuelve el grupo de v1. */
    let solo_v2 = ExportService::manifiesto(&pool, None, Some(2))
        .await
        .expect("manifiesto v2");
    assert_eq!(solo_v2.grupos.len(), 1);

    borrar_bd(&admin, "glory_versiones_c").await;
}
