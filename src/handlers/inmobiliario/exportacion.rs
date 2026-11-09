//! [30AA-1] Manifiesto de exportación para la app de escritorio.

use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::services::exportacion::{ExportService, ManifiestoExport};
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct ExportParams {
    /// Cuenta de Facebook; ausente = la principal.
    pub facebook: Option<Uuid>,
    /// Solo una versión (`1` o `2`); ausente = las dos.
    pub version: Option<i16>,
}

/// Manifiesto de exportación: qué inmuebles y fotos bajar, con su texto
#[utoipa::path(
    get,
    path = "/api/admin/export/inmuebles",
    params(
        ("facebook" = Option<Uuid>, Query, description = "Cuenta (defecto: la principal)"),
        ("version" = Option<i16>, Query, description = "Solo esa versión")
    ),
    responses(
        (status = 200, description = "Manifiesto", body = ManifiestoExport),
        (status = 404, description = "Cuenta no encontrada", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn manifiesto(
    State(state): State<AppState>,
    _auth: AuthUser,
    Query(params): Query<ExportParams>,
) -> Result<Json<ManifiestoExport>, AppError> {
    if let Some(v) = params.version {
        if v != 1 && v != 2 {
            return Err(AppError::Validation("version debe ser 1 o 2".into()));
        }
    }
    Ok(Json(
        ExportService::manifiesto(&state.pool, params.facebook, params.version).await?,
    ))
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/export/inmuebles", get(manifiesto))
}
