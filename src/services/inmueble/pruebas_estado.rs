#![cfg(test)]

use super::*;
use crate::models::CreateInmuebleRequest;

fn pool_si_hay() -> Option<PgPool> {
    let url = crate::config::url_bd_pruebas()?;
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy(&url)
        .ok()
}

fn crear_humo(titulo: &str) -> CreateInmuebleRequest {
    CreateInmuebleRequest {
        titulo: titulo.to_string(),
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

#[tokio::test]
async fn vendido_despublica_reservado_no_e_invalido_rechaza() {
    let Some(pool) = pool_si_hay() else { return };
    let creado = InmuebleService::create(&pool, crear_humo("Humo estado 08AA-33"))
        .await
        .unwrap();
    InmuebleService::set_publicado(&pool, creado.id, true)
        .await
        .unwrap();

    let vendido = InmuebleService::set_estado(&pool, creado.id, "vendido")
        .await
        .unwrap();
    assert_eq!(vendido.estado, "vendido");
    assert!(!vendido.publicado, "vendida sale de la web pública");

    let reservado = InmuebleService::set_estado(&pool, creado.id, "reservado")
        .await
        .unwrap();
    assert_eq!(reservado.estado, "reservado");
    assert!(!reservado.publicado, "reservado no republica solo");

    let err = InmuebleService::set_estado(&pool, creado.id, "quemado")
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::Validation(_)));

    let err = InmuebleService::set_estado(&pool, Uuid::new_v4(), "vendido").await;
    assert!(
        matches!(err, Err(AppError::NotFound(_))),
        "id inexistente es 404"
    );

    InmuebleService::delete(&pool, Path::new("."), creado.id)
        .await
        .unwrap();
}
