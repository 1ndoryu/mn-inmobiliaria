use sqlx::PgPool;
use uuid::Uuid;

use crate::models::Note;

pub struct NoteRepository;

impl NoteRepository {
    pub async fn create(
        pool: &PgPool,
        user_id: Uuid,
        title: &str,
        content: &str,
    ) -> Result<Note, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as!(
            Note,
            "INSERT INTO notes (id, user_id, title, content) \
             VALUES ($1, $2, $3, $4) \
             RETURNING id, user_id, title, content, created_at, updated_at",
            id,
            user_id,
            title,
            content
        )
        .fetch_one(pool)
        .await
    }

    pub async fn find_by_id(
        pool: &PgPool,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<Note>, sqlx::Error> {
        sqlx::query_as!(
            Note,
            "SELECT id, user_id, title, content, created_at, updated_at \
             FROM notes WHERE id = $1 AND user_id = $2",
            id,
            user_id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn list(
        pool: &PgPool,
        user_id: Uuid,
        page: i64,
        per_page: i64,
    ) -> Result<(Vec<Note>, i64), sqlx::Error> {
        let offset = (page - 1) * per_page;

        let notes = sqlx::query_as!(
            Note,
            "SELECT id, user_id, title, content, created_at, updated_at \
             FROM notes WHERE user_id = $1 \
             ORDER BY created_at DESC LIMIT $2 OFFSET $3",
            user_id,
            per_page,
            offset
        )
        .fetch_all(pool)
        .await?;

        let (total,): (i64,) = sqlx::query!(
            "SELECT COUNT(*) AS \"total!\" FROM notes WHERE user_id = $1",
            user_id
        )
        .fetch_one(pool)
        .await
        .map(|r| (r.total,))?;

        Ok((notes, total))
    }

    pub async fn update(
        pool: &PgPool,
        id: Uuid,
        user_id: Uuid,
        title: Option<&str>,
        content: Option<&str>,
    ) -> Result<Option<Note>, sqlx::Error> {
        sqlx::query_as!(
            Note,
            "UPDATE notes \
             SET title = COALESCE($1, title), \
                 content = COALESCE($2, content), \
                 updated_at = NOW() \
             WHERE id = $3 AND user_id = $4 \
             RETURNING id, user_id, title, content, created_at, updated_at",
            title,
            content,
            id,
            user_id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn delete(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query!(
            "DELETE FROM notes WHERE id = $1 AND user_id = $2",
            id,
            user_id
        )
        .execute(pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }
}
