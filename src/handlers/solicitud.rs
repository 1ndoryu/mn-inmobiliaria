use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use serde::Deserialize;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{
    CreateSolicitudRequest, FotoSolicitudSubida, PaginatedSolicitudes, Solicitud,
    SolicitudesAdminParams, UpdateEstadoSolicitud,
};
use crate::services::SolicitudService;
use crate::AppState;

/* [169A-2] "Publicar mi inmueble": el visitante sube fotos y deja la
 * solicitud en `pendiente`; el admin lista y mueve el estado con JWT.
 * Las fotos públicas viajan como bytes crudos igual que las del admin. */

/// Parámetros de subida pública — el cuerpo son los bytes de la imagen
#[derive(Debug, Deserialize)]
pub struct SubidaPublicaParams {
    /// Nombre original (solo se usa su extensión)
    pub filename: String,
}

/// Subir una foto para adjuntar a una solicitud (sin JWT; la clave la genera el servidor)
#[utoipa::path(
    post,
    path = "/api/public/solicitudes/fotos",
    params(
        ("filename" = String, Query, description = "Nombre original (vale su extensión)"),
    ),
    request_body(content = Vec<u8>, description = "Bytes de la imagen (jpg/png/webp, máx 10 MiB)", content_type = "application/octet-stream"),
    responses(
        (status = 201, description = "Foto subida (adjuntar su clave al alta)", body = FotoSolicitudSubida),
        (status = 400, description = "Archivo inválido", body = crate::errors::ErrorResponse)
    )
)]
pub async fn subir_foto_solicitud(
    State(state): State<AppState>,
    Query(params): Query<SubidaPublicaParams>,
    bytes: Bytes,
) -> Result<(StatusCode, Json<FotoSolicitudSubida>), AppError> {
    let foto =
        SolicitudService::subir_foto_publica(&state.upload_dir, &params.filename, bytes.as_ref())
            .await?;
    Ok((StatusCode::CREATED, Json(foto)))
}

/// Dejar una solicitud "Publicar mi inmueble" (entra en `pendiente`)
#[utoipa::path(
    post,
    path = "/api/public/solicitudes",
    request_body = CreateSolicitudRequest,
    responses(
        (status = 201, description = "Solicitud registrada (pendiente de revisión)", body = Solicitud),
        (status = 422, description = "Error de validación", body = crate::errors::ErrorResponse)
    )
)]
pub async fn create_solicitud(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateSolicitudRequest>,
) -> Result<(StatusCode, Json<Solicitud>), AppError> {
    /* [169A-4] IP de cabeceras (tras el proxy manda X-Forwarded-For; en
     * local puede no venir: None, sin tumbar el alta). Solo auditoría. */
    let ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
        });
    let user_agent = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let solicitud = SolicitudService::create(&state.pool, req, ip, user_agent).await?;
    Ok((StatusCode::CREATED, Json(solicitud)))
}

/// Listar solicitudes para revisión (requiere JWT; filtro opcional por estado)
#[utoipa::path(
    get,
    path = "/api/admin/solicitudes",
    params(SolicitudesAdminParams),
    responses(
        (status = 200, description = "Solicitudes pendientes de revisión", body = PaginatedSolicitudes),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_solicitudes(
    State(state): State<AppState>,
    _auth: AuthUser,
    Query(params): Query<SolicitudesAdminParams>,
) -> Result<Json<PaginatedSolicitudes>, AppError> {
    let lista = SolicitudService::list_admin(&state.pool, params).await?;
    Ok(Json(lista))
}

/// Mover el estado de revisión de una solicitud (requiere JWT)
#[utoipa::path(
    patch,
    path = "/api/admin/solicitudes/{id}",
    params(("id" = Uuid, Path, description = "ID de la solicitud")),
    request_body = UpdateEstadoSolicitud,
    responses(
        (status = 200, description = "Estado actualizado", body = Solicitud),
        (status = 404, description = "No encontrada", body = crate::errors::ErrorResponse),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse),
        (status = 422, description = "Error de validación", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn revisar_solicitud(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateEstadoSolicitud>,
) -> Result<Json<Solicitud>, AppError> {
    let solicitud = SolicitudService::update_estado(&state.pool, id, req).await?;
    Ok(Json(solicitud))
}

pub fn public_routes() -> Router<AppState> {
    /* Sin el límite por defecto de 2 MB: el servicio impone 10 MiB con 413 */
    Router::new()
        .route("/solicitudes", post(create_solicitud))
        .route(
            "/solicitudes/fotos",
            post(subir_foto_solicitud).layer(DefaultBodyLimit::disable()),
        )
}

pub fn admin_routes() -> Router<AppState> {
    Router::new()
        .route("/solicitudes", get(list_solicitudes))
        .route("/solicitudes/:id", patch(revisar_solicitud))
}
