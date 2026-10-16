# Revisión de la IA de atención al cliente — MN-Inmobiliaria

> **Fecha:** 2026-10-16 · **Tipo:** revisión (solo lectura, sin cambios de código)
> **Pregunta de la usuaria:** ¿está todo centralizado? ¿el mismo patrón con el
> que se atiende en Marketplace es el mismo en WhatsApp y en la página?
> **Método:** lectura de código en `MN-Inmobiliaria/src`, `frontend/src`,
> `gateway/src` y el crate pineado `glory-agent` (rev `8560269`). Todo dato de
> este documento está verificado a mano; lo inferido va marcado.

---

## 1. Respuesta corta

**Hay un solo motor para atender clientes, y web + WhatsApp ya lo comparten.**
El motor es el crate externo `glory-agent`
(`Cargo.toml:63`, rev `856026948d18bc17d6b68ebc4cfdddbc9e90ffca`), y el punto
único de entrada es `glory_agent::transport::responder_turno_persistido`
(`glory-agent/src/transport/mod.rs:351`).

- **Chat web del visitante** → `process_incoming` → `responder_turno_persistido`
  (`glory-agent/src/transport/mod.rs:445`).
- **WhatsApp** → `src/services/turno.rs:366` → `responder_turno_persistido`.
  El propio comentario del código lo dice: *«la IA (núcleo
  `responder_turno_persistido`, misma vía que el chat web)»* (`turno.rs:322`).

Los dos comparten además el **mismo `AgentState`, el mismo prompt y el mismo
juego de tools**, porque se montan en un único router: `chat::agent_router`
(`src/handlers/chat.rs:181-201`) crea un `AgentState` con
`prompt_config()` (`chat.rs:98`) y el registro de 9 tools
(`tools_definiciones.rs:12`), y le cuelga el webhook de WhatsApp con
`merge(whatsapp::whatsapp_routes())` (`chat.rs:199`).

**Lo que NO es el mismo patrón es el asistente de Marketplace**, que es un
producto distinto (§4). Y no lo es por diseño, no por descuido: el asistente
de Marketplace **no atiende clientes**, le redacta borradores a la dueña y
**el envío siempre es humano** (`Agente/planes/plan-asistente-marketplace-separacion-2026-10-03.md:14`).

---

## 2. Mapa de superficies

### 2.1 Las tres cosas que llevan IA

| # | Producto | ¿Atiende al cliente? | Motor | Dónde vive |
|---|---|---|---|---|
| A | **Chat de atención** (web + WhatsApp) | Sí, autónomo | `glory-agent` (bucle de tools) | crate + `src/handlers/chat`, `src/services/{triage,turno,politica,sesion}` |
| B | **Asistente Marketplace** (borradores para Messenger de Facebook) | No: redacta, la dueña envía | Llamada única sin tools | `opencode-propio` (la float) + `src/handlers/marketplace` como backend |
| C | **Centro de IA de texto / copy de inmuebles** | No: herramienta del panel | Llamada única sin tools | `src/handlers/ia/rutas.rs` + `frontend/.../pestana-ia.tsx`, `modal-ia.tsx` |

Solo **A** es atención al cliente. B y C son herramientas internas que usan IA.

### 2.2 Lo que parece IA y no lo es

- **`/ask`** (`frontend/src/features/ask/**`, `src/handlers/ia/ask.rs`) **no es
  un chat**: es un cuestionario privado de la dueña para rellenar `extras` y
  `precio_minimo` de cada ficha (`pagina-ask.tsx:37-41`,
  `ask.rs:16-38`, ruta `GET /api/admin/inmuebles/{id}/ficha`). Es **entrada**
  de la IA, no un canal de atención.
- **`gateway/`** es transporte puro: Baileys entra y sale, cero IA
  (`gateway/src/index.mjs`, `backend.mjs:4`, `servidor.mjs:86`).
- **`src/services/mensajeria/**`** es entrega y alertas, no IA
  (`alerta_whatsapp`, `canal_resolver`, `transporte`, `outbox_idempotency`).

### 2.3 Los cuatro caminos de entrada

