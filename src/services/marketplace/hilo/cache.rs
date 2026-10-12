use serde::Serialize;
use utoipa::ToSchema;

use crate::errors::AppError;
use crate::models::InmuebleRow;

use crate::services::marketplace::cambios;
use crate::services::marketplace::{clave_hilo, purgar_compartida, strip_ficha_para_prompt, PromptSeguro, STRIP_VERSION};

/// [11AA] Llave de la fila de caché de un hilo: la firma del mensaje, sin el
/// nombre. La fila guarda solo el cuerpo (sin saludo ni nombre): el saludo lo
/// añade `con_saludo` al servir, así que dos compradores con el mismo mensaje
/// comparten fila. Versionada (`v2`): las filas previas, con el nombre en la
/// llave y el saludo horneado, quedan huérfanas y no se sirven.
#[must_use]
pub fn clave_cache_hilo(firma: &str) -> String {
    sha_hex(&format!("v2\n{firma}"))
}

/* [03AA-3 M4] Caché de respuestas (`mp_respuestas_cache`): la clave es
 * (firma, precio_hash, catalog_hash). `precio_hash` ata la respuesta al
 * precio citado (si cambia el precio, miss y se regenera: jamás se sirve un
 * precio viejo). `catalog_hash` sale de `hash_ficha` — hash de los bytes que
 * alimentan el prompt (campos del strip + estado) calculados tras el fetch
 * y antes del strip; cualquier cambio ahí invalida. Campos ajenos al prompt
 * (copy, receta, extras) NO invalidan a propósito: no cambian la respuesta.
 * Solo se cachea `fuente=ia`; el fallback nunca (con la IA caída, cachearlo
 * envenenaría 90 días). La corrección humana (`corregida`) gana sobre
 * generaciones futuras (`guardar` usa DO NOTHING; solo `reemplazar`, vía
 * Regenerar explícito, la pisa). Sin ficha: claves literales "sin-ficha"
 * (la `firma` ya diferencia cada excerpt). */

/// Marca sin ficha para `precio_hash`/`catalog_hash` (la firma diferencia).
pub const SIN_FICHA: &str = "sin-ficha";

/// SHA-256 hex con `sha2` (ya dependencia directa). Solo hashes, sin PII.
#[must_use]
pub fn sha_hex(canon: &str) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(canon.as_bytes()))
}

/// Hash del catálogo: serialización canónica de exactamente lo que entra al
/// prompt (los 7 campos del strip + `estado`, que condiciona disponibilidad),
/// calculada sobre la fila recién leída y antes de stripeear. Si la ficha
/// cambia en algo que la respuesta cita → hash distinto → miss → regenera.
#[must_use]
pub fn hash_ficha(ficha: &InmuebleRow) -> String {
    let canon = match strip_ficha_para_prompt(ficha, STRIP_VERSION) {
        Ok(s) => serde_json::json!({
            "titulo": s.titulo,
            "precio": s.precio_publico,
            "operacion": s.operacion,
            "zona": s.zona,
            "m2": s.m2,
            "hab": s.habitaciones,
            "desc": s.descripcion_corta,
            "estado": ficha.estado,
        }),
        /* Inalcanzable con v2 (el handler lo rechazaría antes); clave
         * estable para no romper el flujo si el strip evoluciona. */
        Err(_) => serde_json::json!({"strip": "error"}),
    };
    sha_hex(&canon.to_string())
}

/// Hash del precio citado: el `precio_publico` ya formateado que ve la IA.
#[must_use]
pub fn precio_hash_seguro(seguro: &PromptSeguro) -> String {
    sha_hex(&seguro.precio_publico)
}

/// Hit de caché: el texto listo + si es corrección de la dueña. El `UPDATE`
/// atómico cuenta el uso en la misma sentencia (sin roundtrip ni carrera).
pub async fn buscar_cache(
    pool: &sqlx::PgPool,
    firma: &str,
    precio_hash: &str,
    catalog_hash: &str,
) -> Result<Option<(String, bool)>, AppError> {
    let fila: Option<(String, bool)> = sqlx::query!(
        "UPDATE mp_respuestas_cache SET usos = usos + 1 \
         WHERE firma = $1 AND precio_hash = $2 AND catalog_hash = $3 \
         AND valida_hasta > now() \
         RETURNING respuesta, corregida",
        firma,
        precio_hash,
        catalog_hash,
    )
    .fetch_optional(pool)
    .await?
    .map(|r| (r.respuesta, r.corregida));
    Ok(fila)
}

