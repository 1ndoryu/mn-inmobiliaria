use std::sync::atomic::{AtomicU64, Ordering};

use crate::errors::AppError;

use crate::services::marketplace::cambios;

/// [08AA-39] Limpieza total del panel: borra TODAS las filas de caché de una
/// vez (la dueña lo pidió como botón al lado de «Recargar» para no depender
/// de limpiezas manuales por SQL). Devuelve cuántas filas cayeron.
pub async fn borrar_todo_cache(pool: &sqlx::PgPool) -> Result<u64, AppError> {
    let r = sqlx::query!("DELETE FROM mp_respuestas_cache")
        .execute(pool)
        .await?;
    if r.rows_affected() > 0 {
        VERSION_BORRADORES_BORRADOS.fetch_add(1, Ordering::SeqCst);
        cambios::notificar();
    }
    Ok(r.rows_affected())
}

/// [09AA-4] Borrado previo a regenerar: elimina las filas del hilo que NO
/// son correcciones de la dueña (borradores viejos de excerpts anteriores).
/// Las correcciones (`corregida`, puestas por ella con el lápiz) jamás se
/// tocan: son su texto, no caché. Devuelve cuántas filas cayeron.
/// Sin esto, cada excerpt nuevo es una firma nueva y sus filas viejas viven
/// 90 días: el panel lista lo viejo junto a lo fresco y parece «cacheado».
pub async fn borrar_hilo_no_corregidas(
    pool: &sqlx::PgPool,
    thread_clave: &str,
) -> Result<u64, AppError> {
    let r = sqlx::query!(
        "DELETE FROM mp_respuestas_cache WHERE thread_id = $1 AND NOT corregida",
        thread_clave,
    )
    .execute(pool)
    .await?;
    if r.rows_affected() > 0 {
        VERSION_BORRADORES_BORRADOS.fetch_add(1, Ordering::SeqCst);
        cambios::notificar();
    }
    Ok(r.rows_affected())
}

/// Archiva un hilo: solo lo oculta de `resumen_chats`. Su caché, sus
/// correcciones y la compartida quedan intactas.
pub async fn archivar_hilo(pool: &sqlx::PgPool, thread_clave: &str) -> Result<(), AppError> {
    sqlx::query!(
        "INSERT INTO mp_chats_archivados (thread_id) VALUES ($1) \
         ON CONFLICT (thread_id) DO NOTHING",
        thread_clave,
    )
    .execute(pool)
    .await?;
    cambios::notificar();
    Ok(())
}

/// Borra la conversación entera (incluidas sus correcciones) y quita su marca
/// de archivado. No toca `mp_respuestas_inmueble`: la compartida es del
/// inmueble y sigue sirviendo a otros hilos.
pub async fn borrar_hilo(pool: &sqlx::PgPool, thread_clave: &str) -> Result<u64, AppError> {
    let mut tx = pool.begin().await?;
    let r = sqlx::query!(
        "DELETE FROM mp_respuestas_cache WHERE thread_id = $1",
        thread_clave
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "DELETE FROM mp_chats_archivados WHERE thread_id = $1",
        thread_clave
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    if r.rows_affected() > 0 {
        VERSION_BORRADORES_BORRADOS.fetch_add(1, Ordering::SeqCst);
        cambios::notificar();
    }
    Ok(r.rows_affected())
}

/// [09AA-31] Veces que un borrado de caché ha quitado filas (`borrar_borrador_hilo`,
/// `borrar_hilo`, `borrar_hilo_no_corregidas`, `borrar_todo_cache`). El float de lab
/// lo lee en cada barrido (`GET .../borradores/version`) y, si cambia, vacía su
/// caché en memoria: un borrador borrado no debe seguir sirviéndose desde ahí.
/// Proceso-local: un reinicio lo pone a 0, y el float lo trata como cambio.
pub(in crate::services::marketplace) static VERSION_BORRADORES_BORRADOS: AtomicU64 =
    AtomicU64::new(0);

pub fn version_borradores_borrados() -> u64 {
    VERSION_BORRADORES_BORRADOS.load(Ordering::SeqCst)
}

/// Borra solo los borradores no corregidos del hilo. De la compartida borra
/// la fila de cada mensaje que el hilo tenía, siempre que no sea corregida y
/// ningún otro hilo (ni una corrección propia) la siga enlazando.
pub async fn borrar_borrador_hilo(
    pool: &sqlx::PgPool,
    thread_clave: &str,
) -> Result<u64, AppError> {
    let mut tx = pool.begin().await?;
    // Las claves se leen antes de borrar: después ya no hay filas que enlazar.
    let claves: Vec<(String, String, String)> = sqlx::query!(
        "SELECT DISTINCT catalog_hash, precio_hash, mensaje_clave AS \"mensaje_clave!\" FROM mp_respuestas_cache \
         WHERE thread_id = $1 AND NOT corregida AND mensaje_clave IS NOT NULL",
        thread_clave,
    )
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .map(|r| (r.catalog_hash, r.precio_hash, r.mensaje_clave))
    .collect();
    let r = sqlx::query!(
        "DELETE FROM mp_respuestas_cache WHERE thread_id = $1 AND NOT corregida",
        thread_clave,
    )
    .execute(&mut *tx)
    .await?;
    for (catalogo, precio, mensaje) in &claves {
        sqlx::query!(
            "DELETE FROM mp_respuestas_inmueble ci WHERE ci.catalog_hash = $1 \
             AND ci.precio_hash = $2 AND ci.mensaje_clave = $3 AND NOT ci.corregida \
             AND NOT EXISTS (SELECT 1 FROM mp_respuestas_cache c \
             WHERE c.catalog_hash = ci.catalog_hash AND c.precio_hash = ci.precio_hash \
             AND c.mensaje_clave = ci.mensaje_clave)",
            catalogo.as_str(),
            precio.as_str(),
            mensaje.as_str(),
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    if r.rows_affected() > 0 {
        VERSION_BORRADORES_BORRADOS.fetch_add(1, Ordering::SeqCst);
        cambios::notificar();
    }
    Ok(r.rows_affected())
}
