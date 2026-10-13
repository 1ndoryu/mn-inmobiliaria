# Roadmap — MN-Inmobiliaria

Cola operativa (AGENTS §7.1). Stack y comandos: `README.md`. Historial íntegro del 2026-10-12: `Agente/planes/roadmap-detalle-2026-10-12.md`. Cierres: `Agente/completados/tareas-YYYY-MM-DD.md`.

## 1. Siguiente bloque ejecutable

- **13AA-4 — Vincular inmuebles con sus avisos de Facebook Marketplace.** Plan: `Agente/planes/plan-vinculo-marketplace-2026-10-13.md`. Estado: **en ejecución; lectura una vez al día.** Hechos: tabla y endpoints de avisos, modal «Vincular con Facebook» y borradores sin título inseguro (F2–F4); lectura diaria del perfil en la copia de pruebas de la lab (F1, verificada contra Facebook real el 2026-10-13: 8 avisos; falta que la usuaria compruebe el modal y que «Vincular» guarde). Bloqueo externo: la float debe enviar `avisoId` (F5, tarea F7b, ventana de promoción de la lab). Rama `13aa-4-vinculo-marketplace`: ya integrada en `main`; no se borra aún porque la carpeta principal está en ella con cambios sin guardar de otra sesión (publica/detalle, superficie). Se borra cuando esos cambios se guarden o se quiten de ahí.
- **12AA-1 — Gateway WhatsApp (3102), cierre por plazo.** Mitigación en `gateway/src/sesion.mjs` (`fireInitQueries: false`, backoff 5→60 s). Cierra si hasta 2026-10-13 05:12 local (09:12Z) no hay `ENOTFOUND` ni 428 repetidos con `wa_a`/`wa_b` estables. **Comprobación 2026-10-13 (investigador, `logs/dev-up-MN-Inmobiliaria.log`): no cumple el criterio.** Desde 09:12Z: 2 `ENOTFOUND` (último 10:28Z) y 2 `428` (A y B, un corte entre 09:59Z y 10:28Z). Desde 10:28Z ambas sesiones quedan abiertas, sin bucle. Tras el último reinicio (12:36Z) solo hay ~6 min de observación: no hay base para cerrar. **Reinicio ~13:19Z para añadir wa_c:** A y B reconectaron solas; la observación cuenta desde ahí. Estado: **abierta**. **Decidido 2026-10-13 (la usuaria):** dejarla abierta y revisar el registro unas horas después; cerrar solo si no hay cortes en ese tiempo. Pendiente: resolución DNS de `web.whatsapp.com`. Detalle: `Agente/planes/roadmap-detalle-2026-10-12.md` (entrada 12AA-1).
  **Decidido 2026-10-12 (la usuaria):** la tarea programada del 13/10 05:15 que lo cerraba se cancela. El cierre queda manual: comprobar el gateway y decidir a mano.

## 2. Decisiones de la usuaria (bloqueos)

- **08AA-6 barrido Sentinel:** 0 hallazgos Sentinel/Varsense, análisis 2026-10-12 (HEAD `8d3f6bfd`). La cifra de ~518 era histórica y ya no aplica. Ver `Agente/completados/tareas-2026-10-12.md`.
- **Riesgo aceptado — 10AA-21 (`rsa` 0.9.10, RUSTSEC-2023-0071):** decidido 2026-10-12. Se reabre al subir `sqlx` sin `sqlx-mysql`; criterio: `rsa` ausente de `Cargo.lock` y `check:back` verde.
- **Webhook del gateway (09AA-8) — secreto `WA_WEBHOOK_SECRETO`:** decidido «tras 12AA-1» (2026-10-12). Al cerrar 12AA-1: fijar el secreto en `.env` y en el gateway, y hacer fail-closed `secreto_valido` (`src/services/transporte.rs:95-103`), que hoy acepta cualquier valor si el esperado está vacío.
- **08AA-30 — filas duplicadas en `mp_respuestas_cache`:** decidido «Dejarlas» (161 filas, 48 `thread_id` repetidos, 82 sobrantes). No se borran; la unificación de `thread_id` sigue en §6.
- **289A-5 — sesión `739bb63e` con `devolver_a_ia`:** decidido «No, pendiente». No se reintenta ahora.
- **03AA-3 Asistente Marketplace:** falta la firma de ella para M1, el remoto y la fecha.
- **279A-4..8 Extras públicos:** decidido 2026-10-12 (la usuaria): la ficha pública no muestra ningún extra, tampoco el precio negociable (revierte la decisión previa de publicarlo). Si mañana decide mostrar algo, se revisa lista por lista. La lista candidata y el criterio quedan en `Agente/planes/plan-extras-publicos-2026-10-12.md`. La IA sigue leyéndolos (ver 279A, §6).
- **Deploy 239A-1, Fase 5.4:** confirmar la password admin rotada (o pedir otra).
- **169A-1 — `AGENTE_CONTACTO`:** confirmar el valor real que va en `.env` (no se muestra aquí).
- **Entrada de personal en producción:** acción de la usuaria (entrar una vez en la app); el agente no entra en producción.
- **Tercer número de WhatsApp (wa_c), decidido 2026-10-13 (la usuaria):** no responde solo a clientes; se activa hoy. Código listo y gateway reiniciado. Falta que escanees el código QR `gateway/qr-wa_c.png` con el teléfono del número nuevo (WhatsApp → Dispositivos vinculados → Vincular). El código caduca en ~1 min: si caduca, el gateway crea otro en el mismo archivo. Ningún envío desde este número sin tu permiso.