/// [08AA-21] Foto del hilo que acompaña a cada fila de caché: clave de la
/// ventana + excerpt limpio (panel) + excerpt crudo tal como llegó del
/// puente (diagnóstico del filtro, 08AA-8). Viaja junta para no engordar
/// la firma de `guardar_cache`/`reemplazar_cache` (clippy: máx 7 args).
pub struct FotoHilo<'a> {
    pub thread_id: &'a str,
    pub excerpt: &'a str,
    pub excerpt_crudo: &'a str,
}

/// [09AA-30] Coste de una generación IA: tokens del `usage` (`None` si el
/// relay no los trajo) y tiempo de la llamada en ms. Viaja con la fila para
/// que el admin muestre lo que costó el texto guardado; un hit no lo repite.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, ToSchema)]
pub struct Coste {
    pub tokens_entrada: Option<i64>,
    pub tokens_salida: Option<i64>,
    pub ms: Option<i64>,
}

/// Guarda una generación fresca; si la dueña ya corrigió esa clave, su texto
/// gana (`DO NOTHING`: la corrección humana no se pisa en silencio).
/// [07AA-7] Anota `thread_id` + `excerpt_texto` para el panel por chat.
/// [08AA-18] Guarda con `clave_hilo()`: la cifra inyectada por el puente
/// (07AA-11) parpadea entre llamadas y el `thread_id` literal no empareja
/// al releer.
/// [08AA-21] Guarda también `excerpt_crudo`: el texto tal como llegó del
/// puente, antes de `normalizar_excerpt`. El filtro por líneas no se puede
/// calibrar a ciegas (el puente aplana el DOM y lo pegado no se ve);
/// con el crudo a la vista se corrige el filtro (08AA-8).
pub async fn guardar_cache(
    pool: &sqlx::PgPool,
    firma: &str,
    precio_hash: &str,
    catalog_hash: &str,
    respuesta: &str,
    foto: &FotoHilo<'_>,
    coste: Coste,
) -> Result<(), AppError> {
    sqlx::query!(
        "INSERT INTO mp_respuestas_cache (firma, precio_hash, catalog_hash, respuesta, thread_id, excerpt_texto, excerpt_crudo, origen, tokens_entrada, tokens_salida, ms_generacion) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, 'ia', $8, $9, $10) ON CONFLICT DO NOTHING",
        firma,
        precio_hash,
        catalog_hash,
        respuesta,
        clave_hilo(foto.thread_id),
        foto.excerpt,
        foto.excerpt_crudo,
        coste.tokens_entrada,
        coste.tokens_salida,
        coste.ms,
    )
    .execute(pool)
    .await?;
    cambios::notificar();
    Ok(())
}

/// Pisa la fila (Regenerar explícito de la dueña): texto nuevo, vigencia
/// renovada, `corregida=FALSE`, contador a cero (nueva versión).
/// [07AA-7] Refresca también `thread_id` + `excerpt_texto` (foto actual).
/// [08AA-18] Guarda con `clave_hilo()` (ver `guardar_cache`).
/// [08AA-21] Refresca también `excerpt_crudo` (ver `guardar_cache`).
/// [09AA-36b] No pisa una corrección escrita entre la lectura del hit y este
/// `UPDATE`: `WHERE corregida = FALSE` deja la fila intacta (10AA-17).
pub async fn reemplazar_cache(
    pool: &sqlx::PgPool,
    firma: &str,
    precio_hash: &str,
    catalog_hash: &str,
    respuesta: &str,
    foto: &FotoHilo<'_>,
    coste: Coste,
) -> Result<(), AppError> {
    sqlx::query!(
        "INSERT INTO mp_respuestas_cache (firma, precio_hash, catalog_hash, respuesta, thread_id, excerpt_texto, excerpt_crudo, origen, tokens_entrada, tokens_salida, ms_generacion) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, 'ia', $8, $9, $10) \
         ON CONFLICT (firma, precio_hash, catalog_hash) DO UPDATE SET \
         respuesta = EXCLUDED.respuesta, valida_hasta = now() + INTERVAL '90 days', \
         corregida = FALSE, usos = 0, thread_id = EXCLUDED.thread_id, \
         excerpt_texto = EXCLUDED.excerpt_texto, excerpt_crudo = EXCLUDED.excerpt_crudo, \
         origen = EXCLUDED.origen, tokens_entrada = EXCLUDED.tokens_entrada, \
         tokens_salida = EXCLUDED.tokens_salida, ms_generacion = EXCLUDED.ms_generacion \
         WHERE mp_respuestas_cache.corregida = FALSE",
        firma,
        precio_hash,
        catalog_hash,
        respuesta,
        clave_hilo(foto.thread_id),
        foto.excerpt,
        foto.excerpt_crudo,
        coste.tokens_entrada,
        coste.tokens_salida,
        coste.ms,
    )
    .execute(pool)
    .await?;
    cambios::notificar();
    Ok(())
}

