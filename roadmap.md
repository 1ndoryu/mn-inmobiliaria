# Roadmap — MN-Inmobiliaria

Cola operativa (AGENTS §7.1). Stack y comandos: `README.md`. Historial íntegro del 2026-10-12: `Agente/planes/roadmap-detalle-2026-10-12.md`. Cierres: `Agente/completados/tareas-YYYY-MM-DD.md`.

## 1. Siguiente bloque ejecutable

- **12AA-1 — Gateway WhatsApp (3102), cierre por plazo.** Mitigación en `gateway/src/sesion.mjs` (`fireInitQueries: false`, backoff 5→60 s). Cierra si hasta 2026-10-13 05:12 local (09:12Z) no hay `ENOTFOUND` ni 428 repetidos con `wa_a`/`wa_b` estables. PID vigente 9032 (verificado: `node src/index.mjs --gateway-mn`; el registro decía 18784). Pendiente: revisar la resolución DNS del host (`ENOTFOUND web.whatsapp.com`). Detalle: `Agente/planes/roadmap-detalle-2026-10-12.md` (entrada 12AA-1).

## 2. Decisiones de la usuaria (bloqueos)

- **08AA-6 barrido Sentinel:** 0 hallazgos Sentinel/Varsense, análisis 2026-10-12 (HEAD `8d3f6bfd`). La cifra de ~518 era histórica y ya no aplica. Ver `Agente/completados/tareas-2026-10-12.md`.
- **Riesgo aceptado — 10AA-21 (`rsa` 0.9.10, RUSTSEC-2023-0071):** decidido 2026-10-12. Se reabre al subir `sqlx` sin `sqlx-mysql`; criterio: `rsa` ausente de `Cargo.lock` y `check:back` verde.
- **Webhook del gateway (09AA-8) — secreto `WA_WEBHOOK_SECRETO`:** decidido «tras 12AA-1» (2026-10-12). Al cerrar 12AA-1: fijar el secreto en `.env` y en el gateway, y hacer fail-closed `secreto_valido` (`src/services/transporte.rs:95-103`), que hoy acepta cualquier valor si el esperado está vacío.
- **08AA-30 — filas duplicadas en `mp_respuestas_cache`:** decidido «Dejarlas» (161 filas, 48 `thread_id` repetidos, 82 sobrantes). No se borran; la unificación de `thread_id` sigue en §6.
- **289A-5 — sesión `739bb63e` con `devolver_a_ia`:** decidido «No, pendiente». No se reintenta ahora.
- **03AA-3 Asistente Marketplace:** falta la firma de ella para M1, el remoto y la fecha.
- **279A-4..8 Extras públicos:** decidir si se cablean a ficha e IA (ver 279A, §6).
- **Deploy 239A-1, Fase 5.4:** confirmar la password admin rotada (o pedir otra).
- **169A-1 — `AGENTE_CONTACTO`:** confirmar el valor real que va en `.env` (no se muestra aquí).
- **Entrada de personal en producción:** acción de la usuaria (entrar una vez en la app); el agente no entra en producción.

## 3. Pruebas de la usuaria (en vivo)

- **10AA-28/29 — Globo, dueña y apertura:** ver «Tú» en burbujas de la dueña; chip «Precio»/«Valor» en un mensaje real.
- **10AA-24/25/26 — Chats en tiempo real:** chat nuevo visible en admin en < 2 s; borrador IA breve con cliente real (no se simulan envíos).
- **09AA-29 — Mensaje del cliente fiable, precio en el borrador y caché por inmueble (pedido por ella 2026-10-09, en curso):** retest desde cero del hilo de Puerto Ordaz.
- **09AA-30 — Caché por inmueble:** verificar F3 en lab (F4 en dev, §6).
- **09AA-23 — Vínculo visible:** crear el inmueble Río Aro con precio y Regenerar el hilo `salazar`.
- **09AA-19 — Burbujas estructuradas:** retest F4 (3 filas intactas, 0 corregidas) y F7d (2 avisos).
- **09AA-4 — Regenerar y prueba JWT:** `/borrador` con JWT → `fuente=ia`, sin WARN de vacío.
- **09AA-3 — Regenerar-todo y dieta del prompt:** su prueba pendiente.
- **09AA-5 — Tab Logs:** verificar en vivo el tab Logs del puente (sin cierre registrado).
- **08AA-39 — Botón «Limpiar»:** prueba pendiente.
- **09AA-14 — Flotante al núcleo:** probar Facebook y confirmar en Logs el borrador del núcleo.
- **199A-3 — Sesión de 10 años:** salir y volver a entrar una vez.
- **199A-5 y 199A-6 — Fotos y frente público:** revisar la propiedad editada y re-subir fotos; Ctrl+F5 en `:5200` (Townhouse/Orquídea).
- **279A-2 — Agente MN:** QR de ambos números, batería real F2/F3, limpieza, deploy.
- **279A-3 — `/ask`:** responder el cuestionario de ficha.
- **289A-1 — Gateway real:** vincular A y B (temporal) y ejecutar la batería real.
- **239A-1 — Deploy (cierre del agente 2026-10-11):** revisar https://mn-inmobiliaria.com con la lista visible.
- **249A-1 — PageSpeed:** rerun móvil y escritorio (cuota API 429 el 2026-09-24).