| Canal | Ruta | Fichero |
|---|---|---|
| Chat web (widget del sitio) | `WS /api/agent/ws`, `POST /api/agent/messages`, `GET /api/agent/history` | `glory-agent/src/transport/mod.rs:124-126` |
| WhatsApp | `POST /api/agent/whatsapp/webhook` | `src/handlers/whatsapp.rs:94`, `whatsapp.rs:187-189` |
| Panel de la dueña (bandeja) | `/api/admin/agent/**` | `src/handlers/chat/staff.rs`, `staff_envio.rs`, `staff_config.rs` |
| Asistente Marketplace | `/api/admin/mp/**` (JWT `mp` atado a máquina) | `src/handlers/marketplace/**` |

---

## 3. El motor central: qué hace y qué no

`glory-agent` (3.727 líneas de `src/`) aporta:

| Pieza | Fichero | Qué da |
|---|---|---|
| Transporte y bucle | `src/transport/mod.rs:351` | Prompt + historial + loop de tools + persistencia + broadcast WS |
| Bucle de tools | `src/transport/mod.rs:407`, `src/tools/mod.rs:112` | Máximo **3 vueltas** (`MAX_TOOL_ITERATIONS = 3`) |
| Proveedores | `src/providers/mod.rs` | `ProviderConfig`, `build_responses_body`, `call_provider`, `extract_first_text` |
| Prompt | `src/prompts/mod.rs` | `PromptConfig` (base + reglas) |
| Contexto | `src/context/mod.rs:7` | Ventana de contexto, truncado, `MAX_CONTEXT_TOKENS = 30_000` |
| Persistencia | `src/persistence/**` | `agent_sessions`, `agent_messages`, `agent_response_cycles`, `agent_outbox`, `agent_config`, `agent_eventos` |
| Canales | `src/channels/**` | `Ingress`/`Sender`/`MediaRef`/`Outbound`, adaptadores, anti-eco, presupuestos |
| Handoff | `src/handoff.rs` | Máquina `activa → consultando → delegada` + auditoría |
| Timing | `src/timing/mod.rs` | Presupuesto por remitente (`check_budget`) |

**Lo que NO hace el crate** (y por eso MN lo pone): el prompt de negocio
(`chat.rs:98`), las 9 tools de inmuebles (`tools_definiciones.rs:12`), el
ejecutor de tools (`tools.rs:32`), la decisión de atender (`triage.rs`,
`politica.rs`) y las tablas de negocio.

**Detalle del turno de WhatsApp** (`turno.rs:319-383`), que es el camino con
más lógica propia: webhook responde 2xx rápido → describe la foto y transcribe
el audio en background (`turno.rs:361-366`) → `responder_turno_persistido` →
parte el texto en mensajes, escala o cae al fallback
(`turno.rs:437-509`) → outbox `whatsapp` con el `via` del canal.

---

## 4. El asistente de Marketplace (producto B)

**Qué es.** Un panel flotante dentro de la bandeja de vendedor de Facebook
Marketplace que genera **una sola opción** de respuesta, con el precio del
aviso, para que la dueña la copie a mano. Cero red hacia Meta, sin auto-envío,
sin credenciales (`Agente/documentacion/integraciones/asistente-marketplace-mp-float-2026-09-26.md:9-13`).

**Dónde vive el código.** En `opencode-propio`
(`src/packages/desktop/src/main/marketplace-*`), en migración a
`plugins-opencode` (plan `03AA-3`, en curso). MN es el **backend** que sirve
catálogo, token, borrador, caché, auditoría y panel.

**Cómo genera.** `generar_borrador` (`src/handlers/marketplace/borrador.rs:442`)
construye su propio system prompt —`sistema_borrador` / `sistema_continuacion`,
`borrador.rs:455-459`— y llama a `ia::rutas::completar_opencode_rapido`
(`borrador.rs:470`), **una sola llamada, sin tools y sin bucle de historial**
(solo el `excerpt` del chat y lo ya dicho). Tiene su propio fallback
(`borrador.rs:476-491`) y su propio control de coste (`Coste`, `borrador.rs:495`).

**Por qué es distinto y está bien que lo sea.** No atiende al cliente: asiste
a la dueña. No necesita tools de catálogo (el precio viaja en el prompt), ni
handoff, ni sesión por cliente×canal, ni bucle agentico. Forzarlo al motor de
atención sería un error de diseño.

