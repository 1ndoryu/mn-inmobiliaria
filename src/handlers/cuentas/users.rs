use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, Router};
use validator::Validate;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{CreateUserRequest, UserResponse};
use crate::services::AuthService;
use crate::AppState;

/* [159A-1] Altas de admins: solo un owner puede crear cuentas.
 * El register público sirve únicamente como bootstrap del primer owner. */

/// Crear un admin (solo owner)
#[utoipa::path(
    post,
    path = "/api/admin/users",
    request_body = CreateUserRequest,
    responses(
        (status = 201, description = "Admin creado", body = UserResponse),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse),
        (status = 403, description = "Solo owner", body = crate::errors::ErrorResponse),
        (status = 409, description = "Email ya registrado", body = crate::errors::ErrorResponse),
        (status = 422, description = "Error de validación", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<UserResponse>), AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let user =
        AuthService::create_user(&state.pool, auth.user_id, &req.email, &req.password).await?;
    Ok((StatusCode::CREATED, Json(UserResponse::from(user))))
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/users", post(create_user))
}
