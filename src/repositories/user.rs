use sqlx::PgPool;
use uuid::Uuid;

use crate::models::User;

pub struct UserRepository;

impl UserRepository {
    /// Crea un usuario con rol y retorna el registro completo
    pub async fn create(
        pool: &PgPool,
        email: &str,
        password_hash: &str,
        role: &str,
    ) -> Result<User, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as::<_, User>(
            "INSERT INTO users (id, email, password_hash, role) \
             VALUES ($1, $2, $3, $4) \
             RETURNING id, email, password_hash, role, created_at",
        )
        .bind(id)
        .bind(email)
        .bind(password_hash)
        .bind(role)
        .fetch_one(pool)
        .await
    }

    /// Número total de usuarios (para bootstrap del owner)
    pub async fn count(pool: &PgPool) -> Result<i64, sqlx::Error> {
        let (total,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
            .fetch_one(pool)
            .await?;
        Ok(total)
    }

    /// Busca un usuario por email
    pub async fn find_by_email(pool: &PgPool, email: &str) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            "SELECT id, email, password_hash, role, created_at FROM users WHERE email = $1",
        )
        .bind(email)
        .fetch_optional(pool)
        .await
    }

    /// Busca un usuario por ID
    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            "SELECT id, email, password_hash, role, created_at FROM users WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(pool)
        .await
    }
}
