use std::future::Future;
use std::pin::Pin;

use serde_json::{json, Value};
use sqlx::PgPool;

use glory_agent::errors::AgentError;
use glory_agent::session::ChatHub;
use glory_agent::tools::{ToolCtx, ToolExecutor};

/* [169A-4] Tools de la inmobiliaria (las ejecuta el loop F6 del núcleo).
 * Salidas compactas: cada token cuenta para la ventana de 30k. La IA nunca
 * inventa datos: precios/direcciones salen de `buscar/detalle`, el teléfono
 * de `datos_contacto` y el humano de `escalar_a_humano`. */

/* [08AA-6] Los schemas del provider viven en `chat_tools_definiciones`
 * (split god-object); se re-exporta `definiciones` para no romper `chat.rs`. */
pub(crate) use super::tools_definiciones::definiciones;

/* [16AA-9 F3] Un modulo por tool (`buscar`, `detalle`, `fotos`) y el espejo
 * con el dedup del hilo (`espejo`); aqui quedan el ejecutor (`Herramientas`)
 * y el dispatcher (`ejecutar`). Mismo criterio que los schemas de
 * `tools_definiciones`: ninguna funcion cambio de cuerpo.
 * Los helpers de test viven en `mod pruebas`, al final: un fichero suelto
 * con `#![cfg(test)]` no lo reconoce Sentinel en `handler-accede-bd-rs` y
 * sus `sqlx::query` contaban como produccion. */
mod buscar;
mod detalle;
mod espejo;
mod fotos;

use buscar::buscar;
use detalle::detalle;
use fotos::enviar_fotos;

/// Executor con acceso a BD y al contacto por defecto (`AGENTE_CONTACTO`).
pub struct Herramientas {
    pool: PgPool,
    contacto_defecto: String,
    /* [299A-4] Hub opcional para espejar en el hilo lo enviado al visitante
     * (tarjeta/foto): `ToolCtx` no trae hub y el núcleo es dependencia
     * externa, así que viaja en el executor que sí construimos nosotros. */
    hub: Option<ChatHub>,
}

impl Herramientas {
    pub fn new(pool: PgPool, contacto_defecto: String) -> Self {
        Self {
            pool,
            contacto_defecto,
            hub: None,
        }
    }

    /// Hub para el espejo 299A-4. Sin hub (tests) se encola igual pero no
    /// se espeja: el envío al visitante nunca depende del espejo.
    pub fn with_hub(mut self, hub: ChatHub) -> Self {
        self.hub = Some(hub);
        self
    }

    fn pool(&self, ctx: &ToolCtx) -> PgPool {
        ctx.pool.clone().unwrap_or_else(|| self.pool.clone())
    }

    async fn ejecutar(&self, name: &str, args: &Value, ctx: &ToolCtx) -> Result<Value, AgentError> {
        let inicio = std::time::Instant::now();
        let pool = self.pool(ctx);
        if deshabilitada(&pool, name).await {
            tracing::warn!(
                "tool {name} sesion={} bloqueada por admin ({} ms)",
                ctx.session_id,
                inicio.elapsed().as_millis()
            );
            return Ok(json!({"error": "herramienta deshabilitada por el administrador"}));
        }
        let salida = match name {
            "buscar_inmuebles" => buscar(&pool, ctx.session_id, args, self.hub.as_ref()).await,
            "detalle_inmueble" => detalle(&pool, args).await,
            "registrar_contacto" => registrar(&pool, ctx.session_id, args).await,
            "enviar_fotos_inmueble" => {
                enviar_fotos(&pool, ctx.session_id, args, self.hub.as_ref()).await
            }
            "registrar_captacion" => captar(&pool, ctx.session_id, args).await,
            "agendar_visita" => agendar(&pool, ctx.session_id, args).await,
            "datos_contacto" => contacto_publico(&pool, &self.contacto_defecto).await,
            "escalar_a_humano" => escalar(&pool, ctx.session_id, args).await,
            "consultar_agente" => consultar(&pool, ctx.session_id, args).await,
            otro => Ok(json!({"error": format!("tool desconocida: {otro}")})),
        };
        match &salida {
            Ok(v) => tracing::info!(
                "tool {name} sesion={} ok ({} ms, {} chars)",
                ctx.session_id,
                inicio.elapsed().as_millis(),
                v.to_string().len()
            ),
            Err(e) => tracing::warn!(
                "tool {name} sesion={} ERROR tras {} ms: {e}",
                ctx.session_id,
                inicio.elapsed().as_millis()
            ),
        }
        salida
    }
}

