# Plan 13AA-12 — Estado de sincronización de inmuebles (local ↔ servidor), en ambos sentidos

Creado 2026-10-13. Estado: **F1–F4 hechas y verificadas; F5 (despliegue) en curso; F6 al cerrar F5.** Petición de la usuaria: ver en tiempo real, por inmueble, si difiere del servidor (fotos, descripción, todos los detalles), en una columna «Sync»; y «a veces el servidor estará actualizado y a veces el local»: debe indicar **qué lado va por delante**. Despliegue autorizado en general (el detalle de cada operación remota se declara al ejecutarla, AGENTS §9.1).

## 1. Objetivo

Columna **Sync** en la tabla de inmuebles del panel local con uno de estos estados por inmueble, calculado contra el servidor (`https://mn-inmobiliaria.com`):

| Estado | Significado | Cómo se decide |
|---|---|---|
| **Igual** | todo coincide | huellas iguales |
| **Local adelante** | el local cambió y el servidor no | 3 vías: base = servidor ≠ local |
| **Servidor adelante** | el servidor cambió y el local no | 3 vías: base = local ≠ servidor |
| **Ambos cambiaron** | hay campos cambiados en cada lado | 3 vías: base distinta de ambos |
| **Solo local** | existe aquí y no en el servidor | slug ausente allá |
| **Solo servidor** | existe allá y no aquí | slug ausente aquí (aviso sobre la tabla, no es fila) |
| **Sin base (estimado)** | difiere y nunca coincidieron | se usa `updated_at` como pista y se marca «estimado» |
| **Sin conexión / sin soporte / desactivado** | no se pudo comparar | estado global, no por inmueble |

Al abrir un inmueble con diferencias: panel **Diferencias** campo por campo (valor local vs. valor servidor, quién va por delante en cada campo) y tabla de fotos (orden, original/mejorada, presente en cada lado, bytes iguales o distintos).

## 2. Alcance / no alcance

**Dentro:** huella determinista por inmueble en ambos entornos; comparación 3 vías con base; tarea de fondo que consulta el servidor; entrega casi en tiempo real al panel; columna, resumen y panel de diferencias (escritorio y tarjetas móviles); despliegue de la parte del servidor.

**Fuera (Decidido: a falta de petición, solo lectura):** botones «Subir al servidor» / «Traer del servidor». Ya existen por CLI (`scripts/dev/sync-pull.mjs`, `scripts/datos/inmueble.mjs push`) y escribir en producción desde un clic exige confirmación por operación; se plantea como plan aparte cuando la usuaria vea el estado real. Tampoco se sincronizan `marketplace_id` ni `alias_titulos` (datos propios de cada entorno: se muestran como informativos y **no** cambian el estado).

## 3. Diseño

**Qué se compara** (huella = sha256 del valor canónico; los valores privados viajan solo como huella):
- Núcleo: `titulo, descripcion, ubicacion, puestos, residencia, precio, tipo, operacion, habitaciones, banos, metros, metros_terreno, estado, publicado` (los 15 de `nucleo()` en `scripts/dev/lib-api.mjs:76` + `publicado`).
- `copy` (corta, larga, modelo; sin `actualizada_en`) y `receta`.
- Ficha: `extras` y `precio_minimo` (solo huella).
- Fotos: conjunto `(origen, orden) → sha256 de los bytes`. Las `mejorada` se comparan solo por presencia/orden: el servidor las recodifica a JPG q70 (`src/services/inmueble.rs:348-351`) y los bytes difieren por diseño.
- Excluidos: `id`, `created_at`, `updated_at` (solo pista del estado «estimado»), `storage_key`, `marketplace_id`, `alias_titulos` (informativos).
- Canonización en Rust (misma en ambos lados, ambos corren el mismo binario): texto `trim` y `\r\n`→`\n`; números a 2 decimales; `NULL` ≡ `""` para textos opcionales. El manifiesto lleva `version` (1): una versión distinta se informa como «servidor sin soporte», no como diferencia.

