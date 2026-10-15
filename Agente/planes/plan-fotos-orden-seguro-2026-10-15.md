# Plan 15AA-8 — Reordenar fotos sin borrarlas (pérdida de fotos al elegir principal)

Fecha: 2026-10-15. Origen: incidencia viva — al elegir imagen principal y guardar,
el inmueble `5a5cd58e-68ff-4f3b-9275-ef1a28e49a10` se quedó con 2 de sus 13 fotos.

Estado: **cerrado el 2026-10-15** (F1–F4 hechas y verificadas). Cierre:
`Agente/completados/tareas-2026-10-15.md`.

## Objetivo

Que cambiar el orden (o la foto principal) de un inmueble **nunca** pueda dejarlo sin
fotos, ni siquiera si el guardado se interrumpe a la mitad o se dispara dos veces.

## Causa (evidencia, no sospecha)

`sincronizarFotos` (`frontend/src/data/inmuebles/api.ts:518`) hace, cuando el orden
cambia: descarga los bytes de todas → **borra todas las filas** (originales y
mejoradas) → las vuelve a subir en el orden nuevo. En el log de la 89
(`_logs/MN-Inmobiliaria/dev-up-MN-Inmobiliaria-3110.log`, 2026-10-15 17:17:31–17:17:41Z):

- dos `PUT /api/admin/inmuebles/<id>` (17:17:31 y 17:17:34) con sus dos `GET` de
  `sincronizarFotos`: dos guardados concurrentes sobre el mismo inmueble;
- los dos bucles de borrado, intercalados, borran 11 de las 13 filas;
- **cero** `POST /api/admin/fotos/upload` después: el bucle se cortó entre el borrado
  y la resubida (última petición 17:17:41.337Z; los dos bucles mueren en el mismo
  instante, con las mismas 2 fotos pendientes).

No hubo error del backend: es la ventana borrar→resubir la que no es atómica. La
operación correcta para reordenar no es destruir y recrear.

## Alcance

1. **Backend:** endpoint de reordenado que renumera en una transacción, sin borrar
   archivos ni filas.
2. **Frontend:** `sincronizarFotos` pasa a «subir lo nuevo → borrar solo lo que
   sobra → reordenar». Una foto que se conserva no se borra nunca.
3. **Concurrencia:** los dos guardados simultáneos dejan de ser peligrosos
   (serialización por inmueble en el front y reordenado idempotente en el back).

Fuera de alcance: la cola de mejora con IA y el flujo de sync local↔servidor.

## Fases

- **F1 — Endpoint de reordenado. HECHA.** `PATCH /api/admin/inmuebles/:id/fotos/orden`
  con `{"ids": ["<uuid>", …]}` (ids de originales, en el orden deseado).
  Renumera el «hueco» completo: cada original y su mejorada hermana (mismo `orden`
  viejo) reciben el mismo `orden` nuevo. Valida que los ids sean exactamente los
  originales del inmueble (permutación) y responde 400 si no. Toca `updated_at`
  (`?v=`) y notifica. Evidencia: `tests/fotos_orden.rs`, 2/2 en verde en la 89.
- **F2 — `sincronizarFotos` sin borrado masivo. HECHA.** Sube primero las nuevas,
  borra solo las que ya no están en la lista deseada, y termina con el reordenado.
  `cambiaron` mantiene su significado (purga de copias locales de mejora).
  Evidencia: `tsc -b` y `oxlint` limpios; prueba viva (F4).
- **F3 — Guardia de concurrencia. HECHA.** Un solo `sincronizarFotos` en vuelo por
  inmueble (`enSerie` en `frontend/src/data/inmuebles/api.ts`: el segundo espera al
  primero en vez de pisarlo). Evidencia: guardado doble en la prueba viva sin pérdida.
- **F4 — Prueba viva y cierre. HECHA.** En el panel de la 89: principal = 5.ª foto,
  guardar, recargar. 13 originales, 0 mejoradas, 26 archivos en disco, orden nuevo
  persistido; red: una `PATCH …/fotos/orden` → 204, cero subidas y cero borrados.

## DoD

- Reordenar y guardar deja el mismo número de fotos y los archivos en disco.
- Interrumpir a mitad (cerrar la pestaña) no borra ninguna foto.
- Dos guardados seguidos no pierden fotos.
- Tests backend en verde, gate sin hallazgos nuevos, docs al día y commit en `main`.
