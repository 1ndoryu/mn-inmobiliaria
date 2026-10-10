# Plan — Caché compartida por inmueble, detección automática y origen visible (09AA-30, 2026-10-09)

## Objetivo
1. Las primeras preguntas del cliente sobre un inmueble se responden desde caché compartida (una sola IA); el nombre del comprador se rellena al servir.
2. El float detecta solo cuándo llega un mensaje del cliente o la dueña envía uno, relee el hilo y genera borrador IA si no hay caché.
3. Se ve en la burbuja y en el admin de dónde sale cada borrador: Caché / IA / Reserva / Plantilla local.

## Decisiones (tomadas por ella, 2026-10-09)
1. **Automático:** al llegar un mensaje nuevo del cliente sin caché, se genera borrador IA sin pulsar nada.
2. **Corrección vale para todo el inmueble:** si la dueña corrige una respuesta compartida, la corrección se sirve a todos los hilos que compartan ese mensaje.
3. **Caché compartida solo para los 2 primeros mensajes del cliente** del hilo (sin ventana de tiempo; cambió la decisión de 3 minutos, 2026-10-09: «si no trae hora no te compliques, que sean los 2 primeros mensajes y ya»). Desde el tercer mensaje del cliente, el hilo va por su cuenta, sin compartir.
   - Implementación: la clave compartida es el texto normalizado de los 1–2 mensajes del cliente del excerpt (`mensaje_clave_de`, `mensaje_clave_v2`). Con 3 o más, no hay clave.
   - Sin hora: no hace falta extraer la hora de las burbujas. F1 (e)–(f) se retiran.
4. **Decidido antes (2026-10-09):** dos compradores con el mismo mensaje sobre el mismo inmueble comparten respuesta; el nombre se rellena al servir.

## Estado actual (verificado en código, 2026-10-09)
- Clave de caché = `firma` (sha v2 de TODAS las burbujas útiles del hilo) + `precio_hash` + `catalog_hash`. PK `(firma, precio_hash, catalog_hash)` (`migrations/20261006000030_mp_cache.up.sql:14`). Cada mensaje nuevo cambia la firma → miss → IA otra vez. Por eso nada se comparte entre compradores ni al avanzar un hilo.
- Hit: firma + precio + catálogo y `valida_hasta > now()` (`src/services/marketplace.rs:1172-1176`), `usos + 1`. `fuente = "cache"` (`src/handlers/marketplace.rs:292`).
- **Riesgo actual (inferido, confirmar con test en F2):** la firma v2 no incluye el nombre del comprador, pero la `respuesta` guardada sí lleva «Hola, <nombre>» (`src/handlers/marketplace.rs:733,845`). Dos compradores con el mismo texto sobre el mismo inmueble comparten fila y el segundo recibe el saludo del primero.
- **Hora del mensaje no disponible hoy (verificado):** `excerpt.hora` se rellena con la hora de la **petición** (`buildBorradorRequest` → `horaCaracasISO(ahora)`, `marketplace-nucleo.ts:126`), no con la hora del mensaje. La hora real vive en el nombre accesible de la burbuja (`marketplace-float.ts:389`; `mpLado` ya lee ese nombre en L381-388). Formatos observados en el hilo: `2:43 am`, `lunes 22:48` (día relativo, sin fecha). Hay que extraerla y resolver el día en F1.
- `fuente` (`cache`, `ia`, `reserva`, `omitido`) no se guarda en BD: no hay columna de origen. El borrador local del float (`marketplace-service.ts:224-225`) no pasa por el backend.
- Float (lab): `setInterval` de 5 s relee el DOM (`marketplace-service.ts:118,293-295`), pero no compara mensajes: la firma djb2 se guarda y nunca se lee (`marketplace-float.ts:202-206,230-253`). Solo genera con ventana nueva o con Regenerar.
- **Bug de firma (inferido, confirmar en F1):** `Cliente/Dueña` solo se etiqueta si `document.hasFocus()` (`marketplace-float.ts:447,728`). Sin foco, el excerpt pierde prefijos y la firma cambia sin mensaje nuevo.
- Burbuja: `floatDraftsFor` devuelve solo `{id, text}` (`marketplace-float.ts:914`); `parseBorradorResponse` ignora `fuente` (`marketplace-nucleo.ts:134-139`). No se pinta ningún origen.
- Admin: `chats-marketplace.tsx` no muestra fuente ni caché por borrador. «N borradores» cuenta también las filas que crea `releer` sin respuesta (firma sintética `releer-sin-borrador`, `src/services/marketplace.rs:1329`). `RegenerarTodoFila.fuente` existe en tipos y nunca se pinta.

## Diseño

### Sin ventana de tiempo (sustituye al diseño de 3 min)
- No se extrae hora de las burbujas ni se resuelven días relativos. `excerpt.hora` sigue siendo la de la petición; no se usa para compartir.

