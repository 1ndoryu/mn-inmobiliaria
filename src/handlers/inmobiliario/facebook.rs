//! [30AA-1] Rutas admin de las cuentas de Facebook y del vínculo por inmueble.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, put};
use axum::{Json, Router};
use uuid::Uuid;
use validator::Validate;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{Facebook, FacebookRequest, VinculoFacebook, VincularFacebookRequest};
use crate::services::facebook::FacebookService;
use crate::AppState;

/// Listar las cuentas de Facebook con su número de inmuebles
#[utoipa::path(
    get,
    path = "/api/admin/facebook",
    responses(
        (status = 200, description = "Cuentas", body = Vec<Facebook>),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn listar(
    State(state): State<AppState>,
    _auth: AuthUser,
) -> Result<Json<Vec<Facebook>>, AppError> {
    Ok(Json(FacebookService::listar(&state.pool).await?))
}

/// Crear una cuenta de Facebook
#[utoipa::path(
    post,
    path = "/api/admin/facebook",
    request_body = FacebookRequest,
    responses(
        (status = 201, description = "Cuenta creada", body = Facebook),
        (status = 422, description = "Error de validación", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn crear(
    State(state): State<AppState>,
    _auth: AuthUser,
    Json(req): Json<FacebookRequest>,
) -> Result<(StatusCode, Json<Facebook>), AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    Ok((StatusCode::CREATED, Json(FacebookService::crear(&state.pool, req).await?)))
}

/// Editar una cuenta (marcar `principal` desmarca las demás)
#[utoipa::path(
    put,
    path = "/api/admin/facebook/{id}",
    request_body = FacebookRequest,
    params(("id" = Uuid, Path, description = "Cuenta")),
    responses(
        (status = 200, description = "Cuenta actualizada", body = Facebook),
        (status = 404, description = "No encontrada", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn actualizar(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<FacebookRequest>,
) -> Result<Json<Facebook>, AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    Ok(Json(FacebookService::actualizar(&state.pool, id, req).await?))
}

/// Borrar una cuenta (sus inmuebles quedan desvinculados)
#[utoipa::path(
    delete,
    path = "/api/admin/facebook/{id}",
    params(("id" = Uuid, Path, description = "Cuenta")),
    responses(
        (status = 204, description = "Borrada"),
        (status = 404, description = "No encontrada", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn borrar(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    FacebookService::borrar(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Vínculos de un inmueble con sus cuentas de Facebook
#[utoipa::path(
    get,
    path = "/api/admin/inmuebles/{id}/facebook",
    params(("id" = Uuid, Path, description = "Inmueble")),
    responses(
        (status = 200, description = "Vínculos", body = Vec<VinculoFacebook>),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn vinculos(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<VinculoFacebook>>, AppError> {
    Ok(Json(FacebookService::vinculos(&state.pool, id).await?))
}

/// Vincular el inmueble a una cuenta, con su aviso de Marketplace
#[utoipa::path(
    put,
    path = "/api/admin/inmuebles/{id}/facebook/{cuenta}",
    request_body = VincularFacebookRequest,
    params(
        ("id" = Uuid, Path, description = "Inmueble"),
        ("cuenta" = Uuid, Path, description = "Cuenta de Facebook")
    ),
    responses(
        (status = 200, description = "Vínculos actualizados", body = Vec<VinculoFacebook>),
        (status = 422, description = "Aviso inválido o ya vinculado", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn vincular(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path((id, cuenta)): Path<(Uuid, Uuid)>,
    Json(req): Json<VincularFacebookRequest>,
) -> Result<Json<Vec<VinculoFacebook>>, AppError> {
    let v = FacebookService::vincular(
        &state.pool,
        id,
        cuenta,
        req.marketplace_id.as_deref(),
    )
    .await?;
    Ok(Json(v))
}

/// Desvincular el inmueble de una cuenta
#[utoipa::path(
    delete,
    path = "/api/admin/inmuebles/{id}/facebook/{cuenta}",
    params(
        ("id" = Uuid, Path, description = "Inmueble"),
        ("cuenta" = Uuid, Path, description = "Cuenta de Facebook")
    ),
    responses(
        (status = 200, description = "Vínculos actualizados", body = Vec<VinculoFacebook>),
        (status = 404, description = "No estaba vinculado", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn desvincular(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path((id, cuenta)): Path<(Uuid, Uuid)>,
) -> Result<Json<Vec<VinculoFacebook>>, AppError> {
    Ok(Json(
        FacebookService::desvincular(&state.pool, id, cuenta).await?,
    ))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/facebook", get(listar).post(crear))
        .route("/facebook/:id", put(actualizar).delete(borrar))
        .route("/inmuebles/:id/facebook", get(vinculos))
        .route(
            "/inmuebles/:id/facebook/:cuenta",
            put(vincular).delete(desvincular),
        )
}
