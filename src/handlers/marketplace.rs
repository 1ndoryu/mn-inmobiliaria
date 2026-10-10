/* [03AA-3 M3] HTTP del asistente Marketplace (token, borrador, audit).
 * [03AA-3 M2] suma `uso`: dashboard agregado (día+evento+conteo, sin PII)
 * con el JWT admin. [03AA-3 M4] suma caché (`regenerar`, `corregir`):
 * hit/miss por (firma, precio, catálogo), sin servir precio viejo.
 * La lógica pura vive en `services::marketplace`; aquí
 * solo boundary HTTP + 429 con `Retry-After`. */

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::repositories::InmuebleRepository;
use crate::services::marketplace::{
    archivar_hilo, aviso_fb_de_thread, borrar_borrador_hilo, borrar_hilo,
    borrar_hilo_no_corregidas, borrar_todo_cache, buscar_compartida, clave_hilo, consumir_minuto,
    corregir_cache, corregir_compartida, detalle_chat, enlazar_compartida, formatear_parrafos,
    guardar_cache, hash_ficha, nombre_de_thread, normalizar_excerpt_hilo, ocurrencias_nombre,
    plantilla_de_nombre, precio_hash_seguro, reemplazar_cache, releer_foto, rellenar_nombre,
    resumen_chats, resumen_uso, sha_hex, strip_ficha_para_prompt, sub_exento, validar_borrador,
    vincular_hilo_compartido, vinculo_compartido, BorradorRequest, ClaveCompartida, FotoHilo,
    FALLBACK_BORRADOR, FIRMA_VERSION_V2, SIN_FICHA, STRIP_VERSION,
};
use crate::AppState;

/* F0 estructuradas/idempotencia vive en `marketplace_estructuradas.rs`
 * (split god-object): aquí solo el wiring para `borrador`/`regenerar`. */
/* [09AA-22] F3 suma al wiring: verificación de la llave contra hint+firma
 * + lectura con convivencia v2→v1 (lógica en el módulo, aquí llamadas). */
use super::marketplace_estructuradas::{
    buscar_cache_convivencia, clave_idempotencia, con_idempotencia, resolver_fuente,
    verificar_idempotencia_conversacion, FuenteBorrador,
};

/* [08AA-8] Token mp (extractor + emisión) vive en `marketplace_token.rs`
 * (split límite 500). Re-export `pub` para las rutas utoipa de `mod.rs`
 * (`marketplace::emitir_token`…); `MpAuth`/`limite` para este boundary. */
use super::marketplace_token::limite;
pub use super::marketplace_token::{
    emitir_token, emitir_token_cli, CliTokenRequest, MpAuth, TokenResponse,
};
/* [09AA-5] Buffer de eventos del puente para la tab de Logs (sin PII). */
use super::mp_logs::{hilo8, log_borrador_cache, log_borrador_ia, mp_log, LogNivel};

pub mod chats_admin;

mod borrador;
mod claves;
mod corregir;
mod regenerar;

pub use borrador::*;
// Los hermanos (borrador, regenerar, corregir) usan estos items vía `use super::*`.
#[allow(clippy::wildcard_imports)]
use claves::*;
pub use corregir::*;
pub use regenerar::*;


#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum EventoAudit {
    Hit,
    Miss,
    Copiar,
    Regenerar,
    Emision,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AuditIn {
    pub thread_id: String,
    pub evento: EventoAudit,
}

