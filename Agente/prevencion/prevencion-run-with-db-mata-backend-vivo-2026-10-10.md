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

## Corrección aplicada (10AA-30, 2026-10-10)
- `liberarBinariosDeLaRama` nunca para un proceso con puerto en escucha: lo deja vivo y avisa con su PID. Solo para los `glory-backend.exe` de la rama sin listener.
- Si el relink falla con `os error 5` por un backend vivo, la decisión de parar es de la usuaria.

## Estado
- Resuelta (10AA-30). Verificada: `node --check` OK; filtro en solo lectura marca VIVO al PID del backend de `glory_backend_main`; `node scripts/run-with-db.mjs --version` exit 0 con el backend arriba, que sigue respondiendo 200 en `:3110`.
- Límite: no se probó un relink real con un backend bloqueante; el caso de `os error 5` sigue dependiendo de la decisión de la usuaria.
- Relacionada: `prevencion-os-error-5-binario-rama-2026-10-10.md` (10AA-11, origen de la función).
