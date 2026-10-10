//! Consultas SQL del envío de mensajes del staff (movidas desde `handlers/chat_staff_envio.rs`).

use serde::Serialize;
use uuid::Uuid;

use crate::models::ClienteRow;

/// Teléfono registrado de una sesión de canal (`None` si la sesión no tiene fila).
pub(crate) async fn telefono_de_sesion(
    pool: &sqlx::PgPool,
    sid: Uuid,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT telefono FROM canal_sesiones WHERE session_id = $1")
        .bind(sid)
        .fetch_optional(pool)
        .await
}

/// Cliente por id con las columnas que usa el envío manual.
pub(crate) async fn cliente_por_id(
    pool: &sqlx::PgPool,
    id: Uuid,
) -> Result<Option<ClienteRow>, sqlx::Error> {
    sqlx::query_as!(
        ClienteRow,
        "SELECT id, nombre, telefono, origen, interes, presupuesto, zona, \
              notas, created_at, updated_at FROM clientes WHERE id = $1",
        id
    )
    .fetch_optional(pool)
    .await
}

/// Fila de uso agregado por día×remitente.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub(crate) struct UsoDia {
    dia: Option<chrono::NaiveDate>,
    remitente: Option<String>,
    mensajes: Option<i64>,
    tokens_est: Option<i64>,
    tokens_in: Option<i64>,
    tokens_out: Option<i64>,
}

/// Uso agregado por día×remitente de los últimos `dias` días.
pub(crate) async fn uso_mensajes_por_dia(
    pool: &sqlx::PgPool,
    dias: i64,
) -> Result<Vec<UsoDia>, sqlx::Error> {
    // Alias `?` fuerza Option en columnas que la struct declara opcionales
    // (agregados y expresiones: sqlx no puede inferir su nulabilidad).
    sqlx::query_as!(
        UsoDia,
        "SELECT date_trunc('day', created_at)::DATE AS \"dia?\", sender AS \"remitente?\", \
          COUNT(*) AS \"mensajes?\", SUM(tokens_est) AS \"tokens_est?\", \
          SUM(COALESCE(tokens_in, 0)) AS \"tokens_in?\", SUM(COALESCE(tokens_out, 0)) AS \"tokens_out?\" \
         FROM uso_mensajes \
         WHERE created_at >= NOW() - ($1::text || ' days')::INTERVAL \
         GROUP BY 1, 2 ORDER BY 1 DESC, 2",
        dias.to_string()
    )
    .fetch_all(pool)
    .await
}

/// Fila de auditoría de una toma humana con contexto del hilo.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub(crate) struct AuditoriaFila {
    session_id: Uuid,
    extracto: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    status: Option<String>,
    ai_enabled: Option<bool>,
    estado_atencion: Option<String>,
    modo_atencion: Option<String>,
    nombre: Option<String>,
    telefono: Option<String>,
    ia: Option<i64>,
    humano: Option<i64>,
    visitante: Option<i64>,
}

/// Últimas tomas humanas con contexto y conteo IA/humano/visitante por hilo.
pub(crate) async fn auditoria_tomas_humanas(
    pool: &sqlx::PgPool,
    limit: i64,
) -> Result<Vec<AuditoriaFila>, sqlx::Error> {
    sqlx::query_as!(
        AuditoriaFila,
        "SELECT m.session_id, LEFT(m.body, 200) AS \"extracto?\", m.created_at, \
          s.status AS \"status?\", s.ai_enabled AS \"ai_enabled?\", \
          a.estado AS \"estado_atencion?\", a.modo AS \"modo_atencion?\", \
          c.nombre AS \"nombre?\", c.telefono AS \"telefono?\", \
          (SELECT COUNT(*) FROM agent_messages WHERE session_id = m.session_id AND sender = 'ai') AS \"ia?\", \
          (SELECT COUNT(*) FROM agent_messages WHERE session_id = m.session_id AND sender = 'staff') AS \"humano?\", \
          (SELECT COUNT(*) FROM agent_messages WHERE session_id = m.session_id AND sender = 'client') AS \"visitante?\" \
         FROM agent_messages m \
         JOIN agent_sessions s ON s.id = m.session_id \
         LEFT JOIN atencion_sesiones a ON a.session_id = m.session_id \
         LEFT JOIN canal_sesiones cs ON cs.session_id = m.session_id \
         LEFT JOIN clientes c ON c.id = cs.cliente_id \
         WHERE m.sender = 'staff' ORDER BY m.created_at DESC LIMIT $1",
        limit
    )
    .fetch_all(pool)
    .await
}
