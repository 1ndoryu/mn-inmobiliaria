# Plan: clave de chat estable por conversación (Marketplace)

- **Fecha:** 2026-10-13
- **Estado:** integrada en `main` (MN) el 2026-10-13. F0–F3 hechas; gate fuera de banda sin hallazgos nuevos. Pendiente de hechos externos: prueba viva y promoción del puente. Roadmap: 13AA-13.
- **Rama:** `clave-chat-estable-2026-10-13` (worktree `C:/tmp/wt-clave-chat-estable`, base `main` d0840175).
- **Origen:** la usuaria pregunta si dos clientes con el mismo nombre se sobrescriben en Mensajes → Marketplace → Chats.

## 1. Objetivo

Dos conversaciones distintas (dos compradores) sobre el mismo aviso deben dar dos chats separados, con borradores, regeneración y borrado independientes.

## 2. Causa (verificada en código, 2026-10-13)

- La clave del chat es `nombre|aviso`. `clave_hilo` (`src/services/marketplace/hilo.rs:62-85`) conserva nombre y aviso; `nombre_de_thread` (`hilo.rs:14`) toma el primer segmento.
- El puente construye esa clave en `threadIdConPrecio` (`opencode-propio-dev/src/packages/desktop/src/main/marketplace-nucleo.ts:296`), que usan `/releer` (:246) y `/borrador` (:320).
- El nombre sale del DOM y no es único. El `avisoId` (`marketplace-float.ts:1014,1037`) identifica el aviso, no al comprador.
- Efectos: los borradores de los dos clientes quedan en una lista; regenerar (`chat/acciones.rs`) borra las filas no corregidas del mismo hilo; borrar el chat borra las de los dos.
- **Segunda vía, independiente:** la caché `mp_respuestas_cache` no tiene hilo en su clave `(firma, precio_hash, catalog_hash)`. `guardar_cache` usa `ON CONFLICT DO NOTHING` (`src/services/marketplace/hilo/cache.rs:139`) y `reemplazar_cache` reasigna `thread_id = EXCLUDED.thread_id` (`cache.rs:176-178`, solo si `corregida = FALSE`, `cache.rs:182`). La firma es un HMAC de texto + `avisoId` (`vendor/nucleo/firmas.ts:18-19`), sin nombre ni comprador. Dos clientes con la misma pregunta pueden compartir una fila.
- **Nota sobre el hash de sesión:** `borrador.rs:463` (`sha_hex(&clave_hilo(...))`) es el hash de la cabecera `x-opencode-session` (ruteo afín y caché del proveedor), no una firma HMAC. Al cambiar la clave, la sesión de cada hilo con id cambia una vez.

No verificado: consultas a la base de datos, ejecución en vivo.

## 3. Alcance

- **Incluye:** id del comprador como tercer segmento de la clave (`nombre|aviso|id`) en backend, puente y visualización.
- **No incluye:** la segunda vía (caché); migrar o reasignar filas existentes; cambiar la firma de la caché; alinear `chats_admin.rs`.

## 4. Solución

- Clave nueva: `nombre|aviso|id`, con `id` = id de perfil del comprador (`/marketplace/profile/<id>`). Sin `id`, la clave es la de hoy (compatibilidad).
- `split_once('|')` (nombre) y `nth(1)` (aviso) siguen funcionando con el tercer segmento. El SQL compara cadenas exactas: no hace falta cambiar esquema.
- El `id` se valida antes de usarlo (viene del DOM): solo dígitos, longitud 5–40. Backend (`separar_id_hilo`), puente (`extraerPerfilId`) y float (`mpPerfilId`) usan el mismo criterio.
- El tope de 120 caracteres no corta el id: se recorta la base y el `|id` va entero (`recortarThreadId` en `marketplace-nucleo.ts`, `floatKeyFor` y `mpKey`).

## 5. Fases

- **F0 — Descubrir el identificador (bloqueante).** Resuelta (§6).
  - **Resultado 2026-10-13 (lectura de solo lectura por CDP, puerto 9333; sin clicks ni envíos):**
    - El id de conversación de Facebook (`/messages/t/<id>`) **no aparece** en el DOM de ningún float (7 floats leídos).
    - Candidato usado: el enlace `/marketplace/profile/<id>`. Cada float tiene 2 enlaces con el mismo id. Los ids son distintos entre floats.
    - Verificación de estabilidad (2026-10-13, dos lecturas con 6 s de diferencia, 8 floats): mismo id en cada float en ambas lecturas; 2 enlaces por float; un solo id por float.
    - **Sin verificar:** que el id sea estable entre sesiones de Facebook; que pertenezca al comprador y no a una cuenta compartida (el nombre no se puede cruzar con el id en el DOM). Se confirma en la prueba viva.
  - **Estado:** resuelta (§6).