### Clave compartida (solo con 1–2 mensajes del cliente)
- `catalog_hash` + `precio_hash` + `mensaje_clave` (texto normalizado de los mensajes del cliente, `mensaje_clave_de`). Sin nombre, sin hora, sin historial.
- Solo si el aviso es conocido (`conocido`) y el hilo tiene nombre (para rellenar `{{nombre}}`).

### Tabla `mp_respuestas_inmueble` (aditiva; no se migra la PK de `mp_respuestas_cache`)
- Migración `20261009000036_mp_respuestas_inmueble`: columnas `catalog_hash`, `precio_hash`, `mensaje_clave` (PK), `respuesta` (con marcador `{{nombre}}`), `corregida`, `valida_hasta` (+90 días), `usos`, `origen`, `tokens_entrada`, `tokens_salida`, `ms_generacion`. Además `mp_respuestas_cache.mensaje_clave` (NULL) para enlazar cada hilo.
- Hit: `buscar_compartida` → sustituye `{{nombre}}` por el nombre del hilo (`rellenar_nombre`), enlaza el hilo con `vincular_hilo_compartido` (copia origen y coste) y responde `fuente = "cache"`.
- Alta tras IA: `enlazar_compartida(pisar=false)`, solo si el nombre aparece a lo sumo una vez en la respuesta (`ocurrencias_nombre`).
- Regenerar explícito por fila (`regenerar_uno`, ruta `/marketplace/regenerar`): `enlazar_compartida(pisar=true)`. Pisa la respuesta compartida del inmueble, incluida una corrección previa.
- Corrección de la dueña (`corregir`): `corregir_cache` del hilo y `corregir_compartida` con el nombre del hilo que corrige → llega a todos los hilos enlazados.
- `regenerar_todo` RETIRADO (2026-10-09, pedido de ella: «regenerar todo ya no importa»). Ya no hay bucle que pise la compartida. Respaldo: `Agente/documentacion/archivados/regenerar-todo-2026-10-09.archivado`.
- Menú ⋮ por chat (2026-10-09): **Archivar** (`mp_chats_archivados`, oculta el hilo del resumen; no toca caché ni compartida), **Borrar borrador** (borra las filas no corregidas del hilo y, de la compartida, solo la fila no corregida que ningún otro hilo ni corrección enlaza), **Borrar** (borra caché y archivo del hilo; no toca la compartida).

### Fuera de ventana (por hilo)
- `mp_respuestas_cache` (firma del hilo) como hoy. Sin compartir.
- Con decisión 1, en miss se genera IA automática igualmente. **Coste:** cada mensaje nuevo fuera de ventana sin caché es una llamada IA. Ella lo aceptó; se anota para vigilar `usos`/logs.

### `mp_respuestas_cache` se mantiene por hilo
- Sigue guardando la firma. Se añade `origen` (`ia | cache | reserva | local`) para el admin.
- Migración aditiva (ADD COLUMN nullable). Filas viejas con `origen NULL` = «desconocido».

### Detección automática (float, lab)
- La firma pasa a ser la de las **burbujas Cliente/Dueña**, no la del excerpt: no depende del foco.
- En el tick de 5 s: si la última burbuja es nueva y del cliente (recibido) o de la dueña (enviado) → auto-`releer` (sin IA). Respeta el tope por minuto de `releer` (08AA-9).
- Si es del cliente y no hay caché → borrador IA automático (decisión 1), con single-flight por clave para no generar dos respuestas iguales a la vez.

### Visibilidad
- Backend: `borrador` y `releer` devuelven `origen` y `usos`. `releer` devuelve además `cache_disponible` para el mensaje actual.
- Float: badge por borrador: «Caché», «IA», «Reserva», «Plantilla local» (convención de `marketplace-float.ts:525-534`).
- Admin (`chats-marketplace.tsx`, lista y detalle): badge de origen con `Badge` del sistema; «compartida por N hilos»; contador que excluye `releer-sin-borrador`.
- `ultimo` del `ChatResumen` es `MAX(valida_hasta)`, no la fecha del último mensaje (`src/services/marketplace.rs:988`). Mostrar la fecha real requiere `ultimo_mensaje_en`; va en F4.

