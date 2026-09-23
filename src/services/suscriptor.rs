use sqlx::PgPool;
use validator::Validate;

use crate::errors::AppError;
use crate::models::{CreateSuscriptorRequest, Suscriptor};
use crate::repositories::{NuevoSuscriptor, SuscriptorRepository};

/* [189A-1] Lógica de suscripción: valida el formato, normaliza el correo
 * (trim + minúsculas, para que el UNIQUE no distinga mayúsculas) y guarda
 * con upsert idempotente. */

pub struct SuscriptorService;

impl SuscriptorService {
    /// Suscribe un correo a las novedades (re-suscribir no duplica).
    pub async fn suscribir(
        pool: &PgPool,
        mut req: CreateSuscriptorRequest,
    ) -> Result<Suscriptor, AppError> {
        /* Normalizar antes de validar: el formato se comprueba ya sobre
         * el valor que se guarda (sin espacios ni mayúsculas). */
        req.email = req.email.trim().to_lowercase();
        req.validate()
            .map_err(|e| AppError::Validation(e.to_string()))?;

        let nuevo = NuevoSuscriptor { email: &req.email };
        let row = SuscriptorRepository::suscribir(pool, &nuevo).await?;
        Ok(Suscriptor::from_row(row))
    }
}
