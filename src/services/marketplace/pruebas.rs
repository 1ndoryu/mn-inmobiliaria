#![cfg(test)]
//! Tests del servicio de marketplace (movidos desde `services/marketplace.rs`, 10AA-5).
//! Helpers compartidos aquí; los casos viven en los submódulos por dominio.

use super::*;

mod borrado;
mod borrador_borrado;
mod cache;
mod corregir;
mod ficha_prompt;
mod forma_salida;
mod formato_config;
mod hilos;
mod titulo;

fn ficha() -> InmuebleRow {
    InmuebleRow {
        id: Uuid::new_v4(),
        titulo: "Casa en Riberas".to_string(),
        descripcion: "Bonita casa".to_string(),
        ubicacion: "Puerto Ordaz".to_string(),
        puestos: 1,
        residencia: "Riberas del Caroní".to_string(),
        precio: 43000.0,
        tipo: "casa".to_string(),
        operacion: "venta".to_string(),
        habitaciones: 3,
        banos: 2,
        metros: 180.0,
        metros_terreno: 300.0,
        estado: "disponible".to_string(),
        publicado: true,
        slug: "casa-riberas".to_string(),
        copy_corta: None,
        copy_larga: None,
        copy_modelo: None,
        copy_actualizada_en: None,
        receta: None,
        extras: sqlx::types::Json(serde_json::json!({})),
        /* El mínimo privado jamás viaja al prompt: el test de claves lo
         * amarra junto al slug y al estado interno. */
        precio_minimo: Some(40000.0),
        marketplace_id: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        alias_titulos: Vec::new(),
    }
}

fn pedido() -> BorradorRequest {
    serde_json::from_value(serde_json::json!({
        "threadId": "hilo-sintetico-001",
        "firma": "ab".repeat(32),
        "firma_version": "firma-v1",
        "lang": "es",
        "excerpt": {
            "remitente_hash": "cd".repeat(32),
            "texto": "Hola, ¿sigue disponible?",
            "hora": "2026-10-05T18:00:00-04:00",
            "leido": true
        },
        "avisoId": null
    }))
    .unwrap()
}

fn pool_si_hay() -> Option<sqlx::PgPool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy(&url)
        .ok()
}

/* Contra BD viva: 2 hit + 1 copiar hoy se agregan en la fila del día;
 * sin `DATABASE_URL` se omite. Solo lee conteos, sin PII. */

fn fila_prueba(precio: f64) -> InmuebleRow {
    InmuebleRow {
        id: uuid::Uuid::new_v4(),
        titulo: "Apartamento en Los Palos Grandes".to_string(),
        descripcion: "Lindo apartamento con vista".to_string(),
        ubicacion: "Chacao".to_string(),
        puestos: 1,
        residencia: "Edif. Los Pinos".to_string(),
        precio,
        tipo: "apartamento".to_string(),
        operacion: "venta".to_string(),
        habitaciones: 2,
        banos: 2,
        metros: 85.0,
        metros_terreno: 0.0,
        estado: "disponible".to_string(),
        publicado: true,
        slug: "apt-test".to_string(),
        copy_corta: None,
        copy_larga: None,
        copy_modelo: None,
        copy_actualizada_en: None,
        receta: None,
        extras: sqlx::types::Json(serde_json::json!({})),
        precio_minimo: None,
        marketplace_id: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        alias_titulos: Vec::new(),
    }
}

fn clave_azar() -> String {
    format!(
        "{:x}{:x}",
        uuid::Uuid::new_v4().as_simple(),
        uuid::Uuid::new_v4().as_simple()
    )
}

/* [08AA-21] Atajo para la foto del hilo en pruebas de caché. */
fn foto_prueba(
    hilo: &'static str,
    limpio: &'static str,
    crudo: &'static str,
) -> FotoHilo<'static> {
    FotoHilo {
        thread_id: hilo,
        excerpt: limpio,
        excerpt_crudo: crudo,
    }
}

/* Clave compartida de un (catálogo, precio, mensaje) para los tests F2. */
fn clave_de<'a>(catalogo: &'a str, precio: &'a str, m: &'a str) -> ClaveCompartida<'a> {
    ClaveCompartida {
        catalog_hash: catalogo,
        precio_hash: precio,
        mensaje_clave: m,
    }
}

