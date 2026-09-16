use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use serde::Deserialize;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{FotoPublica, EXTENSIONES_FOTO};
use crate::services::InmuebleService;
use crate::AppState;

/* [159A-2] Subida de fotos como bytes crudos (sin dependencias nuevas) +
 * servicio estático público de `/uploads`. La clave interna `storage_key`
 * nunca sale del servidor: la API expone `url`. */

/// Parámetros de subida por query — el cuerpo son los bytes de la imagen
#[derive(Debug, Deserialize)]
pub struct UploadParams {
    /// Inmueble destino
    pub inmueble_id: Uuid,
    /// Nombre original (solo se usa su extensión)
    pub filename: String,
    /// `original` (defecto) o `mejorada`
    pub origen: Option<String>,
    /// Posición explícita (defecto: siguiente hueco)
    pub orden: Option<i32>,
}

/// Subir una foto a un inmueble (requiere JWT)
#[utoipa::path(
    post,
    path = "/api/admin/fotos/upload",
    params(
        ("inmueble_id" = Uuid, Query, description = "Inmueble destino"),
        ("filename" = String, Query, description = "Nombre original (vale su extensión)"),
        ("origen" = Option<String>, Query, description = "original o mejorada"),
        ("orden" = Option<i32>, Query, description = "Posición explícita"),
    ),
    request_body(content = Vec<u8>, description = "Bytes de la imagen (jpg/png/webp, máx 10 MiB)", content_type = "application/octet-stream"),
    responses(
        (status = 201, description = "Foto subida", body = FotoPublica),
        (status = 400, description = "Archivo inválido", body = crate::errors::ErrorResponse),
        (status = 404, description = "Inmueble no encontrado", body = crate::errors::ErrorResponse),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn upload_foto(
    State(state): State<AppState>,
    _auth: AuthUser,
    Query(params): Query<UploadParams>,
    bytes: Bytes,
) -> Result<(StatusCode, Json<FotoPublica>), AppError> {
    let foto = InmuebleService::subir_foto(
        &state.pool,
        &state.upload_dir,
        params.inmueble_id,
        &params.filename,
        params.origen.as_deref(),
        params.orden,
        bytes.as_ref(),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(foto)))
}

/// Servir un archivo subido — público, sin JWT (las fotos públicas cuelgan de aquí)
#[utoipa::path(
    get,
    path = "/uploads/{inmueble}/{archivo}",
    params(
        ("inmueble" = Uuid, Path, description = "ID del inmueble"),
        ("archivo" = String, Path, description = "Archivo <uuid>.<ext>"),
    ),
    responses(
        (status = 200, description = "Imagen", body = Vec<u8>, content_type = "image/jpeg"),
        (status = 404, description = "No encontrada", body = crate::errors::ErrorResponse)
    )
)]
pub async fn servir_archivo(
    State(state): State<AppState>,
    Path((inmueble, archivo)): Path<(String, String)>,
) -> Result<Response, AppError> {
    let clave = clave_valida(&format!("{inmueble}/{archivo}"))
        .ok_or_else(|| AppError::NotFound("No encontrado".into()))?;
    let bytes = tokio::fs::read(InmuebleService::ruta_archivo(&state.upload_dir, &clave))
        .await
        .map_err(|_| AppError::NotFound("No encontrado".into()))?;
    let mime = match std::path::Path::new(&clave)
        .extension()
        .and_then(|e| e.to_str())
    {
        Some(e) if e.eq_ignore_ascii_case("png") => "image/png",
        Some(e) if e.eq_ignore_ascii_case("webp") => "image/webp",
        _ => "image/jpeg",
    };
    Ok(([(header::CONTENT_TYPE, mime)], bytes).into_response())
}

