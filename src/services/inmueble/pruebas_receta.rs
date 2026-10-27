#![cfg(test)]

use super::*;
use crate::models::{CreateInmuebleRequest, RecetaPublicidad, UpdateInmuebleRequest};

fn pool_si_hay() -> Option<PgPool> {
    let url = crate::config::url_bd_pruebas()?;
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy(&url)
        .ok()
}

fn receta_valida() -> RecetaPublicidad {
    RecetaPublicidad {
        fondo_idx: 0,
        circular_grande_idx: 1,
        circular_mediano_idx: 2,
        formato: "post-4-5".to_string(),
        con_precio: false,
        titulo1: "Casa".to_string(),
        titulo2: "en Prueba".to_string(),
    }
}

fn crear_humo() -> CreateInmuebleRequest {
    CreateInmuebleRequest {
        titulo: "Humo receta 229A-2".to_string(),
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

fn solo_receta(receta: Option<RecetaPublicidad>) -> UpdateInmuebleRequest {
    UpdateInmuebleRequest {
        titulo: None,
        descripcion: None,
        ubicacion: None,
        puestos: None,
        residencia: None,
        nombre_corto: None,
        precio: None,
        precio_alquiler: None,
        tipo: None,
        operacion: None,
        habitaciones: None,
        banos: None,
        metros: None,
        metros_terreno: None,
        estado: None,
        marketplace_id: None,
        alias_titulos: None,
        copy: None,
        receta,
    }
}

#[tokio::test]
async fn receta_persiste_y_formato_invalido_rechaza() {
    let Some(pool) = pool_si_hay() else { return };
    let creado = InmuebleService::create(&pool, crear_humo()).await.unwrap();
    assert!(creado.receta.is_none());

    let con_receta =
        InmuebleService::update(&pool, creado.id, solo_receta(Some(receta_valida())))
            .await
            .unwrap();
    let guardada = con_receta.receta.expect("receta guardada");
    assert_eq!(guardada.formato, "post-4-5");
    assert!(!guardada.con_precio);

    let releido = InmuebleService::get_admin(&pool, creado.id).await.unwrap();
    assert_eq!(releido.receta.map(|r| r.titulo1), Some("Casa".to_string()));

    let mut mala = receta_valida();
    mala.formato = "banner-9-16".to_string();
    let err = InmuebleService::update(&pool, creado.id, solo_receta(Some(mala)))
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::Validation(_)));

    InmuebleService::delete(&pool, Path::new("."), creado.id)
        .await
        .unwrap();
}
