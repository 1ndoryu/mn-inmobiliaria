# Plan: chat con atención humana + WhatsApp + config admin (2026-09-16)

> **Origen:** el usuario pide escalación como Nakomi (aviso WhatsApp por
> mensaje), panel admin para atender mensajes, config detallada del chat
> (prompt, tools), IA que consulta inmuebles reales, sabe escalar, da el
> teléfono de la inmobiliaria y captura nombre/teléfono del cliente.
> Referencia solo-lectura: `NAKOMI/src/services/{ai_chat,ai_tools,
> chat_alert,chat_alert_worker,whatsapp_gateway}.rs`.

## Hallazgos del diseño (2026-09-16)

1. La base ya existe: `agent_sessions (status open/escalated/closed,
   ai_enabled, visitor_name, contact)`, `agent_messages (…sender
   client/ai/staff/system…)`, `agent_response_cycles (…escalated)`,
   `agent_outbox (kind/payload/pending|sent|failed)`, `enqueue_outbox`,
   `fetch_pending_outbox`, `mark_outbox`.
2. Huecos reales: el transporte **nunca ejecuta** `function_call` (F6
   pendiente); `should_answer_with_ai` existe pero **nadie la llama** y
   `ai_enabled` no se hace cumplir → la IA respondería aunque un humano
   tome el hilo. Sin tabla de config, sin endpoints staff, sin worker.
3. `ChatHub` es `Clone` con interiores `Arc`: un mismo hub puede
   compartirse entre el router visitante (`AgentState`) y las rutas staff
   (`AppState.hub`), así el staff emite por el mismo WS del visitante.
4. Auth staff: `AuthUser: FromRequestParts<AppState>`; por eso las rutas
   staff viven en el router `AppState` (`/api/admin/agent/…`), no en el
   router visitante (`Router<()>`). Sin mezclar estados.
5. Sin `async-trait` en `glory-agent`: el trait ejecutor usa futuro en
   caja, sin deps nuevas.

## Fases

### 169A-3 — glory-agent: loop de tools + gate IA + persistencia (F6)
Núcleo agnóstico, sin lógica de inmobiliaria.
- `tools`: `ToolCtx { session_id, pool }` + trait `ToolExecutor`
  (futuro en caja) + `AgentState::{with_hub,with_executor}`.
- `transport::process_incoming`: gate real (con pool: `ensure_session` →
  `ai_enabled` + `should_answer_with_ai`; si humano manda, persiste +
  reenvía pero `reply: None`) + loop tools (máx 3 vueltas: provider →
  `parse_responses_output` → ejecutar calls registradas vía executor →
  anexar `function_call_output` → repetir; texto final persiste + broadcast).
- `persistence`: `get_session`, `list_sessions`, `set_session_status`,
  `set_session_contact`, `set_session_ai`, `get_config`, `set_config` +
  migración `0002_config.sql` (`agent_config(key PK, value, updated_at)`).
- Claves de config del núcleo: `prompt_extra` (se anexa al system),
  `ai_enabled_global` (`on|off`, kill-switch).
- Tests unitarios (fake executor, gate `ai_may_answer`, config).
- DoD: `fmt --check && check && clippy --all-targets -- -D warnings
  && test` verdes; commit en repo `glory-agent`.

### 169A-4 — MN backend: tools inmobiliarias + escalación + WhatsApp + staff (F7)
- Executor con 5 tools (schemas cortos): `buscar_inmuebles`
  (solo publicados, top-N compacto), `detalle_inmueble` (por id),
  `registrar_contacto` (nombre+teléfono → sesión), `datos_contacto`
  (devuelve teléfono/WhatsApp de la config: la IA nunca inventa el número),
  `escalar_a_humano` (motivo → `status=escalated` + ciclo `escalated` +
  `enqueue_outbox(kind='whatsapp', …)`; responde texto con el teléfono).