/* [09AA-30 F2] Escenario de las pruebas de borrado de borrador: el hilo A
 * tiene dos borradores (a1, a2) y una corrección (a3); el hilo B, un
 * borrador (b2) que enlaza el mismo mensaje que a2. Los hilos son `'static`
 * porque `foto_prueba` lo exige; cada prueba pasa los suyos porque el
 * borrado es por hilo y las pruebas corren en paralelo. */
struct EscenarioBorrador {
    pool: sqlx::PgPool,
    hilo_a: String,
    hilo_b: String,
    catalogo: String,
    precio: String,
    fa1: String,
    fa2: String,
    fa3: String,
    fb2: String,
    m1: String,
    m2: String,
    m3: String,
}

async fn escenario_borrador(
    hilo_a: &'static str,
    hilo_b: &'static str,
) -> Option<EscenarioBorrador> {
    let pool = pool_si_hay()?;
    let catalogo = clave_azar();
    let precio = clave_azar();
    let (fa1, fa2, fa3, fb2) = (clave_azar(), clave_azar(), clave_azar(), clave_azar());
    let m1 = mensaje_clave_de(&["¿sigue disponible?"]);
    let m2 = mensaje_clave_de(&["¿precio final?"]);
    let m3 = mensaje_clave_de(&["¿pagan con zelle?"]);
    guardar_cache(
        &pool,
        &fa1,
        &precio,
        &catalogo,
        "a-m1",
        &foto_prueba(hilo_a, "x", "crudo-x"),
        Coste::default(),
    )
    .await
    .expect("guarda a1");
    guardar_cache(
        &pool,
        &fa2,
        &precio,
        &catalogo,
        "a-m2",
        &foto_prueba(hilo_a, "y", "crudo-y"),
        Coste::default(),
    )
    .await
    .expect("guarda a2");
    guardar_cache(
        &pool,
        &fa3,
        &precio,
        &catalogo,
        "a-m3",
        &foto_prueba(hilo_a, "z", "crudo-z"),
        Coste::default(),
    )
    .await
    .expect("guarda a3");
    corregir_cache(&pool, &fa3, &precio, &catalogo, "texto duena")
        .await
        .expect("corrige a3");
    guardar_cache(
        &pool,
        &fb2,
        &precio,
        &catalogo,
        "b-m2",
        &foto_prueba(hilo_b, "w", "crudo-w"),
        Coste::default(),
    )
    .await
    .expect("guarda b2");
    let enlaces = [
        (fa1.as_str(), clave_de(&catalogo, &precio, &m1), "p1"),
        (fa2.as_str(), clave_de(&catalogo, &precio, &m2), "p2"),
        (fb2.as_str(), clave_de(&catalogo, &precio, &m2), "p2"),
        (fa3.as_str(), clave_de(&catalogo, &precio, &m3), "p3"),
    ];
    for (firma, clave, plantilla) in &enlaces {
        enlazar_compartida(&pool, firma, clave, plantilla, Coste::default(), false)
            .await
            .expect("enlaza");
    }
    Some(EscenarioBorrador {
        pool,
        hilo_a: clave_hilo(hilo_a),
        hilo_b: clave_hilo(hilo_b),
        catalogo,
        precio,
        fa1,
        fa2,
        fa3,
        fb2,
        m1,
        m2,
        m3,
    })
}

/// Deja la BD como estaba: borra los dos hilos y la compartida del escenario.
async fn limpiar_escenario(e: &EscenarioBorrador) {
    borrar_hilo(&e.pool, &e.hilo_a).await.expect("limpia a");
    borrar_hilo(&e.pool, &e.hilo_b).await.expect("limpia b");
    sqlx::query("DELETE FROM mp_respuestas_inmueble WHERE catalog_hash = $1")
        .bind(&e.catalogo)
        .execute(&e.pool)
        .await
        .expect("limpia compartida");
}

/* [09AA-2] La forma del borrador la impone Rust: batería de regresión
 * con los testigos reales (fotos, «sigue vigente», baños). */
fn ia_fabio(medio: &str) -> String {
    format!(
        "Hola, Fabio, buenas noches, la Casa en Riberas del Caroní está disponible en $43.000 negociable.\n\n{medio}\n\nCuéntame qué estás buscando y con gusto te ayudo, cualquier cosa escríbeme al 04249208855\nhttps://wa.me/584249208855"
    )
}
