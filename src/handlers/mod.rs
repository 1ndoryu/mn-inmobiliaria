#![allow(clippy::needless_for_each)] // Generado por utoipa OpenApi derive

pub(crate) mod chat;
mod comercial;
mod cuentas;
mod health;
pub(crate) mod ia;
mod inmobiliario;
pub mod marketplace;
mod public;
mod whatsapp;

use std::path::PathBuf;

use axum::http::{HeaderValue, Method};
use axum::routing::get;
use axum::Router;
use tower_http::compression::predicate::{NotForContentType, Predicate, SizeAbove};
use tower_http::compression::CompressionLayer;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
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
        cuentas::auth::register,
        cuentas::auth::login,
        ia::ask::get_ficha,
        ia::ask::set_ficha,
        cuentas::users::create_user,
        comercial::notes::create_note,
        comercial::notes::get_note,
        comercial::notes::list_notes,
        comercial::notes::update_note,
        comercial::notes::delete_note,
        inmobiliario::inmuebles::create_inmueble,
        inmobiliario::inmuebles::list_inmuebles,
        inmobiliario::inmuebles::get_inmueble,
        inmobiliario::inmuebles::update_inmueble,
        inmobiliario::inmuebles::set_publicacion,
        inmobiliario::inmuebles::set_estado,
        inmobiliario::inmuebles::delete_inmueble,
        inmobiliario::inmuebles::add_foto,
        inmobiliario::inmuebles::delete_foto,
        inmobiliario::uploads::upload_foto,
        inmobiliario::uploads::servir_archivo,
        inmobiliario::uploads::servir_archivo_solicitud,
        inmobiliario::uploads::servir_archivo_whatsapp,
        comercial::solicitud::subir_foto_solicitud,
        comercial::solicitud::create_solicitud,
        comercial::solicitud::list_solicitudes,
        comercial::solicitud::revisar_solicitud,
        comercial::visita::list_visitas,
        comercial::visita::revisar_visita,
        marketplace::token::emitir_token,
        marketplace::token::emitir_token_cli,
        marketplace::borrador,
        marketplace::regenerar,
        marketplace::logs::logs,
        marketplace::corregir,
        marketplace::audit,
        marketplace::uso,
        marketplace::chats_admin::chats,
        marketplace::chats_admin::chat_detalle,
        marketplace::chats_admin::archivar_chat,
        marketplace::chats_admin::borrar_chat,
        marketplace::chats_admin::borrar_borrador_chat,
        marketplace::chats_admin::version_borradores,
        public::list_public,
        public::get_public,
        comercial::suscriptor::suscribir,
    ),
    components(schemas(
        health::HealthResponse,
        crate::models::RegisterRequest,
        crate::models::LoginRequest,
        crate::models::AuthResponse,
        crate::models::UserResponse,
        crate::models::CreateUserRequest,
        crate::models::FichaAskRequest,
        crate::models::FichaAskResponse,
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
        crate::models::EstadoRequest,
        crate::models::AddFotoRequest,
        crate::models::PaginatedInmuebles,
        crate::models::Solicitud,
        crate::models::FotoSolicitud,
        crate::models::FotoSolicitudSubida,
        crate::models::CreateSolicitudRequest,
        crate::models::UpdateEstadoSolicitud,
        crate::models::PaginatedSolicitudes,
        crate::models::Visita,
        crate::models::PaginatedVisitas,
        crate::models::UpdateEstadoVisita,
        crate::services::marketplace::PromptSeguro,
        crate::services::marketplace::BorradorRequest,
        crate::services::marketplace::ExcerptIn,
        crate::services::marketplace::ExtrasIn,
        crate::services::marketplace::Tono,
        crate::services::marketplace::Largo,
        crate::handlers::marketplace::token::TokenResponse,
        crate::handlers::marketplace::BorradorResponse,
        crate::handlers::marketplace::logs::LogEvento,
        crate::handlers::marketplace::logs::LogNivel,
        crate::handlers::marketplace::logs::LogsResponse,
        crate::handlers::marketplace::AuditIn,
        crate::handlers::marketplace::EventoAudit,
        crate::handlers::marketplace::token::CliTokenRequest,
        crate::handlers::marketplace::CorregirRequest,
        crate::handlers::marketplace::CorregirResponse,
        crate::handlers::marketplace::UsoQuery,
        crate::services::marketplace::UsoDia,
        crate::services::marketplace::ChatResumen,
        crate::services::marketplace::PaginaResumen,
        crate::services::marketplace::ChatFila,
        crate::models::CreateSuscriptorRequest,
        crate::models::Suscriptor,
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
        static_dir: config.static_dir.map(PathBuf::from),
        /* [03AA-3 M4] Singleflight por proceso del asistente Marketplace. */
        mp_vuelo: std::sync::Arc::new(crate::services::marketplace::Singleflight::default()),
    };

    /* [239A-1] CORS: abierto solo si no hay `CORS_ORIGINS` (dev). En prod se
     * fija `CORS_ORIGINS=https://mn-inmobiliaria.com` y el resto se rechaza. */
    let permitidos: Vec<HeaderValue> = config
        .cors_origins
        .iter()
        .filter_map(|o| o.parse().ok())
        .collect();
    let cors = if permitidos.is_empty() {
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any)
    } else {
        CorsLayer::new()
            .allow_origin(AllowOrigin::list(permitidos))
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::PATCH,
                Method::DELETE,
                Method::OPTIONS,
            ])
            .allow_headers(Any)
    };

    /* [239A-1] Monorepo: si `STATIC_DIR` trae `index.html`, el front SPA se
     * sirve desde el propio binario (fichero tal cual o `index.html`). En dev
     * (sin `STATIC_DIR`) no se registra nada y nada cambia. */
    let sirve_front = state
        .static_dir
        .as_ref()
        .is_some_and(|d| d.join("index.html").is_file());
    if sirve_front {
        tracing::info!("Front SPA embebido desde {:?}", state.static_dir);
    }

    let app = Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .route(
            "/uploads/:inmueble/:archivo",
            get(inmobiliario::uploads::servir_archivo),
        )
        .route(
            "/uploads/solicitudes/:sesion/:archivo",
            get(inmobiliario::uploads::servir_archivo_solicitud),
        )
        /* [279A-2] Fotos entrantes de WhatsApp archivadas en disco local. */
        .route(
            "/uploads/whatsapp/:telefono/:archivo",
            get(inmobiliario::uploads::servir_archivo_whatsapp),
        )
        /* [249A-1] Sitemap dinámico (ruta explícita: no cae al fallback). */
        .route("/sitemap.xml", get(inmobiliario::superficie::sitemap))
        /* [249A-4] `llms.txt` y catálogo agente dinámicos desde la BD
         * (siempre frescos, sin depender del prebuild): el estático
         * `public/llms.txt` nunca llegaba al build de Coolify y el
         * fallback servía `index.html` en su lugar (agéntica 1/4). */
        .route("/llms.txt", get(inmobiliario::superficie::llms_txt))
        .route(
            "/.well-known/ai-catalog.json",
            get(inmobiliario::superficie::ai_catalog),
        )
        .nest("/api", api_routes())
        /* nest_service porque el chat trae Router<()> (estado propio):
         * nest exige el mismo estado. Despoja /api igual que nest. */
        .nest_service("/api", agent);
    /* [249A-2] El fallback va ANTES de los layers: en axum un layer solo
     * envuelve lo ya registrado; con el fallback despues de los layers el
     * SPA (JS/CSS/HTML) salia sin gzip aunque la API si comprimia
     * (verificado en prod: `Content-Encoding` ausente en el JS/CSS/HTML).
     * `with_state` sigue ultimo y da estado a rutas y fallback por igual. */
    let app = if sirve_front {
        app.fallback(inmobiliario::superficie::fallback_spa)
    } else {
        app
    };
    let app = app
        .layer(TraceLayer::new_for_http())
        /* [249A-1] Compresión gzip de respuestas (el JS de 612 KB viaja
         * comprimido; PageSpeed lo exigía: no había Content-Encoding).
         * [249A-2] Salvo imágenes (JPEG/PNG/WebP ya van comprimidos;
         * comprimirlos quema CPU sin ahorrar bytes) y cuerpos < 1 KB. */
        .layer(
            CompressionLayer::new()
                .compress_when(SizeAbove::new(1024).and(NotForContentType::IMAGES)),
        )
        .layer(cors)
        /* [08AA-3 B4] Rate-limit global por IP (último layer = el primero
         * que corre): cubre las 20 rutas POST sin parchear cada handler.
         * Escrituras 120/min, lecturas 1200/min; el exceso es 429 con
         * `Retry-After`. Ver `rate_limit.rs`. */
        .layer(axum::middleware::from_fn_with_state(
            cuentas::rate_limit::LimitadorTasa::default(),
            cuentas::rate_limit::capa_limite,
        ));
    app.with_state(state)
}

