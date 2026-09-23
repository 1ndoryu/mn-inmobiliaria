use sqlx::PgPool;
use uuid::Uuid;

use crate::models::SuscriptorRow;

/* [189A-1] Acceso a `suscriptores` con prepared statements.
 * El alta es un upsert atómico (sin buscar-crear secuencial): el correo
 * repetido re-suscribe (toca `updated_at`) en vez de duplicar o fallar. */

/// Correo ya normalizado listo para insertar
pub struct NuevoSuscriptor<'a> {
    pub email: &'a str,
}

pub struct SuscriptorRepository;

impl SuscriptorRepository {
    pub async fn suscribir(
        pool: &PgPool,
        nuevo: &NuevoSuscriptor<'_>,
    ) -> Result<SuscriptorRow, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as::<_, SuscriptorRow>(
            "INSERT INTO suscriptores (id, email) VALUES ($1, $2) \
             ON CONFLICT (email) DO UPDATE SET updated_at = NOW() \
             RETURNING id, email, created_at, updated_at",
        )
        .bind(id)
        .bind(nuevo.email)
        .fetch_one(pool)
        .await
    }
}