**Manifiesto** `GET /api/admin/sync/manifiesto` (JWT admin; ambos entornos): por slug `{id, titulo, updated_at, campos:{campo→huella}, fotos:[{origen, orden, sha256, bytes}]}`; sin paginar (tope 2000, con `truncado`). No devuelve valores privados.

**Hash de fotos barato:** migración 42 añade `fotos.sha256 TEXT NULL` y `fotos.bytes INT NULL`; se rellena al subir y, de forma perezosa y acotada (40 por petición), al pedir el manifiesto para las filas antiguas (lee el archivo, hashea, `UPDATE`). Mientras queden pendientes el manifiesto lo declara (`fotos_pendientes`) y el estado muestra «calculando», nunca «igual». Las claves `storage_key` son únicas por foto, así que el hash no caduca.

**Base de 3 vías:** migración 42 añade `sync_base(slug PK, campos jsonb, fotos jsonb, acordado_en)`. La tarea de fondo la escribe **solo** cuando observa un inmueble `Igual` (upsert idempotente); los GET no escriben. Con la base: por campo, `L≠R` y `B==R` ⇒ cambió el local; `B==L` ⇒ cambió el servidor; ninguno ⇒ ambos. El estado del inmueble agrega los campos. Sin base y con diferencias: `updated_at` mayor ⇒ «adelante (estimado)».

**Cliente del servidor y tarea de fondo (solo local):** `services/sync/` con `reqwest` (timeout 10 s, ya es dependencia): login con `PROD_BASE`, `PROD_EMAIL`, `PROD_PASSWORD` leídas del archivo que indica `SYNC_REMOTO_ENV` (token solo en memoria, reintento de login ante 401, nada de credenciales en logs). Consulta cada `SYNC_INTERVALO_SEG` (20 por defecto, mínimo 5). Sin `SYNC_REMOTO_ENV`: estado `desactivado` y la columna no se pinta (así producción no se compara consigo misma). La URL viene solo del entorno, nunca de la petición.

**Tiempo real:** long-poll, el mismo patrón que `src/services/marketplace/cambios.rs` y `GET /api/admin/marketplace/chats/cambios` (pasa por proxies, usa JWT en cabecera, se aborta con `AbortSignal`; `EventSource` no puede enviar `Authorization`). `GET /api/admin/sync/estado?desde=<rev>` responde al instante si cambió la revisión o espera hasta 25 s. El lado local se recalcula en vivo en cada petición (barato) y la revisión sube al instante cuando se crea, edita, publica, borra o toca fotos un inmueble (`notificar()` desde los mismos puntos): los cambios locales se ven en ~1 s; los del servidor, en ≤ `SYNC_INTERVALO_SEG`. `POST /api/admin/sync/comprobar` fuerza una consulta ahora (botón «Comprobar»).

**Detalle:** `GET /api/admin/sync/inmuebles/{slug}` pide al servidor el inmueble completo (`/api/admin/inmuebles/{id}` + `/ficha`, con el `id` remoto del manifiesto) y devuelve ambos lados para el panel de Diferencias (solo bajo demanda; el valor privado se muestra a la admin autenticada, igual que en la ficha actual).

**Frontend:** `frontend/src/data/inmuebles/sync.ts` (llamadas), `hooks/inmuebles/use-sync-inmuebles.ts` (long-poll con pausa si la pestaña está oculta y reintento con espera creciente), columna **Sync** y resumen en `features/inmuebles/tabla-inmuebles.tsx` + `tabla/celdas-tabla-inmuebles.tsx`, `modal-diferencias-sync.tsx` y aviso «Solo en el servidor (N)». Estados con icono + texto (no solo color): ✓ Igual, ↑ Local adelante, ↓ Servidor adelante, ⇅ Ambos cambiaron, + Solo local, ? estimado.

## 4. Fases

