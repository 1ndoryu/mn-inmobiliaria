# Prevención: caducar caché de marketplace por texto y no por fecha (2026-10-12)

## Caso mínimo
10AA-31: se caducaron 55 filas de `mp_respuestas_cache` con `thread_id LIKE '%VEF0%'` (`valida_hasta = now() - 1 day`). Al reiniciar el backend, `purgar_cache` las borró (`C:\tmp\mn-backend.log` línea 14841, 2026-10-11T14:17:51Z). El filtro por texto no coincidía con el bug, que afecta a toda fila escrita antes del fix 73548f32.

## Capa responsable
Procedimiento manual de caducidad sobre `mp_respuestas_cache`, junto con `purgar_cache` (`src/services/marketplace.rs:1764`, llamado desde `src/main.rs:31`).

## Detección esperada
- Antes de caducar: contar con el criterio por fecha (`valida_hasta - 90 días` anterior al commit del fix) y guardar copia CSV.
- Tras reiniciar: el log registra «N vencidas purgadas al arrancar». Si N no coincide con lo caducado, parar.

## Regla
1. Criterio solo por fecha de escritura inferida. Nunca por título, clave ni texto.
2. Backend vivo posterior al commit del fix (fecha del binario) antes de caducar.
3. Copia CSV previa.
4. Caducar equivale a borrar en el siguiente arranque. Si hace falta conservar filas hasta regenerar, no caducarlas.

## Referencia
`Agente/completados/tareas-2026-10-11.md` (10AA-31: «Error de criterio», «Prevención»).
