//! [13AA-4] Avisos del perfil de Facebook Marketplace. La lab envía la lectura
//! diaria (`registrar`, token mp) y el modal de Inmuebles la lee (`listar`,
//! staff). Solo ordena candidatos: nunca vincula nada por su cuenta.

use std::collections::HashSet;

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::MpAuth;
use crate::errors::AppError;
use crate::middleware::AuthStaff;
use crate::repositories::marketplace::{
    avisos_perfil_sin_vincular, inmuebles_sin_vincular, reemplazar_avisos_perfil,
    ultima_lectura_perfil, AvisoPerfilNuevo, InmuebleSinVincular,
};
use crate::services::marketplace::mejor_puntaje_con_alias;
use crate::services::InmuebleService;
use crate::AppState;

/// Tope de avisos por lectura: el perfil real tiene decenas; el tope evita un
/// cuerpo sin límite.
const MAX_AVISOS: usize = 500;
/// Inmuebles que el modal ofrece por aviso, los más parecidos primero.
const MAX_CANDIDATOS: usize = 10;
const MAX_TITULO: usize = 300;
const MAX_PRECIO: usize = 40;

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AvisoLeido {
    /// ID del aviso (`/marketplace/item/<id>` o dígitos 5–32).
    pub aviso_id: String,
    pub titulo: String,
    /// Precio tal como lo muestra Facebook.
    pub precio: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct LecturaPerfil {
    pub avisos: Vec<AvisoLeido>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CandidatoInmueble {
    pub inmueble_id: Uuid,
    pub titulo: String,
    pub precio: f64,
    /// Un título contiene al otro (coincidencia fuerte).
    pub directo: bool,
    /// Palabras que comparten el título del aviso y el del inmueble.
    pub palabras_comunes: usize,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AvisoPendiente {
    pub aviso_id: String,
    pub titulo: String,
    pub precio: Option<String>,
    pub url: String,
    pub candidatos: Vec<CandidatoInmueble>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct InmuebleSinAviso {
    pub inmueble_id: Uuid,
    pub titulo: String,
    pub precio: f64,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PerfilAvisos {
    /// Hora de la última lectura de la lab (`null` si nunca se ha leído).
    pub ultima_lectura: Option<DateTime<Utc>>,
    /// Avisos presentes en el perfil que ningún inmueble tiene vinculado.
    pub avisos: Vec<AvisoPendiente>,
    /// Inmuebles sin aviso en el perfil (informativo).
    pub inmuebles_sin_aviso: Vec<InmuebleSinAviso>,
}

/// Registra una lectura del perfil. Reemplaza el estado: lo que no viene pasa a
/// `presente = false`. Una lectura vacía o con un dato malo se rechaza entera,
/// para no dejar el perfil a medias.
#[utoipa::path(
    post,
    path = "/api/admin/marketplace/perfil/avisos",
    request_body = LecturaPerfil,
    responses(
        (status = 200, description = "Lectura registrada: recibidos y dejados de aparecer"),
        (status = 422, description = "Lectura vacía o con un aviso inválido", body = crate::errors::ErrorResponse)
    )
)]
pub async fn registrar(
    State(state): State<AppState>,
    _auth: MpAuth,
    r: Result<Json<LecturaPerfil>, axum::extract::rejection::JsonRejection>,
) -> Result<Response, AppError> {
    let r = r.map_err(|e| AppError::Validation(format!("JSON inválido: {e}")))?;
    let avisos = validar_lectura(r.0)?;
    let ausentes = reemplazar_avisos_perfil(&state.pool, &avisos).await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "recibidos": avisos.len(),
            "ausentes": ausentes,
        })),
    )
        .into_response())
}