/// Servir una foto de solicitud — pública, sin JWT (vista previa del modal
/// y revisión del admin cuelgan de aquí; la clave la genera el servidor)
#[utoipa::path(
    get,
    path = "/uploads/solicitudes/{sesion}/{archivo}",
    params(
        ("sesion" = Uuid, Path, description = "Carpeta de sesión de subida"),
        ("archivo" = String, Path, description = "Archivo <uuid>.<ext>"),
    ),
    responses(
        (status = 200, description = "Imagen", body = Vec<u8>, content_type = "image/jpeg"),
        (status = 404, description = "No encontrada", body = crate::errors::ErrorResponse)
    )
)]
pub async fn servir_archivo_solicitud(
    State(state): State<AppState>,
    Path((sesion, archivo)): Path<(String, String)>,
) -> Result<Response, AppError> {
    let clave = clave_solicitud_valida(&sesion, &archivo)
        .ok_or_else(|| AppError::NotFound("No encontrado".into()))?;
    let bytes = tokio::fs::read(InmuebleService::ruta_archivo(&state.upload_dir, &clave))
        .await
        .map_err(|_| AppError::NotFound("No encontrado".into()))?;
    let mime = match std::path::Path::new(&clave)
        .extension()
        .and_then(|e| e.to_str())
    {
        Some(e) if e.eq_ignore_ascii_case("png") => "image/png",
        Some(e) if e.eq_ignore_ascii_case("webp") => "image/webp",
        _ => "image/jpeg",
    };
    Ok(([(header::CONTENT_TYPE, mime)], bytes).into_response())
}

/// Solo `solicitudes/<uuid>/<uuid>.<ext permitida>`: el prefijo es fijo,
/// nada de `..`, subrutas ni extensiones raras
fn clave_solicitud_valida(sesion: &str, archivo: &str) -> Option<String> {
    if archivo.contains('/') || archivo.contains('\\') {
        return None;
    }
    Uuid::parse_str(sesion).ok()?;
    let punto = archivo.rfind('.')?;
    Uuid::parse_str(&archivo[..punto]).ok()?;
    let extension = archivo[punto..].to_lowercase();
    if EXTENSIONES_FOTO.iter().any(|e| *e == extension) {
        Some(format!("solicitudes/{sesion}/{}", archivo.to_lowercase()))
    } else {
        None
    }
}
/// Solo `<uuid>/<uuid>.<ext permitida>`: nada de `..`, subrutas ni extensiones raras
fn clave_valida(ruta: &str) -> Option<String> {
    let (inmueble, archivo) = ruta.split_once('/')?;
    if archivo.contains('/') || archivo.contains('\\') {
        return None;
    }
    Uuid::parse_str(inmueble).ok()?;
    let punto = archivo.rfind('.')?;
    Uuid::parse_str(&archivo[..punto]).ok()?;
    let extension = archivo[punto..].to_lowercase();
    if EXTENSIONES_FOTO.iter().any(|e| *e == extension) {
        Some(format!("{inmueble}/{}", archivo.to_lowercase()))
    } else {
        None
    }
}

pub fn routes() -> Router<AppState> {
    /* Sin el límite por defecto de 2 MB: el servicio impone 10 MiB con 413 */
    Router::new().route(
        "/fotos/upload",
        post(upload_foto).layer(DefaultBodyLimit::disable()),
    )
}

#[cfg(test)]
mod tests {
    use super::{clave_solicitud_valida, clave_valida};

    #[test]
    fn clave_valida_acepta_y_rechaza() {
        let id = "aa03ccec-c424-40e1-86ed-5f862d1bfa8f";
        assert_eq!(
            clave_valida(&format!("{id}/{id}.jpg")),
            Some(format!("{id}/{id}.jpg"))
        );
        assert!(clave_valida(&format!("{id}/{id}.WEBP")).is_some());
        assert!(clave_valida("../secreto.jpg").is_none());
        assert!(clave_valida(&format!("{id}/{id}.exe")).is_none());
        assert!(clave_valida(&format!("{id}/a/b.jpg")).is_none());
        assert!(clave_valida("no-es-uuid/archivo.jpg").is_none());
    }

    #[test]
    fn clave_solicitud_solo_prefijo_y_uuids() {
        let id = "aa03ccec-c424-40e1-86ed-5f862d1bfa8f";
        assert_eq!(
            clave_solicitud_valida(id, &format!("{id}.png")),
            Some(format!("solicitudes/{id}/{id}.png"))
        );
        assert!(clave_solicitud_valida(id, &format!("{id}.WEBP")).is_some());
        assert!(clave_solicitud_valida(id, "../fuga.jpg").is_none());
        assert!(clave_solicitud_valida(id, "no-uuid.jpg").is_none());
        assert!(clave_solicitud_valida(id, &format!("{id}.pdf")).is_none());
        assert!(clave_solicitud_valida("no-es-uuid", &format!("{id}.jpg")).is_none());
    }
}