/// [08AA-31] Foto combinada del hilo (pura, sin BD): une la foto vieja con
/// el `limpio` nuevo, línea a línea, con dedup exacto y tope de 30 líneas
/// (las últimas). Sin esto, `releer_foto` pisaba todas las filas con el
/// último snapshot y si ese solo traía lo propio (`Tú:`), el mensaje del
/// cliente desaparecía del panel (hilo angelv). Las líneas del cliente
/// viajan sin etiqueta y las propias como `Tú:` (08AA-29): el dedup es por
/// línea exacta, así que ambos lados conviven.
#[must_use]
pub fn combinar_foto_hilo(vieja: &str, nueva: &str) -> String {
    use std::collections::HashSet as Conjunto;
    let mut vistas: Conjunto<String> = Conjunto::new();
    let mut lineas: Vec<&str> = Vec::new();
    for linea in vieja.lines().chain(nueva.lines()) {
        let t = linea.trim();
        if t.is_empty() || vistas.contains(t) {
            continue;
        }
        vistas.insert(t.to_string());
        lineas.push(t);
    }
    let desde = lineas.len().saturating_sub(30);
    lineas[desde..].join("\n")
}

/// [08AA-28] Releer con creación: refresca la foto del hilo; si no hay
/// fila (caché borrada o hilo nuevo sin borrador), la crea solo con la
/// foto y `respuesta` vacía para que el chat aparezca en el panel sin
/// inventar borrador. La PK sintética (`firma=sha("releer-sin-borrador|hilo")`,
/// `precio/catalog="releer"`) nunca choca con firmas HMAC reales, así un
/// borrador posterior inserta su propia fila y gana por `valida_hasta`.
/// `ON CONFLICT DO UPDATE` lo hace idempotente (doble clic o releers
/// concurrentes convergen; regla 6: upsert atómico, no buscar-crear).
/// [08AA-31] Fusiona con `combinar_foto_hilo` en vez de pisar: la foto
/// vieja aporta las líneas que el último snapshot ya no trae (el mensaje
/// del cliente cuando el eco propio es lo único nuevo).
/// Devuelve `(actualizado, creado)`.
pub async fn releer_foto(
    pool: &sqlx::PgPool,
    hilo: &str,
    limpio: &str,
    crudo: &str,
) -> Result<(bool, bool), AppError> {
    let vieja: Option<(String,)> = sqlx::query!(
        "SELECT excerpt_texto FROM mp_respuestas_cache WHERE thread_id = $1 \
         ORDER BY valida_hasta DESC LIMIT 1",
        clave_hilo(hilo),
    )
    .fetch_optional(pool)
    .await?
    .map(|r| (r.excerpt_texto,));
    let combinada = match &vieja {
        Some((v,)) => combinar_foto_hilo(v, limpio),
        None => limpio.to_string(),
    };
    let tocadas = sqlx::query!(
        "UPDATE mp_respuestas_cache SET excerpt_texto = $1, excerpt_crudo = $2 \
         WHERE thread_id = $3",
        &combinada,
        crudo,
        clave_hilo(hilo),
    )
    .execute(pool)
    .await?
    .rows_affected();
    if tocadas > 0 {
        // Solo avisa si el texto cambió: un releer idéntico no debe refrescar el panel.
        if vieja.as_ref().is_none_or(|(v,)| *v != combinada) {
            cambios::notificar();
        }
        return Ok((true, false));
    }
    let firma = sha_hex(&format!("releer-sin-borrador|{hilo}"));
    sqlx::query!(
        "INSERT INTO mp_respuestas_cache (firma, precio_hash, catalog_hash, respuesta, thread_id, excerpt_texto, excerpt_crudo, origen) \
         VALUES ($1, 'releer', 'releer', '', $2, $3, $4, 'releer') \
         ON CONFLICT (firma, precio_hash, catalog_hash) DO UPDATE SET \
         excerpt_texto = EXCLUDED.excerpt_texto, excerpt_crudo = EXCLUDED.excerpt_crudo",
        firma,
        clave_hilo(hilo),
        &combinada,
        crudo,
    )
    .execute(pool)
    .await?;
    cambios::notificar();
    Ok((true, true))
}

