# Plan — Correcciones de la IA de atención al cliente (16AA-9)

Origen: `Agente/documentacion/integraciones/revision-ia-atencion-cliente-2026-10-16.md`
(revisión del 2026-10-16, §10 áreas de mejora priorizadas).

> Estado: **cerrado el 2026-10-16** (16AA-9, `Agente/completados/tareas-2026-10-16.md`).
> F1–F5 hechas y verificadas; F6 (`09AA-8`) no entra aquí: sigue esperando al
> cierre de `12AA-1` y vive en el roadmap (§2 y §6). Dos correcciones de premisa
> quedaron escritas en la entrada de cierre: F5 no unifica `agent_usage` (no es
> una tabla) ni `mp_uso_minuto` (cuenta peticiones, no tokens), y el modelo del
> chat no lo elige MN sino el crate `glory-agent`.

## Decisiones de la usuaria (2026-10-16)

- **Alcance:** F1 a F4 de una, en tanda. F5 y F6 después.
- **F5 — uso de la IA:** **unificar en una vista de lectura**. Una sola respuesta
  a «cuánto gasta la IA», sin tocar las tablas que ya escriben ni el camino de
  escritura de ninguna de las dos contabilidades.
- **F6 — `09AA-8`:** sigue esperando al cierre de `12AA-1`, como se decidió el
  2026-10-12. No se toca ahora.

## Objetivo

Cerrar lo accionable de la revisión: quitar el ruido (P1), partir los dos ficheros
que hacen de god-object y centralizar modelo y prompts (P2), y dejar decidida la
deuda de fondo del uso y del contrato del asistente (P3). **Sin cambiar
comportamiento observable** en ninguna fase: son restas, movimientos y troceados.

## Alcance

Entra: el área de IA de atención al cliente de MN (`src/handlers/chat/**`,
`src/handlers/ia/**`, `src/services/**` relacionados, `src/config/`, el borrador
del marketplace en lo que toca a prompts y modelo).

No entra, y está escrito para no volver a discutirlo:
- forzar el asistente de Marketplace al bucle agentico;
- que el asistente atienda clientes (el envío humano es decisión de producto);
- multi-réplica / `ChatHub` (es cambio de arquitectura, no ajuste);
- nada que dependa de terceros o de configuración: `08AA-10/11/15`, `199A-1/2`,
  `03AA-3`, `03AA-5`, `169A-*`, `wa_c`. Siguen en su sitio del roadmap.

## Fases

### F1 — Resta y documentación (P1) · sin contrato

1. Borrar los alias muertos de `src/services/mod.rs:37-67` (15 bloques con
   `#[allow(unused_imports)]`).
2. Corregir el comentario de `src/handlers/chat/staff_config.rs:24`: dice que
   `prompt_extra` «sigue guardada sin lector» y sí lo tiene
   (`glory-agent/src/transport/mod.rs:382`). Es la única palanca de tono en
   caliente y hoy parece muerta.
3. Deriva de documentación: `roadmap.md:20` y `roadmap.md:59` citan
   `src/services/transporte.rs`; el fichero es `src/services/mensajeria/transporte.rs`.

**Riesgo:** algún consumidor puede llegar hoy por el re-export. Antes de borrar,
`grep` de cada nombre fuera de `services/`; después, `cargo check` en la 89.

**Verificación:** `node scripts/run-cargo.mjs check` en la 89 (verde) +
`Arranque gate "MN-Inmobiliaria" --cierre` sin hallazgos nuevos.

### F2 — Config central de IA (`src/config/ia.rs`) (P2.6)

Un solo sitio para el nombre del modelo y para los prompts: hoy el modelo se
decide en `chat.rs:182` y en `ia/rutas.rs`, y los prompts viven en `chat.rs:98`,
`marketplace/borrador.rs:455` y `continuacion.rs`.

**Puro movimiento:** los literales se trasladan tal cual, sin reescribir ninguno.

**Verificación:** test que compare los valores servidos antes y después
(modelo y prompt efectivo por superficie) + `cargo check` + los tests del área.
DoD: cambiar de modelo o de tono es un solo commit y no hay literal de modelo
fuera de `config/ia.rs`.

### F3 — Partir `handlers/chat/tools.rs` (P2.4)

Mismo criterio que ya se aplicó a los schemas: un módulo por tool
(`tools/buscar.rs`, `tools/detalle.rs`, `tools/fotos.rs`) y `tools.rs` se queda
con el ejecutor (`:32`) y el dispatcher (`:55`). Los ~17 tests (`:552+`) viajan
con la tool que prueban.

**Criterio:** ninguna función cambia de cuerpo, solo de sitio.
**Verificación:** tests de tools verdes + `cargo check`.
**DoD:** `tools.rs` ≤ 300 líneas efectivas y cada módulo ≤ 300.

### F4 — Partir `handlers/ia/rutas.rs` (P2.5)

Router / estado de proveedores / config admin, en tres ficheros.
**DoD:** `rutas.rs` ≤ 200 efectivas; el endpoint público no cambia de forma.

**Nota:** cuando entre el release de Sentinel con el arreglo del contador
(`4d63473`), `ia/rutas.rs` empieza a salir en el gate y hay que **rehacer la base
del gate** antes de seguir. Si para entonces F4 no está hecha, hacerla primero
que nada.

### F5 — Uso unificado (P3.7) · decidido: vista de lectura

`agent_usage` (crate) y `mp_uso_minuto` (marketplace) son dos contabilidades que
no se suman. **Decidido el 2026-10-16 (la usuaria): unificarlas en una vista de
lectura**, sin tocar las tablas ni el camino de escritura de ninguna de las dos.

**DoD:** una consulta da «cuánto gasta la IA» en un solo número, y los dos
productores siguen escribiendo donde escriben hoy.

Aparte, y sin tocar código hasta decidirlo: valorar si el asistente de
Marketplace adopta el contrato del motor (prompt por `PromptConfig`, uso por el
mismo contador) **sin** adoptar el bucle de tools. Es el único punto donde su
infraestructura diverge del resto.

### F6 — `09AA-8`, webhook fail-closed (P3.9)

Único hallazgo de seguridad abierto del área: `secreto_valido` acepta cualquier
valor si el esperado está vacío (`src/services/mensajeria/transporte.rs:95-103`).
**Bloqueado** por el cierre de `12AA-1` (decisión de la usuaria del 2026-10-12).
No se toca hasta entonces.

## Orden y por qué

F1 → F2 → F3 → F4 en una tanda (decidido el 2026-10-16), F5 después y F6 cuando
se desbloquee. F1 primero porque es resta sin riesgo y quita ruido del gate. F2
antes que F3 y F4 porque los troceados deben dejar el modelo y los prompts ya en
un sitio, no volver a dispersarlos.

Cada fase es un bloque coherente con **su** commit y **su** entrada en
`Agente/completados/tareas-2026-10-16.md` (o del día que toque), y las de código
con `Arranque gate "MN-Inmobiliaria" --inicio` antes y `--cierre` después.

## DoD del plan

- Las tres correcciones de P1 hechas y el gate de cierre limpio.
- `tools.rs` y `rutas.rs` partidos, con los tests verdes y sin cambio de
  comportamiento.
- `config/ia.rs` es el único sitio donde vive el modelo y los prompts.
- F5 con decisión escrita; F6 anotado como pendiente de `12AA-1`.
- Roadmap al día, `16AA-9` cerrado en `Agente/completados/`, y este plan movido a
  `Agente/planes/completados/`.
