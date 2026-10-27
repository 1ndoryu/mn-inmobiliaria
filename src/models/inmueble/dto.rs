//! DTOs de petición del inmueble: alta, actualización parcial, fotos y ficha
//! /ask. Se separaron de `inmueble.rs` por la regla de tamaño de modelo.
use super::*;

/* [18AA-3] Tri-estado de verdad para el PUT parcial. `Option<Option<T>>` a
 * secas NO distingue «ausente» de `null`: serde mete los dos en `None`, así
 * que desvincular mandando `null` no hacía nada por HTTP (el servicio lo leía
 * como «no tocar»). Con `deserialize_with` el campo presente se lee siempre
 * (`null` → `Some(None)`) y el `default` del campo sigue cubriendo el ausente. */
fn doble_opcion<'de, D>(deserializador: D) -> Result<Option<Option<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Some(Option::<String>::deserialize(deserializador)?))
}

fn default_tipo() -> String {
    "apartamento".to_string()
}

fn default_operacion() -> String {
    "venta".to_string()
}

fn default_estado() -> String {
    "disponible".to_string()
}

/// Alta de inmueble — sin campos obligatorios
#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
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
    #[validate(range(min = 0, message = "Puestos no puede ser negativo"))]
    pub puestos: i32,
    #[serde(default)]
    #[validate(length(max = 500, message = "La residencia no debe exceder 500 caracteres"))]
    pub residencia: String,
    /* [18AA-6] Cómo se cita el inmueble en el chat del Marketplace. Vacío o
     * ausente = el servicio lo sugiere a partir de tipo, residencia y
     * ubicación, y lo guarda; a partir de ahí manda lo que venga aquí. */
    #[serde(default)]
    #[validate(length(max = 120, message = "El nombre corto no debe exceder 120 caracteres"))]
    pub nombre_corto: String,
    #[serde(default)]
    #[validate(range(min = 0.0, message = "El precio no puede ser negativo"))]
    pub precio: f64,
    /* [25AA-5] Canon mensual para `ambas`; en venta o alquiler queda en 0. */
    #[serde(default)]
    #[validate(range(min = 0.0, message = "El precio de alquiler no puede ser negativo"))]
    pub precio_alquiler: f64,
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
    /* [09AA-21] Vínculo exacto (`/marketplace/item/<id>` o dígitos).
     * `None`/ausente = sin vincular; el servicio normaliza y valida
     * (dígitos 5–32, URL → dígitos). */
    #[serde(default)]
    pub marketplace_id: Option<String>,
    /* [09AA-24] Nombres alternativos (lista completa; ausente = vacía).
     * El servicio recorta, quita vacíos/duplicados y valida (máx 10,
     * c/u ≤200 caracteres). */
    #[serde(default)]
    pub alias_titulos: Vec<String>,
    /// Copy IA (`None` = sin copy); al crear, `None` deja las columnas NULL
    #[serde(default)]
    #[validate(nested)]
    pub copy: Option<CopyInmueble>,
}

/// Actualización parcial de inmueble
#[derive(Debug, Default, Deserialize, Validate, ToSchema)]
pub struct UpdateInmuebleRequest {
    #[validate(length(max = 500))]
    pub titulo: Option<String>,
    #[validate(length(max = 20000))]
    pub descripcion: Option<String>,
    #[validate(length(max = 500))]
    pub ubicacion: Option<String>,
    #[validate(range(min = 0))]
    pub puestos: Option<i32>,
    #[validate(length(max = 500))]
    pub residencia: Option<String>,
    /* [18AA-6] Ausente o vacío = no tocar el que ya hay (no se borra por PUT:
     * el campo se edita, no se deshace). */
    #[validate(length(max = 120))]
    pub nombre_corto: Option<String>,
    #[validate(range(min = 0.0))]
    pub precio: Option<f64>,
    #[validate(range(min = 0.0))]
    pub precio_alquiler: Option<f64>,
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
    /* [09AA-21] Vínculo exacto tri-estado: ausente = dejar como está,
     * `null` = desvincular (NULL), texto = fijar (URL o dígitos, el
     * servicio normaliza a dígitos 5–32). `Option<Option<..>>` distingue
     * ausente (`None`) de `null` (`Some(None)`) solo con `doble_opcion`
     * ([18AA-3]: sin él serde aplasta los dos a `None`). */
    #[serde(default, deserialize_with = "doble_opcion")]
    pub marketplace_id: Option<Option<String>>,
    /* [09AA-24] Alias: ausente = no tocar, lista (incluso vacía) = reemplazar
     * entera (como el resto del PUT, que es reemplazo entero). */
    #[serde(default)]
    pub alias_titulos: Option<Vec<String>>,
    /// Copy IA (`Some` la fija, `None` la deja como está; no se puede borrar por PUT)
    #[validate(nested)]
    pub copy: Option<CopyInmueble>,
    /// Receta publicitaria (`Some` la fija, `None` la deja como está)
    #[validate(nested)]
    pub receta: Option<RecetaPublicidad>,
}