/// Estado del perfil para el modal: avisos sin vincular con sus candidatos y
/// los inmuebles sin aviso. Lo ya vinculado no aparece en ninguna lista.
#[utoipa::path(
    get,
    path = "/api/admin/marketplace/perfil/avisos",
    responses(
        (status = 200, description = "Avisos del perfil y candidatos", body = PerfilAvisos)
    )
)]
pub async fn listar(State(state): State<AppState>, _auth: AuthStaff) -> Result<Response, AppError> {
    let pool = &state.pool;
    let ultima_lectura = ultima_lectura_perfil(pool).await?;
    let inmuebles = inmuebles_sin_vincular(pool).await?;
    let mut avisos = Vec::new();
    for fila in avisos_perfil_sin_vincular(pool).await? {
        avisos.push(AvisoPendiente {
            candidatos: ordenar_candidatos(&fila.titulo, &inmuebles),
            aviso_id: fila.aviso_id,
            titulo: fila.titulo,
            precio: fila.precio,
            url: fila.url,
        });
    }
    let inmuebles_sin_aviso = inmuebles
        .iter()
        .map(|i| InmuebleSinAviso {
            inmueble_id: i.id,
            titulo: i.titulo.clone(),
            precio: i.precio,
        })
        .collect();
    Ok((
        StatusCode::OK,
        Json(PerfilAvisos {
            ultima_lectura,
            avisos,
            inmuebles_sin_aviso,
        }),
    )
        .into_response())
}

/// Valida la lectura entera antes de tocar la BD. Los IDs repetidos cuentan
/// una vez (se queda la primera aparición).
fn validar_lectura(lectura: LecturaPerfil) -> Result<Vec<AvisoPerfilNuevo>, AppError> {
    if lectura.avisos.is_empty() {
        return Err(AppError::Validation(
            "lectura vacía: no se cambia el perfil".into(),
        ));
    }
    if lectura.avisos.len() > MAX_AVISOS {
        return Err(AppError::Validation(format!(
            "demasiados avisos en una lectura (máximo {MAX_AVISOS})"
        )));
    }
    let mut vistos = HashSet::new();
    let mut avisos = Vec::with_capacity(lectura.avisos.len());
    for leido in lectura.avisos {
        let Some(aviso_id) = InmuebleService::normalizar_marketplace_id(Some(&leido.aviso_id))?
        else {
            return Err(AppError::Validation("aviso_id vacío".into()));
        };
        let titulo = leido.titulo.trim();
        if titulo.is_empty() {
            return Err(AppError::Validation(format!("aviso {aviso_id} sin título")));
        }
        if !vistos.insert(aviso_id.clone()) {
            continue;
        }
        let precio = leido
            .precio
            .as_deref()
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .map(|p| p.chars().take(MAX_PRECIO).collect());
        avisos.push(AvisoPerfilNuevo {
            url: format!("https://www.facebook.com/marketplace/item/{aviso_id}"),
            titulo: titulo.chars().take(MAX_TITULO).collect(),
            precio,
            aviso_id,
        });
    }
    Ok(avisos)
}

