#![cfg(test)]
//! Pruebas de la cola del calendario, sobre la BD de pruebas en transacción.

use sqlx::postgres::PgPoolOptions;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use super::PublicacionRepository;

/// Pool sobre la BD de pruebas; `None` si no hay una (o si apunta a la de desarrollo).
async fn pool_pruebas() -> Option<PgPool> {
    let url = crate::config::url_bd_pruebas()?;
    PgPoolOptions::new().max_connections(1).connect(&url).await.ok()
}

/// Inmueble publicado y disponible, con un slug propio para no chocar con los demás
async fn inmueble(conn: &mut PgConnection, titulo: &str) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO inmuebles (titulo, slug, publicado) VALUES ($1, $2, TRUE) RETURNING id",
    )
    .bind(titulo)
    .bind(format!("prueba-{}", Uuid::new_v4()))
    .fetch_one(conn)
    .await
    .expect("inmueble de prueba")
}

/// Publicación que salió hace `dias` días
async fn publicada_hace(conn: &mut PgConnection, inmueble_id: Uuid, dias: i32) {
    sqlx::query(
        "INSERT INTO publicaciones_redes (inmueble_id, estado, publicada_en)
         VALUES ($1, 'publicada', now() - make_interval(days => $2))",
    )
    .bind(inmueble_id)
    .bind(dias)
    .execute(conn)
    .await
    .expect("publicación de prueba");
}

/// Un salto no cuenta como publicación: el inmueble sigue sin salir
async fn saltado(conn: &mut PgConnection, inmueble_id: Uuid) {
    sqlx::query("INSERT INTO publicaciones_redes (inmueble_id, estado) VALUES ($1, 'saltada')")
        .bind(inmueble_id)
        .execute(conn)
        .await
        .expect("salto de prueba");
}

/// Cada prueba corre en una transacción que se deshace al final. Antes apaga
/// los inmuebles ya existentes para que solo cuenten los de la prueba.
async fn aislada(pool: &PgPool) -> sqlx::Transaction<'_, sqlx::Postgres> {
    let mut tx = pool.begin().await.expect("transacción");
    sqlx::query("UPDATE inmuebles SET publicado = FALSE")
        .execute(&mut *tx)
        .await
        .expect("aislar la BD de pruebas");
    tx
}

/* [28AA-2] Regla de la cola: los que nunca salieron van primero; si todos
 * salieron, vuelve el de publicación más antigua; `excluir` no vuelve. */
#[tokio::test]
async fn cola_vuelve_al_mas_viejo_cuando_todos_estan_publicados() {
    let Some(pool) = pool_pruebas().await else { return };
    let mut tx = aislada(&pool).await;
    let viejo = inmueble(&mut tx, "viejo").await;
    let reciente = inmueble(&mut tx, "reciente").await;
    publicada_hace(&mut tx, viejo, 10).await;
    publicada_hace(&mut tx, reciente, 2).await;

    let elegido = PublicacionRepository::siguiente_inmueble(&mut *tx, None).await.unwrap();
    assert_eq!(elegido, Some(viejo), "todos publicados: vuelve el más antiguo");

    let sin_viejo = PublicacionRepository::siguiente_inmueble(&mut *tx, Some(viejo)).await.unwrap();
    assert_eq!(sin_viejo, Some(reciente), "excluir evita el que se acaba de saltar");
    tx.rollback().await.unwrap();
}

/* [28AA-5] El historial lista cada vez que el inmueble salió publicado, más
 * reciente primero; los borradores y los saltos no entran. */
#[tokio::test]
async fn historial_guarda_cada_salida_y_no_los_borradores() {
    let Some(pool) = pool_pruebas().await else { return };
    let mut tx = aislada(&pool).await;
    let id = inmueble(&mut tx, "historial").await;
    publicada_hace(&mut tx, id, 20).await;
    publicada_hace(&mut tx, id, 5).await;
    sqlx::query("INSERT INTO publicaciones_redes (inmueble_id) VALUES ($1)")
        .bind(id)
        .execute(&mut *tx)
        .await
        .expect("borrador de prueba");
    saltado(&mut tx, id).await;

    let historial = PublicacionRepository::historial_de(&mut *tx, id).await.unwrap();
    let fechas: Vec<_> = historial.iter().filter_map(|p| p.publicada_en).collect();
    assert_eq!(historial.len(), 2, "solo las dos salidas publicadas");
    assert!(fechas[0] > fechas[1], "la más reciente primero");
    tx.rollback().await.unwrap();
}

/* [28AA-2] Un inmueble nunca publicado gana a cualquiera ya publicado, y
 * uno con solo un «saltado» sigue contando como nunca publicado. */
#[tokio::test]
async fn nunca_publicado_gana_y_el_salto_no_cuenta() {
    let Some(pool) = pool_pruebas().await else { return };
    let mut tx = aislada(&pool).await;
    let publicado = inmueble(&mut tx, "publicado").await;
    let solo_saltado = inmueble(&mut tx, "solo saltado").await;
    publicada_hace(&mut tx, publicado, 30).await;
    saltado(&mut tx, solo_saltado).await;

    let elegido = PublicacionRepository::siguiente_inmueble(&mut *tx, None).await.unwrap();
    assert_eq!(elegido, Some(solo_saltado), "el salto no es publicación: sigue primero");
    tx.rollback().await.unwrap();
}
