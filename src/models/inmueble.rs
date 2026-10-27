use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

mod dto;

pub use dto::*;

/* [159A-1] Catálogo de inmuebles: fila plana (FromRow) + vista con fotos.
 * Sin obligatorios: crear admite payload vacío (cada columna con DEFAULT).
 * Enums como texto validado contra allowlists en el servicio.
 * [159A-2] Copy IA como columnas anulables + `FotoPublica` con URL lista. */

/// Valores permitidos para `tipo`
pub const TIPOS: &[&str] = &["apartamento", "casa", "local", "terreno", "townhouse"];
/// Valores permitidos para `operacion`
pub const OPERACIONES: &[&str] = &["venta", "alquiler", "ambas"];
/// Operaciones que acepta una solicitud de la dueña (`ambas` solo existe en el catálogo)
pub const OPERACIONES_SOLICITUD: &[&str] = &["venta", "alquiler"];
/// Valores permitidos para `estado`
pub const ESTADOS: &[&str] = &["disponible", "reservado", "vendido", "alquilado"];
/// Valores permitidos para `origen` de foto
pub const ORIGENES_FOTO: &[&str] = &["original", "mejorada"];
/// Valores permitidos para `formato` de receta publicitaria
pub const FORMATOS_RECETA: &[&str] = &["post-3-4", "post-4-5", "cuadrado-1-1"];

/// Receta de la imagen publicitaria: qué fotos (por ÍNDICE sobre las
/// visibles) + formato + título + precio. Vive en `inmuebles.receta` (JSONB)
/// para que lo configurado en admin llegue al frente público; `None` = usar
/// la automática (portada + 2ª/3ª, 3:4). [229A-2]
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct RecetaPublicidad {
    #[validate(range(min = 0, message = "fondo_idx no puede ser negativo"))]
    pub fondo_idx: i32,
    #[validate(range(min = 0, message = "circular_grande_idx no puede ser negativo"))]
    pub circular_grande_idx: i32,
    #[validate(range(min = 0, message = "circular_mediano_idx no puede ser negativo"))]
    pub circular_mediano_idx: i32,
    pub formato: String,
    #[serde(default = "default_con_precio")]
    pub con_precio: bool,
    #[validate(length(max = 120, message = "titulo1 no debe exceder 120 caracteres"))]
    #[serde(default)]
    pub titulo1: String,
    #[validate(length(max = 120, message = "titulo2 no debe exceder 120 caracteres"))]
    #[serde(default)]
    pub titulo2: String,
}

fn default_con_precio() -> bool {
    true
}
/// Extensiones de imagen aceptadas en subida (minúsculas, con punto)
pub const EXTENSIONES_FOTO: &[&str] = &[".jpg", ".jpeg", ".png", ".webp"];
/// Tope de subida por foto: 10 MiB
pub const MAX_FOTO_BYTES: usize = 10 * 1024 * 1024;

/// Copy generada por IA (nullable: `None` = pendiente de generar)
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct CopyInmueble {
    #[validate(length(max = 500, message = "La copy corta no debe exceder 500 caracteres"))]
    pub corta: String,
    #[validate(length(
        max = 20000,
        message = "La copy larga no debe exceder 20000 caracteres"
    ))]
    pub larga: String,
    #[validate(length(max = 100, message = "El modelo no debe exceder 100 caracteres"))]
    pub modelo: String,
    pub actualizada_en: DateTime<Utc>,
}

