#![cfg(test)]
//! [09AA-30 F4] `resumen_chats` expone cuántos hilos comparten cada respuesta
//! (`compartida_por`) y la fecha del último borrador (`ultimo_borrador_en`).

use super::super::*;

/* [09AA-30 F4] El escenario enlaza el mismo mensaje (m2) en los hilos A y B:
 * ambos ven `compartida_por` = 2. Un hilo sin enlace ve 1. Sin
 * `DATABASE_URL` se omite. */
#[tokio::test]
async fn resumen_cuenta_hilos_que_comparten_respuesta() {
    let Some(e) = escenario_borrador(
        "hilo-compartida-a-09aa30",
        "hilo-compartida-b-09aa30",
    )
    .await
    else {
        return;
    };
    let solo = "hilo-compartida-solo-09aa30";
    let (f, p, c) = (clave_azar(), clave_azar(), clave_azar());
    guardar_cache(
        &e.pool,
        &f,
        &p,
        &c,
        "solo",
        &foto_prueba(solo, "s", "crudo-s"),
        Coste::default(),
    )
    .await
    .expect("guarda hilo sin enlace");

    let chats = resumen_chats(&e.pool, 100, None, false)
        .await
        .expect("resumen")
        .chats;
    let de = |hilo: &str| {
        chats
            .iter()
            .find(|r| r.thread_id == hilo)
            .map(|r| (r.compartida_por, r.ultimo_borrador_en.is_some()))
    };
    let clave_solo = clave_hilo(solo);
    let resultado = (
        de(e.hilo_a.as_str()),
        de(e.hilo_b.as_str()),
        de(clave_solo.as_str()),
    );

    borrar_hilo(&e.pool, &clave_solo).await.expect("limpia solo");
    limpiar_escenario(&e).await;
    assert_eq!(
        resultado,
        (Some((2, true)), Some((2, true)), Some((1, true))),
        "compartida_por cuenta los hilos con la misma respuesta; ultimo_borrador_en viene de la fila"
    );
}
