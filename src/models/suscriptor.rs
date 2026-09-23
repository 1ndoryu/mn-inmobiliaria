use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

/* [189A-1] Suscriptores del pie público ("Suscríbete"): solo el correo.
 * El alta es idempotente (re-suscribir no duplica: UNIQUE + upsert).
 * Sin estado ni gestión: el panel admin queda pendiente. */

/// Fila de `suscriptores` tal cual la devuelve Postgres
#[derive(Debug, Clone, FromRow)]
pub struct SuscriptorRow {
    pub id: Uuid,
    pub email: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Suscriptor tal como lo expone la API
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Suscriptor {
    pub id: Uuid,
    pub email: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Suscriptor {
    #[must_use]
    pub fn from_row(row: SuscriptorRow) -> Self {
        Self {
            id: row.id,
            email: row.email,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// Alta pública de suscripción — solo el correo (el servicio lo normaliza)
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateSuscriptorRequest {
    #[validate(email(message = "El correo no tiene un formato válido"))]
    #[validate(length(max = 254, message = "El correo no debe exceder 254 caracteres"))]
    pub email: String,
}
