use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use validator::Validate;

/* [159A-1] Catálogo de inmuebles: fila plana (FromRow) + vista con fotos.
 * Sin obligatorios: crear admite payload vacío (todo con DEFAULT).
 * Enums como texto validado contra allowlists en el servicio. */

/// Valores permitidos para `tipo`
pub const TIPOS: &[&str] = &["piso", "casa", "local", "terreno", "townhouse", "otro"];
/// Valores permitidos para `operacion`
pub const OPERACIONES: &[&str] = &["venta", "alquiler"];
/// Valores permitidos para `estado`
pub const ESTADOS: &[&str] = &["disponible", "reservado", "vendido", "alquilado"];
/// Valores permitidos para `origen` de foto
pub const ORIGENES_FOTO: &[&str] = &["original", "mejorada"];

/// Fila de `inmuebles` tal cual la devuelve Postgres
#[derive(Debug, Clone, FromRow)]
pub struct InmuebleRow {
    pub id: Uuid,
    pub titulo: String,
    pub descripcion: String,
    pub ubicacion: String,
    pub precio: f64,
    pub tipo: String,
    pub operacion: String,
    pub habitaciones: i32,
    pub banos: i32,
    pub metros: f64,
    pub metros_terreno: f64,
    pub estado: String,
    pub publicado: bool,
    pub slug: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Inmueble con sus fotos — lo que expone la API
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Inmueble {
    pub id: Uuid,
    pub titulo: String,
    pub descripcion: String,
    pub ubicacion: String,
    pub precio: f64,
    pub tipo: String,
    pub operacion: String,
    pub habitaciones: i32,
    pub banos: i32,
    pub metros: f64,
    pub metros_terreno: f64,
    pub estado: String,
    pub publicado: bool,
    pub slug: String,
    pub fotos: Vec<Foto>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Inmueble {
    /// Ensambla la vista a partir de la fila y sus fotos ya ordenadas
    #[must_use]
    pub fn from_row(row: InmuebleRow, fotos: Vec<Foto>) -> Self {
        Self {
            id: row.id,
            titulo: row.titulo,
            descripcion: row.descripcion,
            ubicacion: row.ubicacion,
            precio: row.precio,
            tipo: row.tipo,
            operacion: row.operacion,
            habitaciones: row.habitaciones,
            banos: row.banos,
            metros: row.metros,
            metros_terreno: row.metros_terreno,
            estado: row.estado,
            publicado: row.publicado,
            slug: row.slug,
            fotos,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// Foto almacenada en `fotos` (`storage_key` = clave en el volumen de uploads)
#[derive(Debug, Clone, FromRow, Serialize, ToSchema)]
pub struct Foto {
    pub id: Uuid,
    pub inmueble_id: Uuid,
    pub storage_key: String,
    pub orden: i32,
    pub origen: String,
    pub created_at: DateTime<Utc>,
}

fn default_tipo() -> String {
    "otro".to_string()
}

fn default_operacion() -> String {
    "venta".to_string()
}

fn default_estado() -> String {
    "disponible".to_string()
}

/// Alta de inmueble — sin campos obligatorios
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateInmuebleRequest {
    #[serde(default)]
    #[validate(length(max = 500, message = "El título no debe exceder 500 caracteres"))]
    pub titulo: String,
    #[serde(default)]
    #[validate(length(
        max = 20000,
        message = "La descripción no debe exceder 20000 caracteres"
    ))]
    pub descripcion: String,
    #[serde(default)]
    #[validate(length(max = 500, message = "La ubicación no debe exceder 500 caracteres"))]
    pub ubicacion: String,
    #[serde(default)]
    #[validate(range(min = 0.0, message = "El precio no puede ser negativo"))]
    pub precio: f64,
    #[serde(default = "default_tipo")]
    pub tipo: String,
    #[serde(default = "default_operacion")]
    pub operacion: String,
    #[serde(default)]
    #[validate(range(min = 0, message = "Habitaciones no puede ser negativo"))]
    pub habitaciones: i32,
    #[serde(default)]
    #[validate(range(min = 0, message = "Baños no puede ser negativo"))]
    pub banos: i32,
    #[serde(default)]
    #[validate(range(min = 0.0, message = "Los metros no pueden ser negativos"))]
    pub metros: f64,
    #[serde(default)]
    #[validate(range(min = 0.0, message = "Los metros de terreno no pueden ser negativos"))]
    pub metros_terreno: f64,
    #[serde(default = "default_estado")]
    pub estado: String,
}

/// Actualización parcial de inmueble
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateInmuebleRequest {
    #[validate(length(max = 500))]
    pub titulo: Option<String>,
    #[validate(length(max = 20000))]
    pub descripcion: Option<String>,
    #[validate(length(max = 500))]
    pub ubicacion: Option<String>,
    #[validate(range(min = 0.0))]
    pub precio: Option<f64>,
    pub tipo: Option<String>,
    pub operacion: Option<String>,
    #[validate(range(min = 0))]
    pub habitaciones: Option<i32>,
    #[validate(range(min = 0))]
    pub banos: Option<i32>,
    #[validate(range(min = 0.0))]
    pub metros: Option<f64>,
    #[validate(range(min = 0.0))]
    pub metros_terreno: Option<f64>,
    pub estado: Option<String>,
}

/// Cambio de visibilidad pública — el backend decide qué se publica
#[derive(Debug, Deserialize, ToSchema)]
pub struct PublicacionRequest {
    pub publicado: bool,
}

/// Alta de foto en un inmueble
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct AddFotoRequest {
    #[validate(length(min = 1, max = 500, message = "storage_key requerido"))]
    pub storage_key: String,
    pub orden: Option<i32>,
    pub origen: Option<String>,
}

/// Filtros públicos + paginación
#[derive(Debug, Deserialize, IntoParams)]
pub struct FiltrosPublicos {
    /// Filtrar por tipo (piso, casa, local, terreno, townhouse, otro)
    pub tipo: Option<String>,
    /// Filtrar por operación (venta, alquiler)
    pub operacion: Option<String>,
    /// Precio mínimo
    pub precio_min: Option<f64>,
    /// Precio máximo
    pub precio_max: Option<f64>,
    /// Página (empezando en 1)
    #[serde(default = "default_page")]
    pub page: i64,
    /// Resultados por página
    #[serde(default = "default_per_page")]
    pub per_page: i64,
}

fn default_page() -> i64 {
    1
}

fn default_per_page() -> i64 {
    20
}

/// Response paginada de inmuebles
#[derive(Debug, Serialize, ToSchema)]
pub struct PaginatedInmuebles {
    pub items: Vec<Inmueble>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

/// Alta de usuario por un owner (register público = solo bootstrap)
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateUserRequest {
    #[validate(email(message = "Formato de email inválido"))]
    pub email: String,
    #[validate(length(min = 8, message = "La contraseña debe tener al menos 8 caracteres"))]
    pub password: String,
}
