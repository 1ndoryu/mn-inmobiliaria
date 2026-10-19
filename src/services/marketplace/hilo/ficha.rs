use serde::Serialize;
use utoipa::ToSchema;

use crate::errors::AppError;
use crate::models::InmuebleRow;
use crate::repositories::InmuebleRepository;

use crate::services::marketplace::{mejor_puntaje_con_alias, precio_publico, STRIP_VERSION};

/// Prompt seguro: solo los 7 campos del allowlist. La frase canónica de la
/// ficha vive en la tabla `inmuebles`; el plugin jamás ve el resto.
/// [08AA-25] `operacion` (`venta`|`alquiler`, tal cual en la fila): sin
/// ella el prompt hablaba siempre en lenguaje de venta.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PromptSeguro {
    pub titulo: String,
    pub precio_publico: String,
    pub operacion: String,
    pub zona: String,
    pub m2: f64,
    pub habitaciones: i32,
    pub descripcion_corta: String,
    /// [18AA-5] Cómo se cita el inmueble en la conversación, ya montado
    /// («Apart. en Resd. Kama Merú»): tipo abreviado + lugar corto. La IA lo
    /// usa tal cual en vez de componerlo con el título.
    pub nombre_corto: String,
}

/// Sujeto del borrador: tipo abreviado + lugar corto. [18AA-5] Antes lo
/// componía la IA a partir del título y salía «el apartamento planta baja con
/// patio en Residencias Kama Merú»: leído en un chat suena a ficha, no a
/// conversación.
pub(crate) fn nombre_corto(tipo: &str, residencia: &str, ubicacion: &str) -> String {
    let lugar = if residencia.trim().is_empty() {
        ubicacion
    } else {
        residencia
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

/// El lugar, en corto: se queda con lo de antes de la primera coma (la ciudad
/// y coletillas como «conjunto residencial cerrado» sobran en un chat) y
/// abrevia «Residencias» a «Resd.».
fn lugar_corto(lugar: &str) -> String {
    let base = lugar.split(',').next().unwrap_or("").trim();
    let base = if base.is_empty() { lugar.trim() } else { base };
    base.split_whitespace()
        .map(|p| {
            if p.eq_ignore_ascii_case("residencias") {
                "Resd."
            } else {
                p
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Recorta la ficha al allowlist. `strip` distinto de la versión vigente se
/// rechaza para que un despliegue viejo no cuele campos nuevos en silencio.
pub fn strip_ficha_para_prompt(ficha: &InmuebleRow, strip: &str) -> Result<PromptSeguro, AppError> {
    if strip != STRIP_VERSION {
        return Err(AppError::BadRequest(format!(
            "strip desconocido: {strip} (se esperaba {STRIP_VERSION})"
        )));
    }
    let zona = if ficha.residencia.trim().is_empty() {
        ficha.ubicacion.clone()
    } else {
        format!("{}, {}", ficha.ubicacion, ficha.residencia)
    };
    Ok(PromptSeguro {
        titulo: ficha.titulo.clone(),
        precio_publico: precio_publico(ficha.precio),
        operacion: ficha.operacion.clone(),
        zona,
        m2: ficha.metros,
        habitaciones: ficha.habitaciones,
        /* [18AA-4] El tope era 500 y cortaba la descripción por la mitad. En la
         * ficha de Loma Linda (603 chars) el «Alquiler: $2.800» empieza en el
         * 586: la IA no lo veía nunca y respondía el fallback cuando le
         * preguntaban el canon. El tope sigue (nunca sin límite), pero por
         * encima de cualquier descripción real de la tabla. */
        descripcion_corta: ficha.descripcion.chars().take(1500).collect(),
        nombre_corto: nombre_corto(&ficha.tipo, &ficha.residencia, &ficha.ubicacion),
    })
}

/* [08AA-8] `precio_publico`, schema M3 (`BorradorRequest`…),
 * `validar_borrador` y `normalizar_excerpt` viven en `marketplace/texto.rs`
 * (re-export arriba para usos externos e internos). */

/* [08AA-8] `normalizar_excerpt*` vive en
 * `marketplace/texto.rs` (re-export arriba). */

/// Ficha publicada cuyo título mejor empareja con el del hilo: directo, o
/// solape ≥3 con ≥1 palabra distintiva. Empate entre dos avisos o BD
/// caída = `None` (nunca se cita un precio dudoso; quien llama decide si
/// lo registra: el borrador jamás se bloquea por esto).
/// [09AA-24] Cada ficha puntúa con su título + alias (`mejor_puntaje_con_alias`).
/// [13AA-4] Ya no hay fallback por título para un ID de aviso sin dueño (ver
/// `claves_por_marketplace_id`), así que esta búsqueda compara contra todos los
/// publicados, también los ya vinculados.
pub async fn ficha_por_titulo(
    pool: &sqlx::PgPool,
    titulo_fb: &str,
) -> Result<Option<InmuebleRow>, AppError> {
    let candidatos = InmuebleRepository::titulos_alias_publicados(pool).await?;
    let mut mejor: Option<(uuid::Uuid, usize, usize)> = None;
    let mut empate = false;
    for (id, titulo, alias) in &candidatos {
        let (directo, solape, distintivo) = mejor_puntaje_con_alias(titulo_fb, titulo, alias);
        if !(directo || (solape >= 3 && distintivo >= 1)) {
            continue;
        }
        let clave = (distintivo, solape);
        match mejor {
            Some((_, md, ms)) if (md, ms) == clave => empate = true,
            Some((_, md, ms)) if (md, ms) > clave => {}
            _ => {
                mejor = Some((*id, distintivo, solape));
                empate = false;
            }
        }
    }
    match (mejor, empate) {
        (Some((id, _, _)), false) => Ok(InmuebleRepository::find_by_id(pool, id).await?),
        _ => Ok(None),
    }
}
