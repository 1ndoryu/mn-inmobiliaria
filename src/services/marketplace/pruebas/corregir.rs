#![cfg(test)]
//! Corrección manual de borradores.

use super::*;

#[tokio::test]
async fn corregir_marca_y_guardar_no_pisa_correccion() {
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
    corregir_cache(&pool, &firma, &ph, &ch, "texto de la dueña")
        .await
        .expect("corrige");
    assert_eq!(
        buscar_cache(&pool, &firma, &ph, &ch).await.expect("busca"),
        Some(("texto de la dueña".to_string(), true))
    );
    /* Generación posterior no pisa la corrección (DO NOTHING). */
    guardar_cache(
        &pool,
        &firma,
        &ph,
        &ch,
        "texto-ia-2",
        &foto_prueba("hilo-1", "x", "crudo-x"),
        Coste::default(),
    )
    .await
    .expect("guarda x2");
    assert_eq!(
        buscar_cache(&pool, &firma, &ph, &ch).await.expect("busca"),
        Some(("texto de la dueña".to_string(), true))
    );
    /* Regenerar explícito sí pisa y resetea versión. */
    reemplazar_cache(
        &pool,
        &firma,
        &ph,
        &ch,
        "nueva-ia",
        &foto_prueba(
            "hilo-2",
            "Dueña: sigue disponible?",
            "CRUDO Dueña: sigue disponible?",
        ),
        Coste::default(),
    )
    .await
    .expect("reemplaza");
    assert_eq!(
        buscar_cache(&pool, &firma, &ph, &ch).await.expect("busca"),
        Some(("nueva-ia".to_string(), false))
    );
    /* [07AA-7] Regenerar refresca la foto del chat (+ crudo [08AA-21]). */
    let hilo2: (String, String, Option<String>) = sqlx::query_as(
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
        hilo2,
        (
            "hilo-2".to_string(),
            "Dueña: sigue disponible?".to_string(),
            Some("CRUDO Dueña: sigue disponible?".to_string())
        )
    );
    borrar_cache(&pool, &firma, &ph, &ch).await.expect("limpia");
}

#[tokio::test]
async fn corregir_rechaza_vacio_y_acepta_contacto() {
    /* [08AA-14] Sin matriz negativa por decisión de ella: el texto de
     * la dueña (incluido su contacto) pasa tal cual; solo el vacío
     * se rechaza. (Antes este test exigía rechazar el contacto.) */
    let Some(pool) = pool_si_hay() else { return };
    let (firma, ph, ch) = (clave_azar(), clave_azar(), clave_azar());
    assert!(corregir_cache(&pool, &firma, &ph, &ch, "").await.is_err());
    assert!(
        corregir_cache(&pool, &firma, &ph, &ch, "llámame al 0412 1234567")
            .await
            .is_ok()
    );
    assert_eq!(
        buscar_cache(&pool, &firma, &ph, &ch).await.expect("busca"),
        Some(("llámame al 0412 1234567".to_string(), true))
    );
    borrar_cache(&pool, &firma, &ph, &ch).await.expect("limpia");
}
