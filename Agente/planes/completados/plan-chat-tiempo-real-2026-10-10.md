# Plan 10AA-24/25/26 — Chats en tiempo real, continuación automática y dos opciones en la burbuja

> Origen: pedido de la usuaria, 2026-10-10. Tres deseos: (1) ver los chats en el admin al instante de abrirse; (2) que un mensaje nuevo relea la conversación y la IA conteste el último, breve y siguiendo el hilo; (3) dos textos en la burbuja (plantilla + IA). Prioridad: fluidez.

## Hallazgos (verificados en código, 2026-10-10)

- Los chats entran solo por `POST /api/admin/marketplace/borrador` (persiste únicamente `fuente=ia`) y `POST /releer` (crea fila solo-foto si no existe, [08AA-28]). El lab (float) llama a `releer` solo ante un mensaje nuevo en una ventana ya vista; al abrir una ventana por primera vez solo pide borrador. Por eso un chat no aparece hasta que la IA termina (o nunca, si falla).
- El admin (`features/chat/marketplace/use-chats-marketplace.ts`) carga la lista una vez, sin intervalo. No hay SSE/WebSocket en el backend (axum sin `ws`); el contador `VERSION_BORRADORES_BORRADOS` solo sube al borrar.
- El lab relee el DOM cada 5 s y en cada mutación; ante mensaje nuevo solo refresca el extracto (`auto-releer`). El borrador cacheado se reutiliza y no cambia hasta pulsar Regenerar (`draftDecision`, comentario «Compromiso» en `marketplace-float.ts`).
- El prompt del backend (`sistema_borrador`) es de **primer mensaje**: molde fijo «Hola, nombre… sigue disponible en X$» + bloque final de contacto, forzado por `imponer_forma_borrador`. No sirve para continuar una conversación.
- Contexto enviado: `excerpt.texto`, ~1200 últimas caracteres con marcas `Cliente:`/`Dueña:` (límite backend 2000). Ya trae el hilo, no solo el último mensaje.
- El globo ya pinta 2 opciones (`GLOBO_MAX_ITEMS`, orden IA arriba, `origen: plantilla|ia`); hoy cada motor devuelve un solo borrador y nunca se combinan.

## Diseño

- **Tiempo real = long-poll, no SSE.** Pasa por el gateway (:3102) sin riesgo de buffering y usa `apiFetch` (JWT en cabecera, `AbortSignal`). Contador en memoria con `tokio::sync::watch`; `GET /api/admin/marketplace/chats/cambios?desde=N` espera hasta 25 s y devuelve `{version}`; si `version != desde` (también tras reinicio) responde al instante.
- **Notificar en cada escritura** de `mp_respuestas_cache` visible en el panel: guardar, reemplazar, corregir, releer (update/insert), borrar/archivar.
- **Aparición al abrir:** el lab llama a `releer` en la primera vista de la ventana, sin esperar a la IA (fire-and-forget con log).
- **Continuación:** el backend decide el modo por el extracto: hay marcas `Dueña:` y el último turno es `Cliente:` → modo continuación (prompt breve: contesta lo último del cliente, sigue el hilo, sin saludo de apertura ni bloque fijo de contacto, ≤ 2 frases, sin inventar, sin visitas). Sin respuesta previa de la dueña → apertura como hoy (sin regresión). `imponer_forma_borrador` no se aplica en continuación.
- **Regeneración automática:** en una ventana ya vista, si cambia la firma y el último turno es de cliente, el lab pide IA sola (sin `force`: la firma nueva ya es miss en caché). Con ventana nueva o mensaje de la dueña no regenera.
- **Dos opciones:** el lab muestra la plantilla local al instante y, cuando llega, la IA/caché/reserva; orden IA arriba. `reintentoIAPendiente` pasa a mirar si falta IA aunque la plantilla siga presente (reserva espera 120 s).

## Fases

- **F1 (10AA-24) — Chat al abrir + notificación backend.** Lab: `releer` en primera vista. MN: módulo de cambios (`watch`), `notificar()` en las escrituras, endpoint `chats/cambios`. DoD: tests de cambios (espera, aviso, reinicio), `cargo check/clippy/test` verdes.
- **F2 (10AA-24) — Admin en tiempo real.** Hook con long-poll (reintento con retroceso, pausa en pestaña oculta), refresco de lista y del detalle abierto. DoD: `tsc -b` + oxlint limpios; chat nuevo visible < 2 s en prueba real.
- **F3 (10AA-25) — Continuación automática.** Backend: detección de modo y prompt breve; tests de selección de modo y de que no impone la forma de apertura. Lab: regenerar sola ante mensaje nuevo de cliente; log `[mp-float] mensaje nuevo → IA`. DoD: tests bun + cargo; prueba real con un hilo con respuesta previa de la dueña.
- **F4 (10AA-26) — Dos opciones.** Lab: combinar plantilla + IA, pintar plantilla al instante y empujar de nuevo cuando llegue la IA; ajustar `reintentoIAPendiente` y cache. DoD: tests bun; verificación visual en el lab.
- **F5 — Cierre.** Sentinel MN, gate, reinicios (MN con `Arranque`, lab con `lab-arrancar.ps1`), documentación del flujo del lab/panel, roadmap y completadas.

## No alcance

- Enviar el mensaje por la usuaria (el lab sigue solo copiando).
- Promoción a `opencode-propio` (decisión aparte, ver roadmap).
- Cambiar el prompt de apertura.

## Riesgos

- Falsos «mensaje nuevo» por ruido del DOM → ya mitigado: solo cuenta el último turno `cliente`; se añade espera mínima entre regeneraciones por ventana.
- Tope 30 pet./min por sub en `borrador`/`releer`: la regeneración automática lo respeta; si devuelve 429 se reintenta en el siguiente barrido.
- Coste de IA: un borrador por mensaje de cliente; sin cambios de firma no hay llamada.

## Estado

- **Cerrado en código (2026-10-10).** F1–F5 hechas: `cargo test marketplace` 120 verdes, clippy sin avisos nuevos, front `tsc -b` limpio, lab 173 tests `bun` + `tsgo -b`, Sentinel 0 hallazgos; MN (:3110) y lab (:9333) reiniciados con el código nuevo; `chats/cambios` responde 401 sin token (ruta viva).
- **Sin verificar en vivo:** chat nuevo < 2 s en el admin, mensaje nuevo → IA breve y 2 opciones en la burbuja. Requieren Facebook abierto en el lab y sesión admin (el autologin ya coincide desde 10AA-27). Pendiente anotado en `roadmap.md`.
