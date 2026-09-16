# Plan chat visitante con IA (F5) — 2026-09-16

Tarea: `169A-1`. Estado: activo. Repos: `MN-Inmobiliaria` (rama
`inmobiliaria`, backend) + `INMOBILIARIA` (rama `inmobiliaria`, front).

## 1. Objetivo

Cablear el crate `glory-agent` (F0–F4-live verificados) como chat del
visitante: backend expone `/api/agent/*`, front monta un widget (burbuja en
esquina → ventana) en la página pública. Sin `tools` en v1: el transporte
de `glory-agent` envía definiciones pero no ejecuta tools (no hay hook de
ejecución); registrar definiciones sin ejecutor dejaría al modelo esperando
una `function_call` que nunca se resuelve. La ejecución de tools es F6 en el
repo `glory-agent`.

## 2. Alcance / no alcance

- Sí: dependencia por path, migración `agent_*`, `AgentState` + rutas
  anidadas con estado propio, `PromptConfig` inmobiliario (contacto por
  `AGENTE_CONTACTO`), widget `publica/` (componente ≤300 líneas, hook
  ≤120) con tokens de `disenno.ts`, smoke E2E real.
- No: tools con ejecución, panel staff, streaming token-a-token,
  deploy/Coolify, cambios en `NAKOMI` (solo referencia leída).

## 3. Fases verificables

1. Backend wiring: `glory-agent = { path }` en `Cargo.toml`, migración
   `00005_agent_chat` (up/down, tablas `agent_*` idénticas a
   `glory-agent/migrations/0001_chat.sql`), módulo `handlers/chat.rs`
   (`PromptConfig` + `AgentState::new(...).with_pool(...)` +
   `glory_agent::transport::routes().with_state(...)` anidado en `/api`),
   `ProviderConfig::opencode_go` con `OPENCODE_GO_API_KEY` opcional
   (vacía = degradado probado: persiste + realtime, IA `None`).
   Verificación: `fmt --check`, `check`, `clippy --all-targets -- -D
   warnings`, `test`, `sqlx migrate run` + `POST /api/agent/messages`
   contra backend vivo (scripts/run-with-db.mjs).
2. Front widget: `src/features/publica/chat-visitante.tsx` (burbuja →
   ventana, cuadrada, sin sombras, `#e8e7e3/#050200/#dddbd5`,
   `texto-publica`), `src/hooks/publica/use-chat-visitante.ts` (WS
   `/api/agent/ws?session_id=`, `session_id` en
   `localStorage('inmobiliaria:chat-sesion')`, fallback REST
   `/api/agent/messages`, historial `/api/agent/history`), montado en
   `PaginaPublica`. Verificación: `tsc -b`, `oxlint`, `vite build`.
3. Cierre: commits por repo (`169A-1: ...`), completadas
   `Agente/completados/tareas-2026-09-16.md` en ambos repos, retirar
   169A-1 del roadmap.

## 4. Definition of Done

- `cargo test` MN verde + migración aplicada + smoke REST/WS real OK.
- `tsc` + `build` INMOBILIARIA verdes + widget visible esquina pública.
- Sin secretos en repo ni logs; `OPENCODE_GO_API_KEY` solo server.
- F6 (ejecución de tools) registrado en `glory-agent/roadmap.md`.
