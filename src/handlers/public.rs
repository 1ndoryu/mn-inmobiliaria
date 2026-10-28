use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::{Json, Router};

use crate::errors::AppError;
use crate::models::{FiltrosPublicos, Inmueble, PaginatedInmuebles, PublicidadPublica};
use crate::services::{InmuebleService, PublicidadService};
use crate::AppState;

use super::comercial::{solicitud, suscriptor};

/* [159A-1] Web pública de solo lectura: solo inmuebles con `publicado = TRUE`.
 * Sin JWT: la visibilidad la decide el backend, no el cliente. */

/// Listar inmuebles publicados con filtros completos
#[utoipa::path(
    get,
    path = "/api/public/inmuebles",
    params(FiltrosPublicos),
    responses(
        (status = 200, description = "Inmuebles publicados", body = PaginatedInmuebles),
        (status = 422, description = "Filtro inválido", body = crate::errors::ErrorResponse)
    )
)]
pub async fn list_public(
    State(state): State<AppState>,
    Query(filtros): Query<FiltrosPublicos>,
) -> Result<Json<PaginatedInmuebles>, AppError> {
    let lista = InmuebleService::list_public(&state.pool, filtros).await?;
    Ok(Json(lista))
}

/// Detalle público por slug (404 si no está publicado)
#[utoipa::path(
    get,
    path = "/api/public/inmuebles/{slug}",
    params(("slug" = String, Path, description = "Slug del inmueble")),
    responses(
        (status = 200, description = "Inmueble publicado", body = Inmueble),
        (status = 404, description = "No encontrado o no publicado", body = crate::errors::ErrorResponse)
    )
)]
pub async fn get_public(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<Inmueble>, AppError> {
    let inmueble = InmuebleService::get_public(&state.pool, &slug).await?;
    Ok(Json(inmueble))
}

/* [28AA-7] Página pública `/publicidad`: publicaciones de redes ya salidas, una
 * por inmueble visible. Solo lectura, sin JWT, como el resto de esta web. */
/// Publicaciones ya salidas para la página `/publicidad`, la más reciente primero
#[utoipa::path(
    get,
    path = "/api/public/publicidad",
    responses(
        (status = 200, description = "Publicaciones salidas de inmuebles publicados", body = Vec<PublicidadPublica>)
    )
)]
pub async fn list_publicidad(
    State(state): State<AppState>,
) -> Result<Json<Vec<PublicidadPublica>>, AppError> {
    let lista = PublicidadService::listar(&state.pool).await?;
    Ok(Json(lista))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/inmuebles", get(list_public))
        .route("/inmuebles/:slug", get(get_public))
        .route("/publicidad", get(list_publicidad))
        .merge(solicitud::public_routes())
        .merge(suscriptor::public_routes())
}
