# Roadmap — MN-Inmobiliaria

**Contexto.** App de gestión inmobiliaria: backend Rust (`src/`), frontend (`frontend/`) y gateway de WhatsApp (`gateway/`). Se edita en el portátil y se compila, prueba y ejecuta en el VPS 89 (modo `build` por defecto: el backend sirve el front compilado en `:3110`); producción en el VPS 66 vía Coolify con el modo builder de `coolify-manager-rs`. Dev público: `mn-dev.wandori.us` (Cloudflare Access). Stack y comandos: `README.md`. Cada ítem enlaza su plan en su entrada; detalle íntegro del roadmap anterior: `Agente/planes/roadmap-detalle-2026-10-12.md`; cierres por fecha: `Agente/completados/tareas-YYYY-MM-DD.md`.

## Siguiente bloque

- [ ] `14AA-2` — **Resolver diferencias de sync por inmueble (botón «Resolver», elección por campo, imágenes lado a lado).**
  Plan: `Agente/planes/plan-resolver-diferencias-sync-2026-10-14.md`. Estado: F1 hecha (plan de lectura y panel claro, sin escrituras; `services::sync` 32/32, type-check y lint OK). F5a–F5c (publicar y traer el inmueble completo) implementadas en `main` (`7b991fe4`, `28786374`). F5d hecha el 2026-10-14 (`f109e8b0`): test de integración `tests/sync_f5d.rs` contra dos BD temporales (`DATABASE_URL`, nunca `PROD_BASE`), cubre publicar, traer, slug ocupado y fallo a mitad de fotos; sin `DATABASE_URL` falla en vez de omitirse. De F5d queda la comprobación en navegador contra el remoto de prueba. F2 hecha el 2026-10-16 (16AA-2): la usuaria probó el panel y el botón «Aplicar cambios» estaba deshabilitado; ya está el `POST /api/admin/sync/resolver/{slug}/aplicar` (plan antes de escribir y 409 si el `plan_id` no coincide, D3; cerrojo por slug, D15) y el diálogo que nombra el servidor antes de escribir (D4/D16) y enseña después lo escrito, lo pendiente y el residuo (D5). Verificado con la matriz `tests/sync_aplicar.rs` contra servidor simulado y en el panel real (confirma y no escribe sin pulsar «Aplicar ahora»). F3 hecha el 2026-10-16 (16AA-3): «Aplicar cambios» mueve también las fotos, por bytes y en los dos sentidos (`remoto::borrar_foto` es nuevo); el borrado va de mayor a menor orden y la `mejorada` antes que su `original`, para no pisar la renumeración en cascada, y el contador del plan cuenta solo lo que de verdad se puede escribir. Verificado con `tests/sync_aplicar.rs` (foto subida y luego borrada, con `Estado::Igual` tras cada paso), `sync_f5d` y `fotos_orden`. Pendiente: F4 (verificación end-to-end en el navegador). Ninguna escritura contra producción sin permiso explícito de la usuaria por operación (plan D4).

- [ ] `13AA-13` — **Clave de chat estable por conversación (Marketplace).**
  Plan: `Agente/planes/plan-clave-chat-estable-2026-10-13.md`. Integrada en `main` (MN) el 2026-10-13; rama y worktree borrados. Estado: integrada; gate fuera de banda sin hallazgos nuevos. Decidido 2026-10-13 (la usuaria): id = id de perfil del comprador (`/marketplace/profile/<id>`), tercer segmento de la clave (plan §6). F0 resuelta por CDP (solo lectura). Pendiente de hechos externos: prueba viva con dos chats del mismo nombre (la usuaria) y promoción del puente a opencode-propio (ventana `PROMOCION-VENTANA.md`; regenera `vendor/burbujas.iife.js`).

