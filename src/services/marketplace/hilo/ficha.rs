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
        descripcion_corta: ficha.descripcion.chars().take(500).collect(),
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
/// [09AA-29] `solo_sin_vinculo` (fallback de un ID exacto sin dueño) excluye
/// las fichas ya vinculadas ANTES de puntuar: así no hay falso negativo si la
/// mejor coincidencia es de otro aviso.
pub async fn ficha_por_titulo(
    pool: &sqlx::PgPool,
    titulo_fb: &str,
    solo_sin_vinculo: bool,
) -> Result<Option<InmuebleRow>, AppError> {
    let candidatos = if solo_sin_vinculo {
        InmuebleRepository::titulos_alias_publicados_sin_vinculo(pool).await?
    } else {
        InmuebleRepository::titulos_alias_publicados(pool).await?
    };
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
