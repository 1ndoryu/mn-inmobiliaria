use axum::extract::{Path, Query, State};
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthStaff;
use crate::repositories::chat::staff::{
    persistir_respuesta_staff, ClienteResumen, Persistida, SesionDeCliente, SesionResumen,
};
use crate::AppState;
use glory_agent::errors::AgentError;

use super::staff_config::{guardar_config, leer_config}; // [08AA-6] dominio config

/* [169A-4] Atención humana del chat (panel admin). Cada ruta requiere JWT
 * (`AuthStaff`); vive en el router `AppState` porque el extractor pide ese
 * estado, y emite por `AppState.hub` (mismo `ChatHub` que el visitante).
 * Enviar un mensaje como staff implica tomar el hilo: apaga la IA y marca
 * el ciclo `escalated` (como la pestaña Mensajes de Nakomi). */

pub(super) fn fail(e: &AgentError) -> AppError {
    AppError::Internal(e.to_string())
}

pub fn staff_routes() -> Router<AppState> {
    Router::new()
        .route("/agent/sesiones", get(listar_sesiones))
        .route("/agent/sesiones/:id/historial", get(historial))
        .route("/agent/sesiones/:id/mensajes", post(responder))
        .route("/agent/sesiones/:id/devolver", post(devolver_a_ia))
        .route("/agent/sesiones/:id", patch(actualizar_sesion))
        .route("/agent/config", get(leer_config).put(guardar_config))
        /* [279A-2 F5] Consola dueña: clientes, envío manual, uso y auditoría.
         * Responde JSON para operar por terminal/HTTP (ver plan §8). */
        .route("/agent/clientes", get(listar_clientes).post(crear_cliente))
        .route("/agent/clientes/:id", patch(actualizar_cliente))
        .route("/agent/clientes/:id/sesiones", get(sesiones_de_cliente))
        .route("/agent/enviar", post(super::staff_envio::enviar_manual))
        .route("/agent/uso", get(super::staff_envio::uso_mensajes))
        .route("/agent/auditoria", get(super::staff_envio::auditoria))
        /* [289A-1] Vinculación desde la consola: proxy de estado+QR del
         * gateway (el navegador nunca habla con el gateway directo). */
        .route(
            "/agent/whatsapp/sesiones",
            get(super::staff_envio::sesiones_whatsapp),
        )
        .route(
            "/agent/whatsapp/sesiones/:canal/qr",
            get(super::staff_envio::qr_whatsapp),
        )
        /* [011A-2] Sombra F5-Paso1: huella solo-lectura tras GLORY_SHADOW=1. */
        .merge(crate::handlers::ia::sombra::sombra_routes())
}

#[derive(Debug, Deserialize)]
struct FiltroSesiones {
    estado: Option<String>,
    limit: Option<i64>,
}

/// Bandeja staff: sesiones con último mensaje y avisos pendientes, en UNA
/// consulta (LATERAL + subselect; prohibido N+1 por regla 7).
async fn listar_sesiones(
    _auth: AuthStaff,
    State(state): State<AppState>,
    Query(f): Query<FiltroSesiones>,
) -> Result<Json<Vec<SesionResumen>>, AppError> {
    if let Some(e) = &f.estado {
        if !glory_agent::persistence::valid_session_status(e) {
            return Err(AppError::BadRequest(
                "estado debe ser open|escalated|closed".to_string(),
            ));
        }
    }
    let limit = f.limit.unwrap_or(50).clamp(1, 200);
    let filas: Vec<SesionResumen> = crate::repositories::chat::staff::listar_sesiones_bandeja(
        &state.pool,
        f.estado.clone(),
        limit,
    )
    .await?;
    Ok(Json(filas))
}

#[derive(Debug, Deserialize)]
struct HistorialQuery {
    limit: Option<i64>,
    before_seq: Option<i64>,
}

