# Plan 14AA-2 — Resolver diferencias de sync por inmueble

Estado: **F1 hecha. F5a–F5c implementadas en `main` el 2026-10-14 (`7b991fe4` backend, `28786374` frontend). F5d pendiente: `publicar.rs` y `traer.rs` no tienen tests.** Verificación: `node scripts/run-cargo.mjs test services::sync` 32/32; type-check y lint del frontend sin errores. Prueba funcional (14-oct): servidor simulado en `127.0.0.1:4555` (solo lectura) con un título y una foto distintos en un inmueble; `plan` devolvió 200 con `servidor_adelante`, por defecto 2 cambios en Local; «Conservar lo viejo» en todo pasa a 2 en Servidor; solo título viejo, 1 y 1; sin `SYNC_REMOTO_ENV` responde 409. En el panel: contadores, textos y miniatura local (carga) correctos; «Aplicar» sigue deshabilitado (F2). No hubo escrituras remotas. Sin base previa (`estimado`) no se probó en vivo: cubierto por tests. Siguiente paso: F5a (publicar y traer el inmueble completo, pedido por la usuaria el 2026-10-14). F2–F3 reutilizan su cliente de escritura. Base: 13AA-12 (`plan-sync-inmuebles-2026-10-13.md`), que dejó «subir/traer» y las escrituras fuera de alcance; este plan las recoge.

## 1. Objetivo

Botón «Resolver» en el panel «Diferencias» de cada inmueble. Muestra, campo a campo, qué cambia en cada lado. Por defecto elige el valor más reciente; el usuario puede elegir conservar el viejo. Los campos de imagen se comparan lado a lado. Al aplicar, las dos copias quedan iguales.

## 2. Alcance y no alcance

