# Plan 13AA-4 — Vincular inmuebles con sus avisos de Facebook Marketplace

Fecha: 2026-10-13. Estado: **en ejecución.** F1, F2, F3 y F4 hechas (código y pruebas; F3 sin probar en navegador; F1 probada contra Facebook real, ahora también con la foto del aviso). F5 en la ventana de promoción de la lab. F6 al cerrar.

## 1. Objetivo

Saber qué inmuebles ya están publicados en el perfil de Facebook Marketplace (`https://www.facebook.com/marketplace/profile/1659274025/`) y vincular cada uno a su aviso (`inmuebles.marketplace_id`). Con el vínculo, los borradores usan el aviso exacto y no un inmueble parecido.

## 2. Estado verificado (2026-10-13)

- La columna `inmuebles.marketplace_id` existe (`migrations/20261009000033_marketplace_id.up.sql`). La tabla de inmuebles muestra «verificado» o «sin vincular» (`frontend/src/features/inmuebles/tabla/celdas-tabla-inmuebles.tsx:56-74`).
- Vincular hoy es a mano, ficha por ficha, pegando el ID o la URL del aviso (`src/services/inmueble/vinculo.rs:21-44`).
- El match exacto por ID solo se usa si el borrador trae `avisoId`. El piloto lo envía siempre `null` (`src/services/marketplace/pruebas.rs:63`), así que hoy todo va por título.
- Con `avisoId` presente y sin dueño, el código cae al título entre fichas sin vincular (`src/handlers/marketplace/claves.rs:78-89`). Ahí está el riesgo de inmueble parecido.
- La navegación del agente no abre `facebook.com` (bloqueado, 2026-10-13). La lectura del perfil tiene que hacerla la app de la lab, que ya tiene la sesión de Facebook de la usuaria (`persist:propio-browser-shared`, `opencode-propio/src/packages/desktop/src/main/browser-tabs.ts:22`). El agente no guarda ni pide su contraseña.
- `opencode-propio` y `opencode-propio-dev` no tienen Git: antes de editar, copia de respaldo.

## 3. Decisiones de la usuaria (2026-10-13)

- **D1 — Cómo llega la lista:** automático en segundo plano desde la lab, sin abrir opencode. La lab lee el perfil con su sesión de Facebook.
- **D2 — Aviso sin inmueble vinculado:** no citar otra ficha por título. Se resuelve con un modal de elección.
- **D3 — Vincular:** modal para confirmar y elegir. Aparece primero el inmueble más parecido. Lo ya vinculado no vuelve a aparecer.

Decisiones mías (técnicas, reversibles, se pueden cambiar):
- La lab lee el perfil **una vez al día** (decisión de la usuaria, 2026-10-13): al arrancar, si la última lectura tiene más de 24 h, y luego cada 24 h mientras la lab esté abierta. Nada en bucle ni con clics. Así se limita el riesgo con Facebook. **Sustituido el 2026-10-16: la lectura pasa a manual (botón en el modal). Ver `Agente/planes/completados/plan-lectura-manual-perfil-2026-10-16.md`.**
- La lista se guarda en MN (no en la lab). El modal lee de MN; funciona aunque la lab esté cerrada, con la fecha de la última lectura visible.
- El modal vive en la página de Inmuebles, con un botón «Vincular con Facebook» y el número de avisos pendientes.

## 4. Alcance y no alcance

**Dentro:** lectura del perfil (lab; **a petición desde el 2026-10-16**, antes automática), tabla y endpoints de avisos (MN), modal de vinculación (MN frontend), fallback sin título cuando hay aviso sin dueño (MN), pruebas.

**Fuera:**
- Que el agente abra o scrapee Facebook.
- Tocar la app viva `opencode-propio`. Los cambios van en `opencode-propio-dev`, y la promoción en la ventana de la usuaria (`lab-opencode`).
- Borrar filas de `mp_respuestas_cache` (08AA-30, decidido «Dejarlas»).

## 5. Fases

