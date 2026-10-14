# Plan 14AA-2 — Resolver diferencias de sync por inmueble

Estado: **F1 hecha.** Verificación: `node scripts/run-cargo.mjs test services::sync` 32/32; type-check y lint del frontend sin errores. Prueba funcional (14-oct): servidor simulado en `127.0.0.1:4555` (solo lectura) con un título y una foto distintos en un inmueble; `plan` devolvió 200 con `servidor_adelante`, por defecto 2 cambios en Local; «Conservar lo viejo» en todo pasa a 2 en Servidor; solo título viejo, 1 y 1; sin `SYNC_REMOTO_ENV` responde 409. En el panel: contadores, textos y miniatura local (carga) correctos; «Aplicar» sigue deshabilitado (F2). No hubo escrituras remotas. Sin base previa (`estimado`) no se probó en vivo: cubierto por tests. Siguiente paso: F2. Base: 13AA-12 (`plan-sync-inmuebles-2026-10-13.md`), que dejó «subir/traer» y las escrituras fuera de alcance; este plan las recoge.

## 1. Objetivo

Botón «Resolver» en el panel «Diferencias» de cada inmueble. Muestra, campo a campo, qué cambia en cada lado. Por defecto elige el valor más reciente; el usuario puede elegir conservar el viejo. Los campos de imagen se comparan lado a lado. Al aplicar, las dos copias quedan iguales.

## 2. Alcance y no alcance

- **Dentro:** campos del inmueble (20 huellados, `src/services/sync/huella.rs:30-51`), ficha (`extras`, `precio_minimo`), publicación (`publicado`), estado, y fotos (`foto:<origen>:<orden>`) en ambas direcciones.
- **Fuera:** inmuebles solo en un lado (crear o traer el inmueble completo) → plan aparte. Reordenar fotos sin borrar y volver a subir → no existe endpoint de reordenación.
- **Fuera en esta sesión:** ejecutar escrituras contra producción. Ver D4.

## 3. Hechos verificados (fuente)

- El panel solo lee: `frontend/src/features/inmuebles/modal-diferencias-sync.tsx:20-25` pinta las fotos como texto.
- `valor_de` (`src/services/sync/detalle.rs:110-125`) emite `presente (<8 hex>)`; el modal esperaba `presente` / `sin-archivo` → **la rama de foto nunca se cumplía** (verificado). Corregido en F1 (`describirFoto`).
- Las escrituras locales ya llaman `notificar()` (`src/services/inmueble.rs` importa `vigilancia::notificar`): el panel se refresca solo.
- Escrituras disponibles: `PUT /api/admin/inmuebles/{id}` (núcleo), `PUT .../ficha`, `PATCH .../publicacion`, `PATCH .../estado`, `POST /api/admin/fotos/upload?orden&origen` (bytes → `storage_key`), `POST /api/admin/inmuebles/{id}/fotos` (JSON `storage_key, orden, origen`), `DELETE /api/admin/fotos/{id}` (**renumera** los `orden` posteriores: `src/repositories/inmueble.rs:497-511`).
- `origen` ∈ {`original`, `mejorada`} (CHECK en BD). `mejorada` se compara solo por presencia; `original` por sha256.
- Cliente remoto (`src/services/sync/remoto.rs`): **solo GET**. Token en memoria, renovación en 401 (`:213-219`). Hay que añadir métodos de escritura.
- `GET /uploads/{storage_key}` es público (sin JWT). `FotoPublica.url` es relativa (`/uploads/...`), viene de `GET /api/admin/inmuebles/{id}` (JWT).
- Tests con Postgres (`pool_si_hay`, `src/config/mod.rs:63-75`) **se omiten sin BD de pruebas y pasan en vacío**: F2–F3 exigen BD de pruebas real.

## 4. Decisiones (Decidido)