- [ ] `13AA-4` — **Vincular inmuebles con sus avisos de Facebook Marketplace.**
  Plan: `Agente/planes/plan-vinculo-marketplace-2026-10-13.md`. Estado: en ejecución; la lectura del perfil pasa a manual (16AA-12, hecha el 2026-10-16). Hechos: tabla y endpoints de avisos, modal «Vincular con Facebook» y borradores sin título inseguro (F2–F4); lectura del perfil en la copia de pruebas de la lab (F1, verificada contra Facebook real el 2026-10-13 y de nuevo el 2026-10-16 a petición desde el modal: 13 avisos con foto). Verificado por la usuaria el 2026-10-16: vinculó los 13 avisos desde el modal. Bloqueo externo: la float debe enviar `avisoId` (F5, tarea F7b, ventana de promoción de la lab). Rama `13aa-4-vinculo-marketplace`: integrada en `main` y borrada en local el 2026-10-13. La carpeta principal vuelve a `main`; los cambios sin guardar de publica/detalle y superficie eran idénticos a `main` (respaldo en `C:/tmp/rescate-wip-13aa-4-2026-10-13`).

- [ ] `12AA-1` — **Gateway WhatsApp (3102), cierre por plazo.**
  Mitigación en `gateway/src/sesion.mjs` (`fireInitQueries: false`, backoff 5→60 s). Cierra si hasta 2026-10-13 05:12 local (09:12Z) no hay `ENOTFOUND` ni 428 repetidos con `wa_a`/`wa_b` estables. Comprobación 2026-10-13 (investigador, `logs/dev-up-MN-Inmobiliaria.log`): no cumple el criterio. Desde 09:12Z: 2 `ENOTFOUND` (último 10:28Z) y 2 `428` (A y B, un corte entre 09:59Z y 10:28Z). Desde 10:28Z ambas sesiones quedan abiertas, sin bucle. Tras el último reinicio (12:36Z) solo hay ~6 min de observación: no hay base para cerrar. Reinicio ~13:19Z para añadir wa_c: A y B reconectaron solas; la observación cuenta desde ahí. Corte por error (2026-10-13): `Arranque stop "MN-Inmobiliaria"` paró también el gateway (3102) y el frontend (5199); se reanudaron con `Arranque up "MN-Inmobiliaria"` (3102 pid 4532, 5199 pid 25540). Cuenta como corte para el criterio. Estado: abierta. Decidido 2026-10-13 (la usuaria): dejarla abierta y revisar el registro unas horas después; cerrar solo si no hay cortes en ese tiempo. Pendiente: resolución DNS de `web.whatsapp.com`. Detalle: `Agente/planes/roadmap-detalle-2026-10-12.md` (entrada 12AA-1).
  Decidido 2026-10-12 (la usuaria): la tarea programada del 13/10 05:15 que lo cerraba se cancela. El cierre queda manual: comprobar el gateway y decidir a mano.

- [ ] `15AA-12` — **El panel no alcanza el servidor de mejora desde que el dev está en la 89.**
  El panel llama a `/api/mejora` y el proxy de Vite (`frontend/vite.config.ts:35`) lo manda a `127.0.0.1:3122`, que en la 89 no escucha: el servidor de mejora solo puede correr en el portátil (necesita Chrome y las cookies de Gemini). Síntoma: «Backend no disponible» en la pestaña Imágenes y ningún «Reintentar» funciona. Mitigado a mano el 2026-10-15 con `ssh -f -N -R 3122:127.0.0.1:3122 dev89` (verificado: la 89 responde `{"listo":true}` por su 127.0.0.1:3122 y el panel pasa a «Backend listo»). Pendiente: que no dependa de un túnel a mano — o un `frontend/scripts/arrancar-mejora.mjs` que abra Chrome, renueve cookies, levante el túnel inverso y arranque el `:3122` (mismo patrón que el lanzador de la lab, 15AA-9), o que el panel hable directo con `127.0.0.1:3122` (CORS en `server/mejora.mjs` + `VITE_MEJORA_BASE`). Decidir cuál al empezar.

