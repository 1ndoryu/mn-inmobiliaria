//! Consultas SQL de marketplace (movidas desde `handlers/marketplace*.rs`).

use sqlx::PgPool;

/// `vigente` de un token mp emitido (`NOT revocada AND expira_en > now()`).
/// `None` si el `jti` no existe; el handler decide el 401.
pub(crate) async fn token_vigente(
    pool: &PgPool,
    jti: &str,
) -> Result<Option<bool>, sqlx::Error> {
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
    sqlx::query(
        "INSERT INTO mp_auditoria (hilo_hmac, ts_hora, evento) \
         SELECT encode(sha256(($1 || $2)::bytea), 'hex'), date_trunc('hour', now()), $3",
    )
    .bind(secreto)
    .bind(clave_hilo)
    .bind(evento)
    .execute(pool)
    .await?;
    Ok(())
}