/// Inmuebles más parecidos al título del aviso, primero. Usa el mismo criterio
/// que el emparejamiento de borradores (`mejor_puntaje_con_alias`), así el
/// modal y el borrador no discrepan sobre qué es «parecido».
fn ordenar_candidatos(
    aviso_titulo: &str,
    inmuebles: &[InmuebleSinVincular],
) -> Vec<CandidatoInmueble> {
    let mut puntuados: Vec<((bool, usize, usize), &InmuebleSinVincular)> = inmuebles
        .iter()
        .map(|i| (mejor_puntaje_con_alias(aviso_titulo, &i.titulo, &i.alias), i))
        .collect();
    puntuados.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.titulo.cmp(&b.1.titulo)));
    puntuados
        .into_iter()
        .take(MAX_CANDIDATOS)
        .map(|((directo, palabras_comunes, _), i)| CandidatoInmueble {
            inmueble_id: i.id,
            titulo: i.titulo.clone(),
            precio: i.precio,
            directo,
            palabras_comunes,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leido(id: &str, titulo: &str) -> AvisoLeido {
        AvisoLeido {
            aviso_id: id.to_string(),
            titulo: titulo.to_string(),
            precio: None,
        }
    }

    fn inmueble(titulo: &str) -> InmuebleSinVincular {
        InmuebleSinVincular {
            id: Uuid::new_v4(),
            titulo: titulo.to_string(),
            alias: Vec::new(),
            precio: 0.0,
        }
    }

    #[test]
    fn lectura_vacia_es_422() {
        let vacia = LecturaPerfil { avisos: Vec::new() };
        assert!(matches!(
            validar_lectura(vacia),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn id_invalido_es_422() {
        let lectura = LecturaPerfil {
            avisos: vec![leido("abc", "Casa en Los Samanes")],
        };
        assert!(matches!(
            validar_lectura(lectura),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn acepta_url_de_item_y_construye_la_url_canonica() {
        let lectura = LecturaPerfil {
            avisos: vec![leido(
                "https://www.facebook.com/marketplace/item/123456789012/",
                "Casa en Los Samanes",
            )],
        };
        let avisos = validar_lectura(lectura).unwrap();
        assert_eq!(avisos[0].aviso_id, "123456789012");
        assert_eq!(
            avisos[0].url,
            "https://www.facebook.com/marketplace/item/123456789012"
        );
    }

    #[test]
    fn ids_repetidos_cuentan_una_vez() {
        let lectura = LecturaPerfil {
            avisos: vec![
                leido("123456789012", "Casa en Los Samanes"),
                leido("123456789012", "Casa en Los Samanes (copia)"),
            ],
        };
        let avisos = validar_lectura(lectura).unwrap();
        assert_eq!(avisos.len(), 1);
        assert_eq!(avisos[0].titulo, "Casa en Los Samanes");
    }

    #[test]
    fn candidato_que_comparte_mas_palabras_va_primero() {
        let mas_parecido = inmueble("Casa Los Samanes con piscina y jardín");
        let lejano = inmueble("Apartamento Centro");
        let orden = ordenar_candidatos(
            "Casa Los Samanes con piscina",
            &[lejano, mas_parecido.clone()],
        );
        assert_eq!(orden[0].inmueble_id, mas_parecido.id);
        assert!(orden[0].palabras_comunes > orden[1].palabras_comunes);
    }

    fn pool_si_hay() -> Option<sqlx::PgPool> {
        let url = crate::config::url_bd_pruebas()?;
        sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy(&url)
            .ok()
    }

    fn aviso_nuevo(id: &str) -> AvisoPerfilNuevo {
        AvisoPerfilNuevo {
            aviso_id: id.to_string(),
            titulo: format!("Aviso {id}"),
            precio: None,
            url: format!("https://www.facebook.com/marketplace/item/{id}"),
        }
    }

    /// Contra la BD de rama (si la hay): una lectura reemplaza el estado. Lo que
    /// deja de venir pasa a `presente = false` sin borrarse y no vuelve a listarse.
    #[tokio::test]
    async fn lectura_reemplaza_y_marca_ausentes() {
        let Some(pool) = pool_si_hay() else { return };
        let (a, b) = ("913013000000001", "913013000000002");
        let ids = vec![a.to_string(), b.to_string()];
        let limpiar = "DELETE FROM marketplace_avisos_perfil WHERE aviso_id = ANY($1)";
        sqlx::query(limpiar).bind(&ids).execute(&pool).await.unwrap();

        reemplazar_avisos_perfil(&pool, &[aviso_nuevo(a), aviso_nuevo(b)])
            .await
            .unwrap();
        let antes: Vec<String> = avisos_perfil_sin_vincular(&pool)
            .await
            .unwrap()
            .into_iter()
            .map(|f| f.aviso_id)
            .collect();
        assert!(antes.contains(&a.to_string()) && antes.contains(&b.to_string()));

        let ausentes = reemplazar_avisos_perfil(&pool, &[aviso_nuevo(a)])
            .await
            .unwrap();
        assert!(ausentes >= 1, "b dejó de venir y debe contarse como ausente");
        let despues: Vec<String> = avisos_perfil_sin_vincular(&pool)
            .await
            .unwrap()
            .into_iter()
            .map(|f| f.aviso_id)
            .collect();
        assert!(despues.contains(&a.to_string()));
        assert!(!despues.contains(&b.to_string()));

        let presente_b: bool = sqlx::query_scalar(
            "SELECT presente FROM marketplace_avisos_perfil WHERE aviso_id = $1",
        )
        .bind(b)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(!presente_b, "el aviso ausente se guarda con presente = false");

        sqlx::query(limpiar).bind(&ids).execute(&pool).await.unwrap();
    }
}