## 3. Pruebas de la usuaria (en vivo)

Cerradas el 2026-10-13 por decisión de la usuaria: no hay nada que probar en vivo y no se reinicia el lab. Lista y motivo: `Agente/completados/tareas-2026-10-13.md` («Pruebas vivas cerradas»).

## 4. Promoción a opencode-propio (ventana de la usuaria)

Cola viva en `opencode-propio-dev/PROMOCION-VENTANA.md` («Pendiente por promover»). Ítems de este roadmap:
- 10AA-17 (lab → opencode-propio; prueba viva cerrada 2026-10-13).
- 09AA-14 (timeout 90 s del flotante).
- 09AA-19 (F7b, F1, F6 y F5).
- 08AA-9 (botón Releer).
- 03AA-3 y 07AA-6: C1b cancelado por ella 2026-10-07 (`Agente/planes/plan-flujo-lab-opencode-propio-2026-10-07.md:100`). Queda la promoción en su ventana; la prueba viva de 07AA-6 se cerró 2026-10-13.
- 07AA-8 (V4 de `marketplace-float.ts`).
- 13AA-2 (saludo por franja en burbujas, lab 2026-10-13): en la ventana de la usuaria; detalle en `opencode-propio-dev/PROMOCION-VENTANA.md` («0. 13AA-2»). Prueba viva cerrada 2026-10-13. **Decidido 2026-10-13 (la usuaria): no reiniciar el lab.** El cambio no está activo en el lab en marcha; se activa en su próximo reinicio, cuando ella lo decida.

## 5. Bloqueados por terceros o configuración

- **08AA-10:** texto final con precio pendiente; el proveedor IA devuelve vacío (`/ia/probar` cuelga).
- **08AA-11 y 08AA-15:** verificación visual y viva del borrador; mismo proveedor IA.
- **169A-1:** fijar `AGENTE_CONTACTO` real en `MN-Inmobiliaria/.env`.
- **169A-3..6 (WhatsApp):** `GLORY_ALERT_GATEWAY_URL` y número admin (`whatsapp_admin`).
- **199A-1/199A-2 (IA central):** `GLORY_API_KEY` real en `.env`; sin ella GloryAPI queda `sin-clave`.

## 6. Desarrollo pendiente (por dependencia)

- **09AA-6 §8 — chat backend:**
  - **09AA-8** webhook fail-closed tras 12AA-1 (§2).
  - **09AA-11** F6a/F6b tras la prueba viva de 09AA-4 (cerrada 2026-10-13). Pendiente: GloryAPI sin reintento (`src/handlers/ia/ia_proveedores.rs:102-153`) y `get("uso")` sin productor localizado.
  - **09AA-12** Decidido 2026-10-12: sin discrepancia reproducible; la prueba viva de QR y polling quedó cerrada 2026-10-13.
- **09AA-14** — scraper: fix en origen, `plugins-opencode/src/burbujas` (chip task_7d226a2c). Copia al lab con sha256 hecha; prueba viva en Facebook cerrada 2026-10-13.
- **10AA-17** — Decidido: sin fallo en main (invalidación por contador en `opencode-propio/src/packages/desktop/src/main/marketplace-service.ts:389-404`). Promoción en §4; prueba viva cerrada 2026-10-13.
- **10AA-24/25/26** — Decidido: el lab procesa ventanas en serie; paralelizar solo si la usuaria lo pide.
- **08AA-8b** — Decidido: el camino principal de marcas no usa geometría (`opencode-propio/.../marketplace-float.ts:671-695`). Prueba viva de la pestaña en fondo cerrada 2026-10-13; trabajo en `opencode-propio-dev`.
- **199A-7** — Decidido 2026-10-12: no implementar. Opcional, sin ficha ni urgencia; el reordenado por DELETE+POST (199A-6) funciona. Se reabre si lo pide la usuaria.
- **299A-3** — pendiente de proveedor IA operativo (§5) para reproducir las dos notas de voz (`src/services/turno.rs:511-517`).
- **279A** — en pausa por decisión de la usuaria (§2): ficha pública sin extras. Sin trabajo pendiente. Si decide mostrar alguno, se abre la revisión lista por lista (`Agente/planes/plan-extras-publicos-2026-10-12.md`).
- **03AA-5** — bloqueado por el corpus de la usuaria (5+2+1 HTML y dataset; `Agente/planes/plan-detector-captacion-2026-10-03.md:42-51`).
- **Fuera de MN:** configurar `coolify-manager-rs` para desplegar proyectos Rust (repo separado).

## 7. Planes vinculados

Cada ítem enlaza su plan en su entrada de la sección 3 o 6. Para el detalle íntegro: `Agente/planes/roadmap-detalle-2026-10-12.md`.