**F1 — Lectura del perfil en la lab (`opencode-propio-dev`).** Ventana oculta con la misma sesión de Facebook. Carga el perfil, extrae cada enlace `/marketplace/item/<id>` con título y precio, y los envía a MN. Se repite **una vez al día** (decisión de la usuaria): al arrancar si la última lectura tiene más de 24 h, y luego cada 24 h mientras la lab esté abierta. Criterio: en el perfil real salen todos los avisos visibles, sin duplicados. Antes, confirmar cómo autentica la lab sus llamadas a MN (`MP_MN_TOKEN`, `marketplace-nucleo.ts:51-52`). **Estado:** hecha en la copia de pruebas de la lab (`opencode-propio-dev`, aprobada por la usuaria). Archivos nuevos `marketplace-perfil.ts` (lógica pura), `marketplace-perfil-service.ts` (ventana oculta con la sesión de Facebook de la lab) y `marketplace-perfil.test.ts`; `index.ts` con una línea de arranque. Pruebas puras: 45 pasan, 0 fallan. **Verificado contra Facebook real (2026-10-13):** la lectura trae solo los 8 avisos de la usuaria, del cuadro «Publicaciones de <nombre>» (en la página hay 26 enlaces de aviso; los otros 18 son de otros vendedores y la notificación, y no se leen), con título real; «Gratis» no se envía como precio. Envío a MN con token real: 8 recibidos, 8 `presente` en la BD de la rama. **Foto (2026-10-13, pedido de la usuaria: ver la foto para reconocer el aviso):** la lectura toma también la foto de cada enlace (`img` del enlace; solo https de `fbcdn.net` o `facebook.com`, ≤ 1000 caracteres; si no vale, `null`) y la manda como `imagen_url`. MN la guarda en `marketplace_avisos_perfil.imagen_url` (migración `20261013000041`) y la devuelve en `GET /api/admin/marketplace/perfil/avisos`. El modal muestra la foto del aviso arriba (80×80) y una miniatura de la foto principal de cada inmueble candidato y de los «sin aviso vinculado». Las URLs de fbcdn caducan: se renuevan en cada lectura diaria; si una caduca, el modal muestra el icono de respaldo. Verificado: lab 14 tests, MN 8 tests, 8 de 8 avisos con foto en la BD de la rama tras lectura real. Sin verificar: modal en pantalla (el navegador integrado no tiene la sesión de la usuaria), y que «Vincular» guarde el vínculo. El código de la lab no está en Git (la copia no es repositorio); respaldo de los dos archivos originales en `C:/tmp/respaldo-lab-13aa-4/`. Sin avisos extraídos no envía nada (decisión conservadora: una lectura vacía no marca todo como ausente).

**F2 — Tabla y endpoints de avisos (MN backend).** Tabla `marketplace_avisos_perfil`: `aviso_id` (clave), `titulo`, `precio`, `url`, `visto_en`, `presente`. Endpoint para que la lab envíe la lista; cada lectura reemplaza el estado: los avisos que ya no aparecen quedan `presente = false` (vendidos o borrados), sin borrar la fila. Endpoint de lectura para el modal (solo staff). **Estado:** hecha (migración `20261013000040`; pruebas de BD de rama verdes; sin prueba HTTP contra el backend en marcha).

**F3 — Modal «Vincular con Facebook» (MN frontend).** En la página de Inmuebles:
- Lista de avisos presentes y sin vincular. Por cada uno, los inmuebles sin aviso ordenados por parecido (título y alias, usando la puntuación que ya existe en `src/services/marketplace/hilo/titulo.rs`; precio como dato visible). Botón «Vincular» o «No es este».
- Lista de inmuebles publicados sin aviso (informativa).
- Lo ya vinculado no aparece en ninguna de las dos listas.
- Muestra la fecha de la última lectura.

**Estado:** hecha en código (type-check y lint en verde); sin probar en navegador.

**F4 — Borradores sin título inseguro (MN backend).** Si el borrador trae `avisoId` y ningún inmueble lo reclama, responder «sin ficha». Quitar la caída al título en esa rama (`claves.rs:78-89`). Sin `avisoId` (el piloto de hoy), la ruta por título se mantiene hasta F5; es el riesgo que queda. **Estado:** hecha (`claves.rs`; prueba `id_sin_dueno_no_cita_ninguna_ficha` en BD de rama).

**F5 — Lab: la float envía `avisoId` (tarea F7b).** Sin esto el vínculo no entra en los borradores de chat. Va en la ventana de promoción de la usuaria (`opencode-propio-dev/PROMOCION-VENTANA.md`). No la hace el agente.

**F6 — Verificación y cierre.**
- Tests: el orden de parecido del modal, `normalizar_marketplace_id`, `claves_por_marketplace_id` con ID no vinculado (sin ficha), la clave única al vincular dos veces (422).
- Typecheck del frontend y gate del proyecto, una vez, al final.
- Prueba con el perfil real: los avisos aparecen, vincular uno lo quita de la lista, y ningún inmueble parecido queda vinculado por error.

## 6. Dependencias y riesgos

- F1 depende de que la lab esté abierta y de que la sesión de Facebook siga iniciada (si caduca, la lectura falla y queda anotado en el registro de la lab y en `~/.mn-inmobiliaria-lab/perfil-marketplace.json`; no sale en pantalla).
- Los selectores de Facebook cambian. La lectura usa solo los enlaces `/marketplace/item/`; si Facebook cambia eso, F1 deja de leer y hay que ajustarlo.
- F5 depende de la ventana de la usuaria. Hasta entonces, los borradores sin `avisoId` siguen por título.
- Un aviso mal vinculado hace que el borrador cite un inmueble equivocado. Por eso el modal exige confirmar cada vínculo.

## 7. Definición de hecho

- F1–F4 hechas y probadas con el perfil real. F5 pendiente de la ventana de la usuaria, registrada en `PROMOCION-VENTANA.md`.
- Los inmuebles publicados aparecen vinculados, y los no publicados aparecen como «sin aviso en Facebook».
- Registro en `Agente/completados/tareas-YYYY-MM-DD.md` con evidencia.

Relacionado: 09AA-21 (`marketplace_id`), 09AA-23 (inmueble equivocado), 09AA-29 (fallback por título), F7b (`avisoId` desde la float).