fn api_routes() -> Router<AppState> {
    Router::new()
        .merge(health::routes())
        .merge(cuentas::auth::routes())
        .merge(comercial::notes::routes())
        .nest("/admin", admin_routes())
        .nest("/public", public::routes())
}

/// Rutas de administración: cada ruta requiere JWT (`AuthUser` por handler)
fn admin_routes() -> Router<AppState> {
    Router::new()
        .merge(inmobiliario::inmuebles::routes())
        /* [279A-3] Ficha /ask de la dueña (rutas bajo /api/admin/...). */
        .merge(ia::ask::routes())
        .merge(comercial::solicitud::admin_routes())
        .merge(comercial::visita::admin_routes())
        .merge(inmobiliario::uploads::routes())
        .merge(cuentas::users::routes())
        /* [03AA-3 M3] Asistente Marketplace: token mp, borrador y audit. */
        .merge(marketplace::routes())
        /* [169A-4] Atención del chat: bandeja, hilo, responder, tomar/soltar
         * IA y config (rutas bajo /api/admin/agent). */
        .merge(chat::staff::staff_routes())
        /* [199A-1] Centro de IA de texto: estado/config/probar/completar
         * (rutas bajo /api/admin/ia). */
        .merge(ia::rutas::routes())
}

/* [08AA-7] La superficie web (fallback SPA, sitemap, llms.txt, catálogo)
 * vive en `superficie.rs`. */