/// Hilo para el panel (paginado por cursor).
/// [309A-3] `before_seq` trae mensajes anteriores a esa secuencia (el panel
/// los antepone al hacer scroll arriba); `limit` acota la página (el panel
/// pide `PAGINA+1` y si llegan todas hay más). Sin cursor trae lo último.
/// Orden ASC de lectura; `list_messages` del núcleo no acepta cursor, así
/// que la consulta es propia (columnas en el orden de `ChatMessage`).
async fn historial(
    _auth: AuthStaff,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(q): Query<HistorialQuery>,
) -> Result<Json<Vec<glory_agent::models::ChatMessage>>, AppError> {
    let limit = q.limit.unwrap_or(100).clamp(1, 200);
    let mut msgs: Vec<glory_agent::models::ChatMessage> =
        crate::repositories::chat::staff::mensajes_historial_paginado(
            &state.pool,
            id,
            q.before_seq,
            limit,
        )
        .await
        .map_err(AppError::from)?;
    msgs.reverse();
    Ok(Json(msgs))
}

#[derive(Debug, Deserialize)]
struct RespuestaStaff {
    body: String,
    /// [09AA-7] Clave por envío que genera el panel. Un reintento con la misma
    /// clave no vuelve a guardar ni a encolar el mensaje.
    #[serde(default)]
    idempotency_key: Option<String>,
}

/// Responder como humano: persiste (sender `staff`), emite por el WS del
/// visitante y TOMA el hilo (`ai_enabled=false` + ciclo `escalated`).
/// [289A-2] Si la sesión tiene hilo `WhatsApp`, el mensaje también se encola
/// al outbox `whatsapp` (`motivo: manual`): antes solo quedaba en el panel
/// y al cliente de `WhatsApp` no le llegaba nada.
/// [09AA-7] Mensaje y outbox se escriben en una transacción (o los dos o
/// ninguno) y un reintento con `idempotency_key` devuelve `duplicado: true`.
/// La toma del hilo va después del commit: son escrituras idempotentes que se
/// repiten sin riesgo si fallan.
async fn responder(
    _auth: AuthStaff,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<RespuestaStaff>,
) -> Result<Json<serde_json::Value>, AppError> {
    let body = input.body.trim().to_string();
    if body.is_empty() || body.len() > 8000 {
        return Err(AppError::BadRequest(
            "mensaje vacio o >8000 chars".to_string(),
        ));
    }
    let clave = input
        .idempotency_key
        .map(|k| k.trim().to_string())
        .filter(|k| !k.is_empty());
    if clave.as_ref().is_some_and(|k| k.len() > 64) {
        return Err(AppError::BadRequest(
            "idempotency_key supera 64 chars".to_string(),
        ));
    }
    glory_agent::persistence::ensure_session(&state.pool, id)
        .await
        .map_err(|e| fail(&e))?;
    let outbox = match crate::repositories::ClienteRepository::hilo_whatsapp(&state.pool, id)
        .await
        .map_err(AppError::from)?
    {
        Some((canal, telefono)) => telefono
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(|destino| {
                serde_json::json!({
                    "session_id": id,
                    "destino": destino,
                    "texto": body,
                    "via": canal,
                    "motivo": "manual",
                })
            }),
        None => None,
    };
    let persistida = persistir_respuesta(&state.pool, id, &body, clave.as_deref(), outbox).await?;
    let duplicado = matches!(persistida, Persistida::Duplicada { .. });
    let seq = match persistida {
        Persistida::Nueva(msg) => {
            let seq = msg.sequence_num;
            let _ = state
                .hub
                .broadcast(id, &glory_agent::models::WsServerMessage::live(msg));
            seq
        }
        Persistida::Duplicada { sequence_num } => sequence_num,
    };
    glory_agent::persistence::set_session_ai(&state.pool, id, false)
        .await
        .map_err(|e| fail(&e))?;
    glory_agent::persistence::upsert_response_cycle(&state.pool, id, "escalated")
        .await
        .map_err(|e| fail(&e))?;
    /* [279A-2 F3] La toma humana también mueve la máquina propia a
     * `delegada` (un solo estado de verdad, sin doble fuente). */
    crate::repositories::ClienteRepository::marcar_atencion(&state.pool, id, "delegada")
        .await
        .map_err(AppError::from)?;
    Ok(Json(
        serde_json::json!({"ok": true, "sequence_num": seq, "duplicado": duplicado}),
    ))
}

