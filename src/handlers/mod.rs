#![allow(clippy::needless_for_each)] // Generado por utoipa OpenApi derive

mod auth;
mod chat;
mod chat_staff;
mod chat_tools;
mod health;
mod ia;
mod inmuebles;
mod notes;
mod public;
mod solicitud;
mod suscriptor;
mod uploads;
mod users;

use std::path::{Path, PathBuf};

use axum::http::{HeaderValue, Method, StatusCode, Uri};
use axum::response::IntoResponse;
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
        suscriptor::suscribir,
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
     * (sin `STATIC_DIR`) no se registra nada y todo sigue igual. */
    let sirve_front = state
        .static_dir
        .as_ref()
        .is_some_and(|d| d.join("index.html").is_file());
    if sirve_front {
        tracing::info!("Front SPA embebido desde {:?}", state.static_dir);
    }

    let app = Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .route("/uploads/:inmueble/:archivo", get(uploads::servir_archivo))
        .route(
            "/uploads/solicitudes/:sesion/:archivo",
            get(uploads::servir_archivo_solicitud),
        )
        /* [249A-1] Sitemap dinámico (ruta explícita: no cae al fallback). */
        .route("/sitemap.xml", get(sitemap))
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
        app.fallback(fallback_spa)
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
        .layer(cors);
    app.with_state(state)
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
        /* [199A-1] Centro de IA de texto: estado/config/probar/completar
         * (rutas bajo /api/admin/ia). */
        .merge(ia::routes())
}

/* [239A-1] SPA del monorepo: sirve el fichero tal cual si existe y cae a
 * `index.html` en cualquier otra ruta (el front resuelve sus rutas). Las
 * rutas de API nunca caen aquí: devuelven 404 seco para no enmascarar
 * errores del backend con HTML. */
async fn fallback_spa(
    axum::extract::State(est): axum::extract::State<AppState>,
    uri: Uri,
) -> impl axum::response::IntoResponse {
    use axum::response::IntoResponse as _;
    const RUTAS_API: [&str; 4] = ["/api", "/uploads", "/swagger-ui", "/api-docs"];
    let ruta = uri.path();
    if RUTAS_API
        .iter()
        .any(|p| ruta == *p || ruta.starts_with(&format!("{p}/")))
    {
        return StatusCode::NOT_FOUND.into_response();
    }
    let base = est.static_dir.unwrap_or_default();
    /* `is_file` y no `exists`: `/` resuelve al propio directorio base y debe
     * caer a `index.html` en vez de intentar leer el directorio. */
    let candidato = Path::new(&base).join(ruta.trim_start_matches('/'));
    let archivo = if es_fichero(&candidato).await {
        candidato
    } else {
        Path::new(&base).join("index.html")
    };
    match tokio::fs::read(&archivo).await {
        Ok(bytes) => {
            let ext = archivo
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or_default();
            /* [249A-1] Caché del SPA: los ficheros con hash de Vite
             * (`/assets/`) son inmutables un año; el HTML se revalida
             * siempre para que cada deploy tome efecto sin purgas. */
            let inmutable = ruta.starts_with("/assets/")
                || matches!(
                    ext,
                    "js" | "css" | "woff2" | "png" | "jpg" | "jpeg" | "webp" | "svg" | "ico"
                );
            let control = if inmutable {
                "public, max-age=31536000, immutable"
            } else {
                "no-cache"
            };
            (
                [
                    (axum::http::header::CONTENT_TYPE, tipo_contenido(ext)),
                    (axum::http::header::CACHE_CONTROL, control),
                ],
                bytes,
            )
                .into_response()
        }
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

/* [249A-1] Sitemap mínimo y honesto: el detalle es solo-modal (sin URLs con
 * slug indexables), así que declara `/` con su última modificación. Las URLs
 * `/inmueble/:slug` quedan para el bloque de ruteo futuro. */
async fn sitemap(
    axum::extract::State(est): axum::extract::State<AppState>,
) -> impl axum::response::IntoResponse {
    use crate::repositories::InmuebleRepository;
    let ultima = InmuebleRepository::ultima_modificacion_publica(&est.pool)
        .await
        .ok()
        .flatten()
        .map_or_else(
            || "2026-09-24".to_string(),
            |f| f.format("%Y-%m-%d").to_string(),
        );
    let xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n\
         <url><loc>https://mn-inmobiliaria.com/</loc><lastmod>{ultima}</lastmod></url>\n\
         </urlset>\n"
    );
    (
        [
            (axum::http::header::CONTENT_TYPE, "application/xml"),
            (axum::http::header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        xml,
    )
        .into_response()
}

/// `true` si la ruta es un fichero legible (no directorio).
async fn es_fichero(p: &Path) -> bool {
    tokio::fs::metadata(p).await.is_ok_and(|m| m.is_file())
}

/// `Content-Type` mínimo para los ficheros del SPA (el `index.html` del build
/// solo referencia `.js`, `.css`, imágenes y fuentes).
fn tipo_contenido(ext: &str) -> &'static str {
    match ext {
        "html" => "text/html; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" | "map" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "txt" => "text/plain; charset=utf-8",
        "xml" => "application/xml",
        "woff2" => "font/woff2",
        _ => "application/octet-stream",
    }
}