impl ToolExecutor for Herramientas {
    fn execute<'a>(
        &'a self,
        name: &'a str,
        args: &'a Value,
        ctx: &'a ToolCtx,
    ) -> Pin<Box<dyn Future<Output = Result<Value, AgentError>> + Send + 'a>> {
        Box::pin(async move { self.ejecutar(name, args, ctx).await })
    }
}

/// ¿La apagó el admin en `tools_deshabilitadas` (csv)? Sin config = todas on.
async fn deshabilitada(pool: &PgPool, name: &str) -> bool {
    let csv = glory_agent::persistence::get_config(pool, "tools_deshabilitadas")
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    csv.split(',').map(str::trim).any(|t| t == name)
}

/// Teléfono 6..24 chars de `+0123456789 ()-.` con al menos 6 dígitos.
pub fn telefono_valido(tel: &str) -> bool {
    let t = tel.trim();
    (6..=24).contains(&t.len())
        && t.chars()
            .all(|c| c.is_ascii_digit() || "+ ()-.".contains(c))
        && t.chars().filter(char::is_ascii_digit).count() >= 6
}

/* [08AA-8] Captación/contacto/escalado a humano viven en
 * `chat_tools_captacion.rs` (split god-object); se re-exportan para el
 * dispatcher `Herramientas::ejecutar` y los tests (`super::*`). */
pub use super::tools_captacion::contacto_publico;
/* Solo lo que usan el dispatcher y los tests (`super::*`); los helpers
 * internos (`arg_texto`, `aviso_humano`…) quedan en el módulo hijo. */
pub(super) use super::tools_captacion::{agendar, captar, consultar, escalar, registrar};

/* [08AA-8] `destino_humano`→`agendar` movidos a `chat_tools_captacion.rs`
 * (re-export arriba para dispatcher y tests). */