/// [09AA-7] Escritura atómica del turno humano (SQL en el repositorio). Reintenta
/// solo si otro escritor (p. ej. `insert_message_seq`) tomó el mismo número de
/// secuencia.
async fn persistir_respuesta(
    pool: &PgPool,
    session_id: Uuid,
    body: &str,
    clave: Option<&str>,
    outbox: Option<serde_json::Value>,
) -> Result<Persistida, AppError> {
    const INTENTOS: u8 = 3;
    let mut intento = 1;
    loop {
        match persistir_respuesta_staff(pool, session_id, body, clave, outbox.as_ref()).await {
            Err(e) if es_unique_violation(&e) && intento < INTENTOS => {
                intento += 1;
            }
            otro => return otro.map_err(AppError::from),
        }
    }
}

fn es_unique_violation(e: &sqlx::Error) -> bool {
    e.as_database_error()
        .and_then(sqlx::error::DatabaseError::code)
        .is_some_and(|c| c == "23505")
}

#[derive(Debug, Deserialize)]
struct DevolucionIA {
    nota: String,
}

/// Devolver a la IA con nota (cierre de `consultar_agente`): la nota queda
/// como mensaje `staff`, el ciclo pasa a `answered`, la IA se reactiva y la
/// máquina propia vuelve a `activa`. Solo humanos (JWT).
async fn devolver_a_ia(
    _auth: AuthStaff,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<DevolucionIA>,
) -> Result<Json<serde_json::Value>, AppError> {
    let nota = input.nota.trim().to_string();
    if nota.is_empty() || nota.len() > 2000 {
        return Err(AppError::BadRequest("nota requerida (1..2000)".to_string()));
    }
    glory_agent::persistence::ensure_session(&state.pool, id)
        .await
        .map_err(|e| fail(&e))?;
    /* [289A-2] Igual que Responder: `insert_message_seq`, nunca el contador
     * en memoria del hub (colisiona tras reinicio). */
    let msg = glory_agent::persistence::insert_message_seq(
        &state.pool,
        &state.hub,
        id,
        "staff",
        &nota,
        None,
        None,
    )
    .await
    .map_err(|e| fail(&e))?;
    let seq = msg.sequence_num;
    let _ = state
        .hub
        .broadcast(id, &glory_agent::models::WsServerMessage::live(msg));
    glory_agent::persistence::set_session_ai(&state.pool, id, true)
        .await
        .map_err(|e| fail(&e))?;
    glory_agent::persistence::upsert_response_cycle(&state.pool, id, "answered")
        .await
        .map_err(|e| fail(&e))?;
    crate::repositories::ClienteRepository::marcar_atencion(&state.pool, id, "activa")
        .await
        .map_err(AppError::from)?;
    Ok(Json(serde_json::json!({"ok": true, "sequence_num": seq})))
}

#[derive(Debug, Deserialize)]
struct CambioSesion {
    #[serde(rename = "aiEnabled")]
    ai_enabled: Option<bool>,
    status: Option<String>,
}

/// Soltar la IA (`aiEnabled: true`), tomarla (`false`) o cerrar/archivar
/// (`status`). Validado en boundary.
async fn actualizar_sesion(
    _auth: AuthStaff,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<CambioSesion>,
) -> Result<Json<glory_agent::models::ChatSession>, AppError> {
    if input.ai_enabled.is_none() && input.status.is_none() {
        return Err(AppError::BadRequest("nada que cambiar".to_string()));
    }
    if let Some(status) = &input.status {
        if !glory_agent::persistence::valid_session_status(status) {
            return Err(AppError::BadRequest(
                "status debe ser open|escalated|closed".to_string(),
            ));
        }
        glory_agent::persistence::set_session_status(&state.pool, id, status)
            .await
            .map_err(|e| fail(&e))?;
        if status == "escalated" {
            glory_agent::persistence::upsert_response_cycle(&state.pool, id, "escalated")
                .await
                .map_err(|e| fail(&e))?;
        }
    }
    if let Some(enabled) = input.ai_enabled {
        glory_agent::persistence::set_session_ai(&state.pool, id, enabled)
            .await
            .map_err(|e| fail(&e))?;
        /* [279A-2 F3] El toggle manual también mueve la máquina propia
         * (una sola verdad): soltar → `activa`, tomar → `delegada`. */
        let estado = if enabled { "activa" } else { "delegada" };
        crate::repositories::ClienteRepository::marcar_atencion(&state.pool, id, estado)
            .await
            .map_err(AppError::from)?;
    }
    let sesion = glory_agent::persistence::get_session(&state.pool, id)
        .await
        .map_err(|e| fail(&e))?
        .ok_or_else(|| AppError::NotFound("sesion no existe".to_string()))?;
    Ok(Json(sesion))
}