/* [08AA-3] B5 parametros-excesivos: el repositorio recibía 21 argumentos
 * (sentinel `parametros-excesivos-rs`, máx 8). Agrupa los 18 campos ya
 * normalizados listos para bindear: `&str` prestados, resto propios. El
 * servicio la arma desde `UpdateInmuebleRequest` + normalizados. */
#[derive(Debug)]
pub struct ActualizacionInmueble<'a> {
    pub titulo: Option<&'a str>,
    pub descripcion: Option<&'a str>,
    pub ubicacion: Option<&'a str>,
    pub puestos: Option<i32>,
    pub residencia: Option<&'a str>,
    /* [18AA-6] `None` = no tocar (el vacío no borra). */
    pub nombre_corto: Option<&'a str>,
    pub precio: Option<f64>,
    pub precio_alquiler: Option<f64>,
    pub tipo: Option<&'a str>,
    pub operacion: Option<&'a str>,
    pub habitaciones: Option<i32>,
    pub banos: Option<i32>,
    pub metros: Option<f64>,
    pub metros_terreno: Option<f64>,
    pub estado: Option<&'a str>,
    pub copy_corta: Option<&'a str>,
    pub copy_larga: Option<&'a str>,
    pub copy_modelo: Option<&'a str>,
    pub copy_actualizada_en: Option<DateTime<Utc>>,
    pub receta: Option<sqlx::types::Json<RecetaPublicidad>>,
    /* [09AA-21] Tri-estado listo para bindear: `None` = no tocar,
     * `Some(None)` = SET NULL (desvincular), `Some(Some(v))` = fijar. */
    pub marketplace_id: Option<Option<&'a str>>,
    /* [09AA-24] Alias ya normalizados: `None` = no tocar, `Some` = fijar la
     * lista entera (`COALESCE`, el vacío limpia). */
    pub alias_titulos: Option<Vec<String>>,
}

/// Cambio de visibilidad pública — el backend decide qué se publica
#[derive(Debug, Deserialize, ToSchema)]
pub struct PublicacionRequest {
    pub publicado: bool,
}

/// Cambio de estado (`disponible|reservado|vendido|alquilado`) — [08AA-33]:
/// `vendido`/`alquilado` despublican en la misma query (una propiedad
/// cerrada no debe seguir en la web pública); el resto solo cambia el
/// estado sin tocar la visibilidad.
#[derive(Debug, Deserialize, ToSchema)]
pub struct EstadoRequest {
    pub estado: String,
}

/// Alta de foto en un inmueble
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct AddFotoRequest {
    #[validate(length(min = 1, max = 500, message = "storage_key requerido"))]
    pub storage_key: String,
    pub orden: Option<i32>,
    pub origen: Option<String>,
}

/// [15AA-8] Reordenado de fotos: ids de las originales del inmueble en el
/// orden deseado. Debe ser una permutación exacta de las que hay.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ReordenarFotosRequest {
    pub ids: Vec<Uuid>,
}

/* [279A-3] Ficha /ask: lo que la dueña responde por inmueble. `extras` es
 * objeto plano `{clave: string|number|bool|null}`; las claves las define el
 * esquema del front por tipo (`piso`, `punto_referencia`...). `precio_minimo`
 * solo existe en estas rutas admin: ningún `Inmueble` público lo incluye. */

/// Guardar ficha /ask (admin)
#[derive(Debug, Deserialize, ToSchema)]
pub struct FichaAskRequest {
    pub extras: serde_json::Value,
    pub precio_minimo: Option<f64>,
}

/// Ficha /ask leída (admin): incluye lo privado
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FichaAskResponse {
    pub inmueble_id: Uuid,
    pub extras: serde_json::Value,
    pub precio_minimo: Option<f64>,
}

/// Valida `extras`: objeto, tope de claves y valores simples acotados.
/// Pura (sin BD): testeable sin `DATABASE_URL`.
pub fn validar_extras(v: &serde_json::Value) -> Result<(), String> {
    const MAX_CLAVES: usize = 64;
    const MAX_CLAVE: usize = 64;
    const MAX_TEXTO: usize = 500;
    let obj = v
        .as_object()
        .ok_or_else(|| "extras debe ser un objeto".to_string())?;
    if obj.len() > MAX_CLAVES {
        return Err(format!("extras admite hasta {MAX_CLAVES} campos"));
    }
    for (k, val) in obj {
        if k.len() > MAX_CLAVE || !k.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
            return Err(format!("clave de ficha inválida: {k}"));
        }
        let ok = match val {
            serde_json::Value::Null | serde_json::Value::Bool(_) | serde_json::Value::Number(_) => {
                true
            }
            serde_json::Value::String(s) => s.len() <= MAX_TEXTO,
            serde_json::Value::Array(_) | serde_json::Value::Object(_) => false,
        };
        if !ok {
            return Err(format!("valor de ficha inválido para: {k}"));
        }
    }
    Ok(())
}

/// Alta de usuario por un owner (register público = solo bootstrap)
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateUserRequest {
    #[validate(email(message = "Formato de email inválido"))]
    pub email: String,
    #[validate(length(min = 8, message = "La contraseña debe tener al menos 8 caracteres"))]
    pub password: String,
}