- **Dentro:** campos del inmueble (20 huellados, `src/services/sync/comparacion/huella.rs:30-51`), ficha (`extras`, `precio_minimo`), publicación (`publicado`), estado, y fotos (`foto:<origen>:<orden>`) en ambas direcciones.
- **Dentro (F5):** inmuebles que existen solo en un lado: publicar el inmueble completo de Local al servidor y traerlo completo del servidor a Local, en ambos sentidos.
- **Fuera:** reordenar fotos sin borrar y volver a subir → no existe endpoint de reordenación. Aceptar `slug` en el alta del servidor (D11) → tarea aparte, porque exige despliegue.
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
- **D11 — Slug (F5).** El alta del servidor genera el slug desde el título (`slugify` en `src/services/inmueble/slug.rs:7-29`; el alta en `src/services/inmueble.rs:77-120`, con sufijos -2 a -4 ante colisión) y no acepta `slug`. Publicar exige que el slug generado coincida con el local: si no, aborta en el preflight, antes de escribir nada. Traer fija el slug remoto en local y aborta si ya existe. Aceptar `slug` en el servidor exige despliegue (D4): tarea aparte.
- **D12 — Qué viaja (F5).** Todos los campos huellados (`src/services/sync/comparacion/huella.rs:30-51`), `copy` (`copy_actualizada_en` lo fija el cliente: se envía el local o la hora actual), `receta` (solo se fija con `PUT /api/admin/inmuebles/{id}`, no en el alta), ficha (`extras`, `precio_minimo`), fotos, `publicado` y `estado`. No viajan `marketplace_id` ni `alias_titulos`: son propios de cada entorno (`huella.rs:28-29`).
- **D13 — Orden y fallos al publicar (F5).** Preflight sin escrituras: sesión válida, slug libre y coincidente (D11), manifiesto no truncado, fotos locales legibles y ≤ 10 MiB. Luego: alta (con `estado` y `copy`) → `PUT` receta → `PUT` ficha → subida de cada foto con `POST /fotos/upload` (esa llamada crea la fila; no se usa `POST /inmuebles/{id}/fotos`, que duplicaría) → `PATCH publicacion` al final (el alta siempre nace `publicado = FALSE`). Si un paso falla, no se borra nada: la respuesta dice qué paso falló y el inmueble queda en «diferencias», resoluble con F2–F3. Borrar en servidor o en local es destructivo y requiere permiso.
- **D14 — Orden y fallos al traer (F5).** Preflight: el slug no existe en local. Se descargan todas las fotos (`GET {PROD_BASE}{url}`, máx. 10 MiB cada una) antes de crear nada en local. Luego: alta local con el slug remoto exacto (`crear_con_slug`, sin sufijos), receta, ficha, fotos con `InmuebleService::subir_foto` (genera miniatura y huella) y `publicado` al final. Mismo criterio de fallo que D13.
- **D15 — Tiempos y concurrencia (F5).** Escrituras y descargas de fotos: 60 s por petición (una subida de 10 MiB no cabe en los 10 s de lectura); las lecturas siguen en 10 s. Un bloqueo por slug en el proceso: una segunda petición sobre el mismo inmueble responde 409 «operación en curso».
- **D16 — Confirmación (F5).** El botón abre un diálogo que lista lo que se creará (campos que cambian de lado, receta, ficha, nº de fotos, `publicado`) y el host de destino. Si el host es producción, el diálogo lo nombra, como en D4.
- **D17 — Pruebas (F5).** El «servidor de prueba» es el router real (`create_router`, `src/handlers/mod.rs:201`) en `127.0.0.1:0`, con una BD de pruebas aparte; el cliente remoto lo usa mediante un fichero de credenciales de prueba (`SYNC_REMOTO_ENV`). Ningún paso de F5 escribe en el `PROD_BASE` de producción. Las pruebas de F5 **fallan** si falta la BD de pruebas; no se omiten en vacío (ver §3).

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
- **F5 — Publicar y traer el inmueble completo** (inmuebles que existen solo en un lado). Pedido por la usuaria el 2026-10-14: «apartamento-frente-al-mar-en-villa-sol-suites no existe en el servidor». Decisiones D11–D17. Va antes que F2–F3; estas reutilizan F5a.
  - **F5a — Cliente remoto de escritura** (`src/services/sync/remoto.rs`): `crear_inmueble` (alta), `poner_receta` (PUT inmueble), `poner_ficha` (PUT ficha), `subir_foto` (`POST /api/admin/fotos/upload` con bytes crudos y query `inmueble_id`, `filename`, `origen`, `orden`), `poner_publicacion` (PATCH), `descargar_foto` (GET absoluta con tope de tamaño). Bearer y renovación en 401 como `get_con`; el cuerpo se clona para reintentar. Antes de codificar, comprobar que `PUT /api/admin/inmuebles/{id}` acepta un cuerpo solo con `receta`; si exige campos, enviar los actuales. Tests con el servidor simulado de `remoto.rs` (patrón de sus tests).
  - **F5b — Orquestación local.** `InmuebleService::crear_con_slug` (slug fijo, 409 si existe); `src/services/sync/publicar.rs` y `traer.rs` (preflight y secuencia de D13 y D14); rutas `POST /api/admin/sync/inmuebles/{slug}/publicar` y `POST .../traer` con lock por slug (D15). La respuesta lista los pasos hechos y, si falla, el paso fallido.
  - **F5c — Panel.** `celda-sync-inmueble.tsx`: `solo_local` → «Publicar en el servidor»; `solo_servidor` → «Traer al local» (hoy solo es un aviso). Diálogo de confirmación (D16). Tipos en `frontend/src/data/inmuebles/sync.ts`.
  - **F5d — Verificación.** Test de integración en Rust con dos BD (local y remoto de prueba, D17): (1) publicar un inmueble con dos fotos (`original` y `mejorada`), receta, ficha, copy, `publicado = true` y estado `reservado` → ambos lados comparan `Igual` con `comparar_todo`; (2) traer el mismo inmueble con otro slug → `Igual`; (3) slug ocupado en remoto → aborta antes de escribir; (4) fallo de una foto a mitad → informe del paso y ninguna fila borrada; (5) sin BD de pruebas → el test falla. Después, comprobación en navegador contra el remoto de prueba.
  - Producción: ninguna ejecución hasta permiso por operación (D4, D16).

## 6. Verificación y Definition of Done

- Gate: `Arranque gate "MN-Inmobiliaria" --inicio` (línea base, al empezar; el ID es el proyecto, no la tarea) y `--cierre` (sin hallazgos nuevos).
- Por fase: tests de su lógica con BD de pruebas real (no vacíos), type-check y lint del frontend.
- Cierre: roadmap actualizado, entrada en `Agente/completados/tareas-2026-10-14.md`, plan a `Agente/planes/completados/` cuando F4 esté verde.

## 7. Dependencias

- 13AA-12 (sync de inmuebles) en `main`; pendiente de su F6 (cierre). No bloquea F1.
- Producción: despliegue del código de sync en el servidor (13AA-12 F5) antes de F2 contra producción.
