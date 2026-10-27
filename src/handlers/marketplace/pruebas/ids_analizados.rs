#![cfg(test)]

use super::super::*;
use crate::models::CreateInmuebleRequest;
use crate::repositories::marketplace::{
    borrar_avisos_perfil, ids_analizados, reemplazar_avisos_perfil, AvisoPerfilNuevo,
};
use crate::services::InmuebleService;

fn pool_si_hay() -> Option<sqlx::PgPool> {
    let url = crate::config::url_bd_pruebas()?;
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy(&url)
        .ok()
}

/* [GW-2 M1] Ids ya analizados: avisos presentes en el perfil e inmuebles con
 * `marketplace_id`; un aviso ausente del perfil no cuenta. Los ids llevan un
 * UUID propio para no chocar con datos reales ni con ejecuciones previas. El
 * perfil se fija con las mismas funciones que usa la lectura real. */
#[tokio::test]
async fn ids_analizados_une_perfil_e_inmuebles_sin_ausentes() {
    let Some(pool) = pool_si_hay() else { return };
    let _bloqueo = super::BD_PERFIL.lock().await;
    /* El `marketplace_id` de un inmueble se valida como numérico: los tres ids
     * salen del mismo número aleatorio, con prefijo distinto por caso. */
    let n = Uuid::new_v4().as_u128() % 10u128.pow(14);
    let presente = format!("8{n:014}");
    let ausente = format!("7{n:014}");
    let vinculado = format!("9{n:014}");
    let titulo = format!("Inmueble analizado GW2 {n}");

    /* Dos lecturas: la primera trae ambos avisos; la segunda deja fuera `ausente`,
     * que pasa a `presente = false` como un aviso que dejó de venir. */
    let aviso = |id: &str| AvisoPerfilNuevo {
        aviso_id: id.to_string(),
        titulo: format!("Aviso {id}"),
        precio: None,
        url: format!("https://ejemplo.test/{id}"),
        imagen_url: None,
    };
    reemplazar_avisos_perfil(&pool, &[aviso(&presente), aviso(&ausente)])
        .await
        .unwrap();
    reemplazar_avisos_perfil(&pool, &[aviso(&presente)])
        .await
        .unwrap();

    let creado = InmuebleService::create(
        &pool,
        CreateInmuebleRequest {
            titulo: titulo.clone(),
            descripcion: String::new(),
            ubicacion: String::new(),
            puestos: 0,
            residencia: String::new(),
            nombre_corto: String::new(),
            precio: 0.0,
            tipo: "apartamento".to_string(),
            operacion: "venta".to_string(),
            habitaciones: 0,
            banos: 0,
            metros: 0.0,
            metros_terreno: 0.0,
            estado: "disponible".to_string(),
            marketplace_id: Some(vinculado.clone()),
            alias_titulos: Vec::new(),
            copy: None,
        },
    )
    .await
    .unwrap();

    let ids = ids_analizados(&pool, 100_000).await.unwrap();
    assert!(ids.contains(&presente), "aviso presente del perfil cuenta");
    assert!(ids.contains(&vinculado), "inmueble vinculado cuenta");
    assert!(!ids.contains(&ausente), "aviso ausente del perfil no cuenta");

    let tope = ids_analizados(&pool, 1).await.unwrap();
    assert!(tope.len() <= 1, "el tope acota el resultado");

    InmuebleService::delete(&pool, std::path::Path::new("."), creado.id)
        .await
        .unwrap();
    borrar_avisos_perfil(&pool, &[presente, ausente]).await.unwrap();
}
