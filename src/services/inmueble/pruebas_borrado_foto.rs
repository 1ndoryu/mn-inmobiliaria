#![cfg(test)]

use super::*;
use crate::models::{AddFotoRequest, CreateInmuebleRequest};

fn pool_si_hay() -> Option<PgPool> {
    let url = crate::config::url_bd_pruebas()?;
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy(&url)
        .ok()
}

fn crear_humo() -> CreateInmuebleRequest {
    CreateInmuebleRequest {
        titulo: "Humo borrado foto 08AA-4".to_string(),
        descripcion: String::new(),
        ubicacion: String::new(),
        puestos: 0,
        residencia: String::new(),
        nombre_corto: String::new(),
        precio: 0.0,
        precio_alquiler: 0.0,
        tipo: "apartamento".to_string(),
        operacion: "venta".to_string(),
        habitaciones: 0,
        banos: 0,
        metros: 0.0,
        metros_terreno: 0.0,
        estado: "disponible".to_string(),
        marketplace_id: None,
        alias_titulos: Vec::new(),
        copy: None,
    }
}

fn alta(orden: i32, origen: &str) -> AddFotoRequest {
    AddFotoRequest {
        storage_key: format!("humo-borrado-{orden}-{origen}.jpg"),
        orden: Some(orden),
        origen: Some(origen.to_string()),
    }
}

#[tokio::test]
async fn borrar_original_arrastra_hermana_y_renumera() {
    let Some(pool) = pool_si_hay() else { return };
    let creado = InmuebleService::create(&pool, crear_humo()).await.unwrap();
    let original_cero = InmuebleService::add_foto(&pool, creado.id, alta(0, "original"))
        .await
        .unwrap();
    InmuebleService::add_foto(&pool, creado.id, alta(0, "mejorada"))
        .await
        .unwrap();
    InmuebleService::add_foto(&pool, creado.id, alta(1, "original"))
        .await
        .unwrap();
    InmuebleService::add_foto(&pool, creado.id, alta(1, "mejorada"))
        .await
        .unwrap();

    InmuebleService::delete_foto(&pool, Path::new("."), original_cero.id)
        .await
        .unwrap();

    let releido = InmuebleService::get_admin(&pool, creado.id).await.unwrap();
    assert_eq!(releido.fotos.len(), 2, "cae el par completo del orden 0");
    for foto in &releido.fotos {
        assert_eq!(foto.orden, 0, "el orden 1 se renumera a 0");
    }
    let mut origenes: Vec<&str> = releido
        .fotos
        .iter()
        .map(|foto| foto.origen.as_str())
        .collect();
    origenes.sort_unstable();
    assert_eq!(origenes, vec!["mejorada", "original"]);

    InmuebleService::delete(&pool, Path::new("."), creado.id)
        .await
        .unwrap();
}

#[tokio::test]
async fn borrar_mejorada_conserva_original() {
    let Some(pool) = pool_si_hay() else { return };
    let creado = InmuebleService::create(&pool, crear_humo()).await.unwrap();
    InmuebleService::add_foto(&pool, creado.id, alta(0, "original"))
        .await
        .unwrap();
    let mejorada = InmuebleService::add_foto(&pool, creado.id, alta(0, "mejorada"))
        .await
        .unwrap();
    InmuebleService::add_foto(&pool, creado.id, alta(1, "original"))
        .await
        .unwrap();

    InmuebleService::delete_foto(&pool, Path::new("."), mejorada.id)
        .await
        .unwrap();

    let releido = InmuebleService::get_admin(&pool, creado.id).await.unwrap();
    assert_eq!(releido.fotos.len(), 2, "el original sobrevive");
    assert!(
        releido.fotos.iter().all(|foto| foto.origen == "original"),
        "no quedan mejoradas"
    );
    let mut ordenes: Vec<i32> = releido.fotos.iter().map(|foto| foto.orden).collect();
    ordenes.sort_unstable();
    assert_eq!(ordenes, vec![0, 1], "sin renumerado al caer solo mejora");

    let err = InmuebleService::delete_foto(&pool, Path::new("."), mejorada.id).await;
    assert!(
        matches!(err, Err(AppError::NotFound(_))),
        "segundo borrado es 404"
    );

    InmuebleService::delete(&pool, Path::new("."), creado.id)
        .await
        .unwrap();
}