- [ ] `18AA-8` — **El borrador no tiene tope global: su peor caso son ~285 s.**
  `POST /api/admin/marketplace/borrador` (`src/handlers/marketplace/borrador.rs`) no lleva `TimeoutLayer` en el Router y la IA se da 45 s de vía rápida más dos intentos de 120 s (`src/handlers/ia/ia_proveedores.rs:216,337`), así que un fallo de caché puede tardar casi cinco minutos y el cliente tiene que esperar todo ese rato. Medido el 2026-10-19: 750 peticiones (p50 5,5 s, p99 27,7 s, máx 29,5 s) y 15 abortadas por el cliente; glory-web cortaba a los 30 s y lo pintaba como «MN no responde» (arreglado allí subiendo el tope a 300 s, 18AA-7 de glory-web). Arreglo: un presupuesto total para el handler (p. ej. una sola pasada de 120 s) o devolver el trabajo como tarea y consultarlo.

## Pendientes

- [ ] `16AA-14` — **La sección «Ficha /ask» del modal admin no guarda (encontrado el 2026-10-16 al hacer 16AA-13).**
  Se editan `extras` y el mínimo en el modal de inmueble, pero `conFicha` (`frontend/src/hooks/inmuebles/use-ficha-inmueble.ts:35`) los fusiona en el `Inmueble` y `dominioACuerpo` (`frontend/src/data/inmuebles/api.ts:314`) no los manda; `UpdateInmuebleRequest` (`src/models/inmueble.rs:315`) no tiene ninguno de los dos campos, así que el PUT los descarta en silencio. El único escritor de la ficha es `PUT /api/admin/inmuebles/{id}/ficha` (`set_ficha`), que hoy solo usa /ask. Decidir al empezar: que el modal llame a `guardarFicha` al guardar (una llamada extra, mismo endpoint que /ask) o añadir `extras`/`precio_minimo` al PUT. Sin datos perdidos: 0 de 20 inmuebles tenían algo guardado por esa vía.

- [ ] `09AA-6` — **Chat backend (§8).**
  - `09AA-8` webhook fail-closed tras 12AA-1 (ver Bloqueos).
  - `16AA-9 F5`, aparte — el contrato del asistente de Marketplace (2026-10-16). Valorado al cerrar 16AA-9: su prompt (`marketplace/borrador.rs`, `continuacion.rs`) sí merece subir a `config/ia.rs` cuando se toque esa superficie —interpola runtime, por eso F2 no se lo llevó—; su contador **no**, porque ya registra tokens exactos en `mp_respuestas_cache` y la vista `ia_uso` los hace sumables. Escribir en `uso_mensajes` mezclaría una generación de borrador con turnos de conversación. Sin bucle de tools.
  - `09AA-11` F6a/F6b tras la prueba viva de 09AA-4 (cerrada 2026-10-13). Pendiente: GloryAPI sin reintento (`src/handlers/ia/ia_proveedores.rs:102-153`) y `get("uso")` sin productor localizado.

- [ ] `15AA-6` — **WhatsApp del taller (restos, 2026-10-15).**
  El puente gateway(laptop)↔backend(89) está cerrado y en marcha (`Agente/documentacion/puente-whatsapp-taller-2026-10-15.md`). Prueba viva del 2026-10-15 19:11: 13 fotos mandadas al número A entraron por el webhook y el backend de la 89 las archivó en `uploads/whatsapp/5214427789328/`; se engancharon al inmueble `5a5cd58e-68ff-4f3b-9275-ef1a28e49a10` con `POST /api/admin/fotos/upload` (13 filas, orden 0–12, miniaturas `min160-` generadas). Queda: (a) una foto mandada a las vías mudas B y C no se archiva sola (`gateway/src/sesion.mjs:150` la descarta antes de bajar media; hoy se recupera con `gateway/src/rescatar-historial.mjs --sesion=wa_c` y una copia a mano a la 89) — falta que C archive sin responder; (b) `reparto` solo acepta el número A, así que un webhook con destino C responde 400 `numero_destino desconocido` (`src/services/mensajeria/transporte.rs`); (c) la foto que envía la IA (`src/handlers/chat/tools/fotos.rs:71`) usaría `PUBLIC_BASE_URL` público, detrás de Cloudflare Access: camino dormido.


- [ ] `299A-3` — **Notas de voz.**
  Pendiente de proveedor IA operativo (ver Bloqueos) para reproducir las dos notas de voz (`src/services/turno.rs:511-517`).

