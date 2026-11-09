//! [30AA-1] Rutas admin de versiones de texto y fotos variante.

use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, put};
use axum::{Json, Router};
use serde::Deserialize;
use uuid::Uuid;
use validator::Validate;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{
    EditarVersionRequest, FotoVariante, GenerarVersionRequest, GenerarVersionSalida,
    InmuebleVersion, VersionesInmueble,
};
use crate::repositories::VersionRepository;
use crate::services::versiones::VersionService;
use crate::AppState;

/// Versiones, fotos variante e historial de un inmueble
#[utoipa::path(
    get,
    path = "/api/admin/inmuebles/{id}/versiones",
    params(("id" = Uuid, Path, description = "Inmueble")),
    responses(
        (status = 200, description = "Detalle de versiones", body = VersionesInmueble),
        (status = 404, description = "Inmueble no encontrado", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn detalle(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<VersionesInmueble>, AppError> {
    Ok(Json(VersionService::detalle(&state.pool, id).await?))
}

/// Generar una variante de título y descripción con IA
#[utoipa::path(
    post,
    path = "/api/admin/inmuebles/{id}/versiones",
    request_body = GenerarVersionRequest,
    params(("id" = Uuid, Path, description = "Inmueble")),
    responses(
        (status = 200, description = "Resultado de la generación", body = GenerarVersionSalida),
        (status = 422, description = "Petición inválida", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn generar(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<GenerarVersionRequest>,
) -> Result<Json<GenerarVersionSalida>, AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    Ok(Json(VersionService::generar(&state.pool, id, &req).await?))
}

/// Editar a mano una variante (y marcarla lista)
#[utoipa::path(
    put,
    path = "/api/admin/inmuebles/{id}/versiones/{n}",
    request_body = EditarVersionRequest,
    params(
        ("id" = Uuid, Path, description = "Inmueble"),
        ("n" = i16, Path, description = "Número de versión")
    ),
    responses(
        (status = 200, description = "Versión actualizada", body = InmuebleVersion),
        (status = 404, description = "Versión no encontrada", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn editar(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path((id, n)): Path<(Uuid, i16)>,
    Json(req): Json<EditarVersionRequest>,
) -> Result<Json<InmuebleVersion>, AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    Ok(Json(VersionService::editar(&state.pool, id, n, &req).await?))
}

/// Borrar una variante
#[utoipa::path(
    delete,
    path = "/api/admin/inmuebles/{id}/versiones/{n}",
    params(
        ("id" = Uuid, Path, description = "Inmueble"),
        ("n" = i16, Path, description = "Número de versión")
    ),
    responses(
        (status = 204, description = "Borrada"),
        (status = 404, description = "Versión no encontrada", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn borrar(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path((id, n)): Path<(Uuid, i16)>,
) -> Result<StatusCode, AppError> {
    VersionService::borrar(&state.pool, id, n).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Parámetros de subida de una variante de foto
#[derive(Debug, Deserialize)]
pub struct VarianteParams {
    /// Posición del original al que sustituye
    pub orden: i32,
    /// Extensión del archivo (`foto.jpg`)
    pub filename: String,
    #[serde(default)]
    pub modelo: Option<String>,
    #[serde(default)]
    pub prompt_usado: Option<String>,
}

/// Subir la variante IA de una foto (bytes crudos, como `/fotos/upload`)
#[utoipa::path(
    post,
    path = "/api/admin/inmuebles/{id}/fotos/variante",
    params(
        ("id" = Uuid, Path, description = "Inmueble"),
        ("orden" = i32, Query, description = "Posición del original"),
        ("filename" = String, Query, description = "Nombre (vale la extensión)"),
        ("modelo" = Option<String>, Query, description = "Modelo que generó la variante"),
        ("prompt_usado" = Option<String>, Query, description = "Prompt usado")
    ),
    request_body(content = Vec<u8>, description = "Bytes de la imagen", content_type = "application/octet-stream"),
    responses(
        (status = 201, description = "Variante guardada", body = FotoVariante),
        (status = 422, description = "Posición inválida", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn subir_variante(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
    Query(params): Query<VarianteParams>,
    bytes: Bytes,
) -> Result<(StatusCode, Json<FotoVariante>), AppError> {
    let foto = crate::services::InmuebleService::subir_foto_variante(
        &state.pool,
        &state.upload_dir,
        id,
        &params.filename,
        &crate::services::RecetaVariante {
            orden: params.orden,
            modelo: params.modelo.as_deref().unwrap_or("gemini"),
            prompt_usado: params.prompt_usado.as_deref().unwrap_or(""),
        },
        bytes.as_ref(),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(foto)))
}

/// Borrar la variante de una foto (queda solo el original)
#[utoipa::path(
    delete,
    path = "/api/admin/inmuebles/{id}/fotos/variante/{orden}",
    params(
        ("id" = Uuid, Path, description = "Inmueble"),
        ("orden" = i32, Path, description = "Posición del original")
    ),
    responses(
        (status = 204, description = "Borrada"),
        (status = 404, description = "No había variante", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn borrar_variante(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path((id, orden)): Path<(Uuid, i32)>,
) -> Result<StatusCode, AppError> {
    let clave = VersionRepository::borrar_foto(&state.pool, id, orden, 2)
        .await?
        .ok_or_else(|| AppError::NotFound("No había variante en esa posición".into()))?;
    crate::services::InmuebleService::borrar_archivo_clave(&state.upload_dir, &clave).await;
    Ok(StatusCode::NO_CONTENT)
}

/// Cuántas versiones por inmueble quiere la dueña (1..=5)
#[derive(Debug, Deserialize)]
pub struct VersionesConfig {
    pub versiones: i16,
}

/// Leer la configuración de versiones
#[utoipa::path(
    get,
    path = "/api/admin/configuracion/versiones",
    responses(
        (status = 200, description = "Número de versiones configurado"),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn leer_config(
    State(state): State<AppState>,
    _auth: AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    let n = VersionRepository::configuradas(&state.pool).await?;
    Ok(Json(serde_json::json!({ "versiones": n })))
}

/// Fijar cuántas versiones por inmueble
#[utoipa::path(
    put,
    path = "/api/admin/configuracion/versiones",
    request_body = VersionesConfig,
    responses(
        (status = 200, description = "Configuración guardada"),
        (status = 422, description = "Valor fuera de rango", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn guardar_config(
    State(state): State<AppState>,
    _auth: AuthUser,
    Json(cuerpo): Json<VersionesConfig>,
) -> Result<Json<serde_json::Value>, AppError> {
    if !(1..=5).contains(&cuerpo.versiones) {
        return Err(AppError::Validation(
            "versiones debe estar entre 1 y 5".into(),
        ));
    }
    VersionRepository::guardar_configuradas(&state.pool, cuerpo.versiones).await?;
    Ok(Json(serde_json::json!({ "versiones": cuerpo.versiones })))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/inmuebles/:id/versiones",
            get(detalle).post(generar),
        )
        .route(
            "/inmuebles/:id/versiones/:n",
            put(editar).delete(borrar),
        )
        .route("/inmuebles/:id/fotos/variante", axum::routing::post(subir_variante))
        .route(
            "/inmuebles/:id/fotos/variante/:orden",
            axum::routing::delete(borrar_variante),
        )
        .route(
            "/configuracion/versiones",
            get(leer_config).put(guardar_config),
        )
}
