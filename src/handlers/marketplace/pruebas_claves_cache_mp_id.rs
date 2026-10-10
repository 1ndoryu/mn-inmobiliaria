#![cfg(test)]

use super::*;
use crate::models::CreateInmuebleRequest;
use crate::services::InmuebleService;

fn pool_si_hay() -> Option<sqlx::PgPool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy(&url)
        .ok()
}

fn crear_humo(titulo: &str, marketplace_id: Option<&str>) -> CreateInmuebleRequest {
    CreateInmuebleRequest {
        titulo: titulo.to_string(),
        descripcion: String::new(),
        ubicacion: String::new(),
        puestos: 0,
        residencia: String::new(),
        precio: 0.0,
        tipo: "apartamento".to_string(),
        operacion: "venta".to_string(),
        habitaciones: 0,
        banos: 0,
        metros: 0.0,
        metros_terreno: 0.0,
        estado: "disponible".to_string(),
        marketplace_id: marketplace_id.map(str::to_string),
        alias_titulos: Vec::new(),
        copy: None,
    }
}

/* Restos de una ejecución previa que cayó antes de la limpieza final (un
 * assert fallido no llega al `delete`). Se borran por título: es único de
 * cada test, así la siguiente ejecución arranca limpia. */
async fn limpiar_restos(pool: &sqlx::PgPool, titulo: &str) {
    let ids = InmuebleRepository::ids_por_titulo(pool, titulo)
        .await
        .unwrap();
    for id in ids {
        InmuebleService::delete(pool, std::path::Path::new("."), id)
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn matriz_id_valido_inexistente_titulo_y_uuid_legacy() {
    let Some(pool) = pool_si_hay() else { return };
    let aviso = "123456789066666";
    /* Título sin palabras en común con las fichas reales publicadas sin
     * vínculo: si las tuviera, el fallback por título de F2 citaría una de
     * ellas y «ID inexistente no cita otra ficha» dejaría de probar lo que
     * dice. */
    let titulo = "Quinta clavescache mp-id 09AA-21 Lomas Norte";
    limpiar_restos(&pool, titulo).await;
    let creado = InmuebleService::create(&pool, crear_humo(titulo, Some(aviso)))
        .await
        .unwrap();
    InmuebleService::set_publicado(&pool, creado.id, true)
        .await
        .unwrap();

    let (ficha, precio, catalogo, conocido) =
        claves_cache(&pool, Some(aviso), Some("título que no empareja nada"))
            .await
            .unwrap();
    assert!(ficha.is_some() && conocido, "ID válido da ficha exacta");
    assert_ne!(precio, SIN_FICHA);
    assert_ne!(catalogo, SIN_FICHA);

    let (ficha, precio, _, conocido) = claves_cache(&pool, Some("999999999066666"), Some(titulo))
        .await
        .unwrap();
    assert!(
        ficha.is_none() && !conocido,
        "ID inexistente no cita otra ficha"
    );
    assert_eq!(precio, SIN_FICHA);

    let (ficha, _, _, conocido) = claves_cache(&pool, None, Some(titulo)).await.unwrap();
    assert!(ficha.is_some() && conocido, "sin ID el título empareja");

    let uuid = creado.id.to_string();
    let (ficha, _, _, conocido) = claves_cache(&pool, Some(&uuid), None).await.unwrap();
    assert!(ficha.is_some() && conocido, "UUID legacy sigue resolviendo");

    let (ficha, _, _, conocido) = claves_cache(&pool, Some(&Uuid::new_v4().to_string()), None)
        .await
        .unwrap();
    assert!(
        ficha.is_none() && !conocido,
        "UUID inexistente es sin ficha"
    );

    InmuebleService::delete(&pool, std::path::Path::new("."), creado.id)
        .await
        .unwrap();
}

/* [09AA-29] Fallback por título de un ID sin dueño: una ficha vinculada a
 * OTRO aviso jamás se cita (aunque el título empareje); una sin vínculo sí.
 * Títulos con ≤2 palabras comunes entre sí: sin solape que confunda el
 * emparejamiento. */
#[tokio::test]
async fn id_sin_dueno_no_cita_ficha_vinculada_a_otro_aviso() {
    let Some(pool) = pool_si_hay() else { return };
    let titulo_vinculada = "Zarzal clavescache Vinculado Alfa";
    let titulo_libre = "Pinares clavescache Libre Beta";
    limpiar_restos(&pool, titulo_vinculada).await;
    limpiar_restos(&pool, titulo_libre).await;
    let vinculada =
        InmuebleService::create(&pool, crear_humo(titulo_vinculada, Some("123456789088800")))
            .await
            .unwrap();
    InmuebleService::set_publicado(&pool, vinculada.id, true)
        .await
        .unwrap();
    let libre = InmuebleService::create(&pool, crear_humo(titulo_libre, None))
        .await
        .unwrap();
    InmuebleService::set_publicado(&pool, libre.id, true)
        .await
        .unwrap();

    let (ficha, precio, _, conocido) =
        claves_cache(&pool, Some("999999999088801"), Some(titulo_vinculada))
            .await
            .unwrap();
    assert!(
        ficha.is_none() && !conocido,
        "ficha de otro aviso no se cita por título"
    );
    assert_eq!(precio, SIN_FICHA);

    let (ficha, _, _, conocido) = claves_cache(&pool, Some("999999999088801"), Some(titulo_libre))
        .await
        .unwrap();
    assert!(ficha.is_some() && conocido, "ficha sin vínculo sí se cita");

    InmuebleService::delete(&pool, std::path::Path::new("."), vinculada.id)
        .await
        .unwrap();
    InmuebleService::delete(&pool, std::path::Path::new("."), libre.id)
        .await
        .unwrap();
}
