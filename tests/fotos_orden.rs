//! Test de integración 15AA-8: reordenar las fotos de un inmueble no borra
//! ninguna (ni su mejorada hermana) y rechaza listas que no sean una
//! permutación exacta de las originales. BD desechables en 127.0.0.1
//! (`glory_fotos_orden_a` / `_b`); nunca usa `PROD_BASE`.
//! Requiere `DATABASE_URL` con permiso para crear BD; sin ella falla (no se omite).
//! Ejecutar: `node scripts/run-cargo.mjs test --test fotos_orden`

use std::str::FromStr;

use glory_backend::errors::AppError;
use glory_backend::models::CreateInmuebleRequest;
use glory_backend::repositories::InmuebleRepository;
use glory_backend::services::InmuebleService;
use serde_json::json;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{Connection, PgConnection, PgPool};
use uuid::Uuid;

fn admin_opts() -> PgConnectOptions {
    let url = std::env::var("DATABASE_URL")
        .expect("fotos_orden: falta DATABASE_URL (BD de pruebas con permiso CREATE DATABASE)");
    PgConnectOptions::from_str(&url).expect("fotos_orden: DATABASE_URL no es válida")
}

/// Recrea una BD vacía y la migra. `nombre` es una constante del test (no entrada externa).
async fn recrear_bd(admin: &PgConnectOptions, nombre: &str) -> PgPool {
    let mut conn = PgConnection::connect_with(&admin.clone().database("postgres"))
        .await
        .expect("fotos_orden: no conecta a la BD postgres");
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

/// Inmueble con 3 originales (`a`,`b`,`c` en orden 0,1,2) y una mejorada
/// hermana de la del medio (mismo `orden` 1).
async fn sembrar(pool: &PgPool) -> Uuid {
    let alta: CreateInmuebleRequest = serde_json::from_value(json!({
        "titulo": "Inmueble del test de reordenado",
        "descripcion": "Prueba 15AA-8",
        "ubicacion": "Zona de prueba",
        "puestos": 1,
        "residencia": "Residencia de prueba",
        "precio": 100000.0,
        "habitaciones": 1,
        "banos": 1,
        "metros": 50.0,
        "metros_terreno": 0.0,
        "copy": {
            "corta": "Copy corta de prueba",
            "larga": "Copy larga de prueba",
            "modelo": "15aa-8",
            "actualizada_en": "2026-10-15T10:00:00Z"
        }
    }))
    .expect("alta de prueba válida");
    let inmueble = InmuebleService::create(pool, alta)
        .await
        .expect("alta de prueba");
    for (clave, orden) in [("a.jpg", 0), ("b.jpg", 1), ("c.jpg", 2)] {
        InmuebleRepository::add_foto(pool, inmueble.id, clave, Some(orden), "original")
            .await
            .expect("foto de prueba");
    }
    InmuebleRepository::add_foto(pool, inmueble.id, "b-mejorada.jpg", Some(1), "mejorada")
        .await
        .expect("mejorada de prueba");
    inmueble.id
}

/// `(clave, origen, orden)` de todas las filas, en orden estable.
async fn fotos(pool: &PgPool, inmueble_id: Uuid) -> Vec<(String, String, i32)> {
    sqlx::query_as(
        "SELECT storage_key, origen, orden FROM fotos WHERE inmueble_id = $1 \
         ORDER BY origen, storage_key",
    )
    .bind(inmueble_id)
    .fetch_all(pool)
    .await
    .unwrap()
}

async fn ids_originales(pool: &PgPool, inmueble_id: Uuid) -> Vec<Uuid> {
    sqlx::query_scalar(
        "SELECT id FROM fotos WHERE inmueble_id = $1 AND origen = 'original' ORDER BY orden",
    )
    .bind(inmueble_id)
    .fetch_all(pool)
    .await
    .unwrap()
}

async fn updated_epoch(pool: &PgPool, inmueble_id: Uuid) -> f64 {
    sqlx::query_scalar("SELECT EXTRACT(EPOCH FROM updated_at)::float8 FROM inmuebles WHERE id = $1")
        .bind(inmueble_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn reordenar_mueve_los_huecos_y_no_borra_nada() {
    let admin = admin_opts();
    let pool = recrear_bd(&admin, "glory_fotos_orden_a").await;
    let id = sembrar(&pool).await;
    let antes = fotos(&pool, id).await;
    assert_eq!(antes.len(), 4, "3 originales + 1 mejorada");
    let updated_antes = updated_epoch(&pool, id).await;

    let ids = ids_originales(&pool, id).await;
    let nuevo = vec![ids[2], ids[0], ids[1]]; // a, b, c → c, a, b
    InmuebleService::reordenar_fotos(&pool, id, &nuevo)
        .await
        .expect("reordenado válido");

    let despues = fotos(&pool, id).await;
    assert_eq!(despues.len(), 4, "reordenar no borra filas");
    let orden = |clave: &str| {
        despues
            .iter()
            .find(|(c, _, _)| c == clave)
            .map_or(-1, |(_, _, o)| *o)
    };
    assert_eq!((orden("c.jpg"), orden("a.jpg"), orden("b.jpg")), (0, 1, 2));
    assert_eq!(
        orden("b-mejorada.jpg"),
        2,
        "la mejorada sigue a su hermana (b va a 2)"
    );
    assert!(
        updated_epoch(&pool, id).await > updated_antes,
        "el reordenado invalida la caché de fotos (?v=)"
    );

    /* Intercambio puro 0↔1 (a y c): un UPDATE fila a fila se pisaría. */
    InmuebleService::reordenar_fotos(&pool, id, &[ids[0], ids[2], ids[1]])
        .await
        .expect("intercambio válido");
    let tras_intercambio = fotos(&pool, id).await;
    let orden2 = |clave: &str| {
        tras_intercambio
            .iter()
            .find(|(c, _, _)| c == clave)
            .map_or(-1, |(_, _, o)| *o)
    };
    assert_eq!(
        (orden2("a.jpg"), orden2("c.jpg"), orden2("b.jpg")),
        (0, 1, 2)
    );
    assert_eq!(orden2("b-mejorada.jpg"), 2, "su hermana viajó con ella");
    assert_eq!(tras_intercambio.len(), 4);

    borrar_bd(&admin, "glory_fotos_orden_a").await;
}

#[tokio::test]
async fn lista_que_no_es_permutacion_no_toca_nada() {
    let admin = admin_opts();
    let pool = recrear_bd(&admin, "glory_fotos_orden_b").await;
    let id = sembrar(&pool).await;
    let antes = fotos(&pool, id).await;
    let ids = ids_originales(&pool, id).await;

    let casos: Vec<(&str, Vec<Uuid>)> = vec![
        ("falta una", vec![ids[0], ids[1]]),
        ("repetida", vec![ids[0], ids[0], ids[1]]),
        ("ajena", vec![ids[0], ids[1], Uuid::new_v4()]),
    ];
    for (caso, lista) in casos {
        let error = InmuebleService::reordenar_fotos(&pool, id, &lista)
            .await
            .expect_err(caso);
        assert!(
            matches!(error, AppError::Validation(_)),
            "{caso}: se esperaba validación, llegó {error:?}"
        );
        assert_eq!(fotos(&pool, id).await, antes, "{caso}: no debe tocar nada");
    }

    borrar_bd(&admin, "glory_fotos_orden_b").await;
}
