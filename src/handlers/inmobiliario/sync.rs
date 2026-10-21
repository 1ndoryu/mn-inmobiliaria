use std::collections::BTreeSet;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::services::sync::aplicar::{self, InformeAplicar};
use crate::services::sync::detalle::{self, DetalleSync, DiferenciasSync};
use crate::services::sync::huella::Manifiesto;
use crate::services::sync::manifiesto;
use crate::services::sync::operacion::InformeSync;
use crate::services::sync::plan;
use crate::services::sync::remoto::{self, Credenciales, ErrorRemoto};
use crate::services::sync::resolver::PlanResolucion;
use crate::services::sync::vigilancia::{self, EstadoSync};
use crate::services::sync::{publicar, traer};
use crate::AppState;

/* [13AA-12] Sincronización de inmuebles local ↔ servidor. Las rutas de
 * manifiesto y detalle las consume el otro entorno; estado, comprobar y
 * diferencias las usa el panel. La comparación vive en `services::sync`. */

/// Huellas de todos los inmuebles y sus fotos (sin valores privados)
#[utoipa::path(
    get,
    path = "/api/admin/sync/manifiesto",
    responses(
        (status = 200, description = "Huellas por inmueble", body = Manifiesto),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_manifiesto(
    State(state): State<AppState>,
    _auth: AuthUser,
) -> Result<Json<Manifiesto>, AppError> {
    Ok(Json(
        manifiesto::construir(&state.pool, &state.upload_dir).await?,
    ))
}

#[derive(Debug, Deserialize)]
pub struct EstadoQuery {
    /// `revision` que el cliente ya conoce: si coincide, espera el siguiente cambio.
    pub desde: Option<u64>,
}

/// Estado de sincronización. Con `desde` es long-poll: responde al cambiar la
/// `revision` o, tras unos 25 s, con el estado vigente.
#[utoipa::path(
    get,
    path = "/api/admin/sync/estado",
    params(
        ("desde" = Option<u64>, Query, description = "Revisión ya conocida por el cliente")
    ),
    responses(
        (status = 200, description = "Estado vigente", body = EstadoSync),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_estado(_auth: AuthUser, Query(q): Query<EstadoQuery>) -> Json<EstadoSync> {
    let estado = match q.desde {
        Some(desde) => vigilancia::esperar_cambio(desde).await,
        None => vigilancia::actual(),
    };
    Json((*estado).clone())
}

/// Pide consultar al servidor ya, sin esperar al intervalo. Seguir con `GET /sync/estado`.
#[utoipa::path(
    post,
    path = "/api/admin/sync/comprobar",
    responses(
        (status = 202, description = "Consulta encolada"),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn post_comprobar(_auth: AuthUser) -> StatusCode {
    vigilancia::comprobar_ya();
    StatusCode::ACCEPTED
}

/// Huella y valores canónicos de un inmueble de este entorno (incluye no publicados)
#[utoipa::path(
    get,
    path = "/api/admin/sync/inmuebles/{slug}",
    params(("slug" = String, Path, description = "Slug del inmueble")),
    responses(
        (status = 200, description = "Detalle del inmueble", body = DetalleSync),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse),
        (status = 404, description = "No existe", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_detalle(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(slug): Path<String>,
) -> Result<Json<DetalleSync>, AppError> {
    detalle::detalle_local(&state.pool, &slug)
        .await?
        .map(Json)
        .ok_or_else(|| AppError::NotFound(format!("inmueble {slug}")))
}

/// Diferencias de un inmueble con el servidor, con el valor de cada lado
#[utoipa::path(
    get,
    path = "/api/admin/sync/diferencias/{slug}",
    params(("slug" = String, Path, description = "Slug del inmueble")),
    responses(
        (status = 200, description = "Diferencias y quién va por delante", body = DiferenciasSync),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse),
        (status = 404, description = "No existe en este entorno", body = crate::errors::ErrorResponse),
        (status = 409, description = "Servidor no disponible o sin soporte", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_diferencias(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(slug): Path<String>,
) -> Result<Json<DiferenciasSync>, AppError> {
    let local = detalle::detalle_local(&state.pool, &slug)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("inmueble {slug}")))?;
    let cred = Credenciales::cargar().map_err(|e| error_remoto(&e))?;
    let remoto = remoto::detalle(&cred, &slug)
        .await
        .map_err(|e| error_remoto(&e))?;
    let diferencias = detalle::diferencias(&state.pool, &local, remoto.as_ref()).await?;
    Ok(Json(diferencias))
}

/// Elecciones del panel: claves cuyo valor «viejo» se conserva. Lo demás gana «lo último».
#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct PeticionPlan {
    #[serde(default)]
    pub conservar_viejo: Vec<String>,
}

/// Plan de resolución de las diferencias de un inmueble. Solo calcula: no escribe en ningún lado.
#[utoipa::path(
    post,
    path = "/api/admin/sync/resolver/{slug}/plan",
    params(("slug" = String, Path, description = "Slug del inmueble")),
    request_body = PeticionPlan,
    responses(
        (status = 200, description = "Plan por campo, con el lado que queda", body = PlanResolucion),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse),
        (status = 404, description = "No existe en este entorno", body = crate::errors::ErrorResponse),
        (status = 409, description = "Servidor no disponible, sin soporte o sin este inmueble", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn post_plan(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(slug): Path<String>,
    Json(peticion): Json<PeticionPlan>,
) -> Result<Json<PlanResolucion>, AppError> {
    let cred = Credenciales::cargar().map_err(|e| error_remoto(&e))?;
    let conservar: BTreeSet<String> = peticion.conservar_viejo.into_iter().collect();
    Ok(Json(
        plan::plan_de(&state.pool, &cred, &slug, &conservar).await?,
    ))
}

/// Elecciones del panel más el plan que se mostró: aplicar exige que siga vigente (D3).
#[derive(Debug, Deserialize, ToSchema)]
pub struct PeticionAplicar {
    /// `plan_id` del plan que el panel mostró.
    pub plan_id: String,
    #[serde(default)]
    pub conservar_viejo: Vec<String>,
}

/// Aplica el plan de resolución: escribe en los dos lados. Un fallo antes de
/// escribir es 409; a mitad llega con 200 y `fallo` en el informe, con lo escrito.
#[utoipa::path(
    post,
    path = "/api/admin/sync/resolver/{slug}/aplicar",
    params(("slug" = String, Path, description = "Slug del inmueble")),
    request_body = PeticionAplicar,
    responses(
        (status = 200, description = "Informe de lo aplicado, lo pendiente y el residuo", body = InformeAplicar),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse),
        (status = 404, description = "No existe en este entorno", body = crate::errors::ErrorResponse),
        (status = 409, description = "Sin escribir: el plan cambió, otra operación en curso, servidor no disponible o sin este inmueble", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn post_aplicar(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(slug): Path<String>,
    Json(peticion): Json<PeticionAplicar>,
) -> Result<Json<InformeAplicar>, AppError> {
    let cred = Credenciales::cargar().map_err(|e| error_remoto(&e))?;
    let conservar: BTreeSet<String> = peticion.conservar_viejo.into_iter().collect();
    let informe = aplicar::aplicar(
        &state.pool,
        &state.upload_dir,
        &cred,
        &slug,
        &peticion.plan_id,
        &conservar,
    )
    .await?;
    vigilancia::comprobar_ya();
    Ok(Json(informe))
}

/// Publica el inmueble local en el servidor. Un fallo antes de escribir es 409; un fallo
/// a mitad llega con 200 y `fallo` en el informe, con lo que sí quedó escrito.
#[utoipa::path(
    post,
    path = "/api/admin/sync/inmuebles/{slug}/publicar",
    params(("slug" = String, Path, description = "Slug del inmueble local")),
    responses(
        (status = 200, description = "Informe de lo escrito en el servidor", body = InformeSync),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse),
        (status = 404, description = "No existe en este entorno", body = crate::errors::ErrorResponse),
        (status = 409, description = "Sin escribir: servidor no disponible, slug ocupado, datos no válidos u otra operación en curso", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn post_publicar(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(slug): Path<String>,
) -> Result<Json<InformeSync>, AppError> {
    let cred = Credenciales::cargar().map_err(|e| error_remoto(&e))?;
    let informe = publicar::publicar(&state.pool, &state.upload_dir, &cred, &slug).await?;
    vigilancia::comprobar_ya();
    Ok(Json(informe))
}

/// Trae el inmueble del servidor al local. Mismo contrato que publicar.
#[utoipa::path(
    post,
    path = "/api/admin/sync/inmuebles/{slug}/traer",
    params(("slug" = String, Path, description = "Slug del inmueble en el servidor")),
    responses(
        (status = 200, description = "Informe de lo escrito en el local", body = InformeSync),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse),
        (status = 404, description = "No existe en el servidor", body = crate::errors::ErrorResponse),
        (status = 409, description = "Sin escribir: ya existe en el local, servidor no disponible, datos no válidos u otra operación en curso", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn post_traer(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(slug): Path<String>,
) -> Result<Json<InformeSync>, AppError> {
    let cred = Credenciales::cargar().map_err(|e| error_remoto(&e))?;
    let informe = traer::traer(&state.pool, &state.upload_dir, &cred, &slug).await?;
    vigilancia::comprobar_ya();
    Ok(Json(informe))
}

/// Servidor con el que se compara. Publicar y traer lo nombran antes de escribir (D16).
#[derive(Debug, Serialize, ToSchema)]
pub struct DestinoSync {
    pub host: String,
}

#[utoipa::path(
    get,
    path = "/api/admin/sync/destino",
    responses(
        (status = 200, description = "Servidor configurado", body = DestinoSync),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse),
        (status = 409, description = "Sin credenciales del servidor", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_destino(_auth: AuthUser) -> Result<Json<DestinoSync>, AppError> {
    let cred = Credenciales::cargar().map_err(|e| error_remoto(&e))?;
    Ok(Json(DestinoSync { host: cred.host() }))
}

/// Un fallo del servidor no es un error del panel: se muestra como conflicto con el motivo.
fn error_remoto(e: &ErrorRemoto) -> AppError {
    AppError::Conflict(e.to_string())
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/sync/manifiesto", get(get_manifiesto))
        .route("/sync/estado", get(get_estado))
        .route("/sync/comprobar", post(post_comprobar))
        .route("/sync/inmuebles/:slug", get(get_detalle))
        .route("/sync/diferencias/:slug", get(get_diferencias))
        .route("/sync/resolver/:slug/plan", post(post_plan))
        .route("/sync/resolver/:slug/aplicar", post(post_aplicar))
        .route("/sync/inmuebles/:slug/publicar", post(post_publicar))
        .route("/sync/inmuebles/:slug/traer", post(post_traer))
        .route("/sync/destino", get(get_destino))
}