- [ ] `16AA-6` — **Tope de borradores del marketplace y la lab (2026-10-16).**
  Las burbujas de la lab agotan el tope de MN (`TOPE_BORRADOR_MINUTO = 30` por sub, `src/handlers/marketplace/borrador.rs:9`) y reciben `429` en ráfagas (≈90 peticiones/min observadas en `C:\tmp\lab-dev.log`); la lab ignora el `Retry-After` y cae a su plantilla local, así que el borrador que ve la usuaria empeora sin avisar (`src/packages/desktop/src/main/marketplace-nucleo.ts:216`). Sin efecto en el latido del panel (`borradores/version` no tiene tope). Decidir al empezar: que la lab respete `Retry-After` y agrupe por ventana, o subir/eximir el sub de la lab (`MP_SIN_LIMITE_SUB`, `src/services/marketplace/chat/tokens.rs:57`) — `MP_SIN_LIMITE_SUB` y `MP_CLI_MINUTOS` ya viajan en el `envRemoto` de la ficha desde el 2026-10-19. Cualquier cambio en la lab pasa por la ventana de promoción (ver Bloqueos).

- [ ] `19AA-1` — **`continuacion.rs:acotar` corta mal en dos casos (revisión 2026-10-19, sin cambio).**
  `src/services/marketplace/continuacion.rs:97-107`. El corte por bytes es seguro: `rfind` de un patrón ASCII devuelve frontera de carácter válida. Dos defectos reales, ninguno grave: si la ventana de `max` caracteres no tiene puntuación alguna, corta a mitad de palabra; y una abreviatura (`Av.`, `Sr.`) da la frase por terminada antes de tiempo. Salida: cortar por el último espacio cuando no haya puntuación, y no contar el `.` de una abreviatura conocida.

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

- `10AA-21` — **Riesgo aceptado (`rsa` 0.9.10, RUSTSEC-2023-0071).** Decidido 2026-10-12. Se reabre al subir `sqlx` sin `sqlx-mysql`; criterio: `rsa` ausente de `Cargo.lock` y `check:back` verde.

- `09AA-8` — **Webhook del gateway, secreto `WA_WEBHOOK_SECRETO`.** Decidido «tras 12AA-1» (2026-10-12). Al cerrar 12AA-1: fijar el secreto en `.env` y en el gateway, y hacer fail-closed `secreto_valido` (`src/services/mensajeria/transporte.rs:95-103`), que hoy acepta cualquier valor si el esperado está vacío.

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

- **Modo `build` por defecto y el panel de mejora (16AA-18/16AA-19, 2026-10-17).** Decisión sin tomar: con `build` el panel de mejora de fotos se queda sin camino —pide `/api/mejora` relativo y en `build` eso lo responde el backend Rust, que no tiene esa ruta; solo existe en `frontend/server/mejora.mjs`, auxiliar de arranque manual fuera del mando—. La ficha ya sirve compilado: el front va a `frontend/dist` y lo sirve el backend en `:3110` (`STATIC_DIR=frontend/dist`), sin vite; `Arranque up "MN-Inmobiliaria" --dev` lo levanta en desarrollo solo para esa sesión. El túnel inverso al 3122 que servía en `dev` tampoco vale. Salidas: usar `--dev` cuando se necesite el panel, o darle al Rust un proxy `/api/mejora` → `127.0.0.1:3122`. No bloquea nada más. Detalle: skill `publicar-inmuebles`, §«El panel y el `:3122`».

## Planes activos

- `Agente/planes/plan-resolver-diferencias-sync-2026-10-14.md` — `14AA-2` — resolver diferencias por inmueble (botón «Resolver»).
- `Agente/planes/plan-clave-chat-estable-2026-10-13.md` — `13AA-13` — clave de chat estable por conversación (Marketplace).
- `Agente/planes/plan-vinculo-marketplace-2026-10-13.md` — `13AA-4` — vincular inmuebles con avisos de Facebook Marketplace.
- `Agente/planes/plan-whatsapp-2026-09-27.md` — `12AA-1` — gateway de WhatsApp y atención por IA.
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
