//! [09AA-31] Panel admin de chats de marketplace (listar, detalle, archivar,
//! borrar y borrar borrador). Movido desde `marketplace.rs` para bajar del límite
//! god-object de Sentinel (>800 líneas efectivas).

use super::{
    archivar_hilo, borrar_borrador_hilo, borrar_hilo, borrar_todo_cache, detalle_chat, hilo8,
    mp_log, resumen_chats, AppError, AppState, AuthUser, IntoResponse, Json, LogNivel, MpAuth,
    Path, Query, Response, State, StatusCode,
};
use crate::services::marketplace::CursorChats;
use serde::Deserialize;
use utoipa::IntoParams;

/// [09AA-31] Paginación del panel: `limite` (25 por defecto, 1–100) y el cursor
/// de la última fila vista (`antes_ultimo` + `antes_hilo`, ambos o ninguno).
#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct PaginaChatsQuery {
    /// Tamaño de página (default 25, entre 1 y 100).
    pub limite: Option<i64>,
    /// Cursor: `ultimo` (RFC 3339) de la última fila de la página anterior.
    pub antes_ultimo: Option<String>,
    /// Cursor: `thread_id` de esa misma fila.
    pub antes_hilo: Option<String>,
    /// [10AA-4] `true`: solo hilos sin ficha conocida (filtrado en el backend).
    pub solo_huerfanos: Option<bool>,
}

/// [09AA-31] Traduce el cursor de la query; debe venir completo o no venir.
fn cursor_de(ultimo: Option<&str>, hilo: Option<&str>) -> Result<Option<CursorChats>, AppError> {
    match (ultimo, hilo) {
        (None, None) => Ok(None),
        (Some(u), Some(h)) => {
            let ultimo = chrono::DateTime::parse_from_rfc3339(u)
                .map_err(|_| AppError::Validation("antes_ultimo debe ser RFC 3339".to_string()))?
                .with_timezone(&chrono::Utc);
            Ok(Some(CursorChats {
                ultimo,
                thread_id: h.to_string(),
            }))
        }
        _ => Err(AppError::Validation(
            "antes_ultimo y antes_hilo van juntos".to_string(),
        )),
    }
}

/// [07AA-7] Panel por chat: una página de hilos con conteos (keyset). Solo JWT admin.
#[utoipa::path(
    get,
    path = "/api/admin/marketplace/chats",
    params(PaginaChatsQuery),
    responses(
        (status = 200, description = "Página de chats con borradores", body = crate::services::marketplace::PaginaResumen),
        (status = 400, description = "Cursor inválido")
    )
)]
pub async fn chats(
    State(state): State<AppState>,
    _auth: AuthUser,
    Query(q): Query<PaginaChatsQuery>,
) -> Result<Response, AppError> {
    let limite = q.limite.unwrap_or(25).clamp(1, 100);
    let antes = cursor_de(q.antes_ultimo.as_deref(), q.antes_hilo.as_deref())?;
    let solo_huerfanos = q.solo_huerfanos.unwrap_or(false);
    let pagina = resumen_chats(&state.pool, limite, antes.as_ref(), solo_huerfanos).await?;
    Ok((StatusCode::OK, Json(pagina)).into_response())
}

/// [07AA-7] Panel por chat: filas de un hilo (extracto + respuesta).
/// Solo JWT admin.
#[utoipa::path(
    get,
    path = "/api/admin/marketplace/chats/{thread}",
    params(("thread" = String, Path, description = "Clave del hilo")),
    responses(
        (status = 200, description = "Borradores del hilo", body = Vec<crate::services::marketplace::ChatFila>),
        (status = 422, description = "Hilo vacío", body = crate::errors::ErrorResponse)
    )
)]
pub async fn chat_detalle(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(thread): Path<String>,
) -> Result<Response, AppError> {
    let hilo = thread.trim();
    if hilo.is_empty() {
        return Err(AppError::Validation("thread requerido".to_string()));
    }
    let filas = detalle_chat(&state.pool, hilo).await?;
    Ok((StatusCode::OK, Json(filas)).into_response())
}

/// [08AA-39] Limpieza total del panel: borra toda la caché de borradores.
/// Solo JWT admin. Responde cuántas filas cayeron.
#[utoipa::path(
    delete,
    path = "/api/admin/marketplace/chats",
    responses(
        (status = 200, description = "Caché limpiada")
    )
)]
pub async fn borrar_todo(
    State(state): State<AppState>,
    _auth: AuthUser,
) -> Result<Response, AppError> {
    let n = borrar_todo_cache(&state.pool).await?;
    Ok((StatusCode::OK, Json(serde_json::json!({"borrados": n}))).into_response())
}

