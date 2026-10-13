use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::services::sync::detalle::{self, DetalleSync, DiferenciasSync};
use crate::services::sync::huella::Manifiesto;
use crate::services::sync::manifiesto;
use crate::services::sync::remoto::{self, Credenciales, ErrorRemoto};
use crate::services::sync::vigilancia::{self, EstadoSync};
use crate::AppState;

/* [13AA-12] Sincronización de inmuebles local ↔ servidor. Las rutas de
 * manifiesto y detalle las consume el otro entorno; estado, comprobar y
 * diferencias las usa el panel. La comparación vive en `services::sync`. */

/// Huellas de todos los inmuebles y sus fotos (sin valores privados)
#[utoipa::path(
    get,
    path = "/api/admin/sync/manifiesto",
    responses(
        (status = 200, description = "Huellas por inmueble", body = Manifiesto),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_manifiesto(
    State(state): State<AppState>,
    _auth: AuthUser,
) -> Result<Json<Manifiesto>, AppError> {
    Ok(Json(
        manifiesto::construir(&state.pool, &state.upload_dir).await?,
    ))
}

#[derive(Debug, Deserialize)]
pub struct EstadoQuery {
    /// `revision` que el cliente ya conoce: si coincide, espera el siguiente cambio.
    pub desde: Option<u64>,
}

/// Estado de sincronización. Con `desde` es long-poll: responde al cambiar la
/// `revision` o, tras unos 25 s, con el estado vigente.
#[utoipa::path(
    get,
    path = "/api/admin/sync/estado",
    params(
        ("desde" = Option<u64>, Query, description = "Revisión ya conocida por el cliente")
    ),
    responses(
        (status = 200, description = "Estado vigente", body = EstadoSync),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_estado(_auth: AuthUser, Query(q): Query<EstadoQuery>) -> Json<EstadoSync> {
    let estado = match q.desde {
        Some(desde) => vigilancia::esperar_cambio(desde).await,
        None => vigilancia::actual(),
    };
    Json((*estado).clone())
}

/// Pide consultar al servidor ya, sin esperar al intervalo. Seguir con `GET /sync/estado`.
#[utoipa::path(
    post,
    path = "/api/admin/sync/comprobar",
    responses(
        (status = 202, description = "Consulta encolada"),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn post_comprobar(_auth: AuthUser) -> StatusCode {
    vigilancia::comprobar_ya();
    StatusCode::ACCEPTED
}

/// Huella y valores canónicos de un inmueble de este entorno (incluye no publicados)
#[utoipa::path(
    get,
    path = "/api/admin/sync/inmuebles/{slug}",
    params(("slug" = String, Path, description = "Slug del inmueble")),
    responses(
        (status = 200, description = "Detalle del inmueble", body = DetalleSync),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse),
        (status = 404, description = "No existe", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_detalle(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(slug): Path<String>,
) -> Result<Json<DetalleSync>, AppError> {
    detalle::detalle_local(&state.pool, &slug)
        .await?
        .map(Json)
        .ok_or_else(|| AppError::NotFound(format!("inmueble {slug}")))
}

/// Diferencias de un inmueble con el servidor, con el valor de cada lado
#[utoipa::path(
    get,
    path = "/api/admin/sync/diferencias/{slug}",
    params(("slug" = String, Path, description = "Slug del inmueble")),
    responses(
        (status = 200, description = "Diferencias y quién va por delante", body = DiferenciasSync),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse),
        (status = 404, description = "No existe en este entorno", body = crate::errors::ErrorResponse),
        (status = 409, description = "Servidor no disponible o sin soporte", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_diferencias(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(slug): Path<String>,
) -> Result<Json<DiferenciasSync>, AppError> {
    let local = detalle::detalle_local(&state.pool, &slug)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("inmueble {slug}")))?;
    let cred = Credenciales::cargar().map_err(|e| error_remoto(&e))?;
    let remoto = remoto::detalle(&cred, &slug).await.map_err(|e| error_remoto(&e))?;
    let diferencias = detalle::diferencias(&state.pool, &local, remoto.as_ref()).await?;
    Ok(Json(diferencias))
}

/// Un fallo del servidor no es un error del panel: se muestra como conflicto con el motivo.
fn error_remoto(e: &ErrorRemoto) -> AppError {
    AppError::Conflict(e.to_string())
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/sync/manifiesto", get(get_manifiesto))
        .route("/sync/estado", get(get_estado))
        .route("/sync/comprobar", post(post_comprobar))
        .route("/sync/inmuebles/:slug", get(get_detalle))
        .route("/sync/diferencias/:slug", get(get_diferencias))
}
