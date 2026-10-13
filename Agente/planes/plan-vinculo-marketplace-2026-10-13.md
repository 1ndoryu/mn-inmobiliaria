# Plan 13AA-4 — Vincular inmuebles con sus avisos de Facebook Marketplace

Fecha: 2026-10-13. Estado: **planificado, sin código**. Decisiones D1–D3 pendientes de la usuaria (§6).

## 1. Objetivo

Saber qué inmuebles de la app ya están publicados en el perfil de Facebook Marketplace (`https://www.facebook.com/marketplace/profile/1659274025/`) y vincular cada uno a su aviso (`inmuebles.marketplace_id`). Con el vínculo, los borradores usan el aviso exacto y no un inmueble parecido.

## 2. Estado verificado (2026-10-13)

- La columna `inmuebles.marketplace_id` existe (`migrations/20261009000033_marketplace_id.up.sql`). La tabla de inmuebles ya muestra «verificado» o «sin vincular» (`frontend/src/features/inmuebles/tabla/celdas-tabla-inmuebles.tsx:56-74`).
- Vincular hoy es a mano, ficha por ficha, pegando el ID o la URL del aviso (`src/services/inmueble/vinculo.rs:21-44`).
- La coincidencia exacta por ID solo se usa si el borrador trae `avisoId`. El piloto lo envía siempre `null` (`src/services/marketplace/pruebas.rs:63`), así que hoy todo va por título.
- Con `avisoId` presente y sin dueño, el código cae al título entre fichas sin vincular (`src/handlers/marketplace/claves.rs:78-89`). Ahí está el riesgo de inmueble parecido.
- El navegador del agente no abre `facebook.com` (política, 2026-10-13). La lista del perfil tiene que salir de la navegadora de la usuaria, donde ella ya tiene la sesión. El agente no guarda ni pide su contraseña.

## 3. Alcance y no alcance

**Dentro:** lista del perfil (bookmarklet), pantalla de vinculación en MN, decisión sobre el fallback por título (`claves.rs`), pruebas y vínculo de los inmuebles reales.

**Fuera:**
- Que el agente abra o scrapee Facebook.
- Tocar la app viva `opencode-propio`. Los cambios van en `opencode-propio-dev` y la promoción en la ventana de la usuaria (`lab-opencode`).
- Borrar filas de `mp_respuestas_cache` (08AA-30, decidido «Dejarlas»).

## 4. Fases

**F1 — Lista del perfil (bookmarklet).** Un marcador que, abierto en la página del perfil, lee cada enlace `/marketplace/item/<id>` con su título y precio, y copia el resultado al portapapeles. Criterio: en el perfil real salen todos los avisos visibles, sin duplicados. Se prueba con la página real, ejecutada por la usuaria.

**F2 — Pantalla «Publicados en Facebook» (MN frontend).** Pega la lista y muestra, por aviso:
- **Vinculado:** ya tiene inmueble con ese ID.
- **Propuesto:** un único inmueble con título parecido y precio igual.
- **Ambiguo:** varios candidatos; la usuaria elige.
- **Sin ficha:** aviso sin inmueble en la app.

Y, por inmueble, los que **no están publicados** (no aparecen en la lista). Ningún enlace se guarda sin confirmación fila a fila (D3).

**F3 — Vincular.** Guarda `marketplace_id` con la actualización que ya existe (`ActualizacionInmueble`). El UNIQUE evita que dos fichas reclamen el mismo aviso (422 claro).

**F4 — Borradores.** Según D2:
- *Si D2 = no citar por título:* con `avisoId` presente y sin dueño, devolver «sin ficha» en vez de caer al título. Prueba: aviso no vinculado → sin ficha, nunca otra ficha.
- *Si D2 = mantener:* sin cambio en código; se documenta el riesgo.

**F5 — Lab (fuera de MN).** Que la float de Marketplace envíe `avisoId` (tarea F7b). Sin esto, el ID exacto no entra en los borradores aunque los inmuebles estén vinculados. Va en la ventana de promoción de la usuaria (`opencode-propio-dev/PROMOCION-VENTANA.md`), no la hace el agente.

**F6 — Verificación y cierre.**
- Tests de `normalizar_marketplace_id`, del matcher de F2 y de `claves_por_marketplace_id` con ID no vinculado.
- Typecheck del frontend y gate del proyecto, una vez, al final.
- Vincular los inmuebles reales con la pantalla y revisar que ningún inmueble parecido quede vinculado al aviso equivocado.

## 5. Dependencias y riesgos

- F1 depende de que la usuaria ejecute el marcador en su navegador (un clic, con su sesión).
- F5 depende de la ventana de promoción de la usuaria. Mientras tanto, el beneficio de F3 se limita a no-regresión de la coincidencia por título.
- Los selectores de Facebook cambian; el marcador usa solo enlaces `/marketplace/item/` y puede fallar si Facebook cambia el enlace.
- Tercer riesgo: un aviso mal vinculado hace que el borrador cite un inmueble equivocado. Por eso F2 exige confirmación por fila.

## 6. Decisiones pendientes de la usuaria

- **D1 — Cómo llega la lista del perfil:** marcador de un clic (Recomendada) o pegar a mano las URLs.
- **D2 — Aviso con ID sin inmueble vinculado:** no citar otra ficha por título (Recomendada) o mantener el fallback actual (09AA-29).
- **D3 — Vincular:** confirmar cada aviso (Recomendada) o vincular solo si hay una coincidencia exacta de título y precio.

## 7. Definición de hecho

- F1–F4 hechas y probadas con la lista real del perfil; F5 pendiente de la ventana de la usuaria (queda registrada en `PROMOCION-VENTANA.md`).
- Los inmuebles publicados aparecen vinculados y los no publicados como «no publicado».
- Registro en `Agente/completados/tareas-YYYY-MM-DD.md` con evidencia (tests y prueba con la lista real).

Relacionado: [[09AA-21]] (`marketplace_id`), [[09AA-23]] (inmueble equivocado), [[09AA-29]] (fallback por título), F7b (`avisoId` desde la float).
