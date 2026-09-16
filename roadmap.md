Objetivo: crear un template de Rust + Ts React + OpenAPI + Codegen + Clippy nivel paranoia, para crear sitios web, un solo repositorio, con una pagina sencilla de ejemplo, con el gitignore configurado y todo listo para empezar a desarrollar, adelante a todo, comandos, documentación, he creado https://github.com/1ndoryu/glory-rs para lo subas alli, se tiene que pdoer crear varias ramas para varios sitios y que no haya problema de cambiar a un sitio a otro cambiando de rama, aplica todo lo que por defecto vendría siendo optimización, seguridad, rendimiento, experiencia de desarrolo, etc, de base de datos tengo corriendo postgres aca localmente.

## Estado: ✅ Template base completado (253A-1)

Ver `Agente/completados/tareas-2026-03-25.md` para detalles.

## Stack implementado

| Capa | Herramienta |
|------|-------------|
| Framework web | Axum 0.7 |
| OpenAPI | utoipa 4 + utoipa-swagger-ui 7 |
| Serialización | serde |
| Base de datos | SQLx 0.8 (PostgreSQL) |
| Migraciones | SQLx migrate |
| Validación | validator 0.18 |
| Variables de entorno | dotenvy |
| Logging | tracing + tracing-subscriber |
| Errores | thiserror 2 |
| Auth | jsonwebtoken + argon2 |
| CORS | tower-http |
| Linter | clippy (deny all + warn pedantic) |
| Frontend | React 18 + TypeScript + Vite |
| State | React Query + Zustand |
| Codegen | Orval 8 (reemplaza openapi-typescript-codegen) |

## Pendientes

*(Agregar nuevas tareas aquí)*

## Notas

- Configurar coolify-manager-rs para desplegar proyectos Rust (repo separado)
- Prioridades: 1. Velocidad desarrollo, 2. Decisiones futuras, 3. Rendimiento, 4. Seguridad, 5. Popularidad, 6. Facilidad, 7. Docs, 8. Compatibilidad, 9. Flexibilidad, 10. Escalabilidad

## F5 chat visitante con IA (169A-1, en curso 2026-09-16)

- Plan: `Agente/planes/plan-chat-inmobiliaria-2026-09-16.md`.
- Backend: `glory-agent` por path, migracion `agent_*`, rutas
  `/api/agent/*` (WS + REST + historial), PromptConfig inmobiliario.
- Front (`INMOBILIARIA`, repo hermano): widget en `src/features/publica/`
  montado en `PaginaPublica`, tokens de `disenno.ts` (sin redondeados,
  sin sombras, Söhne).
- Alcance v1: sin tools (la ejecucion de tools es F6 en `glory-agent`).
- Requiere del usuario: fijar `OPENCODE_GO_API_KEY` y `AGENTE_CONTACTO`
  reales en `MN-Inmobiliaria/.env` (sin key el chat persiste y reenvia
  en realtime pero la IA no responde).

## Chat con atencion humana + WhatsApp + config admin (169A-3..6, 2026-09-16)

- Plan: `Agente/planes/plan-chat-atencion-2026-09-16.md` (diseno cerrado
  contra Nakomi como referencia solo-lectura).
- 169A-3 (repo `glory-agent`): loop de tools F6 + gate IA real
  (`ai_enabled` + ciclo `escalated` se hacen cumplir) + `0002_config.sql`
  + persistencia sesiones/config + tests. Nucleo agnostico.
- 169A-4 (este repo) [x] 2026-09-16: 5 tools (`buscar_inmuebles`,
  `detalle_inmueble`, `registrar_contacto`, `datos_contacto`,
  `escalar_a_humano`) + worker WhatsApp (outbox, poll 15 s) + rutas staff
  `/api/admin/agent/*` (JWT) + publicas `/api/agent/info` y
  `/sesiones/:id/contacto` + hub compartido. Sin seeds: claves opcionales
  con defaults en codigo. Gate verde (fmt/check/clippy/test 11 lib OK);
  humo BD rama OK (tools contra esquema real, staff toma hilo, config
  roundtrip, worker marca `failed` sin gateway, 2 tests HTTP OK).
- 169A-5 (repo `INMOBILIARIA`): admin Mensajes (lista + hilo + responder
  + tomar/soltar IA) y Config chat (prompt_extra, telefonos, kill-switch).
- 169A-6 (repo `INMOBILIARIA`): widget con banner escalado, WhatsApp
  `wa.me`, form nombre/telefono + E2E en rama y cierre documental.
- Requiere del usuario: `GLORY_ALERT_GATEWAY_URL` + numero admin
  (`whatsapp_admin`); `OPENCODE_GO_API_KEY` + `AGENTE_CONTACTO` siguen
  pendientes de 169A-1.

