#![cfg(test)]
//! Releer, combinar foto del hilo y agregado de uso.

use super::super::*;

/* [08AA-31] La foto fusiona sin perder al cliente: el snapshot nuevo
 * solo trae lo propio (`Tú:`) y la foto vieja aporta la pregunta;
 * el dedup exacto evita duplicar lo que ya estaba. */
#[test]
fn combinar_foto_hilo_conserva_cliente_ante_eco_propio() {
    let vieja = "¿Sigue disponible?\nTú: Hola, por favor, déjame un número.";
    let nueva = "Tú: Hola, por favor, déjame un número.";
    assert_eq!(combinar_foto_hilo(vieja, nueva), vieja);
    assert_eq!(combinar_foto_hilo("", nueva), nueva);
    assert_eq!(combinar_foto_hilo(vieja, vieja), vieja);
}

/* [08AA-28] Releer crea la fila solo-foto si falta (sin inventar
 * borrador) y la segunda vez solo refresca. Vivo con `pool_si_hay`;
 * sin `DATABASE_URL` se omite. */
#[tokio::test]
async fn releer_crea_fila_si_falta() {
    let Some(pool) = pool_si_hay() else { return };
    let hilo = "releer-test|hilo sintético 08AA-28";
    let limpia = || async {
        sqlx::query("DELETE FROM mp_respuestas_cache WHERE thread_id = $1")
            .bind(clave_hilo(hilo))
            .execute(&pool)
            .await
            .expect("limpia")
    };
    limpia().await;
    let (actualizado, creado) = releer_foto(&pool, hilo, "Hola. ¿Sigue disponible?", "crudo")
        .await
        .expect("releer crea");
    assert!(actualizado && creado);
    let (actualizado2, creado2) =
        releer_foto(&pool, hilo, "Hola. ¿Sigue disponible?", "crudo2")
            .await
            .expect("releer refresca");
    assert!(actualizado2 && !creado2);
    let filas = detalle_chat(&pool, hilo).await.expect("detalle");
    assert_eq!(filas.len(), 1);
    assert_eq!(filas[0].respuesta, "");
    let crudo: (String,) =
        sqlx::query_as("SELECT excerpt_crudo FROM mp_respuestas_cache WHERE thread_id = $1")
            .bind(clave_hilo(hilo))
            .fetch_one(&pool)
            .await
            .expect("lee crudo");
    assert_eq!(crudo.0, "crudo2");
    limpia().await;
}

#[tokio::test]
async fn uso_agrega_por_dia_y_evento() {
    let Some(pool) = pool_si_hay() else { return };
    let base = uuid::Uuid::new_v4().to_string().replace('-', "");
    for (sufijo, evento) in [("a", "hit"), ("b", "hit"), ("c", "copiar")] {
        sqlx::query(
            "INSERT INTO mp_auditoria (hilo_hmac, ts_hora, evento) \
             VALUES ($1, date_trunc('hour', now()), $2)",
        )
        .bind(format!("{base}{sufijo}"))
        .bind(evento)
        .execute(&pool)
        .await
        .expect("inserta auditoria");
    }
    let filas = resumen_uso(&pool, 7).await.expect("resume uso");
    let hoy = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let fila = filas.iter().find(|f| f.dia == hoy).expect("fila de hoy");
    assert!(fila.hit >= 2, "hit={}", fila.hit);
    assert!(fila.copiar >= 1, "copiar={}", fila.copiar);
}