/// Borra la fila (primer paso de Regenerar: la siguiente lectura es miss).
pub async fn borrar_cache(
    pool: &sqlx::PgPool,
    firma: &str,
    precio_hash: &str,
    catalog_hash: &str,
) -> Result<(), AppError> {
    sqlx::query!(
        "DELETE FROM mp_respuestas_cache \
         WHERE firma = $1 AND precio_hash = $2 AND catalog_hash = $3",
        firma,
        precio_hash,
        catalog_hash,
    )
    .execute(pool)
    .await?;
    cambios::notificar();
    Ok(())
}

/// Guarda la corrección de la dueña (su texto manda tal cual: lo revisa
/// ella a mano al enviar). Vigencia renovada; la fila misma es el registro
/// (sin audit separada: `corregida=TRUE` + `usos` ya lo cuentan).
/// [08AA-14] Sin matriz negativa por decisión de ella 2026-10-08.
pub async fn corregir_cache(
    pool: &sqlx::PgPool,
    firma: &str,
    precio_hash: &str,
    catalog_hash: &str,
    texto: &str,
) -> Result<(), AppError> {
    let n = texto.chars().count();
    if n == 0 || n > 2000 {
        return Err(AppError::Validation("texto 1..2000 caracteres".to_string()));
    }
    sqlx::query!(
        "INSERT INTO mp_respuestas_cache (firma, precio_hash, catalog_hash, respuesta, corregida) \
         VALUES ($1, $2, $3, $4, TRUE) \
         ON CONFLICT (firma, precio_hash, catalog_hash) DO UPDATE SET \
         respuesta = EXCLUDED.respuesta, valida_hasta = now() + INTERVAL '90 days', \
         corregida = TRUE, usos = 0",
        firma,
        precio_hash,
        catalog_hash,
        texto,
    )
    .execute(pool)
    .await?;
    cambios::notificar();
    Ok(())
}

/// Purga vencidas; devuelve cuántas cayeron. Se corre al arrancar (siempre) y
/// a diario vía `pg_cron` (solo `DB_24H=true`).
pub async fn purgar_cache(pool: &sqlx::PgPool) -> Result<u64, sqlx::Error> {
    let r = sqlx::query!("DELETE FROM mp_respuestas_cache WHERE valida_hasta <= now()")
        .execute(pool)
        .await?;
    Ok(r.rows_affected() + purgar_compartida(pool).await?)
}

/// Programa la purga diaria en `pg_cron` (07:00 UTC = 03:00 Caracas, sin horario
/// de verano). Idempotente (reemplaza el job si existe). Falla si no hay
/// `pg_cron` en el servidor: el llamador lo deja en `warn` y sigue (la purga al
/// arrancar ya cubre; fail-open documentado, nunca tumba el boot).
pub async fn programar_purga_diaria(pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    sqlx::query!("CREATE EXTENSION IF NOT EXISTS pg_cron")
        .execute(pool)
        .await?;
    sqlx::query!(
        "DO $purga$ BEGIN \
           IF EXISTS (SELECT 1 FROM cron.job WHERE jobname = 'mp-purga-diaria') THEN \
             PERFORM cron.unschedule('mp-purga-diaria'); \
           END IF; \
           PERFORM cron.schedule('mp-purga-diaria', '0 7 * * *', \
             'DELETE FROM mp_respuestas_cache WHERE valida_hasta <= now()'); \
         END $purga$",
    )
    .execute(pool)
    .await?;
    Ok(())
}
