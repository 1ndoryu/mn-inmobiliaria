use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, Router};

use crate::errors::AppError;
use crate::models::{CreateSuscriptorRequest, Suscriptor};
use crate::services::SuscriptorService;
use crate::AppState;

/* [189A-1] Suscripción del pie público: sin JWT, alta idempotente.
 * La gestión (listar/revisar) queda pendiente del panel admin. */

/// Suscribir un correo a las novedades (re-suscribir no duplica)
#[utoipa::path(
    post,
    path = "/api/public/suscriptores",
    request_body = CreateSuscriptorRequest,
    responses(
        (status = 201, description = "Correo suscrito", body = Suscriptor),
        (status = 422, description = "Error de validación", body = crate::errors::ErrorResponse)
    )
)]
pub async fn suscribir(
    State(state): State<AppState>,
    Json(req): Json<CreateSuscriptorRequest>,
) -> Result<(StatusCode, Json<Suscriptor>), AppError> {
    let suscriptor = SuscriptorService::suscribir(&state.pool, req).await?;
    Ok((StatusCode::CREATED, Json(suscriptor)))
}

pub fn public_routes() -> Router<AppState> {
    Router::new().route("/suscriptores", post(suscribir))
}