## Fases (cada una verificable)
- **F0 — Origen visible, sin cambio de comportamiento.** Migración aditiva `origen`; backend lo guarda y lo devuelve; float y admin lo pintan. Test: IA → «IA»; caché → «Caché»; IA caída → «Reserva»; sin núcleo → «Plantilla local». Vivo: los 4 casos en el hilo de Puerto Ordaz.
- **F1 — Detección (lab).** Firma por burbujas sin depender del foco; auto-releer en el tick. Tests: (a) sin foco, misma conversación → cero cambios de firma; (b) mensaje nuevo del cliente → un releer; (c) mensaje de la dueña → un releer; (d) sin cambios → cero llamadas. Estado: hecha en el lab (111 tests `bun test` en verde, tsgo 0); falta el retest vivo.
- **F2 — Caché compartida por inmueble (backend).** Estado: código escrito (migración, `marketplace_compartida.rs`, handlers `borrador`/`regenerar_uno`/`corregir`); sin compilar ni probar. Tests (DB, `#[tokio::test]`, se saltan sin `DATABASE_URL`): (a) dos hilos del mismo inmueble, mismo mensaje (1–2 del cliente) → una llamada IA y nombre correcto en cada uno; (b) con 3 mensajes del cliente → no comparte; (c) corrección en hilo A → hilo B la recibe con su nombre; (d) precio cambia → miss; (e) hilo sin nombre o aviso desconocido → no comparte.
- **F3 — Borrador automático en miss.** Disparo desde el float al llegar mensaje del cliente sin caché; single-flight por clave.
- **F4 — Admin.** Badges, contador corregido, «compartida por N hilos», `ultimo_mensaje_en`.
- Orden: F0 → F1 → F2 → F3 → F4. F2 va antes que F3: sin caché compartida, el automático gastaría IA en cada mensaje dentro de ventana.

## Alcance / no alcance
- SÍ: este plan; tras el visto bueno, F0–F4.
- NO ahora: implementar, migrar `mp_respuestas_cache`, promover el lab a la app viva (solo con ventana de ella), tocar Regenerar (09AA-13 bloqueada).

## Dependencias
- 09AA-29: F1 y F2 hechas (retest y commit pendientes). Sus F3/F4 se sustituyen por este plan.
- 09AA-19 (burbujas estructuradas), 09AA-2 (imposición de forma), 09AA-4 (sesión estable), 09AA-5 (Logs: añadir evento de origen).
- Lab sin git: se verifica allí; la promoción a `opencode-propio` requiere ventana de ella.

## Verificación y Definition of Done
- Gate Rust (`fmt`, `clippy -D warnings`, `test`) y tests del lab en verde.
- Sentinel sin hallazgos nuevos atribuibles.
- Vivo: dos hilos del mismo inmueble con el mismo primer mensaje del cliente («¿Sigue disponible?») → una sola IA, saludo correcto en cada uno, badge «Caché» en el segundo, y badge en admin.
- Commit en rama, documentación actualizada, lección si aplica.

## Estado
Ejecutando. Decisiones tomadas 2026-10-09 (regla de 2 mensajes, sin ventana de tiempo; `regenerar_todo` retirado). F0 backend hecho. F1 hecha en lab (pendiente retest vivo). F2 hecha (archivar/borrar/borrar borrador), gate Rust verde (2026-10-10). F3–F4 pendientes.

**Gate Rust 09AA-31 (2026-10-10, BD `glory_backend_inmobiliaria_test`):** migraciones OK, fmt OK, clippy `-D warnings` OK, tests 201/201 (2 ignorados en `tests/chat_humo.rs`), `tsc` app 0. Sentinel: error 0, warning 468 (`sentinel4.json`).

**Fallos `chat_tools` (resueltos):** los 6 tests exigían inmuebles publicados con fotos y la BD de prueba estaba vacía. Solución: `scripts/fixtures/seed-test-inmobiliaria.sql` (idempotente; `psql "<BASE>/glory_backend_inmobiliaria_test" -v ON_ERROR_STOP=1 -f scripts/fixtures/seed-test-inmobiliaria.sql`).

**Split:** `src/handlers/marketplace/chats_admin.rs` (chats, chat_detalle, borrar_todo, archivar_chat, borrar_chat, borrar_borrador_chat, version_borradores) para bajar `handlers/marketplace.rs` de 912 líneas efectivas (>800 = error); hecho, queda 777 como aviso (tarea 10AA-2).

**Decidido (lab, opción a):** «borrar borrador» vacía también la caché del float de Electron (`floatDraftCache`). Ella eligió la opción (a): contador de borradores borrados en backend (`GET /api/admin/marketplace/borradores/version`) que el tick de 5 s consulta y, si cambia, vacía `floatDraftCache`. Sin puertos abiertos. Lado lab hecho: `marketplace-nucleo.ts` (`pedirVersionBorradores`, `parseVersionBorradores`), `marketplace-service.ts` (`invalidarCacheSiBorradoresBorrados`), tests (`bun test` 114 pass, `tsgo -b` exit 0). Backend escrito: contador `VERSION_BORRADORES_BORRADOS` (services/marketplace.rs, sube al borrar filas) y `GET /api/admin/marketplace/borradores/version` (handlers/marketplace.rs, `MpAuth`). Gate Rust verde. Pendiente: retest vivo del float en lab.