- Prompt: regla "2 respuestas sin resolver o petición de humano → llama
  `escalar_a_humano`"; `AGENTE_CONTACTO` como default, sobreescribible por
  config `contacto_telefono`.
- Worker WhatsApp (tokio task en `main`): poll outbox `kind='whatsapp'`
  cada 15 s → `POST GLORY_ALERT_GATEWAY_URL` con {destino: config
  `whatsapp_admin`, texto}; `sent|failed`; sin gateway: warn + queda
  `pending` visible (nunca silencio).
- Rutas staff `handlers/chat_staff.rs` (`Router<AppState>`, `AuthUser`):
  `GET /api/admin/agent/sesiones?estado=&limit=`,
  `GET /api/admin/agent/sesiones/:id/historial`,
  `POST /api/admin/agent/sesiones/:id/mensajes` (staff + broadcast por
  `AppState.hub`), `PATCH …/:id` (`aiEnabled|status`),
  `GET|PUT /api/admin/agent/config`.
- Públicas: `GET /api/agent/info` (teléfono, WhatsApp, IA on/off),
  `POST /api/agent/sesiones/:id/contacto` (nombre+teléfono, validado).
- `AppState.hub: ChatHub`; `agent_router(pool, hub)` comparte el hub.
- Migración MN que aplica `0002` del núcleo + seeds de config.
- Requiere del usuario: `GLORY_ALERT_GATEWAY_URL` + número admin
  (`whatsapp_admin`); `OPENCODE_GO_API_KEY` + `AGENTE_CONTACTO` siguen
  pendientes de 169A-1.
- DoD: gate backend verde + humo en rama (sesiones, staff con JWT,
  outbox) ; commit en `MN-Inmobiliaria`.
- Hecho 2026-09-16: implementado + gate verde + humo BD rama
  (`tests/chat_humo.rs` 2 OK, 4 tests SQL contra esquema real OK).
  Sin seeds (claves opcionales con defaults). Envio real al gateway
  pendiente de `GLORY_ALERT_GATEWAY_URL` (worker marca `failed`, verificado).

### 169A-5 — INMOBILIARIA admin: Mensajes + Config chat (F8)
- Vista `mensajes` en admin (`Layout`/`VistaApp`): lista de sesiones
  (estado, nombre, último mensaje, alerta pendiente) + hilo + responder
  (mismo WS con `session_id`, rol staff) + botones Tomar IA / Soltar IA /
  Cerrar + badge de alertas WhatsApp pendientes.
- Vista `config-chat`: `prompt_extra`, `contacto_telefono`,
  `whatsapp_admin`, kill-switch IA, `tools_deshabilitadas` (csv). Con
  token (`leerToken`), errores `ok:false` visibles.
- DoD: `oxlint` 0 + `build` ok + verificación visual 2 resoluciones;
  commit en `INMOBILIARIA`.

### 169A-6 — Widget: contacto + escalado visible + E2E y cierre (F9)
- Widget: banner al escalar (teléfono + botón WhatsApp `wa.me` con texto
  precargado), mini-formulario "Dejar mis datos" (nombre+teléfono →
  endpoint contacto), muestra `iaHabilitada=false` como "fuera de
  horario, te atenderá un humano".
- E2E en rama: pregunta por pisos → tool `buscar_inmuebles` responde con
  los 5 reales; "quiero humano" → escalado + outbox `pending`; staff
  responde → llega al WS visitante; sin `OPENCODE_GO_API_KEY` degradado
  ya verificado en 169A-1.
- Cierre: completadas en los 3 repos, roadmap limpio, lecciones si aplica.
- DoD: flujo verificado contra backend real + commits + push por repo.

## No alcance
- Sin streaming token-a-token (v1 ya decidido). Sin multi-instancia
  (hub in-memory). Sin adjuntos en el chat. Sin perms finos (owner/admin
  atienden igual). Sin reintentos con backoff del worker (poll 15 s basta).
