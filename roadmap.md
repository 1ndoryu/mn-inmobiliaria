# Roadmap — MN-Inmobiliaria

**Contexto.** App de gestión inmobiliaria: backend Rust (`src/`), frontend (`frontend/`) y gateway de WhatsApp (`gateway/`). Se edita en el portátil y se compila, prueba y ejecuta en el VPS 89 (modo `build` por defecto: el backend sirve el front compilado en `:3110`); producción en el VPS 66 vía Coolify con el modo builder de `coolify-manager-rs`. Dev público: `mn-dev.wandori.us` (Cloudflare Access). Stack y comandos: `README.md`. Cada ítem enlaza su plan en su entrada; detalle íntegro del roadmap anterior: `Agente/planes/roadmap-detalle-2026-10-12.md`; cierres por fecha: `Agente/completados/tareas-YYYY-MM-DD.md`.

## Siguiente bloque

- [ ] `21AA-1` — **Verificar en el navegador «Aplicar cambios» con fotos (F4 de 14AA-2).**
  Plan (cerrado, queda F4): `Agente/planes/completados/plan-resolver-diferencias-sync-2026-10-14.md`.
  El resto de 14AA-2 está hecho y probado con tests de integración y en el panel; falta el recorrido final en el navegador contra el remoto de prueba, incluidas las fotos (subir y borrar en los dos sentidos, `remoto::borrar_foto`). Ninguna escritura contra producción sin permiso explícito por operación.


## Pendientes

- [ ] `19AA-2` — **`cargo fmt` y `cargo clippy` fallan en la 89 por deriva del toolchain (heredado, 2026-10-19).**
  `rust-toolchain.toml` fija `channel = "stable"`, sin versión, así que la 89 va con rustc 1.99.0: 31 archivos con diff de `rustfmt` y 36 errores con `cargo clippy --all-targets -- -D warnings` (`assert_is_empty` ×19, `assert_is_not_empty` ×11, `#[must_use]`, `Option<Option<T>>`, backticks en doc…). Ninguno está en archivos tocados hoy y `cargo test` pasa (324/324, 2 ignorados), así que es deriva del linter, no código roto: la regla «antes de cada commit: `cargo fmt --check && cargo clippy`» del README no se puede cumplir hoy. Aparte, `cargo test` deja un aviso del compilador en `src/services/sync/comparacion/comparar.rs:466` (`let mut a` que no se muta): heredado, fuera de los archivos tocados. Salida: fijar una versión en `rust-toolchain.toml` o correr clippy sin `-D warnings`.

- [ ] `19AA-3` — **Fuera de MN: `new-site` rust-aware; el despliegue Rust ya va por el modo builder de `coolify-manager-rs`.**
  Queda `new-site` rust-aware (hoy `--build-mode` vale `laptop`).

- [ ] `19AA-4` — **Dev público en la 89 (S6.3).**
  El dev de MN ya no depende del túnel SSH: se sirve en `mn-dev.wandori.us` (Cloudflare Access) con `MN_DEV_PUBLIC_HOST` y proxy `/rust` (`85042446`, `a7929fb2`; comentarios de `.env.example` hechos, S6.2). Auto-entrada del admin (15AA-1, cerrada el 2026-10-15): el host exacto entra solo, como en local; `Arranque up` copia las claves `VITE_DEV_*` de `envRemoto` al `.env` de la 89 y a `frontend/.env.local` (0600), porque Vite lee su propio `envDir`, no el `.env` de la raíz. Verificado en la 89: `POST /rust/api/auth/login` y `inmobiliaria:token`/`email` en `localStorage`, panel con datos sin pasar por el login. Comprobado 2026-10-15: backend en debug (3110), Vite (5199) y `cloudflared` (20241) corriendo en la 89. Pendiente S6.3: actualizar la skill `taller-remoto` y el README de dev de MN, incluido el puente de WhatsApp del gateway con la 89 (`Agente/documentacion/puente-whatsapp-taller-2026-10-15.md`).