/// Fila de `inmuebles` tal cual la devuelve Postgres
#[derive(Debug, Clone, FromRow)]
pub struct InmuebleRow {
    pub id: Uuid,
    pub titulo: String,
    pub descripcion: String,
    pub ubicacion: String,
    /// Puestos de estacionamiento (>= 0).
    pub puestos: i32,
    /// Nombre de la residencia/conjunto ('' = sin especificar).
    pub residencia: String,
    /* [18AA-6] Cómo se cita el inmueble en el chat del Marketplace. Vacío =
     * sin rellenar: quien lo lea cae a la sugerencia calculada. */
    pub nombre_corto: String,
    pub precio: f64,
    /* [25AA-5] Canon mensual cuando `operacion` es `ambas` (0 = sin canon); `precio` sigue siendo la venta. */
    pub precio_alquiler: f64,
    pub tipo: String,
    pub operacion: String,
    pub habitaciones: i32,
    pub banos: i32,
    pub metros: f64,
    pub metros_terreno: f64,
    pub estado: String,
    pub publicado: bool,
    pub slug: String,
    pub copy_corta: Option<String>,
    pub copy_larga: Option<String>,
    pub copy_modelo: Option<String>,
    pub copy_actualizada_en: Option<DateTime<Utc>>,
    /// Receta publicitaria elegida en admin (`None` = automática)
    pub receta: Option<sqlx::types::Json<RecetaPublicidad>>,
    /* [279A-3] Ficha /ask: respuestas de la dueña por clave (`piso`,
     * `punto_referencia`...). Objeto plano validado en `validar_extras`. */
    pub extras: sqlx::types::Json<serde_json::Value>,
    /* [279A-3] Precio mínimo privado: NUNCA se serializa a público ni a
     * tools de IA que citen cifras (solo insinuación). Solo viaja en
     * `FichaAskResponse` (rutas admin con JWT). */
    pub precio_minimo: Option<f64>,
    /* [09AA-21] Vínculo exacto con el aviso (`/marketplace/item/<id>`):
     * dígitos como TEXT (`None` = sin vincular). UNIQUE en BD. */
    pub marketplace_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /* [09AA-24] Nombres alternativos (`alias_titulos TEXT[]`, vacío = sin
     * alias): el mismo inmueble publicado con otro nombre. Va al final para
     * no renumerar los `$n` del repositorio. */
    pub alias_titulos: Vec<String>,
}

