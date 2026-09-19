use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{AuthResponse, LoginRequest, RegisterRequest, User};
use crate::repositories::UserRepository;

/// Claims del JWT — `sub` es el `user_id`, `exp` la expiración Unix
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub exp: usize,
}

pub struct AuthService;

impl AuthService {
    fn hash_password(password: &str) -> Result<String, AppError> {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|e| AppError::Internal(format!("Error al hashear contraseña: {e}")))
    }

    /// Registro bootstrap: solo crea el primer usuario (owner).
    /// A partir de ahí el registro público queda cerrado y los owners
    /// dan de alta con `create_user`.
    pub async fn register(
        pool: &PgPool,
        req: RegisterRequest,
        jwt_secret: &str,
    ) -> Result<AuthResponse, AppError> {
        if UserRepository::count(pool).await? > 0 {
            return Err(AppError::Forbidden(
                "Registro cerrado: pide a un owner que cree tu cuenta".into(),
            ));
        }

        if UserRepository::find_by_email(pool, &req.email)
            .await?
            .is_some()
        {
            return Err(AppError::Conflict("Email ya registrado".into()));
        }

        let password_hash = Self::hash_password(&req.password)?;
        let user = UserRepository::create(pool, &req.email, &password_hash, "owner").await?;
        let token = Self::generate_token(user.id, jwt_secret)?;

        Ok(AuthResponse {
            token,
            user_id: user.id,
        })
    }

    /// Alta de admin por un owner
    pub async fn create_user(
        pool: &PgPool,
        owner_id: Uuid,
        email: &str,
        password: &str,
    ) -> Result<User, AppError> {
        Self::require_owner(pool, owner_id).await?;

        if UserRepository::find_by_email(pool, email).await?.is_some() {
            return Err(AppError::Conflict("Email ya registrado".into()));
        }

        let password_hash = Self::hash_password(password)?;
        Ok(UserRepository::create(pool, email, &password_hash, "admin").await?)
    }

    /// Verifica que el usuario sea owner; si no, 401/403
    pub async fn require_owner(pool: &PgPool, user_id: Uuid) -> Result<User, AppError> {
        let user = UserRepository::find_by_id(pool, user_id)
            .await?
            .ok_or(AppError::Unauthorized)?;
        if user.role != "owner" {
            return Err(AppError::Forbidden("Solo un owner puede hacer esto".into()));
        }
        Ok(user)
    }

    /// Inicia sesión: verifica credenciales y genera JWT
    pub async fn login(
        pool: &PgPool,
        req: LoginRequest,
        jwt_secret: &str,
    ) -> Result<AuthResponse, AppError> {
        let user = UserRepository::find_by_email(pool, &req.email)
            .await?
            .ok_or(AppError::Unauthorized)?;

        let parsed_hash = PasswordHash::new(&user.password_hash)
            .map_err(|e| AppError::Internal(format!("Hash almacenado inválido: {e}")))?;

        Argon2::default()
            .verify_password(req.password.as_bytes(), &parsed_hash)
            .map_err(|_| AppError::Unauthorized)?;

        let token = Self::generate_token(user.id, jwt_secret)?;

        Ok(AuthResponse {
            token,
            user_id: user.id,
        })
    }

    /* [199A-3] La sesion del admin dura 10 años: app local de un solo
     * admin, el front guarda el token en localStorage y solo se sale con
     * "Salir" o secreto rotado (`JWT_SECRET` estable en `.env`, gitignored).
     * Tras este cambio hay que entrar una vez para renovar el token. */
    /// Genera un JWT con expiración de 10 años (3650 días)
    pub fn generate_token(user_id: Uuid, secret: &str) -> Result<String, AppError> {
        let timestamp = chrono::Utc::now()
            .checked_add_signed(chrono::Duration::days(3650))
            .ok_or_else(|| AppError::Internal("Error calculando expiración del token".into()))?
            .timestamp();
        let exp = usize::try_from(timestamp)
            .map_err(|_| AppError::Internal("Timestamp fuera de rango".into()))?;

        let claims = Claims { sub: user_id, exp };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .map_err(|e| AppError::Internal(format!("Error generando token: {e}")))
    }

    /// Verifica un JWT y retorna los claims
    pub fn verify_token(token: &str, secret: &str) -> Result<Claims, AppError> {
        decode::<Claims>(
            token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &Validation::default(),
        )
        .map(|data| data.claims)
        .map_err(|_| AppError::Unauthorized)
    }
}
