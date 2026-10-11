# Prevención: `run-with-db.mjs` para el backend vivo de la rama (2026-10-10)

## Caso mínimo
- Backend de MN sirviendo en el puerto 3110 desde `C:\tmp\glory-target\glory_backend_main\debug\glory-backend.exe` (PID 13572).
- `node scripts/run-with-db.mjs cargo test …` (o `--version`) en esa misma rama.
- El wrapper para cada `glory-backend.exe` cuyo ejecutable cuelga del target de la rama, aunque el build no esté bloqueado. Resultado: el backend que sirve el admin se para sin que la usuaria lo pida.

## Capa responsable
- `scripts/run-with-db.mjs`, `liberarBinariosDeLaRama(targetDir)`, llamada antes de lanzar cargo (solo Windows).

## Detección esperada
- Hoy: solo un aviso en la consola del wrapper (`[db] Paro procesos vivos de esta rama que bloquean el build: PID …`). No basta, porque el backend ya está parado cuando se ve.
- Esperado: no parar un proceso que sirve sin comprobar que bloquea el relink.

## Corrección propuesta (no aplicada)
- Parar solo si el build falla con `os error 5` y reintentar una vez.
- Nunca parar un proceso con puerto en escucha; avisar y dejar que la usuaria decida.

## Estado
- Abierta. Va al roadmap como tarea aparte. Mientras tanto: con el backend arriba, usar `cargo` directo con `DATABASE_URL` de la rama y `CARGO_TARGET_DIR` alterno, no el wrapper.
- Relacionada: `prevencion-os-error-5-binario-rama-2026-10-10.md` (10AA-11, origen de la función).
