# Lecciones aprendidas

## 2026-05-08 — Core editor-agnostico en extensiones
- Para extraer un core real no basta cambiar tipos: hay que eliminar imports indirectos de servicios del editor, como `configService`, `vscode.workspace` o registries que lean settings globales.
- Si una regla aun necesita workspace/watchers, aislarla como callback/adaptador permite avanzar el core sin romper el provider existente.
- Los reportes y scanners deben recibir datos y providers como parametros; escribir archivos, abrir documentos y escuchar watchers pertenece al adaptador, no al core.
- Las pruebas unitarias con mocks de VS Code no garantizan que una CLI arranque en Node puro; despues de compilar hay que ejecutar el JS real y buscar imports indirectos de `vscode`.

## 2026-05-10 — LSP y lint como cierre de arquitectura
- Un LSP fino debe importar core y adaptadores de transporte, no la CLI; si CLI y LSP comparten defaults, moverlos a `core/config.ts` evita drift silencioso.
- Smoke stdio real debe buscar `textDocument/publishDiagnostics` y un `ruleId` esperado; compilar no prueba que el entrypoint LSP no este ejecutando codigo CLI.
- Activar lint tarde puede revelar errores de regex antiguos. Corregir escapes redundantes es bajo riesgo; patrones Unicode compuestos intencionales necesitan excepcion local documentada.
- Si se agregan fixtures `.tsx` fuera de `src`, `tsconfig.json` debe declarar `include` explicito; si no, `tsc` intenta compilar fixtures fuera de `rootDir` y crashea antes de ejecutar tests reales.

## 2026-09-16 - E2E con servidor detached y terminal sin estado

- Los jobs de PowerShell no sobreviven entre llamadas de terminal sin
  estado: los builds largos van en sincrono con timeout amplio y los
  servidores detached con `Start-Process` + archivo de log + probe de
  readiness (`/api/health`), con cleanup (`Stop-Process` + borrar logs)
  en el mismo bloque.
- Tras una llamada ambigua que pudo lanzar un proceso, la siguiente
  accion es una comprobacion discriminante (`Get-Process`, puerto, log),
  no relanzar: evita duplicar servidores en el mismo puerto.
- Un E2E honesto en degradado (sin clave IA: `reply:null`, mensaje
  persistido, sin escalado espurio) vale mas que un E2E simulado; deja
  por escrito que comportamientos quedan pendientes de credenciales.
