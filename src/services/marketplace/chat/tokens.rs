use chrono::DateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::AppError;

/// Claims del JWT mp (`iss mn-backend`, `aud mp`, `scope mp:borrador`).
/// `mid` (E3, solo CLI): hash hex64 de la máquina atada; `None` = token de
/// panel sin binding. `default` para que los tokens de panel en vuelo (sin
/// `mid`) sigan decodificando.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MpClaims {
    pub iss: String,
    pub sub: String,
    pub aud: String,
    pub scope: String,
    pub exp: usize,
    pub jti: String,
    #[serde(default)]
    pub mid: Option<String>,
}

/* [08AA-20] Vida del token CLI configurable: `MP_CLI_MINUTOS` manda
 * (local: 43200 = 30d en `.env`, gitignored); ausente/inválido → 480
 * (8h, lo que sigue viendo producción). El panel queda fijo en 15min.
 * Por qué env y no quitar la expiración: `jti`+revocación y binding a
 * máquina siguen valiendo; solo se estira el `exp`. */
#[must_use]
pub fn minutos_para_cli(es_cli: bool) -> i64 {
    if !es_cli {
        return 15;
    }
    std::env::var("MP_CLI_MINUTOS")
        .ok()
        .and_then(|v| v.trim().parse::<i64>().ok())
        .filter(|&m| m > 0)
        .unwrap_or(480)
}

/// Hash de máquina válido: 64 hex (igual que `firma`; nunca el id en claro).
#[must_use]
pub fn maquina_valida(mid: &str) -> bool {
    mid.len() == 64 && mid.chars().all(|c| c.is_ascii_hexdigit())
}

/// Binding E3: con `mid` en el token, la petición debe traer la misma máquina
/// en `X-MP-Maquina`; sin `mid` (panel) no se exige nada.
#[must_use]
pub fn maquina_autorizada(claims_mid: Option<&str>, cabecera: Option<&str>) -> bool {
    match (claims_mid, cabecera) {
        (None, _) => true,
        (Some(_), None) => false,
        (Some(a), Some(b)) => a == b,
    }
}

/// `MP_SIN_LIMITE_SUB` (coma-separada): la dueña queda exenta del 429 del
/// borrador; el resto cae al tope de 30/min con `Retry-After`.
#[must_use]
pub fn sub_exento(sub: &str) -> bool {
    std::env::var("MP_SIN_LIMITE_SUB")
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .any(|s| !s.is_empty() && s == sub)
}

/// Cubo por minuto atómico (`clave` + `date_trunc('minute')`): devuelve las
/// peticiones acumuladas incluyendo la actual.
pub async fn consumir_minuto(
    pool: &sqlx::PgPool,
    clave: &str,
    limite: i64,
) -> Result<bool, AppError> {
    let n: i64 = sqlx::query_scalar(
        "INSERT INTO mp_uso_minuto (clave, ventana, n) \
         VALUES ($1, date_trunc('minute', now()), 1) \
         ON CONFLICT (clave, ventana) DO UPDATE SET n = mp_uso_minuto.n + 1 \
         RETURNING n::BIGINT",
    )
    .bind(clave)
    .fetch_one(pool)
    .await?;
    Ok(n <= limite)
}

/// Emite un `jti` fresco y lo registra para poder revocarlo.
pub async fn registrar_token(
    pool: &sqlx::PgPool,
    sub: &str,
    expira_en: &DateTime<chrono::Utc>,
) -> Result<String, AppError> {
    let jti = Uuid::new_v4().to_string();
    sqlx::query!(
        "INSERT INTO mp_tokens_emitidos (jti, sub, expira_en) VALUES ($1, $2, $3)",
        &jti,
        sub,
        expira_en,
    )
    .execute(pool)
    .await?;
    Ok(jti)
}
