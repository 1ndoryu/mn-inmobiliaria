# Roadmap — MN-Inmobiliaria

**Contexto.** App de gestión inmobiliaria: backend Rust (`src/`), frontend (`frontend/`) y gateway de WhatsApp (`gateway/`). Se edita en el portátil y se compila, prueba y ejecuta en el VPS 89 (modo `build` por defecto: el backend sirve el front compilado en `:3110`); producción en el VPS 66 vía Coolify con el modo builder de `coolify-manager-rs`. Dev público: `mn-dev.wandori.us` (Cloudflare Access). Stack y comandos: `README.md`. Cada ítem enlaza su plan en su entrada; detalle íntegro del roadmap anterior: `Agente/planes/roadmap-detalle-2026-10-12.md`; cierres por fecha: `Agente/completados/tareas-YYYY-MM-DD.md`.

## Siguiente bloque

- Ninguno.

## Pendientes

- [ ] `19AA-6` — **Backup pre-deploy que no aborta con la app en bucle.** Hecho cuando `coolify-manager-rs` (`fases_inicio.rs:22-55`) deja de abortar el backup si la app está en bucle, y un deploy de prueba en la 89 lo confirma. Detalle en el plan del piloto, §G.
- [ ] `08AA-10` — **Texto final con precio.** Pendiente; el proveedor IA devuelve vacío (`/ia/probar` cuelga).
- [ ] `08AA-11` — y `08AA-15` — **Verificación visual y viva del borrador.** Mismo proveedor IA.

## Bloqueos

- Bloqueo `03AA-5` — detector de captación en Marketplace sobre glory-web (decidido «Hacer» 2026-10-25; plan `Agente/planes/plan-detector-captacion-2026-10-03.md`). C0.1 entregada el 2026-10-24. Hecho cuando el corpus de 5 avisos + 2 perfiles + 1 búsqueda, sacado con la usuaria, alimenta el clasificador. Se desbloquea con ese corpus. La captación por chat (`E14`, `registrar_captacion`) ya está cerrada.
- Bloqueo `19AA-9` — Sentinel: contador de líneas de Rust arreglado en `glory-sentinel` `4d63473`, sin propagar. Decisión de la usuaria (2026-10-16): entra con ese release cuando ella lo decida. Entonces se sube el pin y este proyecto gana un hallazgo (`ia/rutas.rs`) que hay que corregir. Detalle: `Agente/documentacion/integraciones/revision-ia-atencion-cliente-2026-10-16.md:§13`.
- Bloqueo `13AA-13` — clave de chat estable: código en `main` desde el 2026-10-13; falta la prueba viva con dos chats del mismo nombre, que es de la usuaria.
- Bloqueo `299A-3` — notas de voz: falta la prueba viva con notas reales (`GROQ_LIVE_TEST=1`, clave de Groq y gateway con WhatsApp vinculado).
- Bloqueo `15AA-6` — WhatsApp del taller, número C: falta que el gateway corra y que el número C esté configurado (`agent_config wa_numero_c` o `WA_NUMERO_C`). Prueba viva del webhook fail-closed (`09AA-8`, hecho en código): sin secreto el webhook responde 401; con él, 200. Antes del próximo deploy, `WA_WEBHOOK_SECRETO` debe estar en producción (VPS 66) o el canal de WhatsApp deja de recibir mensajes.
- Bloqueo `03AA-3` — asistente Marketplace: falta la firma de la usuaria para M1, el remoto y la fecha.
- Bloqueo `239A-1` — deploy, fase 5.4: confirmar la password admin rotada (o pedir otra).
- Bloqueo `169A-1` — `AGENTE_CONTACTO`: confirmar el valor real que va en `MN-Inmobiliaria/.env` (no se muestra aquí).
- Bloqueo `169A-3..6` — WhatsApp: `GLORY_ALERT_GATEWAY_URL` y número admin (`whatsapp_admin`).
- Bloqueo `199A-1` / `199A-2` — IA central: `GLORY_API_KEY` real en `.env`; sin ella GloryAPI queda `sin-clave`.
- Ref. `10AA-21` — **Riesgo aceptado (`rsa` 0.9.10, RUSTSEC-2023-0071).** Decidido 2026-10-12. Se reabre al subir `sqlx` sin `sqlx-mysql`; criterio: `rsa` ausente de `Cargo.lock` y `check:back` verde.
- Ref. `08AA-30` — **Filas duplicadas en `mp_respuestas_cache`.** Decidido «Dejarlas» (161 filas, 48 `thread_id` repetidos, 82 sobrantes). No se borran; la unificación de `thread_id` sigue pendiente.
- Ref. `289A-5` — **Sesión `739bb63e` con `devolver_a_ia`.** Decidido «No, pendiente». No se reintenta ahora.
- Ref. `279A-4..8` — **Extras públicos.** Decidido 2026-10-12 (la usuaria): la ficha pública no muestra ningún extra, tampoco el precio negociable (revierte la decisión previa de publicarlo). Si mañana decide mostrar algo, se revisa lista por lista. La lista candidata y el criterio quedan en `Agente/planes/plan-extras-publicos-2026-10-12.md`. La IA sigue leyéndolos (ver `279A`).
- `279A` — **Ficha pública sin extras.** En pausa por decisión de la usuaria (arriba): sin trabajo pendiente. Si decide mostrar alguno, se abre la revisión lista por lista (`Agente/planes/plan-extras-publicos-2026-10-12.md`).
- **Entrada de personal en producción.** Acción de la usuaria (entrar una vez en la app); el agente no entra en producción.
- `wa_c` — **Tercer número de WhatsApp.** Decidido 2026-10-13 (la usuaria): no responde solo a clientes; se activa hoy. Código listo y gateway reiniciado. Falta que se escanee el código QR `gateway/qr-wa_c.png` con el teléfono del número nuevo (WhatsApp → Dispositivos vinculados → Vincular). El código caduca en ~1 min: si caduca, el gateway crea otro en el mismo archivo. Ningún envío desde este número sin permiso de la usuaria.