/* [08AA-6] La config editable vive en `chat_staff_config` (split god-object). */

/* [279A-2 F5] Consola dueña (backend operable por HTTP; el panel web existe
 * para bandeja/hilo/responder y estos endpoints lo extienden sin N+1).
 * Ejemplos (con JWT `$T`):
 * `curl -H "Authorization: Bearer $T" /api/admin/agent/clientes?query=0412`
 * `curl -X POST -H "Authorization: Bearer $T" /api/admin/agent/enviar
 *   -d '{"telefono":"04120825234","texto":"Hola, soy MN"}'` */

#[derive(Debug, Deserialize)]
struct FiltroClientes {
    query: Option<String>,
    limit: Option<i64>,
}

/// Lista de clientes con nº de sesiones (UNA consulta, sin N+1).
async fn listar_clientes(
    _auth: AuthStaff,
    State(state): State<AppState>,
    Query(f): Query<FiltroClientes>,
) -> Result<Json<Vec<ClienteResumen>>, AppError> {
    let limit = f.limit.unwrap_or(50).clamp(1, 200);
    let q = f.query.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let filas: Vec<ClienteResumen> =
        crate::repositories::chat::staff::listar_clientes_resumen(&state.pool, q, limit).await?;
    Ok(Json(filas))
}

#[derive(Debug, Deserialize)]
struct NuevoCliente {
    nombre: Option<String>,
    telefono: String,
    origen: Option<String>,
}

/// Alta manual de cliente (la dueña lo crea y luego le envía por `/enviar`).
async fn crear_cliente(
    _auth: AuthStaff,
    State(state): State<AppState>,
    Json(input): Json<NuevoCliente>,
) -> Result<Json<crate::models::Cliente>, AppError> {
    let telefono = input.telefono.trim();
    if !super::tools::telefono_valido(telefono) {
        return Err(AppError::BadRequest("telefono invalido".to_string()));
    }
    let origen = input.origen.as_deref().map_or("web", str::trim);
    if !matches!(origen, "web" | "wa_a" | "wa_b") {
        return Err(AppError::BadRequest(
            "origen debe ser web|wa_a|wa_b".to_string(),
        ));
    }
    let nombre = input
        .nombre
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty());
    if let Some(n) = nombre {
        if n.len() > 80 {
            return Err(AppError::BadRequest("nombre <=80".to_string()));
        }
    }
    let normalizado = crate::repositories::ClienteRepository::normalizar_telefono(telefono);
    let fila = crate::repositories::ClienteRepository::registrar_con_origen(
        &state.pool,
        nombre,
        &normalizado,
        origen,
    )
    .await?;
    Ok(Json(crate::models::Cliente::from_row(fila)))
}

#[derive(Debug, Deserialize)]
struct CambioCliente {
    nombre: Option<String>,
    interes: Option<String>,
    presupuesto: Option<String>,
    zona: Option<String>,
    notas: Option<String>,
}

/// Ficha comercial editable (interés/presupuesto/zona/notas los completa F4
/// con memoria; hoy los edita la dueña a mano).
async fn actualizar_cliente(
    _auth: AuthStaff,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<CambioCliente>,
) -> Result<Json<crate::models::Cliente>, AppError> {
    for (campo, v) in [
        ("nombre", &input.nombre),
        ("interes", &input.interes),
        ("presupuesto", &input.presupuesto),
        ("zona", &input.zona),
        ("notas", &input.notas),
    ] {
        if let Some(t) = v {
            let tope = if campo == "notas" { 2000 } else { 200 };
            if t.trim().is_empty() || t.len() > tope {
                return Err(AppError::BadRequest(format!(
                    "{campo} requerido (1..{tope})"
                )));
            }
        }
    }
    if input.nombre.is_none()
        && input.interes.is_none()
        && input.presupuesto.is_none()
        && input.zona.is_none()
        && input.notas.is_none()
    {
        return Err(AppError::BadRequest("nada que cambiar".to_string()));
    }
    let fila: Option<crate::models::ClienteRow> =
        crate::repositories::chat::staff::actualizar_cliente_campos(
            &state.pool,
            id,
            input.nombre.as_deref().map(str::trim),
            input.interes.as_deref().map(str::trim),
            input.presupuesto.as_deref().map(str::trim),
            input.zona.as_deref().map(str::trim),
            input.notas.as_deref().map(str::trim),
        )
        .await?;
    let Some(row) = fila else {
        return Err(AppError::NotFound("cliente no existe".to_string()));
    };
    Ok(Json(crate::models::Cliente::from_row(row)))
}

