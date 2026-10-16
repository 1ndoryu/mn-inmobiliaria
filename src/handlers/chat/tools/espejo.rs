/* [16AA-9 F3] Espejo de los envios al visitante en el hilo (`agent_messages`)
 * y ventana de dedup entre turnos, compartida por tarjetas y fotos. Salio de
 * `chat/tools.rs`. */

use std::collections::HashSet;

use sqlx::PgPool;
use uuid::Uuid;

use glory_agent::session::ChatHub;

/// Cuerpos de los últimos mensajes `ai` del hilo (ventana de 30): base del
/// dedup 309A-1 (tarjetas y fotos ya enviadas). Vacío + WARN si falla.
/* [011A-1] Foto F5-Paso0: la ventana de 30 queda en const con nombre para
 * que la sombra detecte si cambia (era literal suelto). */
pub(super) const VENTANA_DEDUP_TARJETAS: i64 = 30;
pub(super) async fn cuerpos_ai_recientes(pool: &PgPool, session_id: Uuid) -> HashSet<String> {
    match glory_agent::persistence::list_messages(pool, session_id, VENTANA_DEDUP_TARJETAS).await {
        Ok(msgs) => msgs
            .into_iter()
            .filter(|m| m.sender == "ai")
            .map(|m| m.body)
            .collect(),
        Err(e) => {
            tracing::warn!(
                "dedup sesion={session_id}: sin historial reciente ({e}), se envía todo"
            );
            HashSet::new()
        }
    }
}

/// [299A-4] Espejo de envíos `WhatsApp` en el hilo: `agent_outbox` es cola
/// transitoria (el worker BORRA la fila tras enviar) y `historial` solo lee
/// `agent_messages`, así que tarjeta/foto nunca aparecían en /admin aunque
/// el visitante sí las recibía. Tras cada enqueue al visitante se persiste
/// el mismo contenido como `ai` (la foto en marca `[foto] url — se ve: pie`,
/// que `MessageMedia` ya renderiza). Si el espejo falla se avisa y se sigue:
/// el envío ya está encolado y tumbar la tool duplicaría el envío al
/// reintentar el modelo. Sin hub no hay espejo (tests y widget web).
pub(super) async fn espejar_en_hilo(
    pool: &PgPool,
    hub: Option<&ChatHub>,
    session_id: Uuid,
    body: &str,
) {
    let Some(hub) = hub else { return };
    if let Err(e) =
        glory_agent::persistence::insert_message_seq(pool, hub, session_id, "ai", body, None, None)
            .await
    {
        tracing::warn!("espejo sesion={session_id}: envío sin reflejar en el hilo ({e})");
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    /* [011A-1] Foto F5-Paso0: el dedup de tarjetas mira los últimos 30
     * mensajes `ai` del hilo. */
    #[test]
    fn foto_dedup_mira_ultimos_30() {
        assert_eq!(VENTANA_DEDUP_TARJETAS, 30);
    }
}
