#![cfg(test)]
//! Caché de borradores: hit, miss y purga.

use super::*;

#[tokio::test]
async fn cache_guarda_hit_y_cuenta_usos() {
    let Some(pool) = pool_si_hay() else { return };
    let (firma, ph, ch) = (clave_azar(), clave_azar(), clave_azar());
    guardar_cache(
        &pool,
        &firma,
        &ph,
        &ch,
        "texto-ia",
        &foto_prueba("hilo-1", "Dueña: hola", "CRUDO Dueña: hola"),
        Coste::default(),
    )
    .await
    .expect("guarda");
    let hit = buscar_cache(&pool, &firma, &ph, &ch).await.expect("busca");
    assert_eq!(hit, Some(("texto-ia".to_string(), false)));
    /* [07AA-7] El panel agrupa por chat: hilo + foto guardados.
     * [08AA-21] El crudo viaja junto al limpio. */
    let hilo: (String, String, Option<String>) = sqlx::query_as(
        "SELECT thread_id, excerpt_texto, excerpt_crudo FROM mp_respuestas_cache \
         WHERE firma = $1 AND precio_hash = $2 AND catalog_hash = $3",
    )
    .bind(&firma)
    .bind(&ph)
    .bind(&ch)
    .fetch_one(&pool)
    .await
    .expect("lee hilo");
    assert_eq!(
        hilo,
        (
            "hilo-1".to_string(),
            "Dueña: hola".to_string(),
            Some("CRUDO Dueña: hola".to_string())
        )
    );
    buscar_cache(&pool, &firma, &ph, &ch)
        .await
        .expect("busca x2");
    let usos: i64 = sqlx::query_scalar(
        "SELECT usos::BIGINT FROM mp_respuestas_cache WHERE firma = $1 AND precio_hash = $2 AND catalog_hash = $3",
    )
    .bind(&firma).bind(&ph).bind(&ch)
    .fetch_one(&pool).await.expect("lee usos");
    assert_eq!(usos, 2);
    borrar_cache(&pool, &firma, &ph, &ch).await.expect("limpia");
}

#[tokio::test]
async fn cache_miss_si_cambia_precio_o_catalogo() {
    let Some(pool) = pool_si_hay() else { return };
    let (firma, ph, ch) = (clave_azar(), clave_azar(), clave_azar());
    guardar_cache(
        &pool,
        &firma,
        &ph,
        &ch,
        "texto-ia",
        &foto_prueba("hilo-1", "x", "crudo-x"),
        Coste::default(),
    )
    .await
    .expect("guarda");
    assert!(buscar_cache(&pool, &firma, &clave_azar(), &ch)
        .await
        .expect("busca")
        .is_none());
    assert!(buscar_cache(&pool, &firma, &ph, &clave_azar())
        .await
        .expect("busca")
        .is_none());
    borrar_cache(&pool, &firma, &ph, &ch).await.expect("limpia");
}

#[tokio::test]
async fn cache_vencida_no_devuelve_y_purga_limpia() {
    let Some(pool) = pool_si_hay() else { return };
    let (firma, ph, ch) = (clave_azar(), clave_azar(), clave_azar());
    guardar_cache(
        &pool,
        &firma,
        &ph,
        &ch,
        "viejo",
        &foto_prueba("hilo-1", "x", "crudo-x"),
        Coste::default(),
    )
    .await
    .expect("guarda");
    sqlx::query(
        "UPDATE mp_respuestas_cache SET valida_hasta = now() - INTERVAL '1 day' \
         WHERE firma = $1 AND precio_hash = $2 AND catalog_hash = $3",
    )
    .bind(&firma)
    .bind(&ph)
    .bind(&ch)
    .execute(&pool)
    .await
    .expect("envejece");
    assert!(buscar_cache(&pool, &firma, &ph, &ch)
        .await
        .expect("busca")
        .is_none());
    let n = purgar_cache(&pool).await.expect("purga");
    assert!(n >= 1, "purga={n}");
    let queda: i64 =
        sqlx::query_scalar("SELECT COUNT(*)::BIGINT FROM mp_respuestas_cache WHERE firma = $1")
            .bind(&firma)
            .fetch_one(&pool)
            .await
            .expect("cuenta");
    assert_eq!(queda, 0);
}