**Lo que sí es deuda aquí** (no el motor, la infraestructura):
- Prompt propio fuera del sistema de prompts del crate.
- Su vía de llamada (`completar_opencode_rapido`, `ia_proveedores.rs:402`) es
  una rama aparte de `glory_agent::providers::call_provider`.
- Tablas propias: `mp_tokens_emitidos`, `mp_uso_minuto`, `mp_auditoria`,
  `mp_respuestas_cache`, `mp_respuestas_inmueble`, `mp_chats_archivados`.

---

## 5. Centralización: qué está unificado y qué está dividido

### 5.1 Unificado ✅

| Aspecto | Dónde |
|---|---|
| Motor conversacional (prompt, historial, bucle, persistencia) | `glory_agent::transport::responder_turno_persistido` |
| `AgentState`, prompt y tools de web y WhatsApp | `chat.rs:181-201` (un router) |
| Capa de proveedor | `glory_agent::providers::*`, reusada en `ia_proveedores.rs:61,209,341` |
| Handoff humano (tomar/devolver/cerrar) | `glory_agent::handoff` + `src/handlers/chat/staff.rs:193,275` |
| Outbox de salida con idempotencia | `glory_agent::channels::outbox` + `src/services/mensajeria/outbox_idempotency.rs` |
| Decisión de atender | `src/services/triage.rs` (único llamador: `whatsapp.rs:90`) |
| Rol y tono del remitente | `src/services/politica.rs` (`whatsapp.rs:118`, `chat.rs:169`) |
| Sesión cliente×canal | `src/services/sesion.rs` |
| Topes de uso | `src/services/tope_uso.rs` |

### 5.2 Dividido ⚠️

| Aspecto | Estado | Consecuencia |
|---|---|---|
| **Almacén de conversación** | 3 familias de tablas: `agent_*` (crate), `clientes`/`canal_sesiones`/`atencion_sesiones`/`uso_mensajes` (MN), `mp_*` (marketplace) | Un hilo de Marketplace no es un hilo del motor; no hay panel único |
| **Rastreo de uso** | `agent_usage` (crate) y `mp_uso_minuto` (marketplace) | Dos contabilidades de tokens que no se suman |
| **Configuración de IA** | **No hay config central** en `src/config/` (solo env, `config/mod.rs:13-24`). Los topes viven en la tabla `agent_config`; modelo y prompts, dispersos en `chat.rs`, `ia/rutas.rs` y `marketplace/*` | Cambiar el modelo o el tono obliga a tocar varios sitios |
| **Dos mundos de handlers** | `handlers/chat/**` (producto: tools + panel) y `handlers/ia/**` (infra: proveedores + admin). Se tocan por llamada directa | Frontera difusa; `handlers/ia/rutas.rs` (623 líneas) hace de router, config y fachada de proveedor |
| **Modo sombra** | Solo WhatsApp (`sombra.rs`, tras `GLORY_SHADOW=1`) | El chat web no tiene sombra equivalente |

---

## 6. SOLID y acoplamiento

**Lo que está bien:**
- El núcleo conversacional vive **fuera** del producto y se consume por crate
  pineado: inversión de dependencia correcta (`Cargo.toml:63`).
- `ToolExecutor` / `ToolCtx` son traits con futuro en caja
  (`glory-agent/src/tools/mod.rs`): la ejecución de tools se inyecta, no se
  hereda. Cumple OCP y DIP.
- El estado de la app se inyecta por `AppState` (`lib.rs:18-35`), sin globales.
- Los servicios de negocio (`triage`, `politica`, `sesion`, `tope_uso`) son
  funciones puras o casi puras y tienen un solo llamador cada uno.

**Puntos flojos:**
- **`src/handlers/chat/tools.rs` (1.656 líneas)** es un god-object: junta el
  ejecutor (`tools.rs:32`), el dispatcher (`tools.rs:55`), tres tools con SQL
  (`buscar` `:124`, `detalle` `:389`, `enviar_fotos` `:418`) y ~17 tests
  (`:552+`). El patrón de los schemas ya se separó bien
  (`tools_definiciones.rs`); falta el mismo tratamiento para la ejecución.
