#![cfg(test)]
//! Borrado de borradores por hilo.

use super::*;

/* [09AA-30 F2] Borrar borrador quita solo los no corregidos del hilo; la
 * corrección de ese hilo y el otro hilo quedan intactos. */
#[tokio::test]
async fn borrar_borrador_quita_solo_no_corregidas_del_hilo() {
    let Some(e) = escenario_borrador("hilo-bb-a-09aa30", "hilo-bb-b-09aa30").await else {
        return;
    };
    let version_antes = version_borradores_borrados();
    let n = borrar_borrador_hilo(&e.pool, &e.hilo_a)
        .await
        .expect("borra borrador");
    assert_eq!(n, 2, "caen a1 y a2; la corrección a3 queda");
    assert!(
        version_borradores_borrados() > version_antes,
        "el contador del float sube al borrar"
    );
    assert!(buscar_cache(&e.pool, &e.fa1, &e.precio, &e.catalogo)
        .await
        .expect("busca a1")
        .is_none());
    assert!(buscar_cache(&e.pool, &e.fa2, &e.precio, &e.catalogo)
        .await
        .expect("busca a2")
        .is_none());
    assert!(
        buscar_cache(&e.pool, &e.fa3, &e.precio, &e.catalogo)
            .await
            .expect("busca a3")
            .is_some(),
        "la corrección queda"
    );
    assert!(
        buscar_cache(&e.pool, &e.fb2, &e.precio, &e.catalogo)
            .await
            .expect("busca b2")
            .is_some(),
        "el otro hilo queda"
    );
    limpiar_escenario(&e).await;
}

/* [09AA-30 F2] Borrar borrador: la compartida de un mensaje se borra si
 * nadie más la enlaza, pero se queda si otro hilo o una corrección la siguen
 * enlazando. */
#[tokio::test]
async fn borrar_borrador_quita_compartida_solo_sin_otro_enlace() {
    let Some(e) = escenario_borrador("hilo-bb-c-09aa30", "hilo-bb-d-09aa30").await else {
        return;
    };
    borrar_borrador_hilo(&e.pool, &e.hilo_a)
        .await
        .expect("borra borrador");
    let c1 = clave_de(&e.catalogo, &e.precio, &e.m1);
    let c2 = clave_de(&e.catalogo, &e.precio, &e.m2);
    let c3 = clave_de(&e.catalogo, &e.precio, &e.m3);
    assert!(
        buscar_compartida(&e.pool, &c1)
            .await
            .expect("c m1")
            .is_none(),
        "nadie más enlaza m1: se borra"
    );
    assert!(
        buscar_compartida(&e.pool, &c2)
            .await
            .expect("c m2")
            .is_some(),
        "el hilo B sigue enlazando m2"
    );
    assert!(
        buscar_compartida(&e.pool, &c3)
            .await
            .expect("c m3")
            .is_some(),
        "la corrección de A sigue enlazando m3"
    );
    limpiar_escenario(&e).await;
}