## 4. Promoción a opencode-propio (ventana de la usuaria)

Cola viva en `opencode-propio-dev/PROMOCION-VENTANA.md` («Pendiente por promover»). Ítems de este roadmap:
- 10AA-17 (lab → opencode-propio; prueba viva con `MP_NUCLEO=on` después).
- 09AA-14 (timeout 90 s del flotante).
- 09AA-19 (F7b, F1, F6 y F5).
- 08AA-9 (botón Releer).
- 03AA-3 y 07AA-6: C1b cancelado por ella 2026-10-07 (`Agente/planes/plan-flujo-lab-opencode-propio-2026-10-07.md:100`). Queda la promoción en su ventana y, en 07AA-6, la prueba viva.
- 07AA-8 (V4 de `marketplace-float.ts`).

## 5. Bloqueados por terceros o configuración

- **08AA-10:** texto final con precio pendiente; el proveedor IA devuelve vacío (`/ia/probar` cuelga).
- **08AA-11 y 08AA-15:** verificación visual y viva del borrador; mismo proveedor IA.
- **299A-1 — Fase 3 (batería completa de escenarios IA):** E14, E15 y E16 verificados (`Agente/completados/tareas-2026-09-29.md`, secciones E14, E15 y E16). La batería completa necesita proveedor IA operativo.
- **169A-1:** fijar `AGENTE_CONTACTO` real en `MN-Inmobiliaria/.env`.
- **169A-3..6 (WhatsApp):** `GLORY_ALERT_GATEWAY_URL` y número admin (`whatsapp_admin`).
- **199A-1/199A-2 (IA central):** `GLORY_API_KEY` real en `.env`; sin ella GloryAPI queda `sin-clave`.
- **249A-1:** cuota PageSpeed (429) y sub-chequeo agéntico en 3/4.

## 6. Desarrollo pendiente (por dependencia)

- **09AA-6 §8 — chat backend:**
  - **09AA-7** invariante de persistencia (espera a 09AA-4/5 o serializar).
  - **09AA-8** tope de media hecho (§1); webhook fail-closed según §2.
  - **09AA-10** dedup y espejo M3 (decidir qué lado gana).
  - **09AA-11** proveedores IA (coordinado con 09AA-4).
  - **09AA-12** frontend de chat (paralelizable; buscar la discrepancia de estado).
- **09AA-30 F4** — panel admin con origen visible (tras F3).
- **09AA-14** — fix del scraper (lee chrome, no burbujas): `parseBurbujaLado` + tests bun.
- **10AA-17** — `floatDraftCache` no se invalida al borrar en MN (cubierto por `force`).
- **10AA-24/25/26** — el lab procesa ventanas en serie; paralelizar si molesta.
- **08AA-8b** — política de refresco del excerpt ante mensajes nuevos; rescatar marcas Cliente/Dueña con pestaña en fondo.
- **199A-7** — pareo Orquídea: endpoint PATCH para reordenar fotos (opcional).
- **299A-3** — demo fotos/audio: la IA escala tras 2 notas de voz (sin fix registrado).
- **169A-2** — UI admin de revisión de solicitudes (no existe).
- **279A** — cablear extras públicos a ficha e IA (depende de §2).
- **03AA-5** — detector de captación: C0 (corpus 5+2+1, dataset 30–50, DDL) antes de código.
- **Fuera de MN:** configurar `coolify-manager-rs` para desplegar proyectos Rust (repo separado).

## 7. Planes vinculados

Cada ítem enlaza su plan en su entrada de la sección 3 o 6. Para el detalle íntegro: `Agente/planes/roadmap-detalle-2026-10-12.md`.
