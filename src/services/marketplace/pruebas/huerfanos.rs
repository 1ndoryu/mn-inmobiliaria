#![cfg(test)]
//! [10AA-4] Filtro de huérfanos en `resumen_chats`: solo hilos sin ficha
//! conocida, paginados por cursor, sin archivados.

use super::*;

/* [10AA-4] Avisos numéricos que ninguna ficha tiene: son huérfanos sin depender
 * de la BD de inmuebles, y un aviso de dígitos no cae a título. Uno se archiva
 * y no debe salir. Sin `DATABASE_URL` se omite. */
#[tokio::test]
async fn solo_huerfanos_pagina_sin_conocidos() {
    let Some(pool) = pool_si_hay() else { return };
    let hilos: [&'static str; 3] = [
        "huerfano-10aa4-a|9900000000000000001",
        "huerfano-10aa4-b|9900000000000000002",
        "huerfano-10aa4-c|9900000000000000003",
    ];
    for hilo in hilos {
        let (f, p, c) = (clave_azar(), clave_azar(), clave_azar());
        guardar_cache(
            &pool,
            &f,
            &p,
            &c,
            "borrador",
            &foto_prueba(hilo, "x", "crudo-x"),
            Coste::default(),
        )
        .await
        .expect("guarda");
    }
    archivar_hilo(&pool, hilos[1]).await.expect("archiva b");

    /* Página de uno en uno: fuerza el cursor entre páginas. El tope evita un
     * bucle infinito si el cursor no avanza. */
    let mut vistos: Vec<String> = Vec::new();
    let mut cursor: Option<CursorChats> = None;
    let mut terminado = false;
    for _ in 0..500 {
        let pagina = resumen_chats(&pool, 1, cursor.as_ref(), true)
            .await
            .expect("pagina");
        for chat in &pagina.chats {
            assert!(!chat.aviso_conocido, "solo_huerfanos no trae hilos con ficha");
            vistos.push(chat.thread_id.clone());
        }
        if !pagina.hay_mas {
            terminado = true;
            break;
        }
        let Some(ultimo) = pagina.chats.last() else {
            panic!("hay_mas sin chats en la página");
        };
        cursor = Some(CursorChats {
            ultimo: chrono::DateTime::parse_from_rfc3339(&ultimo.ultimo)
                .expect("ultimo en RFC 3339")
                .with_timezone(&chrono::Utc),
            thread_id: ultimo.thread_id.clone(),
        });
    }
    let mut ordenados = vistos.clone();
    ordenados.sort();
    ordenados.dedup();
    for hilo in hilos {
        borrar_hilo(&pool, hilo).await.expect("limpia");
    }
    assert!(terminado, "el recorrido por cursor termina");
    assert_eq!(ordenados.len(), vistos.len(), "ningún hilo se repite");
    assert!(vistos.contains(&hilos[0].to_string()), "sale el huérfano a");
    assert!(vistos.contains(&hilos[2].to_string()), "sale el huérfano c");
    assert!(
        !vistos.contains(&hilos[1].to_string()),
        "el archivado no sale"
    );
}