- **`src/handlers/ia/rutas.rs` (623 líneas)** acumula router, estado de
  proveedores, config admin y fachada de completado.
- **`borrador.rs`** mezcla prompt, llamada, forma de salida, fallback y coste
  en una sola función larga.
- **Alias de compatibilidad muertos** en `src/services/mod.rs:37-67`: una
  docena de `#[allow(unused_imports)] pub(crate) use` que existen solo por la
  reorganización por dominio. Candidatos a borrar.

---

## 7. Optimización y rendimiento

- **Caché de respuestas del marketplace**: `mp_respuestas_cache` con
  `valida_hasta` — firma conocida = 0 tokens
  (`20261006000030_mp_cache.up.sql`, `20261007000031_mp_panel_chat.up.sql:8`).
  Bien. **Riesgo documentado**: caducar por fecha, nunca por texto
  (`Agente/prevencion/prevencion-cache-mp-2026-10-12.md`).
- **Vía rápida sin razonamiento** en el borrador (`09AA-15`), con caída
  automática a la estándar (`ia_proveedores.rs:402-426`). Bien.
- **Sesión estable por hilo** (`sha_hex(clave_hilo(...))`, `borrador.rs:464`)
  para que el relay premie prompt caching. Bien.
- **Historial acotado a 30 turnos** (`HISTORIAL_TURNOS = 30`,
  `glory-agent/src/transport/mod.rs:33`) y ventana de contexto configurable.
  Bien.
- **STT/Visión se pagan por llamada** sin caché propia (`turno.rs:210,277`).
  Una foto reenviada se vuelve a describir.
- **`08AA-10`/`08AA-11`/`08AA-15` abiertos**: el proveedor IA devuelve vacío y
  `/ia/probar` cuelga (roadmap §5). Es el problema de rendimiento más visible
  hoy.

---

## 8. Escalabilidad

**Frenos estructurales (documentados y aceptados):**

| Freno | Dónde | Efecto |
|---|---|---|
| `ChatHub` en memoria | `lib.rs:30`, `glory-agent/src/session/mod.rs` | **Single-instance explícito**: con 2 réplicas, el WS de un visitante y el panel staff caen en procesos distintos y el tiempo real se rompe |
| Singleflight por proceso | `lib.rs:33`, `services/marketplace/vuelo.rs` | Con N réplicas, N generaciones simultáneas para la misma clave |
| Gateway Baileys en el portátil | `gateway/` | El número de WhatsApp depende de una máquina de escritorio y de 3 sesiones QR (`index.mjs`) |
| Turno IA en background sin cola | `turno.rs:319` (`tokio::spawn`) | Sin backpressure real más allá de `check_budget`; se pierde al reiniciar el proceso |
| Acuse a los 6 s | `turno.rs:383` (`ESPERA_ACUSE_MS`) | SLO de latencia atado al proceso vivo |
| Rate-limit por IP | `handlers/mod.rs:315-319` | Correcto hoy; el marketplace añade su propio tope por minuto (`mp_uso_minuto`) |

**Lo que sí escala bien:** el outbox con idempotencia y estados
(`pending/sent/failed`) desacopla la entrega; las tablas de negocio están
indexadas (`uq_agent_messages_canal_externo`,
`uq_agent_outbox_idem`, `idx_agent_eventos_sesion`).

**Conclusión:** el diseño es single-instance **a propósito** y está escrito
así en el plan (`glory-agent/Agente/planes/plan-extraccion-whatsapp-2026-10-01.md:§8`).
Escalar a varias réplicas es un cambio de arquitectura, no un ajuste.

---

## 9. Trabajo pendiente (del roadmap y planes)

