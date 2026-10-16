/* [16AA-9 F3] `enviar_fotos_inmueble`: hasta 3 `image+caption` por WhatsApp,
 * con el mismo dedup del hilo que las tarjetas. Salio de `chat/tools.rs`. */

use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use crate::repositories::chat::tools::{claves_fotos_inmueble, titulo_inmueble_publicado};
use crate::repositories::ClienteRepository;
use crate::services::{
    clave_idempotencia, corte_cubre, debe_usar_clave, encolar_outbox_idem, Encolado,
};
use glory_agent::errors::AgentError;
use glory_agent::session::ChatHub;

use super::espejo::{cuerpos_ai_recientes, espejar_en_hilo};

/* [299A-1 E13] La IA envía fotos del catálogo por WhatsApp: hasta 3
 * `image+caption` (pie = título) vía outbox `whatsapp` con `media_url`
 * absoluta (`/uploads/<storage_key>` bajo `PUBLIC_BASE_URL`). `destino` =
 * teléfono del visitante (ficha) y `via` = canal de la sesión, igual que
 * `consultar`. Sin fotos o sin teléfono responde `error` (la IA lo dice). */
pub(super) async fn enviar_fotos(
    pool: &PgPool,
    session_id: Uuid,
    args: &Value,
    hub: Option<&ChatHub>,
) -> Result<Value, AgentError> {
    let id: Uuid = args
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .parse()
        .map_err(|_| AgentError::BadRequest("id de inmueble invalido".to_string()))?;
    let max = args
        .get("max")
        .and_then(Value::as_i64)
        .unwrap_or(3)
        .clamp(1, 3);
    let titulo: Option<String> = titulo_inmueble_publicado(pool, id)
        .await
        .map_err(|e| AgentError::Db(e.to_string()))?;
    let Some(titulo) = titulo.filter(|t| !t.trim().is_empty()) else {
        return Ok(json!({"error": "inmueble no disponible"}));
    };
    let claves: Vec<String> = claves_fotos_inmueble(pool, id, max)
        .await
        .map_err(|e| AgentError::Db(e.to_string()))?;
    if claves.is_empty() {
        return Ok(json!({"error": "ese inmueble aún no tiene fotos"}));
    }
    let ficha = ClienteRepository::ficha_para_aviso(pool, session_id)
        .await
        .map_err(|e| AgentError::Db(e.to_string()))?;
    let Some(telefono) = ficha
        .telefono
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
    else {
        return Ok(json!({"error": "sin teléfono del visitante"}));
    };
    let via = ClienteRepository::canal_de(pool, session_id)
        .await
        .ok()
        .flatten()
        .filter(|v| v == "wa_a" || v == "wa_b")
        .unwrap_or_else(|| "wa_a".to_string());
    /* Base pública para `media_url`: en local el gateway descarga de este
     * mismo backend; en producción `PUBLIC_BASE_URL` lleva el dominio
     * (los servidores de WhatsApp deben alcanzarla). */
    let base = std::env::var("PUBLIC_BASE_URL")
        .ok()
        .map(|b| b.trim().trim_end_matches('/').to_string())
        .filter(|b| !b.is_empty())
        .unwrap_or_else(|| "http://127.0.0.1:3000".to_string());
    let total = claves.len();
    /* [309A-1] Dedup de fotos entre turnos (misma causa que las tarjetas:
     * re-llamar con el mismo id re-enviaba las 3 fotos). Se compara la URL
     * contra los espejos `[foto] url ...` ya presentes en el hilo. */
    let ya_enviadas = cuerpos_ai_recientes(pool, session_id).await;
    let mut enviadas = 0;
    let mut repetidas = 0;
    for (i, clave) in claves.iter().enumerate() {
        let pie = if total > 1 {
            format!("{} ({}/{})", titulo.trim(), i + 1, total)
        } else {
            titulo.trim().to_string()
        };
        /* [299A-4] La misma foto que viaja por WhatsApp queda en el hilo
         * (`[foto] url — se ve: pie` la renderiza `MessageMedia`). */
        let url = format!("{base}/uploads/{clave}");
        if ya_enviadas.iter().any(|b| b.contains(&url)) {
            repetidas += 1;
            continue;
        }
        let espejo = format!("[foto] {url} — se ve: {}", pie.trim());
        /* [011A-5 Fase1] Igual que tarjetas: clave bajo corte, duplicado
         * en vuelo cuenta como repetida (el gemelo ya espejó). Se calcula
         * antes del `json!` porque este mueve `pie` y `via`. */
        let clave_foto;
        let clave_foto_ref = if debe_usar_clave("ia_foto") && corte_cubre(pool, &via).await {
            clave_foto = clave_idempotencia(&session_id.to_string(), "ia_foto", &pie);
            Some(clave_foto.as_str())
        } else {
            None
        };
        let aviso = json!({
            "session_id": session_id.to_string(),
            "destino": telefono,
            "texto": pie,
            "media_url": url,
            "via": via,
            "motivo": "ia_foto",
        });
        match encolar_outbox_idem(pool, "whatsapp", aviso, clave_foto_ref).await {
            Ok(Encolado::Duplicado) => {
                repetidas += 1;
                continue;
            }
            /* [011A-5 Fase3] Revivido: el gemelo `failed` vuelve a
             * `pending`; no se espeja (el primer encolado ya dejó la
             * foto en el hilo). */
            Ok(Encolado::Revivido(_)) => {}
            Err(e) => return Err(e.into()),
            Ok(Encolado::Nuevo(_)) => {
                espejar_en_hilo(pool, hub, session_id, &espejo).await;
            }
        }
        enviadas += 1;
    }
    Ok(json!({"ok": true, "enviadas": enviadas, "repetidas": repetidas, "titulo": titulo}))
}

