/* [169A-1] Chat del visitante con IA (F5): cablea el crate agnostico
 * `glory-agent` (regla 17: lo reutilizable vive en el nucleo) con la
 * identidad del producto.
 * [169A-4] F7: registra las 5 tools (`chat_tools`), comparte el `ChatHub`
 * con las rutas staff (el humano responde por el mismo WS) y expone
 * `/api/agent/info` + `/api/agent/sesiones/:id/contacto` sin auth.
 * Gotcha: `transport::routes()` usa `AgentState` propio, no `AppState`;
 * por eso se anida con `with_state` (Router<()>) en vez de `merge`.
 * Sin `OPENCODE_GO_API_KEY` el chat persiste + reenvia en realtime pero
 * la IA no responde (degradado verificado en glory-agent F2). */

pub mod staff;
pub mod staff_config;
pub mod staff_envio;
pub mod tools;
pub mod tools_captacion;
pub mod tools_definiciones;

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::whatsapp;
use crate::repositories::ClienteRepository;
use glory_agent::errors::AgentError;
use glory_agent::session::ChatHub;

use chrono::Timelike as _;

fn contacto_defecto() -> String {
    std::env::var("AGENTE_CONTACTO")
        .unwrap_or_else(|_| "el WhatsApp de la inmobiliaria".to_string())
}

/* [E17] Reloj de Venezuela (UTC-4 fijo, sin horario de verano): el modelo
 * no recibe reloj y el saludo salía no determinista (Fase 1: "buenos días"
 * y "buenas tardes" a las 23:35). Se inyecta la hora real en el prompt y
 * el saludo queda atado a ella. Pura para testear los bordes. */
pub(crate) fn periodo_venezuela(hora: u32) -> (&'static str, &'static str) {
    match hora {
        5..=11 => ("de mañana", "buenos días"),
        12..=18 => ("de tarde", "buenas tardes"),
        _ => ("de noche", "buenas noches"),
    }
}

fn linea_hora_venezuela() -> String {
    use chrono::Datelike as _;
    let ahora = chrono::Utc::now() - chrono::Duration::hours(4);
    let dias = [
        "lunes",
        "martes",
        "miércoles",
        "jueves",
        "viernes",
        "sábado",
        "domingo",
    ];
    let (periodo, saludo) = periodo_venezuela(ahora.hour());
    format!(
        "Hora actual en Venezuela: {} {:02}/{:02}/{} {:02}:{:02} (es {}: saluda \"{}\").",
        dias[ahora.weekday().num_days_from_monday() as usize],
        ahora.day(),
        ahora.month(),
        ahora.year(),
        ahora.hour(),
        ahora.minute(),
        periodo,
        saludo
    )
}


/// Router del chat con estado propio (`AgentState`), listo para anidar.
/// Comparte `hub` con las rutas staff para el realtime humano→visitante.
pub fn agent_router(pool: sqlx::PgPool, hub: ChatHub) -> Router<()> {
    let provider = glory_agent::providers::ProviderConfig::opencode_go(
        std::env::var("OPENCODE_GO_API_KEY").unwrap_or_default(),
    );
    let mut registro = glory_agent::tools::ToolRegistry::new();
    for def in tools::definiciones() {
        registro.register(def);
    }
    let executor: Arc<dyn glory_agent::tools::ToolExecutor> =
        Arc::new(tools::Herramientas::new(pool.clone(), contacto_defecto()).with_hub(hub.clone()));
    let prompt = crate::config::ia::prompt_chat(&contacto_defecto(), &linea_hora_venezuela());
    let mut state = glory_agent::transport::AgentState::new(provider, prompt)
        .with_pool(pool)
        .with_hub(hub);
    state.tools = Arc::new(registro);
    state.executor = Some(executor);
    glory_agent::transport::routes()
        .route("/agent/info", get(info))
        .route("/agent/sesiones/:id/contacto", post(guardar_contacto))
        .merge(whatsapp::whatsapp_routes())
        .with_state(state)
}

#[derive(Debug, Serialize)]
struct InfoPublica {
    #[serde(rename = "contactoTelefono")]
    contacto_telefono: String,
    #[serde(rename = "whatsappUrl")]
    whatsapp_url: String,
    #[serde(rename = "iaHabilitada")]
    ia_habilitada: bool,
}

/// Datos públicos del chat para el widget (teléfono, `WhatsApp`, IA on/off).
async fn info(
    State(state): State<glory_agent::transport::AgentState>,
) -> Result<Json<InfoPublica>, AgentError> {
    let pool = state
        .pool
        .clone()
        .ok_or_else(|| AgentError::Internal("sin BD".to_string()))?;
    let contacto = tools::contacto_publico(&pool, &contacto_defecto()).await?;
    let global = glory_agent::persistence::get_config(&pool, "ai_enabled_global").await?;
    Ok(Json(InfoPublica {
        contacto_telefono: contacto
            .get("telefono")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        whatsapp_url: contacto
            .get("whatsapp_url")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        ia_habilitada: global.as_deref() != Some("off"),
    }))
}

#[derive(Debug, Deserialize)]
struct ContactoVisitante {
    nombre: String,
    telefono: String,
}

/// El visitante deja nombre+teléfono (widget o tool). Validado en boundary.
async fn guardar_contacto(
    State(state): State<glory_agent::transport::AgentState>,
    Path(id): Path<Uuid>,
    Json(input): Json<ContactoVisitante>,
) -> Result<Json<serde_json::Value>, AgentError> {
    let pool = state
        .pool
        .clone()
        .ok_or_else(|| AgentError::Internal("sin BD".to_string()))?;
    let nombre = input.nombre.trim();
    let telefono = input.telefono.trim();
    if nombre.is_empty() || nombre.len() > 80 {
        return Err(AgentError::BadRequest(
            "nombre requerido (1..80)".to_string(),
        ));
    }
    if !tools::telefono_valido(telefono) {
        return Err(AgentError::BadRequest("telefono invalido".to_string()));
    }
    glory_agent::persistence::ensure_session(&pool, id).await?;
    glory_agent::persistence::set_session_contact(&pool, id, Some(nombre), Some(telefono)).await?;
    /* [279A-2 F1] Igual que la tool: el widget también crea `clientes`
     * (una fila por teléfono, upsert idempotente). Si falla se propaga. */
    ClienteRepository::registrar_y_vincular(&pool, id, Some(nombre), telefono)
        .await
        .map_err(|e| AgentError::Db(e.to_string()))?;
    Ok(Json(serde_json::json!({"ok": true})))
}

#[cfg(test)]
mod pruebas {
    use super::{linea_hora_venezuela, periodo_venezuela};

    /* [E17] Bordes del saludo determinista (el prompt lleva la hora real). */
    #[test]
    fn periodo_cubre_las_24_horas() {
        for h in 0..5 {
            assert_eq!(periodo_venezuela(h), ("de noche", "buenas noches"));
        }
        for h in 5..12 {
            assert_eq!(periodo_venezuela(h), ("de mañana", "buenos días"));
        }
        for h in 12..19 {
            assert_eq!(periodo_venezuela(h), ("de tarde", "buenas tardes"));
        }
        for h in 19..24 {
            assert_eq!(periodo_venezuela(h), ("de noche", "buenas noches"));
        }
        let linea = linea_hora_venezuela();
        assert!(linea.starts_with("Hora actual en Venezuela: "));
        assert!(linea.contains("saluda"));
    }
}