/// Auditoría agregada: guarda `HMAC(secreto, threadId)` (sal del servidor,
/// jamás el texto) con la hora truncada. Sin PII en reposo ni en tránsito.
#[utoipa::path(
    post,
    path = "/api/admin/marketplace/audit",
    request_body = AuditIn,
    responses(
        (status = 201, description = "Evento registrado"),
        (status = 422, description = "Evento inválido", body = crate::errors::ErrorResponse)
    )
)]
pub async fn audit(
    State(state): State<AppState>,
    _auth: MpAuth,
    r: Result<Json<AuditIn>, axum::extract::rejection::JsonRejection>,
) -> Result<Response, AppError> {
    let r = r.map_err(|e| AppError::Validation(format!("JSON inválido: {e}")))?;
    if r.thread_id.trim().is_empty() {
        return Err(AppError::Validation("thread_id requerido".to_string()));
    }
    let evento = match r.evento {
        EventoAudit::Hit => "hit",
        EventoAudit::Miss => "miss",
        EventoAudit::Copiar => "copiar",
        EventoAudit::Regenerar => "regenerar",
        EventoAudit::Emision => "emision",
    };
    /* HMAC sin dependencias nuevas: `sha256()` y `encode()` son nativos de
     * Postgres; el secreto viaja solo en el parámetro dentro del servidor.
     * [08AA-18] HMAC sobre `clave_hilo()`: la misma auditoría aunque la
     * cifra inyectada (07AA-11) parpadee entre llamadas. */
    sqlx::query(
        "INSERT INTO mp_auditoria (hilo_hmac, ts_hora, evento) \
         SELECT encode(sha256(($1 || $2)::bytea), 'hex'), date_trunc('hour', now()), $3",
    )
    .bind(&state.jwt_secret)
    .bind(clave_hilo(r.thread_id.trim()))
    .bind(evento)
    .execute(&state.pool)
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({"registrado": true})),
    )
        .into_response())
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/marketplace/token", post(emitir_token))
        .route("/marketplace/token/cli", post(emitir_token_cli))
        .route("/marketplace/borrador", post(borrador))
        .route("/marketplace/regenerar", post(regenerar))
        .route("/marketplace/releer", post(releer))
        .route("/marketplace/corregir", post(corregir))
        .route("/marketplace/audit", post(audit))
        .route("/marketplace/uso", get(uso))
        .route(
            "/marketplace/chats",
            get(chats_admin::chats).delete(chats_admin::borrar_todo),
        )
        .route(
            "/marketplace/chats/:thread",
            get(chats_admin::chat_detalle).delete(chats_admin::borrar_chat),
        )
        .route(
            "/marketplace/chats/:thread/archivar",
            post(chats_admin::archivar_chat),
        )
        .route(
            "/marketplace/chats/:thread/borrar-borrador",
            post(chats_admin::borrar_borrador_chat),
        )
        .route(
            "/marketplace/borradores/version",
            get(chats_admin::version_borradores),
        )
        /* [09AA-5] Tab de Logs: eventos del puente, recientes-primero. */
        .route("/marketplace/logs", get(super::mp_logs::logs))
}

/// [03AA-3 M2] Dashboard agregado para el panel: conteos por día y evento de
/// los últimos `dias` (default 7, tope 90). Solo JWT admin; sin PII.
#[derive(Debug, Clone, Deserialize, utoipa::IntoParams, utoipa::ToSchema)]
pub struct UsoQuery {
    pub dias: Option<i32>,
}

#[utoipa::path(
    get,
    path = "/api/admin/marketplace/uso",
    params(UsoQuery),
    responses(
        (status = 200, description = "Uso agregado por día", body = Vec<crate::services::marketplace::UsoDia>)
    )
)]
pub async fn uso(
    State(state): State<AppState>,
    _auth: AuthUser,
    Query(q): Query<UsoQuery>,
) -> Result<Response, AppError> {
    let filas = resumen_uso(&state.pool, q.dias.unwrap_or(7)).await?;
    Ok((StatusCode::OK, Json(filas)).into_response())
}

/* [09AA-21] Matriz de `claves_cache`: ID exacto válido (ficha + conocido),
 * ID exacto inexistente (sin ficha, sin fallback al título), sin ID con
 * título que empareja (ficha por título) y UUID legacy intacto (ficha por
 * UUID). Humo contra la BD real de rama (`DATABASE_URL`); sin ella se omite.
 * El borrador jamás se bloquea: los casos negativos dan `SIN_FICHA`. */
#[cfg(test)]
mod pruebas_claves_cache_mp_id;
