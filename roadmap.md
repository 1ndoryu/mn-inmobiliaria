Objetivo: crear un template de Rust + Ts React + OpenAPI + Codegen + Clippy nivel paranoia, para crear sitios web, un solo repositorio, con una pagina sencilla de ejemplo, con el gitignore configurado y todo listo para empezar a desarrollar, adelante a todo, comandos, documentación, he creado https://github.com/1ndoryu/glory-rs para lo subas alli, se tiene que pdoer crear varias ramas para varios sitios y que no haya problema de cambiar a un sitio a otro cambiando de rama, aplica todo lo que por defecto vendría siendo optimización, seguridad, rendimiento, experiencia de desarrolo, etc, de base de datos tengo corriendo postgres aca localmente.

## Estado: ✅ Template base completado (253A-1)

Ver `Agente/completados/tareas-2026-03-25.md` para detalles.

## Stack implementado

| Capa | Herramienta |
|------|-------------|
| Framework web | Axum 0.7 |
| OpenAPI | utoipa 4 + utoipa-swagger-ui 7 |
| Serialización | serde |
| Base de datos | SQLx 0.8 (PostgreSQL) |
| Migraciones | SQLx migrate |
| Validación | validator 0.18 |
| Variables de entorno | dotenvy |
| Logging | tracing + tracing-subscriber |
| Errores | thiserror 2 |
| Auth | jsonwebtoken + argon2 |
| CORS | tower-http |
| Linter | clippy (deny all + warn pedantic) |
| Frontend | React 18 + TypeScript + Vite |
| State | React Query + Zustand |
| Codegen | Orval 8 (reemplaza openapi-typescript-codegen) |

## Pendientes

## Receta publicitaria en el servidor (229A-2, en curso 2026-09-22)

- El modal público genera la publi con receta automática; lo configurado en
  admin vive solo en `localStorage` del admin y nunca llega al público.
- Backend: columna `inmuebles.receta JSONB` (migración `...12`), struct
  `RecetaPublicidad` validada (formato allowlist, índices >= 0), `PUT`
  la fija, el `GET` público la expone.
- Front (repo `INMOBILIARIA`): `guardar` hace PUT con la receta; el modal
  público usa `i.receta` (servidor manda, local queda de reserva).

## Notas

- Configurar coolify-manager-rs para desplegar proyectos Rust (repo separado)
- Prioridades: 1. Velocidad desarrollo, 2. Decisiones futuras, 3. Rendimiento, 4. Seguridad, 5. Popularidad, 6. Facilidad, 7. Docs, 8. Compatibilidad, 9. Flexibilidad, 10. Escalabilidad

## F5 chat visitante con IA (169A-1, en curso 2026-09-16)

- Plan: `Agente/planes/plan-chat-inmobiliaria-2026-09-16.md`.
- Backend: `glory-agent` por path, migracion `agent_*`, rutas
  `/api/agent/*` (WS + REST + historial), PromptConfig inmobiliario.
- Front (`INMOBILIARIA`, repo hermano): widget en `src/features/publica/`
  montado en `PaginaPublica`, tokens de `disenno.ts` (sin redondeados,
  sin sombras, Söhne).
- Alcance v1: sin tools (la ejecucion de tools es F6 en `glory-agent`).
- Requiere del usuario: fijar `OPENCODE_GO_API_KEY` y `AGENTE_CONTACTO`
  reales en `MN-Inmobiliaria/.env` (sin key el chat persiste y reenvia
  en realtime pero la IA no responde).

## Publicar mi inmueble: solicitudes pendientes (169A-2, en curso 2026-09-16)

- Plan: `Agente/planes/plan-solicitudes-2026-09-16.md`.
- Backend: tabla `solicitudes`, subida publica de fotos (reutiliza
  validacion magic-bytes + 10 MiB), `POST /api/public/solicitudes`,
  `POST /api/public/solicitudes/fotos`, `GET/PATCH /api/admin/solicitudes`
  (JWT). Validacion basica de email/telefono.
- Front (`INMOBILIARIA`, repo hermano): `ModalPublicar` cuadrado con
  borrador persistente; las fotos ya subidas son claves (no pesan).
- Sin UI admin de revision (pendiente explicito para despues).

