# Prevención: `os error 5` al reconstruir `glory-backend.exe` (10AA-11)

## Caso mínimo
- Un `glory-backend.exe` vivo de la misma rama (`C:\tmp\glory-target\<rama>\debug\`) mantiene bloqueado el binario.
- `cargo test`/`build` falla al relinkar: `Acceso denegado (os error 5)`.
- Visto en la rama `feat/10aa-4-chats-marketplace`: PID 18800 (padre `cargo.exe` 9660).

## Capa responsable
- `scripts/run-with-db.mjs`, función `liberarBinariosDeLaRama(cargoTargetDir)`, antes de lanzar cargo (solo Windows).

## Qué hace
- Para solo los `glory-backend.exe` cuyo ejecutable cuelga del target de **esta** rama (prefijo con `\` final, así no casa con `..._otra_rama`).
- Nunca toca otros proyectos (p. ej. `PROYECTO TASKS`) ni opencode. Imprime los PID parados.
- Permiso: AGENTS.md §5 («puedes parar cualquier proceso que necesites»); para solo ese PID.

## Detección esperada
- Aviso en consola: `[db] Paro procesos vivos de esta rama que bloquean el build: PID …`.
- Sin ese aviso, el build no tenía bloqueo de esta rama.

## Verificación hecha
- `node --check` OK.
- Filtro en seco: solo casa el backend de la rama, no `PROYECTO TASKS`.
- `node scripts/run-with-db.mjs --version`: paró PID 27644 de la rama, dejó 25112 intacto, exit 0.

## Límite
- No probado con un `cargo test` que relinke de verdad. Queda en el roadmap (10AA-11).