| ID | Qué | Estado |
|---|---|---|
| `03AA-3` | Asistente Marketplace: falta la firma de la usuaria para M1, el remoto y la fecha | bloqueado por ella |
| `08AA-10`, `08AA-11`, `08AA-15` | El proveedor IA devuelve vacío; `/ia/probar` cuelga | bloqueado por terceros |
| `199A-1/2` | `GLORY_API_KEY` real en `.env`; sin ella GloryAPI queda `sin-clave` | bloqueado por configuración |
| `09AA-11` | GloryAPI sin reintento (`ia_proveedores.rs:102-153`); `get("uso")` sin productor | pendiente |
| `09AA-8` | Webhook fail-closed (`mensajeria/transporte.rs:95-103` acepta cualquier valor si el esperado está vacío) | decidido «tras 12AA-1» |
| `12AA-1` | Gateway WhatsApp: cierre por plazo, sin base para cerrar | abierta |
| WhatsApp taller `15AA-6` | (a) fotos a `wa_b`/`wa_c` no se archivan solas; (b) `reparto` solo acepta el número A → 400 con destino C; (c) la foto que envía la IA usaría `PUBLIC_BASE_URL` público tras Cloudflare Access: **camino dormido** | pendiente |
| `299A-3` | Dos notas de voz por reproducir (el roadmap cita `turno.rs:511-517`; ese rango ya apunta a otra función tras los refactores) | bloqueado por el proveedor |
| `03AA-5` | Detector de captación: falta el corpus de la usuaria | bloqueado por ella |
| `169A-1`, `169A-3..6` | `AGENTE_CONTACTO`, `GLORY_ALERT_GATEWAY_URL`, `whatsapp_admin` | bloqueado por configuración |
| `wa_c` | Falta escanear el QR (`gateway/qr-wa_c.png`) | acción de la usuaria |
| `F10` (glory-agent) | Plataforma agnóstica: `channels`/`handoff` hechos; `F11` historial al front y `F12` herramienta+skill, solo diseño | en ejecución |

---

## 10. Áreas de mejora y simplificación (priorizadas)

**P1 — Barato y de efecto inmediato**
1. **Borrar los alias muertos** de `src/services/mod.rs:37-67`. Pura resta.
2. **Corregir el comentario obsoleto** de `staff_config.rs:24`: dice que
   `prompt_extra` «sigue guardada sin lector», pero **sí tiene lector** —
   `glory-agent/src/transport/mod.rs:382` la anexa al system prompt. Es la
   única palanca de tono en caliente y hoy parece muerta.
3. **Arreglar la deriva de documentación**: el roadmap cita
   `src/services/transporte.rs` (§2) cuando el fichero es
   `src/services/mensajeria/transporte.rs` (llega por re-export).

**P2 — Estructura, sin cambiar comportamiento**
4. **Partir `handlers/chat/tools.rs` (1.656 líneas)** con el mismo criterio que
   ya se aplicó a los schemas: un módulo por tool (`buscar.rs`, `detalle.rs`,
   `fotos.rs`), dejando `tools.rs` con el ejecutor y el dispatcher.
5. **Partir `handlers/ia/rutas.rs`**: router / estado de proveedores / config
   admin en tres ficheros.
6. **Un solo sitio para el modelo y los prompts.** Hoy el modelo se decide en
   `chat.rs:182` y en `ia/rutas.rs`; los prompts viven en `chat.rs:98`,
   `borrador.rs:455` y `continuacion.rs`. Un `config/ia.rs` con los nombres de
   modelo y una tabla de prompts haría el cambio de modelo un solo commit.

**P3 — Deuda de fondo, decidir antes de tocar**
7. **Unificar el rastreo de uso** (`agent_usage` + `mp_uso_minuto`) o dejar
   escrito por qué no se suman. Hoy «cuánto gasta la IA» no tiene una respuesta
   única.
8. **Valorar si el asistente de Marketplace debe adoptar el contrato del
   motor** (prompt por `PromptConfig`, uso por el mismo contador), sin
   adoptar el bucle de tools. Es el único punto donde su infraestructura
   diverge del resto.
9. **Cerrar `09AA-8`** (webhook fail-closed): es el único hallazgo de
   seguridad abierto del área.

**No hacer:** no forzar el asistente de Marketplace al bucle agentico. No
atender clientes desde el asistente (el envío humano es decisión de producto).
No migrar a multi-réplica sin resolver antes `ChatHub`.

---

## 11. Riesgos