- [ ] `19AA-5` — **Piloto builder (`build_mode=builder`).**
  MN es el sitio piloto: compila en la 89 y despliega en 66. Intento 3 OK el 2026-10-14 (`85d2a73`): `https://mn-inmobiliaria.com/api/health` = 200, contenedor `Up (healthy)` con el ID de la imagen pedida. Pendiente: fase B (ensayo en la 89 con `--sin-envio`), D (respaldo `pre-builder`) y E (producción) — D y E con permiso de la usuaria por operación.

- [ ] `19AA-6` — **Tareas aparte de `coolify-manager-rs`.**
  Arreglar el backup pre-deploy que aborta cuando la app está en bucle (`fases_inicio.rs:22-55`); hacer `new-site` rust-aware (`src/cli/mod.rs:96`). Detalle en el plan del piloto, §G.

- [ ] `19AA-7` — **Alta de proyecto: 8/11.**
  Faltan `quality:check`, `target-dir` en `.cargo/config.toml` y `.runtime/` en `.gitignore` (`Agente/documentacion/alta-proyectos-2026-10-11.md:43`).

- [ ] `19AA-9` — **Sentinel: el contador de líneas efectivas de Rust estaba mal (arreglado, sin propagar).**
  `contarLineasEfectivas` truncaba el archivo en el primer `#[cfg(test)]`; un `#[cfg(test)] use ...` de cabecera dejaba el archivo entero fuera del recuento. `src/handlers/ia/rutas.rs` contaba 12 efectivas en vez de 537, así que `limite-lineas` no disparaba. Arreglado en `glory-sentinel` `4d63473` (`origin/main`), certificado y sin propagar: el pin de los consumidores sigue en `f691510` y subirlo arrastra 14 commits CSS ya commiteados del lote de KAMPLES. Decisión de la usuaria del 2026-10-16: que entre con ese release, no forzarlo ahora. Cuando entre, este proyecto gana un hallazgo (`ia/rutas.rs`) y hay que rehacer la base del gate. Detalle: `Agente/documentacion/integraciones/revision-ia-atencion-cliente-2026-10-16.md:§13`.

## Bloqueos

- `13AA-13` — **Clave de chat estable: falta la prueba viva.** Código integrado en `main` el 2026-10-13 (rama y worktree borrados) y decisión tomada el 2026-10-13: el id de perfil del comprador (`/marketplace/profile/<id>`) es el tercer segmento de la clave. Lo que queda no es trabajo de este repo: la prueba viva con dos chats del mismo nombre, que es de la usuaria. La promoción del puente a opencode-propio (`ventana PROMOCION-VENTANA.md`) queda fuera de este barrido por decisión de la usuaria (2026-10-20).

- `13AA-4` — **Vinculación con Facebook Marketplace bloqueada por la float del lab.** Tabla y endpoints de avisos, modal «Vincular con Facebook» y borradores sin título inseguro hechos (F2–F4); la lectura del perfil pasa a manual (16AA-12, 2026-10-16) y la usuaria vinculó los 13 avisos desde el modal el 2026-10-16. Bloqueo externo: la float debe enviar `avisoId` (F5, tarea F7b, ventana de promoción de la lab).
- `10AA-21` — **Riesgo aceptado (`rsa` 0.9.10, RUSTSEC-2023-0071).** Decidido 2026-10-12. Se reabre al subir `sqlx` sin `sqlx-mysql`; criterio: `rsa` ausente de `Cargo.lock` y `check:back` verde.

- `09AA-8` — **Webhook del gateway, secreto `WA_WEBHOOK_SECRETO`.** Su condición («tras 12AA-1») se cumplió el 2026-10-21 con el cierre de 12AA-1, pero sigue bloqueada por un hecho material: el gateway no corre en el modo `build` de la 89 (solo hay backend 3110 y cloudflared), así que el fail-closed no se puede probar. Trabajo listo para hacer: fijar el secreto en `.env` y en el gateway, y hacer fail-closed `secreto_valido` (`src/services/mensajeria/transporte.rs:95-103`), que hoy acepta cualquier valor si el esperado está vacío. Se desbloquea cuando la usuaria levante el gateway para volver a usar WhatsApp en el taller.