#[cfg(test)]
mod pruebas {
    use super::super::pruebas::pool_si_hay;
    use super::super::Herramientas;
    use super::*;
    use glory_agent::tools::{ToolCtx, ToolExecutor};

    /* [299A-1 E13] Enviar fotos encola un outbox `whatsapp` con `media_url`
     * por foto (tope `max`) y rechaza id inválido/ausente sin tocar BD. */
    #[tokio::test]
    async fn enviar_fotos_encola_media_url() {
        let Some(pool) = pool_si_hay() else { return };
        let h = Herramientas::new(pool.clone(), "Test 600111222".to_string());
        let sesion = Uuid::new_v4();
        glory_agent::persistence::ensure_session(&pool, sesion)
            .await
            .unwrap();
        let ctx = ToolCtx::new(sesion, Some(pool.clone()));

        let mala = h
            .execute("enviar_fotos_inmueble", &json!({"id": "no-uuid"}), &ctx)
            .await;
        assert!(mala.is_err());
        let ausente = h
            .execute(
                "enviar_fotos_inmueble",
                &json!({"id": Uuid::new_v4().to_string()}),
                &ctx,
            )
            .await
            .unwrap();
        assert!(ausente.get("error").is_some());

        h.execute(
            "registrar_contacto",
            &json!({"nombre": "Foto Test", "telefono": "+34622222222"}),
            &ctx,
        )
        .await
        .unwrap();
        /* Inmueble semilla con fotos (solo lectura; no se toca). */
        let id: String = sqlx::query_scalar(
            "SELECT id::TEXT FROM inmuebles WHERE publicado \
             AND (SELECT COUNT(*) FROM fotos WHERE inmueble_id = inmuebles.id) > 0 LIMIT 1",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        let r = h
            .execute("enviar_fotos_inmueble", &json!({"id": id, "max": 2}), &ctx)
            .await
            .unwrap();
        assert_eq!(r.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(r.get("enviadas").and_then(Value::as_i64), Some(2));
        let fotos: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM agent_outbox WHERE status = 'pending' \
             AND kind = 'whatsapp' AND payload->>'session_id' = $1 \
             AND payload->>'motivo' = 'ia_foto' AND payload ? 'media_url'",
        )
        .bind(sesion.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(fotos, 2);

        sqlx::query("DELETE FROM agent_outbox WHERE payload->>'session_id' = $1")
            .bind(sesion.to_string())
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM agent_messages WHERE session_id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM canal_sesiones WHERE session_id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM atencion_sesiones WHERE session_id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM clientes WHERE telefono = '34622222222'")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM agent_response_cycles WHERE session_id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM agent_sessions WHERE id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
    }

    /* [299A-4] Con hub, cada `ia_foto` persiste su espejo `ai [foto] ...`
     * en el hilo (sin hub no hay espejo: el resto de tests lo confirma). */
    #[tokio::test]
    async fn enviar_fotos_espeja_hilo_con_hub() {
        let Some(pool) = pool_si_hay() else { return };
        let h = Herramientas::new(pool.clone(), "Test 600111222".to_string())
            .with_hub(ChatHub::default());
        let sesion = Uuid::new_v4();
        glory_agent::persistence::ensure_session(&pool, sesion)
            .await
            .unwrap();
        let ctx = ToolCtx::new(sesion, Some(pool.clone()));

        h.execute(
            "registrar_contacto",
            &json!({"nombre": "Espejo Test", "telefono": "+34633333333"}),
            &ctx,
        )
        .await
        .unwrap();
        let id: String = sqlx::query_scalar(
            "SELECT id::TEXT FROM inmuebles WHERE publicado \
             AND (SELECT COUNT(*) FROM fotos WHERE inmueble_id = inmuebles.id) > 0 LIMIT 1",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        let r = h
            .execute("enviar_fotos_inmueble", &json!({"id": id, "max": 2}), &ctx)
            .await
            .unwrap();
        assert_eq!(r.get("enviadas").and_then(Value::as_i64), Some(2));
        let espejos: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM agent_messages WHERE session_id = $1 \
             AND sender = 'ai' AND body LIKE '[foto] %'",
        )
        .bind(sesion)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(espejos, 2);

        sqlx::query("DELETE FROM agent_outbox WHERE payload->>'session_id' = $1")
            .bind(sesion.to_string())
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM agent_messages WHERE session_id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM canal_sesiones WHERE session_id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM atencion_sesiones WHERE session_id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM clientes WHERE telefono = '34633333333'")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM agent_response_cycles WHERE session_id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM agent_sessions WHERE id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
    }

    /* [309A-1] Re-llamar `enviar_fotos` con el mismo id no reenvía las fotos
     * ya presentes en el hilo (mismo dedup que las tarjetas). */
    #[tokio::test]
    async fn enviar_fotos_no_repite_fotos_ya_enviadas() {
        let Some(pool) = pool_si_hay() else { return };
        let h = Herramientas::new(pool.clone(), "Test 600111222".to_string())
            .with_hub(ChatHub::default());
        let sesion = Uuid::new_v4();
        glory_agent::persistence::ensure_session(&pool, sesion)
            .await
            .unwrap();
        let ctx = ToolCtx::new(sesion, Some(pool.clone()));
        h.execute(
            "registrar_contacto",
            &json!({"nombre": "Dedup Foto", "telefono": "+34655555555"}),
            &ctx,
        )
        .await
        .unwrap();
        let id: String = sqlx::query_scalar(
            "SELECT id::TEXT FROM inmuebles WHERE publicado \
             AND (SELECT COUNT(*) FROM fotos WHERE inmueble_id = inmuebles.id) > 0 LIMIT 1",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        let primera = h
            .execute("enviar_fotos_inmueble", &json!({"id": id, "max": 2}), &ctx)
            .await
            .unwrap();
        assert_eq!(primera.get("enviadas").and_then(Value::as_i64), Some(2));
        let segunda = h
            .execute("enviar_fotos_inmueble", &json!({"id": id, "max": 2}), &ctx)
            .await
            .unwrap();
        assert_eq!(segunda.get("enviadas").and_then(Value::as_i64), Some(0));
        assert_eq!(segunda.get("repetidas").and_then(Value::as_i64), Some(2));
        let espejos: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM agent_messages WHERE session_id = $1 \
             AND sender = 'ai' AND body LIKE '[foto] %'",
        )
        .bind(sesion)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(espejos, 2);

        sqlx::query("DELETE FROM agent_outbox WHERE payload->>'session_id' = $1")
            .bind(sesion.to_string())
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM agent_messages WHERE session_id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM canal_sesiones WHERE session_id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM atencion_sesiones WHERE session_id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM clientes WHERE telefono = '34655555555'")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM agent_response_cycles WHERE session_id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM agent_sessions WHERE id = $1")
            .bind(sesion)
            .execute(&pool)
            .await
            .unwrap();
    }
}
