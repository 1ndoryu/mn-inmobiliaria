use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use uuid::Uuid;

use crate::errors::AppError;
use crate::services::{AuthService, Claims};
use crate::AppState;

/// Valida el `Authorization: Bearer` y devuelve los claims del JWT.
fn claims_del_header(parts: &Parts, state: &AppState) -> Result<Claims, AppError> {
    let auth_header = parts
        .headers
        .get("Authorization")
        .and_then(|value| value.to_str().ok())
        .ok_or(AppError::Unauthorized)?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(AppError::Unauthorized)?;

    AuthService::verify_token(token, &state.jwt_secret)
}

/// Extractor que valida el JWT del header Authorization y extrae el `user_id`.
/// Usar como parámetro en handlers que requieren autenticación.
pub struct AuthUser {
    pub user_id: Uuid,
}

#[async_trait]
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let claims = claims_del_header(parts, state)?;

        Ok(Self {
            user_id: claims.sub,
        })
    }
}

/// [09AA-9] Extractor de la consola staff: además de JWT válido exige un rol
/// staff (owner/admin) en el claim `role`. Sin `role` (token anterior a
/// 09AA-9) responde 401 para forzar un nuevo login; con otro rol, 403.
pub struct AuthStaff {
    pub user_id: Uuid,
}

#[async_trait]
impl FromRequestParts<AppState> for AuthStaff {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let claims = claims_del_header(parts, state)?;

        if claims.role.is_empty() {
            return Err(AppError::Unauthorized);
        }
        if !AuthService::es_rol_staff(&claims.role) {
            return Err(AppError::Forbidden(
                "La consola staff requiere rol owner o admin".into(),
            ));
        }

        Ok(Self {
            user_id: claims.sub,
        })
    }
}
