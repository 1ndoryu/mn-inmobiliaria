//! Claves de caché del borrador: ID exacto, título y hilo (`claves_cache`, `claves_por_marketplace_id`, `claves_por_titulo`).

// Mismos imports que la cabecera del hub (marketplace.rs); el glob es intencional.
#[allow(clippy::wildcard_imports)]
use super::*;

/// Claves de caché para un `avisoId` opaco: UUID de `inmuebles` → hashes de
/// la fila recién leída; lo demás (o UUID inexistente) es ruta sin-ficha.
/// Mismo cálculo en `borrador`, `regenerar` y `corregir` para que los tres
/// hablen de la misma fila (si la ficha cambia entre llamadas, miss honesto).
/// [08AA-10] Sin UUID (piloto: siempre) se empareja el título del hilo
/// contra publicados (`ficha_por_titulo`): el precio del catálogo entra al
/// prompt en vez del dodge. Sin emparejamiento o con la BD caída, sin ficha
/// (el borrador jamás se bloquea por esto).
/// [09AA-21] Rama prioritaria exacta: si hay `avisoId` con dígitos de aviso
/// (`/marketplace/item/<id>` o dígitos 5–32) va `WHERE marketplace_id = $1`
/// sin pasar por el título. UUID legacy intacto (misma ruta de siempre);
/// ID exacto inexistente = sin ficha (no se cita un precio dudoso);
/// forma no-UUID-no-dígitos o sin ID = fallback por título.
pub(super) async fn claves_cache(
    pool: &sqlx::PgPool,
    aviso_id: Option<&str>,
    titulo_fb: Option<&str>,
) -> Result<
    (
        Option<crate::services::marketplace::PromptSeguro>,
        String,
        String,
        bool,
    ),
    AppError,
> {
    match aviso_id.map(str::trim) {
        Some(a) if !a.is_empty() => {
            /* UUID legacy: ruta intacta (existe → ficha, no existe → sin ficha). */
            if let Ok(id) = Uuid::parse_str(a) {
                match InmuebleRepository::find_by_id(pool, id).await? {
                    Some(f) => {
                        let seguro = strip_ficha_para_prompt(&f, STRIP_VERSION)?;
                        let precio = precio_hash_seguro(&seguro);
                        let catalogo = hash_ficha(&f);
                        return Ok((Some(seguro), precio, catalogo, true));
                    }
                    None => {
                        return Ok((None, SIN_FICHA.to_string(), SIN_FICHA.to_string(), false));
                    }
                }
            }
            /* Dígitos de aviso: rama exacta prioritaria. */
            match crate::services::InmuebleService::normalizar_marketplace_id(Some(a)) {
                Ok(Some(mp)) => claves_por_marketplace_id(pool, &mp, titulo_fb).await,
                /* Vacío normalizado o forma inválida: cae al título como antes. */
                Ok(None) | Err(_) => claves_por_titulo(pool, titulo_fb, false).await,
            }
        }
        _ => claves_por_titulo(pool, titulo_fb, false).await,
    }
}

/// Ficha por ID exacto de aviso (`WHERE marketplace_id = $1`): con vínculo,
/// hashes reales de la fila. [09AA-29] Si el ID no lo reclama nadie, cae al
/// título del hilo, pero solo a una ficha SIN vínculo propio: una ficha
/// vinculada a OTRO aviso jamás se cita por aproximación. La BD caída sí
/// degrada a sin ficha.
pub(super) async fn claves_por_marketplace_id(
    pool: &sqlx::PgPool,
    marketplace_id: &str,
    titulo_fb: Option<&str>,
) -> Result<
    (
        Option<crate::services::marketplace::PromptSeguro>,
        String,
        String,
        bool,
    ),
    AppError,
> {
    match InmuebleRepository::find_by_marketplace_id(pool, marketplace_id).await {
        Ok(Some(f)) => {
            let seguro = strip_ficha_para_prompt(&f, STRIP_VERSION)?;
            let precio = precio_hash_seguro(&seguro);
            let catalogo = hash_ficha(&f);
            Ok((Some(seguro), precio, catalogo, true))
        }
        Ok(None) => claves_por_titulo(pool, titulo_fb, true).await,
        Err(e) => {
            tracing::warn!("claves_cache: buscar por marketplace_id falló ({e}), va sin ficha");
            Ok((None, SIN_FICHA.to_string(), SIN_FICHA.to_string(), false))
        }
    }
}

/// Ficha por título del hilo (ver `ficha_por_titulo`): con emparejamiento,
/// hashes reales de la fila; si no, ruta sin-ficha. `solo_sin_vinculo` (fallback
/// de un ID exacto sin dueño) descarta una ficha ya vinculada a otro aviso.
pub(super) async fn claves_por_titulo(
    pool: &sqlx::PgPool,
    titulo_fb: Option<&str>,
    solo_sin_vinculo: bool,
) -> Result<
    (
        Option<crate::services::marketplace::PromptSeguro>,
        String,
        String,
        bool,
    ),
    AppError,
> {
    let titulo = titulo_fb.map(str::trim).unwrap_or_default();
    if titulo.is_empty() {
        return Ok((None, SIN_FICHA.to_string(), SIN_FICHA.to_string(), false));
    }
    match crate::services::marketplace::ficha_por_titulo(pool, titulo, solo_sin_vinculo).await {
        Ok(Some(f)) => {
            let seguro = strip_ficha_para_prompt(&f, STRIP_VERSION)?;
            let precio = precio_hash_seguro(&seguro);
            let catalogo = hash_ficha(&f);
            Ok((Some(seguro), precio, catalogo, true))
        }
        /* Sin emparejamiento (o empate). */
        Ok(None) => Ok((None, SIN_FICHA.to_string(), SIN_FICHA.to_string(), false)),
        Err(e) => {
            tracing::warn!("claves_cache: emparejar por título falló ({e}), va sin ficha");
            Ok((None, SIN_FICHA.to_string(), SIN_FICHA.to_string(), false))
        }
    }
}

