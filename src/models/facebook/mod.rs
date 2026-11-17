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
    /// [30AA-13] Versión que se publica en ESTA cuenta (v1 = Maye, v2 = Wan).
    pub version: Option<i16>,
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
    /// [30AA-13] Versión de los avisos de esta cuenta (`None` = sin asignar).
    pub version: Option<i16>,
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
            version: row.version,
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
    /// [30AA-13] Versión que se publica aquí (1 = Maye, 2 = Wan). `None` = sin
    /// asignar; una versión solo puede pertenecer a una cuenta.
    #[serde(default)]
    pub version: Option<i16>,
}

/// Vínculo de un inmueble con una cuenta: lo que pinta el modal
#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct VinculoFacebook {
    pub facebook_id: Uuid,
    pub nombre: String,
    pub url: String,
    pub principal: bool,
    /// [30AA-7] Versión del anuncio a la que pertenece este aviso (1 = ficha).
    pub version: i16,
    /// Dígitos del aviso en esa cuenta (`None` = cuenta vinculada sin aviso).
    pub marketplace_id: Option<String>,
    pub publicado_en: Option<DateTime<Utc>>,
}

/// Versiones de un inmueble que todavía no tienen aviso en ninguna cuenta.
#[derive(Debug, Clone, Default, Serialize, ToSchema)]
pub struct FaltanAvisos {
    /// Números de versión sin ningún aviso vinculado.
    pub versiones: Vec<i16>,
    /// Cuántas son (`versiones.len()`, para no contarlas en el front).
    pub total: usize,
}

/// Fijar el vínculo de un inmueble con una cuenta
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct VincularFacebookRequest {
    /// `None`/ausente = cuenta vinculada sin aviso; `""` = borrar el aviso.
    /// Un texto con URL o dígitos se normaliza igual que `marketplace_id`.
    #[serde(default)]
    pub marketplace_id: Option<String>,
}