/* [169A-4] Las consultas SQL no usan macros verificadas en compilación:
 * estos tests las ejecutan contra la BD real de rama (`DATABASE_URL`).
 * Sin `DATABASE_URL` se omiten (gate local sin BD sigue verde). */

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::repositories::{ClienteRepository, VisitaRepository};
    use uuid::Uuid;

    /* [16AA-9 F3] Helpers compartidos con los `mod pruebas` de `buscar` y
     * `fotos` (antes vivian aqui, en el unico `mod pruebas` de `tools.rs`).
     * `pub(super)` los hace visibles desde cualquier submodulo de `tools`. */
    pub(super) fn pool_si_hay() -> Option<PgPool> {
        let url = crate::config::url_bd_pruebas()?;
        sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy(&url)
            .ok()
    }

    pub(super) async fn limpiar_sesion(pool: &PgPool, sesion: Uuid) {
        sqlx::query("DELETE FROM agent_messages WHERE session_id = $1")
            .bind(sesion)
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM agent_sessions WHERE id = $1")
            .bind(sesion)
            .execute(pool)
            .await
            .unwrap();
    }

    /* [E15] Limpieza de los rastros de sesión que dejan estos tests
     * (outbox, mensajes, canal, atención, ciclo y sesión sintética). */
    pub(super) async fn limpiar_rastros(pool: &PgPool, sesion: Uuid) {
        for (sql, texto) in [
            (
                "DELETE FROM agent_outbox WHERE payload->>'session_id' = $1",
                true,
            ),
            ("DELETE FROM agent_messages WHERE session_id = $1", false),
            ("DELETE FROM canal_sesiones WHERE session_id = $1", false),
            ("DELETE FROM atencion_sesiones WHERE session_id = $1", false),
            (
                "DELETE FROM agent_response_cycles WHERE session_id = $1",
                false,
            ),
            ("DELETE FROM agent_sessions WHERE id = $1", false),
        ] {
            if texto {
                sqlx::query(sql)
                    .bind(sesion.to_string())
                    .execute(pool)
                    .await
                    .unwrap();
            } else {
                sqlx::query(sql).bind(sesion).execute(pool).await.unwrap();
            }
        }
    }

    #[test]
    fn telefono_acepta_formatos_reales() {
        assert!(telefono_valido("+34 600 123 456"));
        assert!(telefono_valido("600123456"));
        assert!(telefono_valido("+1 (555) 123-4567"));
        assert!(!telefono_valido("abc"));
        assert!(!telefono_valido("12345"));
        assert!(!telefono_valido(""));
    }

    /* [Fase3-H1] Un número dictado no re-clavea el hilo: la sesión sigue
     * atada al remitente real y el número nuevo queda como ficha en
     * `clientes` (antes partía el hilo y la segunda vuelta perdía el
     * historial). */
    #[tokio::test]
    async fn registrar_no_reclavea_hilo() {
        let Some(pool) = pool_si_hay() else { return };
        let h = Herramientas::new(pool.clone(), "Test 600111222".to_string());
        let sesion = Uuid::new_v4();
        glory_agent::persistence::ensure_session(&pool, sesion)
            .await
            .unwrap();
        let dueno = ClienteRepository::registrar(&pool, Some("Dueno Hilo"), "34111111111")
            .await
            .unwrap();
        ClienteRepository::vincular_canal(
            &pool,
            sesion,
            dueno.id,
            "34111111111",
            "wa_b",
            "inicial",
        )
        .await
        .unwrap();
        let ctx = ToolCtx::new(sesion, Some(pool.clone()));
        let salida = h
            .execute(
                "registrar_contacto",
                &json!({"nombre": "Otro Numero", "telefono": "34222222222"}),
                &ctx,
            )
            .await
            .unwrap();
        assert_eq!(salida.get("ok").and_then(Value::as_bool), Some(true));
        let canal: (String, Uuid) =
            sqlx::query_as("SELECT telefono, cliente_id FROM canal_sesiones WHERE session_id = $1")
                .bind(sesion)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(canal.0, "34111111111");
        assert_eq!(canal.1, dueno.id);
        let ficha: (String,) =
            sqlx::query_as("SELECT nombre FROM clientes WHERE telefono = '34222222222'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(ficha.0, "Otro Numero");
        sqlx::query("DELETE FROM clientes WHERE telefono IN ('34111111111','34222222222')")
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
        limpiar_sesion(&pool, sesion).await;
    }

    #[tokio::test]
    async fn tools_consultan_esquema_real() {
        let Some(pool) = pool_si_hay() else { return };
        let h = Herramientas::new(pool.clone(), "Test 600111222".to_string());
        let sesion = Uuid::new_v4();
        glory_agent::persistence::ensure_session(&pool, sesion)
            .await
            .unwrap();
        let ctx = ToolCtx::new(sesion, Some(pool.clone()));

        let lista = h
            .execute("buscar_inmuebles", &json!({}), &ctx)
            .await
            .unwrap();
        assert!(lista.get("inmuebles").and_then(Value::as_array).is_some());
        let mala = h
            .execute("buscar_inmuebles", &json!({"tipo": "castillo"}), &ctx)
            .await
            .unwrap();
        assert!(mala.get("error").is_some());
        let ausente = h
            .execute(
                "detalle_inmueble",
                &json!({"id": Uuid::new_v4().to_string()}),
                &ctx,
            )
            .await
            .unwrap();
        assert!(ausente.get("error").is_some());

        sqlx::query("DELETE FROM agent_messages WHERE session_id = $1")
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

    #[tokio::test]
    async fn contacto_y_escalado_persisten() {
        let Some(pool) = pool_si_hay() else { return };
        let h = Herramientas::new(pool.clone(), "Test 600111222".to_string());
        let sesion = Uuid::new_v4();
        glory_agent::persistence::ensure_session(&pool, sesion)
            .await
            .unwrap();
        let ctx = ToolCtx::new(sesion, Some(pool.clone()));

        let r = h
            .execute(
                "registrar_contacto",
                &json!({"nombre": "Humo Test", "telefono": "+34611111111"}),
                &ctx,
            )
            .await
            .unwrap();
        assert_eq!(r.get("ok").and_then(Value::as_bool), Some(true));
        /* [279A-2 F1] La tool también deja la fila en `clientes`. */
        let cliente: String =
            sqlx::query_scalar("SELECT telefono FROM clientes WHERE telefono = '34611111111'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(cliente, "34611111111");
        let datos = contacto_publico(&pool, "Test 600111222").await.unwrap();
        assert!(datos.get("whatsapp_url").and_then(Value::as_str).is_some());
        let esc = h
            .execute("escalar_a_humano", &json!({"motivo": "prueba humo"}), &ctx)
            .await
            .unwrap();
        assert_eq!(esc.get("ok").and_then(Value::as_bool), Some(true));
        let estado = glory_agent::persistence::get_session(&pool, sesion)
            .await
            .unwrap()
            .unwrap()
            .status;
        assert_eq!(estado, "escalated");
        let pendientes: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM agent_outbox WHERE status = 'pending' \
             AND kind = 'whatsapp' AND payload->>'session_id' = $1",
        )
        .bind(sesion.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(pendientes, 1);

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
        sqlx::query("DELETE FROM clientes WHERE telefono = '34611111111'")
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

    /* [279A-2 F3] Matriz mínima: consultar congela (`consultando` +
     * `ai_enabled=false` + ciclo `waiting` + aviso con resumen) y escalar
     * delega (`delegada` + triple freno). */
    #[tokio::test]
    async fn consultar_congela_y_escalar_delega() {
        let Some(pool) = pool_si_hay() else { return };
        let h = Herramientas::new(pool.clone(), "Test 600111222".to_string());
        let sesion = Uuid::new_v4();
        glory_agent::persistence::ensure_session(&pool, sesion)
            .await
            .unwrap();
        let ctx = ToolCtx::new(sesion, Some(pool.clone()));

        let c = h
            .execute(
                "consultar_agente",
                &json!({"motivo": "duda precio", "resumen": "pregunta margen"}),
                &ctx,
            )
            .await
            .unwrap();
        assert_eq!(c.get("ok").and_then(Value::as_bool), Some(true));
        let fila = glory_agent::persistence::get_session(&pool, sesion)
            .await
            .unwrap()
            .unwrap();
        assert!(!fila.ai_enabled);
        let estado_at: String =
            sqlx::query_scalar("SELECT estado FROM atencion_sesiones WHERE session_id = $1")
                .bind(sesion)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(estado_at, "consultando");
        let ciclo: String =
            sqlx::query_scalar("SELECT status FROM agent_response_cycles WHERE session_id = $1")
                .bind(sesion)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(ciclo, "waiting");

        let e = h
            .execute(
                "escalar_a_humano",
                &json!({"motivo": "pide humano", "resumen": "quiere visita"}),
                &ctx,
            )
            .await
            .unwrap();
        assert_eq!(e.get("ok").and_then(Value::as_bool), Some(true));
        let fila2 = glory_agent::persistence::get_session(&pool, sesion)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(fila2.status, "escalated");
        assert!(!fila2.ai_enabled);
        let estado_at2: String =
            sqlx::query_scalar("SELECT estado FROM atencion_sesiones WHERE session_id = $1")
                .bind(sesion)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(estado_at2, "delegada");

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

    /* [E14] `registrar_captacion`: valida args, crea la solicitud con el
     * servicio web (origen `whatsapp`, `pendiente`), marca `captacion`
     * y encola el aviso al captador. */
    #[tokio::test]
    async fn captacion_registra_y_avisa() {
        let Some(pool) = pool_si_hay() else { return };
        let h = Herramientas::new(pool.clone(), "Test 600111222".to_string());
        let sesion = Uuid::new_v4();
        glory_agent::persistence::ensure_session(&pool, sesion)
            .await
            .unwrap();
        let ctx = ToolCtx::new(sesion, Some(pool.clone()));

        let mala = h
            .execute(
                "registrar_captacion",
                &json!({"nombre": "Vende Casas", "telefono": "abc"}),
                &ctx,
            )
            .await
            .unwrap();
        assert!(mala.get("error").is_some());

        let r = h
            .execute(
                "registrar_captacion",
                &json!({
                    "nombre": "Vende Casas",
                    "telefono": "+34633333333",
                    "operacion": "venta",
                    "ubicacion": "Chacao",
                    "descripcion": "Apartamento 80m2, 2 hab",
                    "precio_estimado": 95000.0
                }),
                &ctx,
            )
            .await
            .unwrap();
        assert_eq!(r.get("ok").and_then(Value::as_bool), Some(true));
        let sid = r
            .get("solicitud_id")
            .and_then(Value::as_str)
            .expect("solicitud_id");
        let fila: (String, String) =
            sqlx::query_as("SELECT estado, origen_contacto FROM solicitudes WHERE id = $1::UUID")
                .bind(sid)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(fila, ("pendiente".to_string(), "whatsapp".to_string()));
        let estado: String =
            sqlx::query_scalar("SELECT estado FROM atencion_sesiones WHERE session_id = $1")
                .bind(sesion)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(estado, "captacion");
        let avisos: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM agent_outbox WHERE status = 'pending' \
             AND kind = 'whatsapp' AND payload->>'session_id' = $1 \
             AND payload->>'motivo' = 'captacion'",
        )
        .bind(sesion.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(avisos, 1);

        sqlx::query("DELETE FROM solicitudes WHERE id = $1::UUID")
            .bind(sid)
            .execute(&pool)
            .await
            .unwrap();
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

    /* [E15] Matriz mínima de visitas: rechazo de args malos sin tocar
     * la BD; agenda feliz (fila `pendiente` + congelar como `consultar`
     * + aviso `visita`) y confirmación del admin con fecha. */
    /* [E15] `agendar_visita` rechaza args malos sin tocar la BD (salvo el
     * uuid inexistente, que solo lee): la IA se corrige en el turno. */
    #[tokio::test]
    async fn visita_rechaza_args_malos() {
        let Some(pool) = pool_si_hay() else { return };
        let h = Herramientas::new(pool.clone(), "Test 600111222".to_string());
        let sesion = Uuid::new_v4();
        glory_agent::persistence::ensure_session(&pool, sesion)
            .await
            .unwrap();
        let ctx = ToolCtx::new(sesion, Some(pool.clone()));
        let id = Uuid::new_v4().to_string();

        let mala = h
            .execute("agendar_visita", &json!({"id": "no-uuid"}), &ctx)
            .await;
        assert!(mala.is_err());
        for args in [
            json!({"id": id, "telefono": "+34633333333", "cuando": "hoy"}),
            json!({"id": id, "nombre": "T", "telefono": "abc", "cuando": "hoy"}),
            json!({"id": id, "nombre": "T", "telefono": "+34633333333"}),
            json!({"id": id, "nombre": "T", "telefono": "+34633333333", "cuando": "hoy", "fecha": "ayer"}),
        ] {
            let r = h.execute("agendar_visita", &args, &ctx).await.unwrap();
            assert!(r.get("error").is_some(), "args: {args}");
        }
        let ausente = h
            .execute(
                "agendar_visita",
                &json!({
                    "id": id,
                    "nombre": "Nadie",
                    "telefono": "+34633333333",
                    "cuando": "mañana"
                }),
                &ctx,
            )
            .await
            .unwrap();
        assert!(ausente.get("error").is_some());

        limpiar_rastros(&pool, sesion).await;
    }

    /* [E15] Matriz mínima feliz: abre la fila en `pendiente`, congela
     * como `consultar` y avisa con motivo `visita`; el admin confirma
     * con fecha desde el repo. */
    #[tokio::test]
    async fn visita_agenda_y_avisa() {
        let Some(pool) = pool_si_hay() else { return };
        let h = Herramientas::new(pool.clone(), "Test 600111222".to_string());
        let sesion = Uuid::new_v4();
        glory_agent::persistence::ensure_session(&pool, sesion)
            .await
            .unwrap();
        let ctx = ToolCtx::new(sesion, Some(pool.clone()));

        /* Inmueble semilla publicado (solo lectura; no se toca). */
        let Some(id): Option<String> =
            sqlx::query_scalar("SELECT id::TEXT FROM inmuebles WHERE publicado LIMIT 1")
                .fetch_optional(&pool)
                .await
                .unwrap()
        else {
            limpiar_rastros(&pool, sesion).await;
            return;
        };

        let r = h
            .execute(
                "agendar_visita",
                &json!({
                    "id": id,
                    "nombre": "Visita Test",
                    "telefono": "+34633333333",
                    "cuando": "el sábado en la mañana"
                }),
                &ctx,
            )
            .await
            .unwrap();
        assert_eq!(r.get("ok").and_then(Value::as_bool), Some(true));
        let vid = r
            .get("visita_id")
            .and_then(Value::as_str)
            .expect("visita_id");
        let fila: (String, String, Option<chrono::NaiveDate>) =
            sqlx::query_as("SELECT estado, cuando, fecha FROM visitas WHERE id = $1::UUID")
                .bind(vid)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(fila.0, "pendiente".to_string());
        assert_eq!(fila.1, "el sábado en la mañana".to_string());
        assert_eq!(fila.2, None);
        let estado: String =
            sqlx::query_scalar("SELECT estado FROM atencion_sesiones WHERE session_id = $1")
                .bind(sesion)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(estado, "consultando");
        let fila_sesion = glory_agent::persistence::get_session(&pool, sesion)
            .await
            .unwrap()
            .unwrap();
        assert!(!fila_sesion.ai_enabled);
        let avisos: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM agent_outbox WHERE status = 'pending' \
             AND kind = 'whatsapp' AND payload->>'session_id' = $1 \
             AND payload->>'motivo' = 'visita'",
        )
        .bind(sesion.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(avisos, 1);
        /* El admin confirma con fecha: cierra el ciclo de la cita. */
        let conf = VisitaRepository::cambiar_estado(
            &pool,
            vid.parse().unwrap(),
            "confirmada",
            Some(chrono::NaiveDate::from_ymd_opt(2026, 10, 4).unwrap()),
        )
        .await
        .unwrap()
        .expect("visita confirmada");
        assert_eq!(conf.estado, "confirmada");
        assert_eq!(
            conf.fecha,
            Some(chrono::NaiveDate::from_ymd_opt(2026, 10, 4).unwrap())
        );

        sqlx::query("DELETE FROM visitas WHERE id = $1::UUID")
            .bind(vid)
            .execute(&pool)
            .await
            .unwrap();
        limpiar_rastros(&pool, sesion).await;
    }
}
