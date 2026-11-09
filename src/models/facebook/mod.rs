//! [30AA-1] Facebook como entidad: el mismo inmueble se publica en varias
//! cuentas y cada una tiene su propio aviso de Marketplace.
//!
//! `inmuebles.marketplace_id` sigue existiendo y se mantiene como espejo del
//! aviso de la cuenta principal: es lo que leen el laboratorio de Marketplace
//! y el chat, y quitarlo obligaría a tocar medio proyecto por un dato que ya
//! está donde se espera. La fuente de verdad de los vínculos es
//! `inmueble_facebook`; la columna es una copia derivada.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

/// Fila de `facebook`
#[derive(Debug, Clone, FromRow)]
pub struct FacebookRow {
    pub id: Uuid,
    pub nombre: String,
    pub url: String,
    pub notas: String,
    pub principal: bool,
    pub creado_en: DateTime<Utc>,
}

/// Cuenta de Facebook tal como la expone la API
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Facebook {
    pub id: Uuid,
    pub nombre: String,
    /// Perfil o página (`''` = sin URL; la cuenta existe igual).
    pub url: String,
    pub notas: String,
    /// Cuenta por defecto: su aviso es el que refleja `inmuebles.marketplace_id`.
    pub principal: bool,
    /// Cuántos inmuebles tiene vinculados (lo pinta la lista de cuentas).
    pub inmuebles: i64,
    pub creado_en: DateTime<Utc>,
}

impl Facebook {
    #[must_use]
    pub fn from_row(row: FacebookRow, inmuebles: i64) -> Self {
        Self {
            id: row.id,
            nombre: row.nombre,
            url: row.url,
            notas: row.notas,
            principal: row.principal,
            inmuebles,
            creado_en: row.creado_en,
        }
    }
}

/// Alta o edición de una cuenta (`PUT` reemplaza los cuatro campos)
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct FacebookRequest {
    #[validate(length(
        min = 1,
        max = 120,
        message = "El nombre debe tener entre 1 y 120 caracteres"
    ))]
    pub nombre: String,
    #[serde(default)]
    #[validate(length(max = 500, message = "La URL no debe exceder 500 caracteres"))]
    pub url: String,
    #[serde(default)]
    #[validate(length(max = 2000, message = "Las notas no deben exceder 2000 caracteres"))]
    pub notas: String,
    /// Marca la cuenta como principal; desmarca cualquier otra (BD lo impone).
    #[serde(default)]
    pub principal: bool,
}

/// Vínculo de un inmueble con una cuenta: lo que pinta el modal
#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct VinculoFacebook {
    pub facebook_id: Uuid,
    pub nombre: String,
    pub url: String,
    pub principal: bool,
    /// Dígitos del aviso en esa cuenta (`None` = cuenta vinculada sin aviso).
    pub marketplace_id: Option<String>,
    pub publicado_en: Option<DateTime<Utc>>,
}

/// Fijar el vínculo de un inmueble con una cuenta
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct VincularFacebookRequest {
    /// `None`/ausente = cuenta vinculada sin aviso; `""` = borrar el aviso.
    /// Un texto con URL o dígitos se normaliza igual que `marketplace_id`.
    #[serde(default)]
    pub marketplace_id: Option<String>,
}