| Fase | Contenido | Verificación |
|---|---|---|
| **F1** | Migración 42 (`fotos.sha256/bytes`, `sync_base`, `.down.sql`); `services/sync/{huella,comparar}.rs` (puro); repositorio; hash al subir + relleno perezoso; `GET /sync/manifiesto` | tests unitarios: canonización, huellas estables, tabla de verdad de `comparar` (todos los estados, base presente/ausente, fotos extra/faltantes/bytes distintos, `mejorada` ignorada, informativos sin efecto) |
| **F2** | Cliente remoto, tarea de fondo, base, long-poll `estado`, `comprobar`, `detalle`; `notificar()` en mutaciones | tests con servidor simulado; `cargo check --tests` y tests filtrados por wrapper |
| **F3** | Frontend: hook, columna, resumen, panel, solo-servidor, móvil | `tsc -b`, `oxlint`, build; revisión visual en el navegador (vacío, carga, error, sin soporte, teclado, contraste) |
| **F4** | **Prueba bidireccional sin tocar producción:** segunda instancia local como «servidor» (otra BD de rama y puerto, `PROD_BASE` en `SYNC_REMOTO_ENV` apuntando a ella). Casos: igual → editar local ⇒ «Local adelante»; editar «servidor» ⇒ «Servidor adelante»; ambos ⇒ «Ambos cambiaron»; foto añadida/borrada/reordenada en cada lado; solo local; solo servidor; servidor caído; servidor sin la ruta | capturas y respuestas reales |
| **F5** | **Despliegue** (flujo ya probado el 2026-09-24: push `main` → `cm deploy`; Coolify construye `origin/main` y `sqlx::migrate!()` aplica la migración al arrancar): merge a `main`, push, `cm` sobre `mn-inmobiliaria` (UUID `as0scgwg44wkkkccgwcwg8w0`), health `GET /api/health`, prueba del manifiesto en producción. Luego, con las credenciales en el `.env` local, comparación real **solo lectura** contra producción | health 200; manifiesto 200 con `version` 1 y `fotos_pendientes` → 0; estados reales anotados |
| **F6** | Documentación (`README`/documentación del panel), completada, roadmap | AGENTS §7–§8 |

## 5. Riesgos y mitigaciones

- **Migración en producción:** aditiva (columna nullable + tabla nueva), con `.down.sql`; no toca datos existentes. El relleno de `fotos.sha256` es un `UPDATE` de datos derivados en la BD de producción: se declara al desplegar.
- **Sesión admin del servidor en el backend local:** token solo en memoria, solo `GET` del manifiesto/detalle y el `POST` de login; si la contraseña admin rota (Fase 5.4 del plan de deploy), la comparación pasa a «sin conexión: credenciales» hasta actualizar el `.env`. Cuenta dedicada de solo lectura: mejora futura, no se crean cuentas desde aquí.
- **Falsos «Diferente»:** por recodificación de `mejorada` (excluida de bytes), por redondeo (canonización) o por datos propios del entorno (informativos). Cualquier otro caso se trata como bug de la huella.
- **Primera comparación sin base:** direcciones «estimadas» hasta que cada inmueble coincida una vez; se declara en la interfaz.
- **Coste:** 16 inmuebles / 322 fotos hoy; el relleno perezoso acota la primera carga; el manifiesto no lee archivos tras el relleno.

## 6. Definition of Done

Estados correctos en las dos direcciones demostrados en F4 y contra producción en F5; sin falsos positivos en el conjunto real (16 inmuebles) salvo diferencias reales anotadas; despliegue verificado; `cargo fmt/check/clippy` y tests de lo tocado, `tsc -b` y build del front; diff revisado; documentación y roadmap al día; rama integrada en `main` y borrada.

## 7. Decidido (sin preguntas abiertas)

- Solo lectura en esta entrega; acciones subir/traer quedan para un plan aparte (motivo: escribir en producción exige confirmación por operación y el estado real aún no se ha visto).
- Long-poll en vez de SSE/WebSocket (precedente del proyecto, JWT en cabecera).
- Intervalo de consulta al servidor: 20 s; los cambios locales, al instante.
- `marketplace_id` y `alias_titulos` informativos (no cuentan para el estado).
- Credenciales de comparación: sin duplicarlas. El backend local lee `SYNC_REMOTO_ENV` (ruta; apunta a `scripts/.env.prod.local`, gitignored, con `PROD_BASE`, `PROD_EMAIL`, `PROD_PASSWORD` que ya usan los scripts). Sin esa variable, o sin archivo, el estado es `desactivado`. Nunca se muestran ni se registran.