/// [09AA-30] Archiva un hilo: lo quita de la lista del panel sin borrar nada.
/// Solo JWT admin.
#[utoipa::path(
    post,
    path = "/api/admin/marketplace/chats/{thread}/archivar",
    params(("thread" = String, Path, description = "Clave del hilo")),
    responses(
        (status = 200, description = "Hilo archivado"),
        (status = 422, description = "Hilo vacío", body = crate::errors::ErrorResponse)
    )
)]
pub async fn archivar_chat(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(thread): Path<String>,
) -> Result<Response, AppError> {
    let hilo = thread.trim();
    if hilo.is_empty() {
        return Err(AppError::Validation("thread requerido".to_string()));
    }
    archivar_hilo(&state.pool, hilo).await?;
    mp_log(
        LogNivel::Info,
        "chat.archivar",
        "ok",
        "hilo archivado desde el panel".to_string(),
        &[("hilo", serde_json::json!(hilo8(hilo)))],
    );
    Ok((StatusCode::OK, Json(serde_json::json!({"archivado": true}))).into_response())
}

/// [09AA-30] Borra la conversación entera: borradores, correcciones y su marca
/// de archivado. La respuesta compartida del inmueble no se toca. Solo JWT admin.
#[utoipa::path(
    delete,
    path = "/api/admin/marketplace/chats/{thread}",
    params(("thread" = String, Path, description = "Clave del hilo")),
    responses(
        (status = 200, description = "Conversación borrada"),
        (status = 422, description = "Hilo vacío", body = crate::errors::ErrorResponse)
    )
)]
pub async fn borrar_chat(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(thread): Path<String>,
) -> Result<Response, AppError> {
    let hilo = thread.trim();
    if hilo.is_empty() {
        return Err(AppError::Validation("thread requerido".to_string()));
    }
    let n = borrar_hilo(&state.pool, hilo).await?;
    mp_log(
        LogNivel::Info,
        "chat.borrar",
        "ok",
        "conversación borrada desde el panel".to_string(),
        &[
            ("hilo", serde_json::json!(hilo8(hilo))),
            ("filas", serde_json::json!(n)),
        ],
    );
    Ok((StatusCode::OK, Json(serde_json::json!({"borrados": n}))).into_response())
}

/// [09AA-30] Borra solo los borradores no corregidos del hilo, y de la
/// compartida lo que quede sin enlazar. Solo JWT admin.
#[utoipa::path(
    post,
    path = "/api/admin/marketplace/chats/{thread}/borrar-borrador",
    params(("thread" = String, Path, description = "Clave del hilo")),
    responses(
        (status = 200, description = "Borradores no corregidos borrados"),
        (status = 422, description = "Hilo vacío", body = crate::errors::ErrorResponse)
    )
)]
pub async fn borrar_borrador_chat(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(thread): Path<String>,
) -> Result<Response, AppError> {
    let hilo = thread.trim();
    if hilo.is_empty() {
        return Err(AppError::Validation("thread requerido".to_string()));
    }
    let n = borrar_borrador_hilo(&state.pool, hilo).await?;
    mp_log(
        LogNivel::Info,
        "chat.borrar_borrador",
        "ok",
        "borradores del hilo borrados desde el panel".to_string(),
        &[
            ("hilo", serde_json::json!(hilo8(hilo))),
            ("filas", serde_json::json!(n)),
        ],
    );
    Ok((StatusCode::OK, Json(serde_json::json!({"borrados": n}))).into_response())
}

/// [09AA-31] Contador de borradores borrados desde el panel, para que el float
/// de lab vacíe su caché en memoria. Lo consulta cada 5 s con el token del
/// puente (`MpAuth`), igual que `borrador`; sin sesión admin.
#[utoipa::path(
    get,
    path = "/api/admin/marketplace/borradores/version",
    responses(
        (status = 200, description = "Versión del contador de borradores borrados"),
        (status = 401, description = "Token del puente inválido", body = crate::errors::ErrorResponse)
    )
)]
pub async fn version_borradores(_auth: MpAuth) -> Result<Response, AppError> {
    let version = crate::services::marketplace::version_borradores_borrados();
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({"version": version})),
    )
        .into_response())
}
