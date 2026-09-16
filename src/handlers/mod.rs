#![allow(clippy::needless_for_each)] // Generado por utoipa OpenApi derive

mod auth;
mod chat;
mod chat_staff;
mod chat_tools;
mod health;
mod inmuebles;
mod notes;
mod public;
mod solicitud;
mod uploads;
mod users;

use axum::routing::get;
use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::AppState;

/// Define el esquema de seguridad Bearer para Swagger UI
struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        /* components existe porque el derive ya registra schemas */
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                utoipa::openapi::security::SecurityScheme::Http(
                    utoipa::openapi::security::Http::new(
                        utoipa::openapi::security::HttpAuthScheme::Bearer,
                    ),
                ),
            );
        }
    }
}

#[derive(OpenApi)]
#[openapi(
    paths(
        health::health_check,
        auth::register,
        auth::login,
        users::create_user,
        notes::create_note,
        notes::get_note,
        notes::list_notes,
        notes::update_note,
        notes::delete_note,
        inmuebles::create_inmueble,
        inmuebles::list_inmuebles,
        inmuebles::get_inmueble,
        inmuebles::update_inmueble,
        inmuebles::set_publicacion,
        inmuebles::delete_inmueble,
        inmuebles::add_foto,
        inmuebles::delete_foto,
        uploads::upload_foto,
        uploads::servir_archivo,
        uploads::servir_archivo_solicitud,
        solicitud::subir_foto_solicitud,
        solicitud::create_solicitud,
        solicitud::list_solicitudes,
        solicitud::revisar_solicitud,
        public::list_public,
        public::get_public,
    ),
    components(schemas(
        health::HealthResponse,
        crate::models::RegisterRequest,
        crate::models::LoginRequest,
        crate::models::AuthResponse,
        crate::models::UserResponse,
        crate::models::CreateUserRequest,
        crate::models::Note,
        crate::models::CreateNoteRequest,
        crate::models::UpdateNoteRequest,
        crate::models::PaginatedNotes,
        crate::models::Inmueble,
        crate::models::Foto,
        crate::models::FotoPublica,
        crate::models::CopyInmueble,
        crate::models::CreateInmuebleRequest,
        crate::models::UpdateInmuebleRequest,
        crate::models::PublicacionRequest,
        crate::models::AddFotoRequest,
        crate::models::PaginatedInmuebles,
        crate::models::Solicitud,
        crate::models::FotoSolicitud,
        crate::models::FotoSolicitudSubida,
        crate::models::CreateSolicitudRequest,
        crate::models::UpdateEstadoSolicitud,
        crate::models::PaginatedSolicitudes,
        crate::errors::ErrorResponse,
    )),
    modifiers(&SecurityAddon),
    info(
        title = "Glory RS API",
        version = "0.1.0",
        description = "Template API — Rust + Axum + OpenAPI"
    )
)]
#[allow(clippy::needless_for_each)]
pub struct ApiDoc;

/// Crea el router principal con CORS, tracing, Swagger UI y todas las rutas
pub fn create_router(pool: sqlx::PgPool, config: crate::config::AppConfig) -> Router {
    /* [169A-1] El chat trae estado propio (AgentState): se anida ya con
     * estado en /api para exponer /api/agent/{ws,messages,history}.
     * [169A-4] Comparte un `ChatHub` con las rutas staff: el humano
     * responde por el mismo WS que escucha el visitante. */
    let hub = glory_agent::session::ChatHub::new();
    let agent = chat::agent_router(pool.clone(), hub.clone());
    let state = AppState {
        pool,
        jwt_secret: config.jwt_secret,
        upload_dir: config.upload_dir.into(),
        hub,
    };

    /* CORS: en desarrollo se permite todo. En producción, restringir orígenes */
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .route("/uploads/:inmueble/:archivo", get(uploads::servir_archivo))
        .route(
            "/uploads/solicitudes/:sesion/:archivo",
            get(uploads::servir_archivo_solicitud),
        )
        .nest("/api", api_routes())
        /* nest_service porque el chat trae Router<()> (estado propio):
         * nest exige el mismo estado. Despoja /api igual que nest. */
        .nest_service("/api", agent)
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}

fn api_routes() -> Router<AppState> {
    Router::new()
        .merge(health::routes())
        .merge(auth::routes())
        .merge(notes::routes())
        .nest("/admin", admin_routes())
        .nest("/public", public::routes())
}

/// Rutas de administración: todo requiere JWT (`AuthUser` por handler)
fn admin_routes() -> Router<AppState> {
    Router::new()
        .merge(inmuebles::routes())
        .merge(solicitud::admin_routes())
        .merge(uploads::routes())
        .merge(users::routes())
        /* [169A-4] Atención del chat: bandeja, hilo, responder, tomar/soltar
         * IA y config (rutas bajo /api/admin/agent). */
        .merge(chat_staff::staff_routes())
}
