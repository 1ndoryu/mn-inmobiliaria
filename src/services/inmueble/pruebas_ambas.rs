use super::*;
use crate::models::{CreateInmuebleRequest, FiltrosPublicos};

fn pool_si_hay() -> Option<PgPool> {
    let url = crate::config::url_bd_pruebas()?;
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy(&url)
        .ok()
}

fn ambas() -> CreateInmuebleRequest {
    CreateInmuebleRequest {
        titulo: "Humo ambas 25AA-5".to_string(),
        descripcion: String::new(),
        ubicacion: String::new(),
        puestos: 0,
        residencia: String::new(),
        nombre_corto: String::new(),
        precio: 90_000.0,
        precio_alquiler: 400.0,
        tipo: "apartamento".to_string(),
        operacion: "ambas".to_string(),
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

async fn aparece(pool: &PgPool, id: Uuid, operacion: &str, min: f64, max: f64) -> bool {
    let filtros = FiltrosPublicos {
        tipo: None,
        operacion: Some(operacion.to_string()),
        precio_min: Some(min),
        precio_max: Some(max),
        page: 1,
        per_page: 100,
    };
    let pagina = InmuebleService::list_public(pool, filtros).await.unwrap();
    pagina.items.iter().any(|i| i.id == id)
}

#[tokio::test]
async fn ambas_guarda_los_dos_precios_y_cada_filtro_usa_el_suyo() {
    let Some(pool) = pool_si_hay() else { return };
    let creado = InmuebleService::create(&pool, ambas()).await.unwrap();
    assert_eq!(creado.operacion, "ambas");
    assert!((creado.precio - 90_000.0).abs() < f64::EPSILON);
    assert!((creado.precio_alquiler - 400.0).abs() < f64::EPSILON);
    InmuebleService::set_publicado(&pool, creado.id, true)
        .await
        .unwrap();

    // Venta mira `precio`; alquiler mira `precio_alquiler`.
    assert!(aparece(&pool, creado.id, "venta", 89_999.0, 90_001.0).await);
    assert!(!aparece(&pool, creado.id, "venta", 399.0, 401.0).await);
    assert!(aparece(&pool, creado.id, "alquiler", 399.0, 401.0).await);
    assert!(!aparece(&pool, creado.id, "alquiler", 89_999.0, 90_001.0).await);
    assert!(aparece(&pool, creado.id, "ambas", 89_999.0, 90_001.0).await);

    InmuebleService::delete(&pool, Path::new("."), creado.id)
        .await
        .unwrap();
}

#[tokio::test]
async fn operacion_fuera_del_allowlist_se_rechaza() {
    let Some(pool) = pool_si_hay() else { return };
    let mut req = ambas();
    req.operacion = "trueque".to_string();
    let err = InmuebleService::create(&pool, req).await.unwrap_err();
    assert!(matches!(err, AppError::Validation(_)));
}