## Chat con atencion humana + WhatsApp + config admin (169A-3..6, 2026-09-16)

- Plan: `Agente/planes/completados/plan-chat-atencion-2026-09-16.md`
  (cerrado 2026-09-16; diseno contra Nakomi como referencia
  solo-lectura).
- 169A-3 (repo `glory-agent`): loop de tools F6 + gate IA real
  (`ai_enabled` + ciclo `escalated` se hacen cumplir) + `0002_config.sql`
  + persistencia sesiones/config + tests. Nucleo agnostico.
- 169A-4 (este repo) [x] 2026-09-16: 5 tools (`buscar_inmuebles`,
  `detalle_inmueble`, `registrar_contacto`, `datos_contacto`,
  `escalar_a_humano`) + worker WhatsApp (outbox, poll 15 s) + rutas staff
  `/api/admin/agent/*` (JWT) + publicas `/api/agent/info` y
  `/sesiones/:id/contacto` + hub compartido. Sin seeds: claves opcionales
  con defaults en codigo. Gate verde (fmt/check/clippy/test 11 lib OK);
  humo BD rama OK (tools contra esquema real, staff toma hilo, config
  roundtrip, worker marca `failed` sin gateway, 2 tests HTTP OK).
- 169A-5 (repo `INMOBILIARIA`) [x] 2026-09-16, commit `fae846e`: admin
  Mensajes (lista + hilo + responder + tomar/soltar IA) y Config chat
  (prompt_extra, telefonos, kill-switch). `oxlint` 0 errores, `npm run
  build` verde, dev 200. Incluye frente visitante 169A-1 (cliente, canal
  WS+REST, hook, ventana).
- 169A-6 (repo `INMOBILIARIA`) [x] 2026-09-16, commit `fa745e0`: tarjeta
  contacto/escalado en el widget (`pedirInfo` + `enviarContacto`,
  auto-apertura si pide humano) + E2E en vivo contra este backend
  (health/messages/history/info/contacto OK, staff 401 sin token).
- Requiere del usuario: `GLORY_ALERT_GATEWAY_URL` + numero admin
  (`whatsapp_admin`); `OPENCODE_GO_API_KEY` + `AGENTE_CONTACTO` siguen
  pendientes de 169A-1.

## Centro de IA: GloryAPI + OpenCode Go via backend (199A-1/199A-2, en curso 2026-09-19)

- Plan: `Agente/planes/plan-ia-central-2026-09-19.md`.
- 199A-1 (este repo): `src/handlers/ia.rs` con `GET /api/admin/ia/estado`,
  `PUT /api/admin/ia/config`, `POST /api/admin/ia/probar`,
  `POST /api/admin/ia/completar` (JWT); config en `agent_config`
  (`ia_activo`, `ia_hab_*`, `ia_check_*`); claves solo en `.env`
  (`GLORY_API_URL`, `GLORY_API_KEY`, `OPENCODE_GO_API_KEY`).
  Codigo completo 2026-09-19: `fmt` + `check` + `clippy -D warnings` +
  `test --lib` 14/14 (3 nuevos `handlers::ia::pruebas`).
  Pendiente humo HTTP: el dev-server vivo (otra sesion) usa binario anterior;
  reiniciarlo y probar `estado` + `completar` antes de 199A-2.
- 199A-2 (repo `INMOBILIARIA`): redaccion y copy via `completar` (activo
  manual + fallback); pestana `ia` en Config con estado, habilitar/elegir
  activo y probar por proveedor.
- Requiere del usuario: `GLORY_API_KEY` real en `MN-Inmobiliaria/.env`
  (sin ella GloryAPI queda `sin-clave` y todo va por OpenCode Go).

## Sesion admin de 10 años (199A-3, en curso 2026-09-19)

- El JWT ya duraba 365 dias con `JWT_SECRET` estable en `.env` (gitignored):
  la sesion solo caia por secreto rotado o expiracion real.
- 199A-3: `generate_token` pasa a 10 años. Tras arrancar el servidor nuevo,
  salir y entrar una vez para que el token traiga la expiracion larga
  (los tokens viejos conservan la suya).

## Editar inmueble 405: front mandaba PATCH, contrato es PUT (199A-4, 2026-09-19)

