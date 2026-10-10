//! Consultas SQL del staff de chat (movidas desde `handlers/chat_staff*.rs`).

use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub(crate) struct SesionResumen {
    id: Uuid,
    visitor_name: Option<String>,
    contact: Option<String>,
    status: String,
    ai_enabled: bool,
    /* [279A-2 F3] Estado propio de delegación (puede faltar en sesiones
     * viejas: LEFT JOIN + default en el panel). */
    estado_atencion: Option<String>,
    modo_atencion: Option<String>,
    /* [289A-2] Teléfono del cliente (`canal_sesiones`; el panel muestra el
     * número en la conversación). */
    telefono: Option<String>,
    last_body: Option<String>,
    last_sender: Option<String>,
    #[sqlx(rename = "last_at")]
    last_at: Option<chrono::DateTime<chrono::Utc>>,
    alertas: Option<i64>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

pub(crate) async fn listar_sesiones_bandeja(
    pool: &PgPool,
    estado: Option<String>,
    limit: i64,
) -> Result<Vec<SesionResumen>, sqlx::Error> {
    let filas: Vec<SesionResumen> = sqlx::query_as(
        "SELECT s.id, s.visitor_name, s.contact, s.status, s.ai_enabled, \
         a.estado AS estado_atencion, a.modo AS modo_atencion, \
         cs.telefono AS telefono, \
         m.body AS last_body, m.sender AS last_sender, m.created_at AS last_at, \
         (SELECT COUNT(*) FROM agent_outbox o WHERE o.status = 'pending' \
          AND o.kind = 'whatsapp' AND o.payload->>'session_id' = s.id::TEXT) AS alertas, \
          s.updated_at \
          FROM agent_sessions s \
          LEFT JOIN atencion_sesiones a ON a.session_id = s.id \
          LEFT JOIN canal_sesiones cs ON cs.session_id = s.id \
          LEFT JOIN LATERAL (SELECT body, sender, created_at FROM agent_messages \
            WHERE session_id = s.id ORDER BY sequence_num DESC LIMIT 1) m ON true \
          WHERE ($1::TEXT IS NULL OR s.status = $1) \
          ORDER BY s.updated_at DESC LIMIT $2",
    )
    .bind(estado)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(filas)
}

/// Orden DESC por `sequence_num` con cursor opcional: el handler lo invierte a ASC.
pub(crate) async fn mensajes_historial_paginado(
    pool: &PgPool,
    session_id: Uuid,
    before_seq: Option<i64>,
    limit: i64,
) -> Result<Vec<glory_agent::models::ChatMessage>, sqlx::Error> {
    let msgs: Vec<glory_agent::models::ChatMessage> = sqlx::query_as(
        "SELECT id, session_id, sender, body, sequence_num, input_tokens, output_tokens, created_at \
         FROM agent_messages WHERE session_id = $1 \
         AND ($2::BIGINT IS NULL OR sequence_num < $2) \
         ORDER BY sequence_num DESC LIMIT $3",
    )
    .bind(session_id)
    .bind(before_seq)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(msgs)
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub(crate) struct ClienteResumen {
    id: Uuid,
    nombre: Option<String>,
    telefono: String,
    origen: String,
    interes: Option<String>,
    presupuesto: Option<String>,
    zona: Option<String>,
    notas: Option<String>,
    sesiones: Option<i64>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

pub(crate) async fn listar_clientes_resumen(
    pool: &PgPool,
    query: Option<&str>,
    limit: i64,
) -> Result<Vec<ClienteResumen>, sqlx::Error> {
    let filas: Vec<ClienteResumen> = sqlx::query_as(
        "SELECT c.id, c.nombre, c.telefono, c.origen, c.interes, c.presupuesto, c.zona, \
          c.notas, (SELECT COUNT(*) FROM canal_sesiones cs WHERE cs.cliente_id = c.id) AS sesiones, \
          c.created_at, c.updated_at \
         FROM clientes c \
         WHERE ($1::TEXT IS NULL OR c.nombre ILIKE '%' || $1 || '%' OR c.telefono ILIKE '%' || $1 || '%') \
         ORDER BY c.updated_at DESC LIMIT $2",
    )
    .bind(query)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(filas)
}

/// UPDATE parcial: cada `None` conserva el valor actual (COALESCE).
/// Devuelve `None` si el cliente no existe.
pub(crate) async fn actualizar_cliente_campos(
    pool: &PgPool,
    id: Uuid,
    nombre: Option<&str>,
    interes: Option<&str>,
    presupuesto: Option<&str>,
    zona: Option<&str>,
    notas: Option<&str>,
) -> Result<Option<crate::models::ClienteRow>, sqlx::Error> {
    let fila: Option<crate::models::ClienteRow> = sqlx::query_as(
        "UPDATE clientes SET \
          nombre = COALESCE($2, nombre), interes = COALESCE($3, interes), \
          presupuesto = COALESCE($4, presupuesto), zona = COALESCE($5, zona), \
          notas = COALESCE($6, notas), updated_at = NOW() \
         WHERE id = $1 \
         RETURNING id, nombre, telefono, origen, interes, presupuesto, zona, \
           notas, created_at, updated_at",
    )
    .bind(id)
    .bind(nombre)
    .bind(interes)
    .bind(presupuesto)
    .bind(zona)
    .bind(notas)
    .fetch_optional(pool)
    .await?;
    Ok(fila)
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub(crate) struct SesionDeCliente {
    session_id: Uuid,
    canal: Option<String>,
    telefono: Option<String>,
    modo: Option<String>,
    estado_atencion: Option<String>,
    status: Option<String>,
    ai_enabled: Option<bool>,
    last_body: Option<String>,
    last_sender: Option<String>,
    #[sqlx(rename = "last_at")]
    last_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub(crate) async fn listar_sesiones_de_cliente(
    pool: &PgPool,
    cliente_id: Uuid,
) -> Result<Vec<SesionDeCliente>, sqlx::Error> {
    let filas: Vec<SesionDeCliente> = sqlx::query_as(
        "SELECT cs.session_id, cs.canal, cs.telefono, cs.modo, a.estado AS estado_atencion, \
          s.status, s.ai_enabled, \
          m.body AS last_body, m.sender AS last_sender, m.created_at AS last_at \
         FROM canal_sesiones cs \
         JOIN agent_sessions s ON s.id = cs.session_id \
         LEFT JOIN atencion_sesiones a ON a.session_id = cs.session_id \
         LEFT JOIN LATERAL (SELECT body, sender, created_at FROM agent_messages \
           WHERE session_id = cs.session_id ORDER BY sequence_num DESC LIMIT 1) m ON true \
         WHERE cs.cliente_id = $1 ORDER BY s.updated_at DESC LIMIT 100",
    )
    .bind(cliente_id)
    .fetch_all(pool)
    .await?;
    Ok(filas)
}
