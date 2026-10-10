use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, Router};
use validator::Validate;

use crate::errors::AppError;
use crate::models::{AuthResponse, LoginRequest, RegisterRequest};
use crate::services::AuthService;
use crate::AppState;

/// Registrar nuevo usuario — bootstrap: solo funciona si no hay usuarios (crea el owner)
#[utoipa::path(
    post,
    path = "/api/auth/register",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "Owner registrado", body = AuthResponse),
        (status = 403, description = "Registro cerrado tras el bootstrap", body = crate::errors::ErrorResponse),
        (status = 409, description = "Email ya registrado", body = crate::errors::ErrorResponse),
        (status = 422, description = "Error de validación", body = crate::errors::ErrorResponse)
    )
)]
pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let response = AuthService::register(&state.pool, req, &state.jwt_secret).await?;
    Ok((StatusCode::CREATED, Json(response)))
}

/// Iniciar sesión
#[utoipa::path(
    post,
    path = "/api/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login exitoso", body = AuthResponse),
        (status = 401, description = "Credenciales inválidas", body = crate::errors::ErrorResponse)
    )
)]
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let response = AuthService::login(&state.pool, req, &state.jwt_secret).await?;
    Ok(Json(response))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
}