/// Inmueble con sus fotos — lo que expone la API
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Inmueble {
    pub id: Uuid,
    pub titulo: String,
    pub descripcion: String,
    pub ubicacion: String,
    /// Puestos de estacionamiento (>= 0).
    pub puestos: i32,
    /// Nombre de la residencia/conjunto ('' = sin especificar).
    pub residencia: String,
    /* [18AA-6] Cómo se cita el inmueble en el chat del Marketplace («Apart. en
     * Resd. Kama Merú»): dato del inmueble, no algo que el borrador deduzca. */
    pub nombre_corto: String,
    pub precio: f64,
    pub precio_alquiler: f64,
    pub tipo: String,
    pub operacion: String,
    pub habitaciones: i32,
    pub banos: i32,
    pub metros: f64,
    pub metros_terreno: f64,
    pub estado: String,
    pub publicado: bool,
    pub slug: String,
    pub copy: Option<CopyInmueble>,
    /// Receta publicitaria elegida en admin (`None` = automática)
    pub receta: Option<RecetaPublicidad>,
    /* [279A-3] Respuestas /ask (piso, referencia...): enriquecen la ficha
     * pública y el contexto de la IA. Lo privado (`precio_minimo`) jamás
     * entra aquí: la frontera es por construcción (no existe el campo). */
    pub extras: serde_json::Value,
    /* [09AA-21] Vínculo exacto con el aviso (dígitos, `None` = sin vincular).
     * Viaja en admin y en público (es el ID del aviso, ya público en FB). */
    pub marketplace_id: Option<String>,
    /* [09AA-24] Nombres alternativos: el admin los edita y el emparejado los
     * puntúa; en público solo informan (son nombres ya publicados). */
    pub alias_titulos: Vec<String>,
    pub fotos: Vec<FotoPublica>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/* [18AA-6] Sugerencia de `nombre_corto` para un inmueble que aún no lo trae
 * (altas por API sin el campo, o filas anteriores a la columna). El valor de
 * verdad es el guardado: esto solo evita que el borrador del Marketplace se
 * quede sin sujeto con el que citar el inmueble. */
pub fn sugerir_nombre_corto(tipo: &str, residencia: &str, ubicacion: &str) -> String {
    /* La residencia manda tal cual, con su coma si la lleva: si dice «Villa
     * Sol Suites, Lechería» es porque ahí la ciudad hace falta para
     * distinguirlo. Solo cuando no hay residencia se recurre a la ubicación,
     * que es una dirección completa: de esa se toma el primer tramo y el
     * resto (la ciudad) sobra en un chat. */
    let lugar = if residencia.trim().is_empty() {
        ubicacion.split(',').next().unwrap_or("").trim()
    } else {
        residencia.trim()
    };
    let lugar = if lugar.is_empty() {
        ubicacion.trim()
    } else {
        lugar
    };
    format!("{} en {}", tipo_corto(tipo), lugar_corto(lugar))
}

/// El tipo como lo dice la dueña. Solo `apartamento` se abrevia; el resto va
/// tal cual con la inicial en mayúscula, porque abre la frase.
fn tipo_corto(tipo: &str) -> String {
    let t = tipo.trim().to_lowercase();
    if t == "apartamento" {
        return "Apart.".to_string();
    }
    let mut letras = t.chars();
    match letras.next() {
        Some(p) => p.to_uppercase().collect::<String>() + letras.as_str(),
        /* Sin tipo en la tabla, un genérico antes que un hueco. */
        None => "Inmueble".to_string(),
    }
}

/// El lugar, en corto: «Residencias» pasa a «Resd.» y el «Edificio»/«Edif.» de
/// delante se cae («Apart. en Coronado Plaza», no «en Edificio Coronado
/// Plaza»). El resto va tal cual: lo que no vale se arregla en la residencia,
/// no adivinando aquí.
fn lugar_corto(lugar: &str) -> String {
    lugar
        .split_whitespace()
        .map(|p| {
            if p.eq_ignore_ascii_case("residencias") {
                "Resd."
            } else if p.eq_ignore_ascii_case("edificio") || p.eq_ignore_ascii_case("edif.") {
                ""
            } else {
                p
            }
        })
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

impl Inmueble {
    /// Ensambla la vista a partir de la fila y sus fotos ya ordenadas
    #[must_use]
    pub fn from_row(row: InmuebleRow, fotos: Vec<Foto>) -> Self {
        let copy = match (
            row.copy_corta,
            row.copy_larga,
            row.copy_modelo,
            row.copy_actualizada_en,
        ) {
            (Some(corta), Some(larga), Some(modelo), Some(actualizada_en)) => Some(CopyInmueble {
                corta,
                larga,
                modelo,
                actualizada_en,
            }),
            _ => None,
        };
        Self {
            id: row.id,
            titulo: row.titulo,
            descripcion: row.descripcion,
            ubicacion: row.ubicacion,
            puestos: row.puestos,
            residencia: row.residencia,
            nombre_corto: row.nombre_corto,
            precio: row.precio,
            precio_alquiler: row.precio_alquiler,
            tipo: row.tipo,
            operacion: row.operacion,
            habitaciones: row.habitaciones,
            banos: row.banos,
            metros: row.metros,
            metros_terreno: row.metros_terreno,
            estado: row.estado,
            publicado: row.publicado,
            slug: row.slug,
            copy,
            receta: row.receta.map(|j| j.0),
            extras: row.extras.0,
            marketplace_id: row.marketplace_id,
            alias_titulos: row.alias_titulos,
            fotos: fotos.into_iter().map(FotoPublica::from).collect(),
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

/// Foto tal como la expone la API: con URL pública en vez de clave interna
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FotoPublica {
    pub id: Uuid,
    pub inmueble_id: Uuid,
    pub url: String,
    pub orden: i32,
    pub origen: String,
    pub created_at: DateTime<Utc>,
}

/// [09AA-28] URL pública de una clave de `fotos.storage_key` (una sola
/// fuente: `FotoPublica` y la portada del panel de chats la comparten).
#[must_use]
pub fn url_publica_de_foto(storage_key: &str) -> String {
    format!("/uploads/{storage_key}")
}

impl From<Foto> for FotoPublica {
    fn from(f: Foto) -> Self {
        Self {
            id: f.id,
            inmueble_id: f.inmueble_id,
            url: url_publica_de_foto(&f.storage_key),
            orden: f.orden,
            origen: f.origen,
            created_at: f.created_at,
        }
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    /* [18AA-3] El contrato del PUT parcial: ausente no toca, `null` desvincula,
     * texto fija. Lo que se rompía era justo la distinción ausente/null, así que
     * se fija aquí, en el sitio donde serde la decide. */
    #[test]
    fn el_vinculo_del_put_distingue_ausente_null_y_texto() {
        let leer = |cuerpo: &str| -> Option<Option<String>> {
            serde_json::from_str::<UpdateInmuebleRequest>(cuerpo)
                .expect("cuerpo válido")
                .marketplace_id
        };
        assert_eq!(leer("{}"), None, "ausente: no se toca");
        assert_eq!(
            leer(r#"{"marketplace_id":null}"#),
            Some(None),
            "null: desvincula"
        );
        assert_eq!(
            leer(r#"{"marketplace_id":"1234567890"}"#),
            Some(Some("1234567890".into())),
            "texto: fija"
        );
    }
}
