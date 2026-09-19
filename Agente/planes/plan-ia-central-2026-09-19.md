# Plan 199A-1/199A-2 — Centro de IA: GloryAPI + OpenCode Go vía backend (2026-09-19)

## Objetivo
Centralizar la IA de texto (redacción de fichas + copy; imágenes excluidas) en el
backend con 2 proveedores (GloryAPI y OpenCode Go), y gestionarla desde
Configuración: estado, activar/desactivar, proveedor activo, probar. Decisiones
del usuario: proxy en backend, manual + fallback, claves solo en `.env`.

## Estado inicial verificado
- Front llama directo a GloryAPI (`POST {base}/v1/chat/completions`, `model:auto`,
  key en `VITE_GLORY_API_KEY`): `INMOBILIARIA/src/data/ia/ia.ts:142`,
  `copy-ia.ts:67`. Sin estado en UI.
- `glory-agent` ya habla OpenCode Go (`POST https://opencode.ai/zen/go/v1/responses`,
  Responses API, header `x-opencode-session`, `call_provider` reutilizable,
  `extract_first_text` para parsear). Sin soporte previo de imagen, pero `input`
  es opaco: el backend puede pasar partes `input_text` + `input_image`.
- Fotos del front: dataURL base64 + URLs remotas, tope 4 al enviar (`ia.ts:136`).
- `apiFetch` reutilizable con JWT (`INMOBILIARIA/src/data/inmuebles/api.ts:73`).
- Admin back: cualquier JWT vale para `/api/admin/*` (solo crear usuarios exige
  `owner`). `AppState` tiene `pool`, sin cliente HTTP (crear por llamada).
- Límite: `axum::Json` por defecto 2 MiB → `completar` necesita límite mayor
  (4 fotos base64). `tower-http` disponible.

## Fase A — Backend 199A-1 (`MN-Inmobiliaria`, rama `inmobiliaria`)
1. `src/handlers/ia.rs` (nuevo): rutas `GET /api/admin/ia/estado`,
   `PUT /api/admin/ia/config`, `POST /api/admin/ia/probar`,
   `POST /api/admin/ia/completar`. Registrar en `handlers/mod.rs`
   (`admin_routes`, merge como `chat_staff`).
2. Config persistida con `glory_agent::persistence::{get,set}_config`:
   `ia_activo` (defecto `gloryapi`), `ia_hab_gloryapi` / `ia_hab_opencode_go`
   (defecto `on`); diagnóstico `ia_check_<p>` = `ok|<epoch>|<ms>|<modelo>` o
   `error|<epoch>|<motivo-corto>`.
3. Env: `GLORY_API_URL` (defecto `http://127.0.0.1:3101`), `GLORY_API_KEY`,
   reutilizar `OPENCODE_GO_API_KEY`. Añadir las 3 a `.env.example` (sin valores).
   Claves nunca en logs ni respuestas (regla del provider).
4. `completar {system, texto, fotos[≤4], session?}`: orden [activo, otro];
   saltar deshabilitado/sin clave (motivo registrado); GloryAPI = chat
   completions `model:auto`; OpenCode Go = `call_provider` con input Responses
   (`input_text` + `input_image` con dataURL/URL) y `x-opencode-session` uuid por
   llamada; parseo con `extract_first_text`. Error 503 agregado si ambos fallan.
5. `probar {proveedor}`: llamada mínima real (`responde OK`, pocos tokens),
   guarda `ia_check_<p>`, devuelve latencia/modelo/error.
6. Validar: `cargo fmt --check`, `check`, `clippy -D warnings`, `test`
   (wrappers `scripts/run-cargo.mjs`, target en `C:\tmp`); humo contra BD rama
   (`estado` + `completar` si hay claves en `.env`).

## Fase B — Frontend 199A-2 (`INMOBILIARIA`)
1. `src/domain/ia.ts`: `ProveedorIA`, `EstadoProveedor`, `EstadoIA`,
   `ResultadoCompletar`.
2. `src/data/ia/cliente-ia.ts`: `getEstadoIA`, `guardarConfigIA`,
   `probarProveedor`, `completarIA` vía `apiFetch` (JWT).
3. `ia.ts` / `copy-ia.ts`: transporte al backend (prompts y normalización se
   quedan en el front); `organizarConIA({texto, fotos, signal})`,
   `generarCopyConIA` igual; `modelo` informa `proveedor:modelo`.
   Limpieza: `VITE_GLORY_API_KEY` deja de usarse (anotar en completada).
4. `src/hooks/ia/use-config-ia.ts` (≤120 líneas): estado/guardar/probar.
5. `src/features/configuracion/pestana-ia.tsx` (nuevo, ≤300 líneas): tarjetas
   por proveedor (punto de estado, modelo, latencia, último chequeo/error,
   interruptor habilitar, botón Usar, botón Probar) + nota claves en `.env`.
   `modal-config.tsx` añade pestaña `ia` (grid 4) y la monta; cablear en
   `App.tsx`/`modales-app.tsx` con el hook.
6. Validar: `npm run type-check` + `build` + preview `:5200` 200; probar flujo
   real (redactar + copy) e indicar proveedor usado.

## Definition of Done
- Redacción y copy funcionan vía backend con activo manual y fallback al otro.
- Config muestra por proveedor: habilitado, configurado, estado, latencia,
  último error; permite activar/desactivar, elegir activo y probar.
- Sin claves en front, logs ni respuestas. Commits 199A-1 (back) y 199A-2
  (front). Roadmap actualizado y tarea archivada con evidencia.

## Riesgos
- `POST /v1/responses` con `input_image`: si el relay lo rechaza, degradar
  OpenCode Go a solo-texto y documentarlo (GloryAPI seguiría con fotos).
- Cuerpo grande (fotos): límite explícito en la ruta `completar`.
- Otra sesión en paralelo en `INMOBILIARIA`: no tocar sus archivos; avisar
  antes de commitear el front.
