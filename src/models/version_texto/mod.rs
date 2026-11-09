//! [30AA-1] Versiones de texto (v2 en adelante) e historial de cambios.
//!
//! La v1 no se guarda aquí: sigue viviendo en `inmuebles.titulo`/`descripcion`,
//! que es lo que sirve la web pública. Esta tabla solo tiene las variantes que
//! se publican en otro Facebook.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

/// Estados de una variante: nace en borrador y solo pasa a `lista` cuando la
/// dueña confirma que no inventó datos.
pub const ESTADOS_VERSION: &[&str] = &["borrador", "lista"];

/// Fila de `inmueble_versiones`
#[derive(Debug, Clone, FromRow, Serialize, ToSchema)]
pub struct InmuebleVersion {
    pub id: Uuid,
    pub inmueble_id: Uuid,
    pub version: i16,
    pub titulo: String,
    pub descripcion: String,
    pub estado: String,
    pub modelo: String,
    pub prompt_usado: String,
    pub creada_en: DateTime<Utc>,
    pub actualizada_en: DateTime<Utc>,
}

/// Entrada del historial de textos
#[derive(Debug, Clone, FromRow, Serialize, ToSchema)]
pub struct TextoHistorial {
    pub id: Uuid,
    pub version: i16,
    pub campo: String,
    pub valor_anterior: String,
    pub valor_nuevo: String,
    pub origen: String,
    pub modelo: String,
    pub creado_en: DateTime<Utc>,
}

/// Todo lo que necesita la pestaña de versiones del modal
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct VersionesInmueble {
    /// Cuántas versiones exige la configuración (2 por defecto).
    pub configuradas: i16,
    /// v1 tal como está hoy en la ficha (es lo que ve el público).
    pub titulo_v1: String,
    pub descripcion_v1: String,
    pub variantes: Vec<InmuebleVersion>,
    /// Fotografías variante ya generadas, por versión y `orden` del original.
    pub fotos: Vec<FotoVariante>,
    pub historial: Vec<TextoHistorial>,
}

/// Variante IA de una foto (`fotos_variantes`)
#[derive(Debug, Clone, FromRow, Serialize, ToSchema)]
pub struct FotoVarianteRow {
    pub id: Uuid,
    pub inmueble_id: Uuid,
    pub orden: i32,
    pub storage_key: String,
    pub version: i16,
    pub modelo: String,
    pub prompt_usado: String,
    pub creada_en: DateTime<Utc>,
}

/// Variante tal como la expone la API (URL en vez de clave interna)
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FotoVariante {
    pub id: Uuid,
    pub orden: i32,
    pub version: i16,
    pub url: String,
    pub creada_en: DateTime<Utc>,
}

impl From<FotoVarianteRow> for FotoVariante {
    fn from(f: FotoVarianteRow) -> Self {
        Self {
            id: f.id,
            orden: f.orden,
            version: f.version,
            url: crate::models::url_publica_de_foto(&f.storage_key),
            creada_en: f.creada_en,
        }
    }
}

/// Petición de generación de una variante (`POST /inmuebles/:id/versiones`)
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct GenerarVersionRequest {
    /// Número de versión; ausente = la siguiente libre (v2 la primera vez).
    #[serde(default)]
    pub version: Option<i16>,
    /// Instrucción extra de la dueña ('' = solo el criterio por defecto).
    #[serde(default)]
    #[validate(length(max = 1000, message = "La instrucción no debe exceder 1000 caracteres"))]
    pub instruccion: String,
}

/// Edición manual de una variante (`PUT /inmuebles/:id/versiones/:n`)
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct EditarVersionRequest {
    #[serde(default)]
    #[validate(length(max = 500, message = "El título no debe exceder 500 caracteres"))]
    pub titulo: Option<String>,
    #[serde(default)]
    #[validate(length(max = 20000, message = "La descripción no debe exceder 20000 caracteres"))]
    pub descripcion: Option<String>,
    /// `borrador` o `lista`; ausente = no tocar.
    #[serde(default)]
    pub estado: Option<String>,
}

/// Resultado de generar una variante con IA
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GenerarVersionSalida {
    pub ok: bool,
    pub version: Option<InmuebleVersion>,
    pub modelo: Option<String>,
    pub motivos: Vec<String>,
}