| Riesgo | Estado |
|---|---|
| Webhook acepta cualquier secreto si el esperado está vacío | **Abierto** (`09AA-8`) |
| Caché de marketplace caducada por texto en vez de fecha | Prevenido por doc (`prevencion-cache-mp-2026-10-12.md`) |
| Foto de la IA apuntando a `PUBLIC_BASE_URL` tras Cloudflare Access | **Dormido**, camino no usado (`roadmap.md:59`) |
| Proveedor IA devuelve vacío → borrador cae a plantilla fija | **Activo** (`08AA-10`) |
| Multi-réplica rompe el tiempo real del chat | Estructural, aceptado y documentado |

---

## 12. Cómo verificar lo afirmado

```bash
# El motor único, y que WhatsApp y web entran por él
grep -rn "responder_turno_persistido" src/ ../glory-agent/src/

# Un solo AgentState, prompt y tools para web y WhatsApp
sed -n '181,201p' src/handlers/chat.rs

# El marketplace va por otra vía
sed -n '442,478p' src/handlers/marketplace/borrador.rs

# Tres familias de tablas
grep -rn "CREATE TABLE" migrations/*.up.sql

# Los alias muertos
sed -n '37,67p' src/services/mod.rs
```

---

## 13. Por qué el gate no marcó los dos archivos grandes

Respuesta a la pregunta «¿esto no lo detectaba workspace-manager?»: **no lo
marca hoy, y por dos motivos distintos.** Ninguno de los dos es que el gate no
cubra Rust: `**/*.rs` está en `sentinel.config.json` (`includePatterns`) y los
`.rs` tienen techo propio.

| Archivo | Físicas | Efectivas que ve Sentinel | Techo | Resultado |
|---|---|---|---|---|
| `handlers/chat/tools.rs` | 1.657 | **429** | 500 (`handlers/`) | No salta — correcto |
| `handlers/ia/rutas.rs` | 624 | **12** (deberían ser 537) | 500 (`handlers/`) | No salta — **fallo del contador** |

**`tools.rs` pasa por diseño.** `limite-lineas` no cuenta líneas físicas sino
**efectivas** (sin comentarios ni líneas vacías), y en Rust **corta el archivo
en `#[cfg(test)]`**: los tests del final no son complejidad de producción
(`glory-sentinel/src/utils/lineCounter.ts:9-17`). El techo de `handlers/` en
Rust es 500 (`lineCounter.ts:158-160`). Las ~1.100 líneas de tests de `tools.rs`
no cuentan; quedan 429 efectivas y no llega al nivel 1 (factor 1). **Es deuda
real de producción —el god-object existe— pero el gate no la ve por ahí.**

**`rutas.rs` pasa por un fallo del contador.** El archivo tiene un `#[cfg(test)]`
en la **línea 27** (un `use` que solo usan los tests) y el módulo de tests real
en la **464**. `contarLineasEfectivas` usa `indexOf`, es decir el **primero**, así
que trunca en la 27 y cuenta **12 líneas efectivas** en vez de 537
(`lineCounter.ts:14-17`). Con 537 sí saltaría (`limite-lineas`, nivel 1,
warning). El fallo es de `glory-sentinel`, no de MN: afecta a cualquier `.rs`
con un `#[cfg(test)]` temprano. Barrido del área el 2026-10-16: **8 archivos en
4 proyectos** (`MN-Inmobiliaria` 2, `RESTAURANTE` 2, `WANDORIUS` 3,
`limpiador-pc` 1).

**Consecuencia práctica:** el gate no sustituye a esta revisión. Un «0 hallazgos
nuevos» del gate significa «nada nuevo respecto a su base», no «nada que
arreglar» — y en este caso tampoco cubre dos archivos que sí conviene partir
(§10, P2.4 y P2.5).

**Verificar:**

```bash
# El contador corta en el primer #[cfg(test)], no en el módulo de tests
grep -n "#\[cfg(test)\]" src/handlers/ia/rutas.rs   # 27 y 464
grep -n "#\[cfg(test)\]" src/handlers/chat/tools.rs # 552 (solo el correcto)
sed -n '9,17p' ../glory-sentinel/src/utils/lineCounter.ts
sed -n '150,165p' ../glory-sentinel/src/utils/lineCounter.ts
```