- `actualizarRemoto` enviaba `PATCH /api/admin/inmuebles/:id` pero el router
  solo acepta `PUT` (asi desde 159A-2 + utoipa `put`): 405 preexistente, no
  causado por los reinicios de 199A-1/199A-3.
- Fix en repo `INMOBILIARIA`: `PUT` en `actualizarRemoto` (manda el objeto
  entero = reemplazo, semantica PUT correcta). `fijarPublicado` sigue PATCH
  (su ruta si es `patch`). Verificado con PUT+JWT a id inexistente → 404.

## Fotos 404 al editar: sincronizarFotos borraba antes de re-descargar (199A-5, 2026-09-19)

- Ante CUALQUIER cambio de fotos (anadir, quitar, reordenar, elegir
  principal), `sincronizarFotos` borraba TODOS los originales (fila + archivo
  en disco via `delete_foto`) y luego intentaba re-descargar las conservadas
  del servidor para re-subirlas: 404 inevitable ("No se pudo leer la foto
  para subirla") y el inmueble quedaba sin fotos (perdida real; los datos
  texto si se guardaban porque el PUT va antes).
- Fix en repo `INMOBILIARIA` (commit `c0037dc`): leer los bytes de TODAS las
  deseadas ANTES de borrar; si alguna falla no se ha borrado nada. Servidor
  verificado sano (fotos existentes responden 200, 92 archivos en
  `uploads/`); causa 100 % front, no backend.
- Pendiente usuario: revisar la propiedad editada y re-subirle las fotos
  perdidas; recargar `:5200` con `Ctrl+F5`.

## Principal + mejoradas en el frente público (199A-6, 2026-09-19)

- Dos causas front (repo `INMOBILIARIA`, commit `7e245b2`; backend intacto,
  la API pública ya devuelve originales + mejoradas emparejadas por `orden`):
  1. La galería del detalle público (`modal-detalle-publico.tsx`) mostraba
     `i.fotos` = solo originales; las mejoradas solo aparecían en la portada
     de la tarjeta. Ahora usa `fotosVisiblesDe` (mejorada por posición si
     existe, si no el original; `portadaDe` = primera visible).
  2. Al reordenar/elegir principal, `sincronizarFotos` re-creaba los
     originales con `orden` nuevo pero dejaba las mejoradas con el viejo: el
     pareo por `orden` quedaba roto y la portada mostraba la mejora de otra
     foto. Ahora las mejoradas se descargan antes de borrar y se re-suben
     siguiendo a su original (misma garantía sin-pérdida de 199A-5); la de un
     original eliminado se descarta.
- Pendiente usuario: `Ctrl+F5` en `:5200` y comprobar Townhouse/Orquídea
  (editadas hoy 17:07): si alguna portada sigue mostrando la mejora de otra
  foto (pareo roto anterior, irreparable por contenido), avisar cuál para
  limpiar sus mejoradas y regenerarlas pareadas.

## Pareo roto en Orquídea: reparado por contenido (199A-7, 2026-09-19)

- La usuaria elegía principal en admin (se guardaba bien) pero en público
  seguía viendo otra foto: el pareo `orden` original↔mejorada de Orquídea
  estaba roto por un reorden anterior (las mejoradas conservaban el `orden`
  viejo). Demostrado por hash perceptual (dHash, distancias 0-8 = misma
  escena): mejorada 0→original 8, 1→9, 2→10, 3→0, 4→11, 5→12. La portada
  (mejorada 0) mostraba la mejora del original 8, no de la principal.
- Townhouse comprobada igual: sus 6 parejas correctas, no se tocó.
- Reparación solo-datos vía API (sin código ni reinicio): subidas primero
  las 6 mejoradas con `orden` correcto (`origen=mejorada`, mismos bytes),
  luego borradas las 6 viejas por id, verificado por contenido (las 6
  parejas distancia ≤8). Total Orquídea: 15 originales + 6 mejoradas = 21.
- El fix 199A-6 impide futuros despareos al reordenar desde el front.
- Seguimiento propuesto (no urgente): endpoint `PATCH /api/admin/fotos/:id`
  con `{orden}` para reordenar sin borrar+re-subir (menos churn y sin
  cambiar urls/ids que cachea el store local de mejora).