- `299A-3` — **Notas de voz: falta la prueba con notas reales.** El camino está hecho y probado con mocks (`309A-4`, 2026-09-30): el gateway baja el audio, el backend lo archiva en `uploads/whatsapp/<tel>/` y `transcribir_y_anexar` lo pasa a Groq Whisper (`GROQ_API_KEY`). Falta la prueba viva con dos notas de verdad, que necesita `GROQ_LIVE_TEST=1`, clave de Groq y el gateway corriendo con WhatsApp vinculado. El defecto de tipo MIME que podía perder el audio (`.bin`) quedó corregido el 2026-10-21.

- `15AA-6` — **WhatsApp del taller: prueba viva del número C.** Código en `main` (2026-10-21): C archiva sin responder, `reparto` reconoce C y la foto de la IA puede salir por `MEDIA_BASE_URL`. Falta lo que no depende del código: que el gateway corra y que el número C esté configurado (`agent_config wa_numero_c` o `WA_NUMERO_C`); sin él el webhook sigue respondiendo 400 `numero_destino desconocido`. Mismo desbloqueo que `09AA-8`.

- `08AA-30` — **Filas duplicadas en `mp_respuestas_cache`.** Decidido «Dejarlas» (161 filas, 48 `thread_id` repetidos, 82 sobrantes). No se borran; la unificación de `thread_id` sigue pendiente.

- `289A-5` — **Sesión `739bb63e` con `devolver_a_ia`.** Decidido «No, pendiente». No se reintenta ahora.

- `03AA-3` — **Asistente Marketplace.** Falta la firma de la usuaria para M1, el remoto y la fecha.

- `279A-4..8` — **Extras públicos.** Decidido 2026-10-12 (la usuaria): la ficha pública no muestra ningún extra, tampoco el precio negociable (revierte la decisión previa de publicarlo). Si mañana decide mostrar algo, se revisa lista por lista. La lista candidata y el criterio quedan en `Agente/planes/plan-extras-publicos-2026-10-12.md`. La IA sigue leyéndolos (ver `279A`).

- `279A` — **Ficha pública sin extras.** En pausa por decisión de la usuaria (arriba): sin trabajo pendiente. Si decide mostrar alguno, se abre la revisión lista por lista (`Agente/planes/plan-extras-publicos-2026-10-12.md`).

- `239A-1` — **Deploy, Fase 5.4.** Confirmar la password admin rotada (o pedir otra).

- `169A-1` — **`AGENTE_CONTACTO`.** Confirmar el valor real que va en `MN-Inmobiliaria/.env` (no se muestra aquí).

- **Entrada de personal en producción.** Acción de la usuaria (entrar una vez en la app); el agente no entra en producción.

- `wa_c` — **Tercer número de WhatsApp.** Decidido 2026-10-13 (la usuaria): no responde solo a clientes; se activa hoy. Código listo y gateway reiniciado. Falta que se escanee el código QR `gateway/qr-wa_c.png` con el teléfono del número nuevo (WhatsApp → Dispositivos vinculados → Vincular). El código caduca en ~1 min: si caduca, el gateway crea otro en el mismo archivo. Ningún envío desde este número sin permiso de la usuaria.

- `08AA-10` — **Texto final con precio.** Pendiente; el proveedor IA devuelve vacío (`/ia/probar` cuelga).

- `08AA-11` y `08AA-15` — **Verificación visual y viva del borrador.** Mismo proveedor IA.

- `169A-3..6` — **WhatsApp.** `GLORY_ALERT_GATEWAY_URL` y número admin (`whatsapp_admin`).

- `199A-1`/`199A-2` — **IA central.** `GLORY_API_KEY` real en `.env`; sin ella GloryAPI queda `sin-clave`.

