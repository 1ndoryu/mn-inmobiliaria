//! Consultas SQL de marketplace (movidas desde `handlers/marketplace*.rs`).

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

/// `vigente` de un token mp emitido (`NOT revocada AND expira_en > now()`).
/// `None` si el `jti` no existe; el handler decide el 401.
pub(crate) async fn token_vigente(pool: &PgPool, jti: &str) -> Result<Option<bool>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT NOT revocada AND expira_en > now() FROM mp_tokens_emitidos WHERE jti = $1",
    )
    .bind(jti)
    .fetch_optional(pool)
    .await
}

/// Inserta un evento de auditoría con `HMAC(secreto, clave_hilo)` y la hora
/// truncada. `sha256()` y `encode()` son nativos de Postgres: no hay HMAC en Rust.
pub(crate) async fn registrar_auditoria(
    pool: &PgPool,
    secreto: &str,
    clave_hilo: &str,
    evento: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO mp_auditoria (hilo_hmac, ts_hora, evento) \
         SELECT encode(sha256(($1 || $2)::bytea), 'hex'), date_trunc('hour', now()), $3",
        secreto,
        clave_hilo,
        evento
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// [13AA-4] Aviso validado por el handler, listo para guardar.
pub(crate) struct AvisoPerfilNuevo {
    pub aviso_id: String,
    pub titulo: String,
    pub precio: Option<String>,
    pub url: String,
    pub imagen_url: Option<String>,
}

/// [13AA-4] Aviso del perfil presente y sin inmueble vinculado.
#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct AvisoPerfilRow {
    pub aviso_id: String,
    pub titulo: String,
    pub precio: Option<String>,
    pub url: String,
    pub imagen_url: Option<String>,
}

/// [13AA-4] Inmueble sin `marketplace_id`: candidato del modal de vinculación.
#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct InmuebleSinVincular {
    pub id: Uuid,
    pub titulo: String,
    pub alias: Vec<String>,
    pub precio: f64,
}

/// [13AA-4] Reemplaza el estado del perfil con una lectura, en una transacción:
/// los avisos que no vienen pasan a `presente = FALSE` (no se borran) y los que
/// vienen se insertan o actualizan. Devuelve cuántos dejaron de aparecer.
pub(crate) async fn reemplazar_avisos_perfil(
    pool: &PgPool,
    avisos: &[AvisoPerfilNuevo],
) -> Result<u64, sqlx::Error> {
    let ids: Vec<String> = avisos.iter().map(|a| a.aviso_id.clone()).collect();
    let mut tx = pool.begin().await?;
    // `query!` comprueba el SQL y los tipos contra la BD en compilación (caché `.sqlx`).
    let ausentes = sqlx::query!(
        "UPDATE marketplace_avisos_perfil SET presente = FALSE \
         WHERE presente AND NOT (aviso_id = ANY($1))",
        &ids
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();
    for a in avisos {
        sqlx::query!(
            "INSERT INTO marketplace_avisos_perfil (aviso_id, titulo, precio, url, imagen_url, visto_en, presente) \
             VALUES ($1, $2, $3, $4, $5, now(), TRUE) \
             ON CONFLICT (aviso_id) DO UPDATE SET titulo = EXCLUDED.titulo, \
             precio = EXCLUDED.precio, url = EXCLUDED.url, imagen_url = EXCLUDED.imagen_url, \
             visto_en = now(), presente = TRUE",
            a.aviso_id,
            a.titulo,
            a.precio,
            a.url,
            a.imagen_url
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(ausentes)
}

/// [13AA-4] Avisos presentes en el perfil que ningún inmueble reclama.
pub(crate) async fn avisos_perfil_sin_vincular(
    pool: &PgPool,
) -> Result<Vec<AvisoPerfilRow>, sqlx::Error> {
    // sentinel-disable-next-line sqlx-query-as-sin-macro -- SQL fijo sin macro: no se regenera la caché `.sqlx/` para una consulta de lectura nueva
    sqlx::query_as::<_, AvisoPerfilRow>(
        "SELECT aviso_id, titulo, precio, url, imagen_url FROM marketplace_avisos_perfil \
         WHERE presente AND aviso_id NOT IN \
         (SELECT marketplace_id FROM inmuebles WHERE marketplace_id IS NOT NULL) \
         ORDER BY titulo",
    )
    .fetch_all(pool)
    .await
}

/// [13AA-4] Inmuebles sin `marketplace_id` (publicados o no): candidatos del modal.
pub(crate) async fn inmuebles_sin_vincular(
    pool: &PgPool,
) -> Result<Vec<InmuebleSinVincular>, sqlx::Error> {
    // sentinel-disable-next-line sqlx-query-as-sin-macro -- SQL fijo sin macro: no se regenera la caché `.sqlx/` para una consulta de lectura nueva
    sqlx::query_as::<_, InmuebleSinVincular>(
        "SELECT id, titulo, alias_titulos AS alias, precio FROM inmuebles \
         WHERE marketplace_id IS NULL ORDER BY titulo",
    )
    .fetch_all(pool)
    .await
}

/// [13AA-4] Hora de la última lectura: la más reciente de un aviso presente.
/// `None` si el perfil nunca se ha leído.
pub(crate) async fn ultima_lectura_perfil(
    pool: &PgPool,
) -> Result<Option<DateTime<Utc>>, sqlx::Error> {
    sqlx::query_scalar("SELECT MAX(visto_en) FROM marketplace_avisos_perfil WHERE presente")
        .fetch_one(pool)
        .await
}