- **D1 — «Lo último» por defecto.** Campo con `lado` local o servidor → ese lado (solo uno cambió desde la base). Lado `ambos`, o sin base → lado del inmueble con `updated_at` más reciente. La UI indica el criterio («fecha del inmueble»): no hay fecha por campo.
- **D2 — «Conservar lo viejo».** Elegir el otro lado para ese campo. Es binario por campo.
- **D3 — Plan antes de aplicar.** `POST /api/admin/sync/resolver/{slug}/plan` devuelve las operaciones exactas y un `plan_id` (hash de operaciones + revisión de cada lado). `POST .../aplicar` exige `plan_id`; si algo cambió desde el plan responde 409 y hay que volver a ver el plan.
- **D4 — Producción.** Aplicar hacia producción exige un clic con confirmación que nombra producción y el número de operaciones. Cada ejecución real contra producción requiere permiso explícito de la usuaria (regla 13AA-12 «cada operación remota se declara al ejecutarla» y AGENTS §9.1). En esta sesión no se ejecuta ninguna escritura contra producción.
- **D5 — Fotos.** Como no hay reordenación y el borrado renumera, las operaciones de foto se calculan sobre el estado final esperado. Tras aplicar se relee ambos lados y se recalcula; el residuo se informa, no se reintenta a ciegas.
- **D6 — Bytes.** Servidor → local: `GET {PROD_BASE}/uploads/...` (público) y subida local. Local → servidor: leer del almacenamiento local y subir con `POST /fotos/upload`.
- **D7 — Miniaturas.** Local: `/uploads/...` relativo (proxy de Vite a 3110). Servidor: URL absoluta `PROD_BASE + url`, obtenida con `GET /api/admin/inmuebles/{remote_id}` (con JWT). El detalle remoto no trae URLs y el servidor no está desplegado con cambios.
- **D8 — Endpoint de plan (F1).** `POST /api/admin/sync/resolver/{slug}/plan` con `{"conservar_viejo": [claves]}`. Solo calcula. Sin remoto o sin el inmueble allí → 409. Las miniaturas se piden solo si hay claves `foto:` (evita dos peticiones extra al servidor).
- **D9 — Proxy de Vite (F1): sin cambio.** `frontend/vite.config.ts` manda `/api` a 3122 (mini-backend de mejora, `frontend/server/mejora.mjs`). El cliente de la API usa `API_URL` absoluta (3110, `src/data/inmuebles/api.ts:12`) y ningún `fetch` de `src` usa `/api` relativo, así que el proxy no afecta a sync. No se toca.
- **D10 — Panel (F1).** El modal pide el plan al abrir y al cambiar la elección o la revisión del estado; muestra la fila de cada campo con «Lo último» / «Conservar lo viejo», el lado que queda, las dos columnas y, en fotos, las miniaturas. «Aplicar cambios» deshabilitado hasta F2.

## 5. Fases

- **F1 — Plan de lectura y panel claro (sin escrituras).**
  - `POST .../plan` calcula elección por campo (D1, D2), resumen por lado y lista de operaciones, y `plan_id`.
  - Panel: por fila, selector «Lo último» (por defecto) / «Lo viejo»; resumen «Se cambiará en Local: N · en Servidor: M»; fotos en dos columnas con miniatura; botón «Resolver» que abre el plan. «Aplicar» deshabilitado hasta F2.
  - Corrige el desajuste `presente`/`sin-archivo` (D7 y `describirValor`).
  - Verificación: tests unitarios de la elección (por defecto, conservar viejo, `ambos`, sin base, foto solo en un lado); `cargo test` del módulo sync; `tsc` y lint del frontend.
- **F2 — Aplicar campos en ambos sentidos.**
  - Cliente remoto con métodos de escritura (PUT/PATCH con bearer y renovación en 401).
  - `POST .../aplicar` con `plan_id` (409 si no coincide); verificación posterior.
  - Verificación: matriz contra servidor simulado (patrón de 13AA-12 F1–F4) con BD de pruebas real.
- **F3 — Aplicar fotos.** Subir y borrar en ambos sentidos con bytes (D5, D6). Verificación con BD de pruebas y servidor simulado.
- **F4 — Verificación end-to-end local** con la UI (backend local + servidor simulado). Producción: solo con permiso explícito (D4).

## 6. Verificación y Definition of Done

- Gate: `Arranque gate "MN-Inmobiliaria" --inicio` (línea base, al empezar; el ID es el proyecto, no la tarea) y `--cierre` (sin hallazgos nuevos).
- Por fase: tests de su lógica con BD de pruebas real (no vacíos), type-check y lint del frontend.
- Cierre: roadmap actualizado, entrada en `Agente/completados/tareas-2026-10-14.md`, plan a `Agente/planes/completados/` cuando F4 esté verde.

## 7. Dependencias

- 13AA-12 (sync de inmuebles) en `main`; pendiente de su F6 (cierre). No bloquea F1.
- Producción: despliegue del código de sync en el servidor (13AA-12 F5) antes de F2 contra producción.