/// Sesiones de un cliente (sus hilos web/WhatsApp enlazados por `clientes`).
async fn sesiones_de_cliente(
    _auth: AuthStaff,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<SesionDeCliente>>, AppError> {
    let filas: Vec<SesionDeCliente> =
        crate::repositories::chat::staff::listar_sesiones_de_cliente(&state.pool, id).await?;
    Ok(Json(filas))
}

/* [08AA-8] Envío manual + uso + auditoría + gateway WhatsApp: ver
 * `chat_staff_envio.rs` (mismo dominio de consola, `pub(super)`). */

/* [08AA-6] El test de la allowlist vive con su dominio en
 * `chat_staff_config::pruebas`. */

#[cfg(test)]
mod pruebas {
    use super::*;

    /* Sin `DATABASE_URL` de rama (o apuntando a la BD de desarrollo) se omite,
     * como el resto de tests de BD viva. */
    fn pool_si_hay() -> Option<PgPool> {
        let url = crate::config::url_bd_pruebas()?;
        sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy(&url)
            .ok()
    }

    /* [09AA-7] Un reintento con la misma clave no duplica el mensaje ni el
     * envío a WhatsApp; sin hilo WhatsApp no se encola nada. */
    #[tokio::test]
    async fn responder_con_clave_no_duplica_mensaje_ni_envio() {
        let Some(pool) = pool_si_hay() else { return };
        let sid = Uuid::new_v4();
        glory_agent::persistence::ensure_session(&pool, sid)
            .await
            .unwrap();
        let outbox = serde_json::json!({
            "session_id": sid,
            "destino": "584120825234",
            "texto": "Hola",
            "via": "wa_a",
            "motivo": "manual",
        });
        let clave = format!("prueba-{sid}");
        let primera = persistir_respuesta(&pool, sid, "Hola", Some(&clave), Some(outbox.clone()))
            .await
            .unwrap();
        assert!(matches!(primera, Persistida::Nueva(ref m) if m.sequence_num == 1));
        let segunda = persistir_respuesta(&pool, sid, "Hola", Some(&clave), Some(outbox))
            .await
            .unwrap();
        assert!(matches!(segunda, Persistida::Duplicada { sequence_num: 1 }));

        let mensajes = sqlx::query_scalar!(
            r#"SELECT COUNT(*)::BIGINT AS "n!" FROM agent_messages WHERE session_id = $1"#,
            sid
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(mensajes, 1);
        let encolados = sqlx::query_scalar!(
            r#"SELECT COUNT(*)::BIGINT AS "n!" FROM agent_outbox WHERE payload->>'session_id' = $1"#,
            sid.to_string()
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(encolados, 1);

        let web = Uuid::new_v4();
        glory_agent::persistence::ensure_session(&pool, web)
            .await
            .unwrap();
        let sin_hilo = persistir_respuesta(&pool, web, "Hola", Some("web-1"), None)
            .await
            .unwrap();
        assert!(matches!(sin_hilo, Persistida::Nueva(_)));
        let sin_outbox = sqlx::query_scalar!(
            r#"SELECT COUNT(*)::BIGINT AS "n!" FROM agent_outbox WHERE payload->>'session_id' = $1"#,
            web.to_string()
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(sin_outbox, 0);

        sqlx::query!(
            "DELETE FROM agent_outbox WHERE payload->>'session_id' = $1",
            sid.to_string()
        )
        .execute(&pool)
        .await
        .unwrap();
        for s in [sid, web] {
            sqlx::query!("DELETE FROM agent_messages WHERE session_id = $1", s)
                .execute(&pool)
                .await
                .unwrap();
        }
    }
}
