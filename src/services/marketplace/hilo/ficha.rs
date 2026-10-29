use serde::Serialize;
use utoipa::ToSchema;

use crate::errors::AppError;
use crate::models::{sugerir_nombre_corto, InmuebleRow};
use crate::repositories::InmuebleRepository;

use crate::services::marketplace::{mejor_puntaje_con_alias, precio_publico, STRIP_VERSION};

/// Prompt seguro: solo los 8 campos del allowlist. La frase canónica de la
/// ficha vive en la tabla `inmuebles`; el plugin jamás ve el resto.
/// [08AA-25] `operacion` (`venta`|`alquiler`, tal cual en la fila): sin
/// ella el prompt hablaba siempre en lenguaje de venta.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PromptSeguro {
    pub titulo: String,
    pub precio_publico: String,
    /// [25AA-7] Solo en «ambas»: el canon mensual ya formateado. `precio_publico`
    /// queda como la cifra de venta. Ausente en el resto: el prompt no cambia.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub precio_alquiler_publico: Option<String>,
    pub operacion: String,
    pub zona: String,
    pub m2: f64,
    pub habitaciones: i32,
    pub descripcion_corta: String,
    /// [18AA-5] Cómo se cita el inmueble en la conversación («Apart. en Resd.
    /// Kama Merú»). [18AA-6] Es un dato guardado del inmueble, no algo que la
    /// IA componga con el título.
    pub nombre_corto: String,
    /* [28AD-1] Solo para decidir el borrador de «no disponible» y excluir al
     * propio inmueble del similar. `skip`: no entra al prompt ni a los hashes
     * de caché, así que no invalida nada. */
    #[serde(skip)]
    pub inmueble_id: uuid::Uuid,
    #[serde(skip)]
    pub estado: String,
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
        precio_alquiler_publico: (ficha.operacion == "ambas" && ficha.precio_alquiler > 0.0)
            .then(|| precio_publico(ficha.precio_alquiler)),
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
        /* [18AA-6] Manda el valor guardado. Solo si viene vacío (altas viejas
         * o por API sin el campo) se sugiere uno: el borrador nunca se queda
         * sin sujeto con el que citar el inmueble. */
        nombre_corto: if ficha.nombre_corto.trim().is_empty() {
            sugerir_nombre_corto(&ficha.tipo, &ficha.residencia, &ficha.ubicacion)
        } else {
            ficha.nombre_corto.trim().to_string()
        },
        inmueble_id: ficha.id,
        estado: ficha.estado.clone(),
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
    let mut candidatos = InmuebleRepository::titulos_alias_publicados(pool).await?;
    /* [28AD-1] Los cerrados y despublicados también compiten: si el hilo habla de
     * uno, el borrador debe decir que ya no está (el precio tampoco se cita). */
    candidatos.extend(InmuebleRepository::titulos_alias_cerrados(pool).await?);
    let puntajes: Vec<(uuid::Uuid, (bool, usize, usize))> = candidatos
        .iter()
        .filter_map(|(id, titulo, alias)| {
            let (directo, solape, distintivo) = mejor_puntaje_con_alias(titulo_fb, titulo, alias);
            /* [28AB-1] Un título exacto (`directo`) manda sobre uno parcial con las
             * mismas palabras: «Casa en venta en Riberas del Caroní» contra «…en
             * Urbanización Riberas del Caroní» empataba y la casa salía sin precio. */
            (directo || (solape >= 3 && distintivo >= 1)).then_some((*id, (directo, distintivo, solape)))
        })
        .collect();
    match mejor_unico(&puntajes) {
        Some(id) => Ok(InmuebleRepository::find_by_id(pool, id).await?),
        None => Ok(None),
    }
}

/// Mejor clave `(directo, distintivo, solape)`; dos con la misma clave máxima
/// no reclaman a ninguna (`None`), igual que un empate entre avisos.
pub(crate) fn mejor_unico(puntajes: &[(uuid::Uuid, (bool, usize, usize))]) -> Option<uuid::Uuid> {
    let max = puntajes.iter().map(|(_, clave)| *clave).max()?;
    let mut con_max = puntajes.iter().filter(|(_, clave)| *clave == max);
    match (con_max.next(), con_max.next()) {
        (Some((id, _)), None) => Some(*id),
        _ => None,
    }
}
