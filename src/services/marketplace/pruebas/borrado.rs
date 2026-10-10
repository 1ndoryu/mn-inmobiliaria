#![cfg(test)]
//! Borrado y archivado de hilos.

use super::*;

/* [09AA-4] Borrado previo a regenerar: caen los borradores viejos del
 * hilo, queda la corrección de la dueña y no se toca otro hilo. */
#[tokio::test]
async fn borrar_hilo_respeta_correccion_y_otro_hilo() {
    let Some(pool) = pool_si_hay() else { return };
    let (f1, p1, c1) = (clave_azar(), clave_azar(), clave_azar());
    guardar_cache(
        &pool,
        &f1,
        &p1,
        &c1,
        "viejo-1",
        &foto_prueba("hilo-r", "x", "crudo-x"),
        Coste::default(),
    )
    .await
    .expect("guarda 1");
    let (f2, p2, c2) = (clave_azar(), clave_azar(), clave_azar());
    guardar_cache(
        &pool,
        &f2,
        &p2,
        &c2,
        "viejo-2",
        &foto_prueba("hilo-r", "y", "crudo-y"),
        Coste::default(),
    )
    .await
    .expect("guarda 2");
    corregir_cache(&pool, &f2, &p2, &c2, "texto duena")
        .await
        .expect("corrige 2");
    let (f3, p3, c3) = (clave_azar(), clave_azar(), clave_azar());
    guardar_cache(
        &pool,
        &f3,
        &p3,
        &c3,
        "otro-hilo",
        &foto_prueba("hilo-otro", "z", "crudo-z"),
        Coste::default(),
    )
    .await
    .expect("guarda 3");
    let n = borrar_hilo_no_corregidas(&pool, &clave_hilo("hilo-r"))
        .await
        .expect("borra");
    assert_eq!(n, 1, "solo cae el borrador viejo del hilo");
    assert!(
        buscar_cache(&pool, &f2, &p2, &c2)
            .await
            .expect("busca 2")
            .is_some(),
        "la corrección de la dueña queda"
    );
    assert!(
        buscar_cache(&pool, &f3, &p3, &c3)
            .await
            .expect("busca 3")
            .is_some(),
        "el otro hilo no se toca"
    );
    borrar_cache(&pool, &f1, &p1, &c1).await.expect("limpia 1");
    borrar_cache(&pool, &f2, &p2, &c2).await.expect("limpia 2");
    borrar_cache(&pool, &f3, &p3, &c3).await.expect("limpia 3");
}

/* [09AA-30 F2] Archivar oculta el hilo de `resumen_chats` y no toca su
 * caché. Sin `DATABASE_URL` se omite. */
#[tokio::test]
async fn archivar_oculta_del_resumen_y_conserva_cache() {
    let Some(pool) = pool_si_hay() else { return };
    let hilo = clave_hilo("hilo-archivar-09aa30");
    let (f, p, c) = (clave_azar(), clave_azar(), clave_azar());
    guardar_cache(
        &pool,
        &f,
        &p,
        &c,
        "borrador",
        &foto_prueba("hilo-archivar-09aa30", "x", "crudo-x"),
        Coste::default(),
    )
    .await
    .expect("guarda");
    let antes = resumen_chats(&pool, 100, None, false)
        .await
        .expect("resumen antes")
        .chats;
    assert!(
        antes.iter().any(|r| r.thread_id == hilo),
        "control: el hilo sale en el resumen antes de archivar"
    );
    archivar_hilo(&pool, &hilo).await.expect("archiva");
    let tras = resumen_chats(&pool, 100, None, false)
        .await
        .expect("resumen tras")
        .chats;
    assert!(
        !tras.iter().any(|r| r.thread_id == hilo),
        "el archivado no sale del resumen"
    );
    assert!(
        buscar_cache(&pool, &f, &p, &c)
            .await
            .expect("busca")
            .is_some(),
        "la caché del archivado sigue viva"
    );
    borrar_hilo(&pool, &hilo).await.expect("limpia");
}

/* [09AA-30 F2] Borrar el chat quita sus filas (incluida la corrección) y su
 * marca de archivado, pero no toca la compartida del inmueble. */
#[tokio::test]
async fn borrar_hilo_quita_todo_menos_la_compartida() {
    let Some(pool) = pool_si_hay() else { return };
    let hilo = clave_hilo("hilo-borrar-09aa30");
    let (f1, p1, c1) = (clave_azar(), clave_azar(), clave_azar());
    guardar_cache(
        &pool,
        &f1,
        &p1,
        &c1,
        "borrador",
        &foto_prueba("hilo-borrar-09aa30", "x", "crudo-x"),
        Coste::default(),
    )
    .await
    .expect("guarda 1");
    corregir_cache(&pool, &f1, &p1, &c1, "texto duena")
        .await
        .expect("corrige 1");
    let (f2, p2, c2) = (clave_azar(), clave_azar(), clave_azar());
    guardar_cache(
        &pool,
        &f2,
        &p2,
        &c2,
        "otro",
        &foto_prueba("hilo-borrar-09aa30", "y", "crudo-y"),
        Coste::default(),
    )
    .await
    .expect("guarda 2");
    let mensaje = mensaje_clave_de(&["¿sigue disponible?"]);
    let clave = ClaveCompartida {
        catalog_hash: &c2,
        precio_hash: &p2,
        mensaje_clave: &mensaje,
    };
    enlazar_compartida(
        &pool,
        &f2,
        &clave,
        "Hola {{nombre}}.",
        Coste::default(),
        false,
    )
    .await
    .expect("enlaza 2");
    archivar_hilo(&pool, &hilo).await.expect("archiva");
    let n = borrar_hilo(&pool, &hilo).await.expect("borra");
    assert_eq!(n, 2, "caen la corrección y el borrador del hilo");
    assert!(
        buscar_cache(&pool, &f1, &p1, &c1)
            .await
            .expect("busca 1")
            .is_none(),
        "la corrección también cae con el chat"
    );
    let archivados: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM mp_chats_archivados WHERE thread_id = $1",
    )
    .bind(&hilo)
    .fetch_one(&pool)
    .await
    .expect("cuenta archivados");
    assert_eq!(archivados, 0, "sale la marca de archivado");
    assert!(
        buscar_compartida(&pool, &clave)
            .await
            .expect("compartida")
            .is_some(),
        "la compartida del inmueble no se toca"
    );
    sqlx::query("DELETE FROM mp_respuestas_inmueble WHERE catalog_hash = $1")
        .bind(&c2)
        .execute(&pool)
        .await
        .expect("limpia compartida");
}