- **F1 — Backend MN.** Hecha: commit `858ee9b2`. `clave_hilo` acepta el tercer segmento validado (`separar_id_hilo`, solo dígitos, 5–40, exige `nombre|aviso` delante). `chats_admin.rs` no se toca (§6). Tests: `clave_hilo_separa_dos_compradores_con_el_mismo_nombre_y_aviso`, `clave_hilo_sin_id_valido_es_la_clave_de_hoy`, `nombre_y_aviso_ignoran_el_id_final` (`src/services/marketplace/pruebas/hilos/hilo.rs`). Verificado: `node scripts/run-cargo.mjs test clave_hilo` (3 OK) y `test nombre_y_aviso` (2 OK).
- **F2 — Frontend.** Hecha: commit `858ee9b2`. `partirHilo` oculta el tercer segmento (`quitarIdHilo` en `frontend/src/features/chat/marketplace/chats-marketplace.tsx`). El `title` muestra el hilo completo (decidido, admin). `hilos-huerfanos.tsx` tolera el tercer segmento: sin cambios. Verificación: type-check y lint sin errores.
- **F3 — Puente.** Hecha (repos distintos, ver abajo): `plugins-opencode` en `main` `3390dad`; `opencode-propio-dev` no es git y queda pendiente de promoción.
  - `plugins-opencode` (git): `extraerPerfilId` y `floatKeyFor(buyer, listing, perfilId)` en `src/burbujas/parseo/cabecera.ts`; `perfilDelHilo` y la clave en `leerHilos` (`src/burbujas/adaptador-fb/lector.ts`). Tests en `test/burbujas/burbujas-parseo.test.ts`.
  - `opencode-propio-dev` (sin git): `mpPerfilId` y `mpKey(buyer, listing, perfilId)` en `marketplace-float.ts` (en `mpScan` y `mpReposition`; `mpReposition` calcula ahora `root` antes de la clave); `recortarThreadId` en `marketplace-nucleo.ts` (`buildBorradorRequest` y `buildReleerRequest`); tests [13AA-13] en `marketplace-nucleo.test.ts`.
  - Respaldo previo a la edición: `C:/tmp/respaldo-13aa-13-<fecha>/`.
  - **No editado a mano:** `opencode-propio-dev/src/packages/desktop/src/main/vendor/burbujas.iife.js:527` usa `floatKeyFor` de la copia compilada. Se regenera al promover (ventana de la usuaria, `PROMOCION-VENTANA.md`); no se edita el bundle.
- **F4 — Despliegue.** Primero el backend (acepta ambas formas); después el puente en la lab. El paso a opencode-propio solo en la ventana de la usuaria (`PROMOCION-VENTANA.md`). La lab no se reinicia.

## 6. Decisiones

- **Decidido (2026-10-13, la usuaria, AskUserQuestion):** identificador = id de perfil del comprador (`/marketplace/profile/<id>`) (opción «Id de perfil del comprador (Recomendada)»). Va como tercer segmento. El id de conversación no existe en el DOM (F0).
- **Decidido (2026-10-13, la usuaria):** la lab no se reinicia para F0.
- **Decidido:** las filas legacy `nombre|aviso` no se reasignan ni se borran. Quedan como hilo histórico aparte; los chats nuevos empiezan vacíos. Motivo: cero escrituras sobre datos existentes (AGENTS §5). Coste: el panel puede mostrar, para un mismo nombre, un chat antiguo y uno nuevo.
- **Decidido:** la firma de la caché no cambia aquí. La segunda vía va en tarea aparte.
- **Decidido:** no alinear `chats_admin.rs` con `clave_hilo`. La clave guardada debe llegar tal cual; alinearla podría apuntar a filas legacy con `$` y borrar 0 filas en silencio.

## 7. Verificación

- **Backend (Rust):** hecha (§5 F1): tests de `clave_hilo` y `nombre_y_aviso` en verde.
- **Puente:** `threadIdConPrecio` con y sin id; recorte de 120 con id; `floatKeyFor` y `extraerPerfilId` (tests de `plugins-opencode` y `marketplace-nucleo.test.ts`). Los tests existentes siguen en verde.
- **Frontend:** `npm run type-check` y `npm run lint` en `frontend/`.
- **Gate:** `Arranque gate "MN-Inmobiliaria" --cierre` no mide el worktree (analiza la carpeta principal). Se hizo fuera de banda con el mismo CLI Sentinel sobre `d0840175` y sobre el worktree: 0 hallazgos nuevos (`completados/tareas-2026-10-13.md`, 13AA-13).
- **Viva:** dos chats reales del mismo nombre (la usuaria). Confirma que el id de perfil es estable entre sesiones.

## 8. Definition of Done

Según AGENTS §8: pruebas, gate sin hallazgos nuevos, documentación (roadmap, este plan, completada), commit y limpieza. El plan pasa a `Agente/planes/completados/` solo con evidencia funcional.

## 9. Próximo paso

Integrado en `main` (MN). Pendiente de la usuaria: prueba viva con dos chats del mismo nombre, y la promoción del puente en su ventana (`opencode-propio-dev/PROMOCION-VENTANA.md`, «0c»: regenerar `vendor/burbujas.iife.js`).