## Planes activos

- `Agente/planes/plan-clave-chat-estable-2026-10-13.md` — `13AA-13` — clave de chat estable por conversación (Marketplace).
- `Agente/planes/plan-whatsapp-2026-09-27.md` — `12AA-1` (cerrada el 2026-10-21); cubre los restos abiertos del gateway (`09AA-8`, `15AA-6`) — dos modos, delegación, clientes y observabilidad.
- `Agente/planes/plan-chat-inmobiliaria-2026-09-16.md` — `169A-1` — chat del visitante con IA (widget público).
- `Agente/planes/plan-solicitudes-2026-09-16.md` — `169A-2` — «Publicar mi inmueble» (solicitudes públicas).
- `Agente/planes/plan-ia-central-2026-09-19.md` — `199A-1` — IA central: estado, proveedor y prueba.
- `Agente/planes/plan-deploy-produccion-2026-09-23.md` — `239A-1` — despliegue en producción (fase 5.4 pendiente).
- `Agente/planes/plan-optimizacion-pagespeed-seo-2026-09-24.md` — `279A` — PageSpeed y SEO profundo (medición pendiente).
- `Agente/planes/plan-bateria-escenarios-ia-2026-09-29.md` — `199A-1` — batería de escenarios IA por WhatsApp (bloqueada por la clave).
- `Agente/planes/plan-detector-captacion-2026-10-03.md` — `03AA-5` — detector de captación (bloqueado hasta C0).
- `Agente/planes/plan-asistente-marketplace-separacion-2026-10-03.md` — `03AA-3` — asistente de Marketplace en repo propio.
- `Agente/planes/plan-cache-inmueble-automatica-2026-10-09.md` — `09AA-30` — caché compartida por inmueble y detección automática.
- `Agente/planes/plan-cache-mensajes-inmueble-2026-10-09.md` — `09AA-30` — caché de mensajes por inmueble (parcialmente superado; se archiva al cerrar 09AA-30).
- `Agente/planes/plan-extras-publicos-2026-10-12.md` — `279A` — extras públicos (en pausa; se reabre si la usuaria lo decide).
