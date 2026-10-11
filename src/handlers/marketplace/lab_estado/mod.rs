//! [10AA-23] Estado de la conexión del lab (opencode-propio-dev) con este backend.
//!
//! El lab pregunta `borradores/version` cada 5 s con su token CLI (`MpAuth`),
//! así que el último éxito o rechazo de `MpAuth` es un latido real. Vive en
//! memoria (como `version_borradores_borrados`): sin BD ni `AppState`; tras un
//! reinicio queda «sin señal» hasta el siguiente barrido del lab (≤ 5 s).

use std::sync::atomic::{AtomicI64, Ordering};

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::Utc;

use crate::errors::AppError;
use crate::middleware::AuthUser;

/// Segundos desde el último token CLI aceptado.
static ULTIMO_OK: AtomicI64 = AtomicI64::new(0);
/// Segundos desde el último token rechazado (con cabecera Bearer).
static ULTIMO_FALLO: AtomicI64 = AtomicI64::new(0);

/// Seis barridos de 5 s sin señal y el lab deja de contar como conectado.
const VENTANA_CONECTADO_S: i64 = 30;
/// Un rechazo sigue siendo noticia este tiempo si el lab no se recupera.
const VENTANA_RECHAZO_S: i64 = 60;

pub(super) fn registrar_ok() {
    ULTIMO_OK.store(Utc::now().timestamp(), Ordering::Relaxed);
}

pub(super) fn registrar_fallo() {
    ULTIMO_FALLO.store(Utc::now().timestamp(), Ordering::Relaxed);
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum EstadoLab {
    Conectado,
    SinSenal,
    Rechazado,
}

impl EstadoLab {
    fn como_str(&self) -> &'static str {
        match self {
            Self::Conectado => "conectado",
            Self::SinSenal => "sin_senal",
            Self::Rechazado => "rechazado",
        }
    }
}

/// `ok`/`fallo` en segundos Unix (0 = nunca). Un rechazo más nuevo que el
/// último éxito manda; en el mismo segundo gana el éxito.
pub(super) fn clasificar(ahora: i64, ok: i64, fallo: i64) -> EstadoLab {
    let fallo_vigente = fallo > 0 && ahora - fallo <= VENTANA_RECHAZO_S;
    if fallo_vigente && fallo > ok {
        return EstadoLab::Rechazado;
    }
    if ok > 0 && ahora - ok <= VENTANA_CONECTADO_S {
        return EstadoLab::Conectado;
    }
    EstadoLab::SinSenal
}

/// Estado del lab para el indicador de la cabecera del panel (solo admin).
#[utoipa::path(
    get,
    path = "/api/admin/marketplace/lab/estado",
    responses(
        (status = 200, description = "Estado de la conexión del lab: conectado, sin_senal o rechazado"),
        (status = 401, description = "Sin sesión admin", body = crate::errors::ErrorResponse)
    )
)]
pub async fn estado_lab(_auth: AuthUser) -> Result<Response, AppError> {
    let ahora = Utc::now().timestamp();
    let ok = ULTIMO_OK.load(Ordering::Relaxed);
    let fallo = ULTIMO_FALLO.load(Ordering::Relaxed);
    let hace = |t: i64| (t > 0).then_some(ahora - t);
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "estado": clasificar(ahora, ok, fallo).como_str(),
            "ultimo_ok_hace_s": hace(ok),
            "ultimo_fallo_hace_s": hace(fallo),
        })),
    )
        .into_response())
}

#[cfg(test)]
mod pruebas;