- `03AA-5` — **Detector de captación.** Bloqueado por el corpus de la usuaria (5+2+1 HTML y dataset; `Agente/planes/plan-detector-captacion-2026-10-03.md:42-51`).

- `10AA-17` — **Invalidación por contador (promoción).** Decidido: sin fallo en `main` (`opencode-propio/src/packages/desktop/src/main/marketplace-service.ts:389-404`). Promoción en la ventana de la usuaria; prueba viva cerrada 2026-10-13.

- **Promoción a opencode-propio (ventana de la usuaria).** Cola viva en `opencode-propio-dev/PROMOCION-VENTANA.md` («Pendiente por promover»): `09AA-14` (timeout 90 s del flotante), `09AA-19` (F7b, F1, F6 y F5), `08AA-9` (botón Releer), `03AA-3` y `07AA-6` (C1b cancelado por ella 2026-10-07, `Agente/planes/plan-flujo-lab-opencode-propio-2026-10-07.md:100`; queda la promoción en su ventana, la prueba viva de 07AA-6 se cerró 2026-10-13), `07AA-8` (V4 de `marketplace-float.ts`) y `13AA-2` (saludo por franja en burbujas, lab 2026-10-13; detalle en `opencode-propio-dev/PROMOCION-VENTANA.md` «0. 13AA-2», prueba viva cerrada 2026-10-13). Decidido 2026-10-13 (la usuaria): no reiniciar el lab; el cambio de 13AA-2 se activa en su próximo reinicio, cuando ella lo decida.

## Planes activos

- `Agente/planes/plan-clave-chat-estable-2026-10-13.md` — `13AA-13` — clave de chat estable por conversación (Marketplace).
- `Agente/planes/plan-vinculo-marketplace-2026-10-13.md` — `13AA-4` — vincular inmuebles con avisos de Facebook Marketplace.
- `Agente/planes/plan-whatsapp-2026-09-27.md` — `12AA-1` (cerrada el 2026-10-21); cubre los restos abiertos del gateway (`09AA-8`, `15AA-6`) — dos modos, delegación, clientes y observabilidad.
- `Agente/planes/plan-chat-inmobiliaria-2026-09-16.md` — `169A-1` — chat del visitante con IA (widget público).
- `Agente/planes/plan-solicitudes-2026-09-16.md` — `169A-2` — «Publicar mi inmueble» (solicitudes públicas).
- `Agente/planes/plan-ia-central-2026-09-19.md` — `199A-1` — IA central: estado, proveedor y prueba.
- `Agente/planes/plan-deploy-produccion-2026-09-23.md` — `239A-1` — despliegue en producción (fase 5.4 pendiente).
- `Agente/planes/plan-optimizacion-pagespeed-seo-2026-09-24.md` — `279A` — PageSpeed y SEO profundo (medición pendiente).
- `Agente/planes/plan-bateria-escenarios-ia-2026-09-29.md` — `199A-1` — batería de escenarios IA por WhatsApp (bloqueada por la clave).
- `Agente/planes/plan-detector-captacion-2026-10-03.md` — `03AA-5` — detector de captación (bloqueado hasta C0).
- `Agente/planes/plan-asistente-marketplace-separacion-2026-10-03.md` — `03AA-3` — asistente de Marketplace en repo propio.
- `Agente/planes/plan-flujo-lab-opencode-propio-2026-10-07.md` — `07AA-6` — flujo lab ↔ opencode-propio (promoción).
- `Agente/planes/plan-burbujas-estructuradas-2026-10-09.md` — `09AA-19` — burbujas estructuradas (promoción).
- `Agente/planes/plan-cache-inmueble-automatica-2026-10-09.md` — `09AA-30` — caché compartida por inmueble y detección automática.
- `Agente/planes/plan-cache-mensajes-inmueble-2026-10-09.md` — `09AA-30` — caché de mensajes por inmueble (parcialmente superado; se archiva al cerrar 09AA-30).
- `Agente/planes/plan-extras-publicos-2026-10-12.md` — `279A` — extras públicos (en pausa; se reabre si la usuaria lo decide).
