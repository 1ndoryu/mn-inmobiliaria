# Plan deploy producción mn-inmobiliaria.com — 2026-09-23

> Estado: plan. Sin ejecución remota todavía.
> Decisiones del usuario: estructura **monorepo**; en producción se conserva
> **admin@admin.com** con contraseña nueva segura; DNS en Contabo = paso final (lo hace el usuario).

## Mapa verificado (descubrimiento 2026-09-23)

- Backend `MN-Inmobiliaria`, rama `inmobiliaria`, binario `glory-backend`, `sqlx::migrate!()` al arranque
  (el esquema se crea solo en prod), health `GET /api/health`, 13 migraciones `.up.sql`.
- `frontend/` del backend = stub React 18 del template. App real = repo `INMOBILIARIA`
  (React 19 + Vite 8 + Tailwind v4, build `tsc -b && vite build`), **sin remoto git**.
- Backend **no sirve estático** hoy (solo `/uploads/*`, `/api/*`, `/swagger-ui`). Sin `ServeDir`.
- No existe `Dockerfile.rust` en el repo (el template `rust-stack.yaml` lo exige:
  `dockerfile: Dockerfile.rust` + args `REPO_URL/BRANCH/APP_BIN/FRONTEND_DIR`).
- DB canonical rama = `glory_backend_inmobiliaria`: 11 inmuebles, 214 fotos, 0 solicitudes,
  0 suscriptores, `agent_config` 4 filas, tablas `users/inmuebles/fotos/solicitudes/suscriptores/notes/agent_*`.
  `fotos` usa `storage_key` (portable: se sirve en `/uploads/<storage_key>`).
- Usuarios locales: `admin@admin.com` + `import@example.com` (rescate; no va a prod).
- Uploads locales: 230 ficheros, ~28 MB.
- `.env` aún apunta a `inmobiliaria_db` (trampa: el wrapper la overridea, `cargo` a pelo no).
- Front lee API de `VITE_API_URL` o default `http://127.0.0.1:3000` (horneado en build;
  `""` hace fallback a localhost: en prod hay que hornear el dominio https).
- CORS hoy `allow_origin(Any)` con comentario "restringir en producción".
- Binario `coolify-manager-rs` **ausente** (`C:\tmp\...` purgado, sin `target/release`):
  hay que recompilar desde `coolify-manager-rs` rama `main`.
- `gh` sin login: crear repo + push lo hace el usuario (o se hace con su sesión).
- Submódulo `glory-rs` fijado en `4d95fb4`: el Dockerfile debe inicializar submódulos.

## Envs producción (mapeo)

| Local (.env) | Prod (Coolify) |
|---|---|
| `DATABASE_URL` | Fija del template (`rust_app/rust_db`, `postgres-{uuid}`) — auto |
| `JWT_SECRET` | Generado `$SERVICE_PASSWORD_64_JWTSECRET` — auto |
| `HOST/PORT` | `0.0.0.0` / `3000` — auto |
| `UPLOAD_DIR` | `/app/uploads` (bind `/data/uploads/inmobiliaria`) — auto |
| `RUST_LOG` | `info` — auto |
| `STATIC_DIR` | `/app/dist` (nuevo: código debe servirlo) |
| `OPENCODE_GO_API_KEY`, `GLORY_API_URL`, `AGENTE_CONTACTO` | `sync-env` post-creación |
| `VITE_API_URL` | build-arg Docker = `https://mn-inmobiliaria.com` |

## Fases

### Fase 0 — Cierre y alineación local (sin remoto)
1. Commitear frente INMOBILIARIA receta-servidor (10 ficheros, ID 229A-2) + backend
   suscriptor pendiente (`handlers/models/repositories/services/suscriptor` + migración 09).
2. `.env` → `DATABASE_URL=.../glory_backend_inmobiliaria` (canonical de la rama).
3. Gate local: `tsc -b` + `vite build` (front), `fmt/check/clippy/test` (backend), E2E receta.

### Fase 1 — Fusión monorepo (local, commit en `inmobiliaria`) — COMPLETADA 2026-09-23 (`39f383de`)
> Humo verificado: `/` 200 html, fallback SPA 200, `/api/health` 200 JSON, API inexistente 404
> seco, asset `.js` con `Content-Type` correcto, 11 publicados. Gate: `fmt` + `clippy -D warnings`
> + 15 tests + `tsc -b` + `vite build` (2.17 s). `glory-agent` se clona en el Dockerfile a ref
> pineado (`GLORY_AGENT_REF=9e357fb`); publicar `github.com/1ndoryu/glory-agent` es Fase 2.
> Gotchas: `try_exists` no vale para `/` (el dir pasa; usar `is_file`); `fallback` antes de
> `with_state`; clippy prohíbe `fn` anidada tras statements; `frontend/node_modules` requiere
> `npm ci` (un `tsc` global viejo da `erasableSyntaxOnly` falso).
> Corrección 2026-09-23 (239A-2): en prod manda el `Dockerfile.rust` DEL TEMPLATE del manager
> (`config/templates/Dockerfile.rust`: clona el repo dentro de la imagen en `/build`);
> el `Dockerfile.rust` del repo NO se usa (el `dockerfile: Dockerfile.rust` del compose lo
> resuelve el manager con su plantilla). Por eso el path-dep `../glory-agent` rompía el build
> (`failed to read /glory-agent/Cargo.toml`): se cambió a git-dep pineado
> `rev=f2f19e7` (publicado en `origin/main` de glory-agent, con caché `.sqlx`); build hermético
> en cualquier layout. `check/clippy/test` verdes con el git-dep.
1. Mover `INMOBILIARIA/*` → `MN-Inmobiliaria/frontend/` (reemplaza stub; conservar `.gitignore`,
   `orval.config.ts` del template si aplica; `prebuild generar-llms` sigue válido).
2. Backend: servir `STATIC_DIR` (`/app/dist`) con fallback SPA a `index.html` (solo si el
   dir existe; en dev sin cambio). `GET /api/health` intacto.
3. CORS: en prod solo `https://mn-inmobiliaria.com` (env o `cfg!(debug_assertions)`).
4. Crear `Dockerfile.rust` multi-stage (modelo agape, verificado contra el manager al ejecutar):
   `node:20` build frontend con `ARG VITE_API_URL` → `rust` `cargo build --release --bin glory-backend`
   (con `git submodule update --init`) → runtime `debian-slim` con `curl` (healthcheck),
   `UPLOAD_DIR=/app/uploads`, `STATIC_DIR=/app/dist`, `EXPOSE 3000`, CMD migraciones+binario.
5. Validación: `cargo check/clippy/test`, `tsc -b`, `vite build`, arranque local sirviendo `dist`,
   pack de humo (11 publicados, login admin, receta, subida foto).
6. Mejora IA solo-local: la mejora de fotos usa `GEMINI_PSID/PSIDTS` de tu Chrome
   (cuenta Horacio) vía `server/mejora.mjs :3122` + proxy Vite `/api` (solo dev). En prod
   ese servidor no existe y las cookies NO van al VPS (caducan, son sesión personal y el
   scraping web viola ToS de Google: IP de datacenter = baneo casi seguro). La UI debe
   detectar ausencia de `/api/estado` y desactivar "Mejorar" con aviso
   ("mejorar desde el equipo local"). Flujo prod para fotos nuevas: se mejoran en local
   y se suben a prod con el modal de fotos normal. Las 34 mejoradas ya importadas viajan
   con `uploads/` sin problema (son ficheros estáticos).

### Fase 2 — GitHub — COMPLETADA 2026-09-23 (repos PÚBLICOS, rama `main`)
1. Sesión `gh` como `1ndoryu` guardada en el equipo (token classic con scope `repo`;
   el fine-grained no puede crear repos: `createRepository` denegado).
2. `github.com/1ndoryu/glory-agent` (público): `master` local → `main` remoto (`962e463`).
   `Dockerfile.rust` sigue pineado a `GLORY_AGENT_REF=9e357fb` (estado verificado en el humo;
   subir el pin solo tras revalidar).
3. `github.com/1ndoryu/mn-inmobiliaria` (público): `inmobiliaria` local → `main` remoto
   (`0407ff8e` + `944bacf8` con el git-dep 239A-2). Ramas locales intactas para no interferir
   con la sesión paralela.

### Fase 3 — Recompilar manager + preflight (local) — COMPLETADA 2026-09-23
1. `cargo build --release` con `CARGO_TARGET_DIR=C:\tmp\glory-target\coolify-manager`.
2. `cm --version` (1.0.0), `cm list` (10 sitios), `cm new --help` (la ayuda manda; confirmar flags
   `--repo-url/--app-bin/--frontend-dir/--glory-branch`).
3. Build inicial 13m50s; la tarea horaria `GloryTmpSweep` purgó el target en idle (>60 min) y
   hubo que recompilar (9m24s con `RUSTC_WRAPPER=sccache`): para rachas deploy, recompilar
   justo antes o verificar el binario con `Test-Path` antes de invocarlo.

### Fase 4 — Crear sitio — COMPLETADA 2026-09-23 (autorización explícita del usuario)
- `cm new --name inmobiliaria --domain "https://mn-inmobiliaria.com" --template rust
  --glory-branch main --repo-url "https://github.com/1ndoryu/mn-inmobiliaria.git"
  --app-bin glory-backend --frontend-dir frontend --skip-theme --skip-cache`
  → UUID `as0scgwg44wkkkccgwcwg8w0`.
- `sync-env push` BLOQUEADO por validación del manager: exige `VITE_STRIPE_PUBLISHABLE_KEY`,
  `GLORY_STRIPE_SECRET_KEY`, `GLORY_STRIPE_WEBHOOK_SECRET` en local aunque el proyecto no usa
  Stripe, y el `RUST_PUSH_ALLOWLIST` no incluye `AGENTE_CONTACTO`/`OPENCODE_GO_API_KEY` (además
  el backend no lee esas 3 claves: ni `src/`, ni `frontend/src/`, ni glory-agent las referencian).
  Mejora pendiente al manager: template rust sin Stripe / required-keys por sitio. No se pusieron
  dummies (ensuciarían prod y el `VITE_*` se hornearía en el front).

### Fase 5 — Migración de datos — COMPLETADA 2026-09-24 (autorización explícita)
1. `pg_dump --data-only` (11 tablas incl. `agent_*`, sin `_sqlx_migrations`) → `C:\tmp`
   (54 KB; prod vacío confirmado: 0 users, 0 inmuebles). `import` del manager es solo-WP
   (busca contenedor `wordpress` inexistente en stacks Rust) → se usó `run-sql --file`
   (COPY 11/218/2). Mejora pendiente al manager: `import` agnóstico al stack.
2. Uploads (218 filas .jpg, 234 ficheros en disco con 16 huérfanos, 27.4 MB, máx 265 KB):
   el manager no tiene push local→volumen, así que se usó el canal propio de la app:
   `DELETE FROM fotos` (filas huérfanas) + 218× `POST /api/admin/fotos/upload`
   (`inmueble_id` UUID preservado, `orden`+`origen` explícitos, claves nuevas generadas
   por el servidor). 218/218 sin fallos (2 mitades de 109, reintento ×3).
3. Rotación PRIMERO (evita pedir la password local): password nueva de 27 chars generada
   con `secrets.token_urlsafe`, hash argon2id vía Python (`argon2-cffi`; el crate Rust
   `Argon2::default().verify_password` acepta cualquier param PHC) aplicado con
   `run-sql`; login prod con la nueva password verificado (token 165 chars).
   `import@example.com` eliminado; queda solo `admin@admin.com` (role admin).

### Fase 6 — Deploy + verificación (autorización explícita) — COMPLETADA 2026-09-24
1. `deploy --name inmobiliaria --update --skip-backup` (primer deploy: no hay contenedor que
   respaldar; build 456 s). Falla 1: path-dep glory-agent (→ 239A-2 git-dep). Tras el fix:
   swap OK, contenedor vivo (`Servidor iniciando en 0.0.0.0:3000`, `Front SPA embebido desde
   Some("/app/dist")`, cwd `/app` ⇒ `./uploads` cae en el bind `/data/uploads/inmobiliaria`).
2. Salud INTERNA verificada por `exec`: `curl localhost:3000/api/health` →
   `{"status":"ok","version":"0.1.0"}`; `/app/dist/index.html` existe; `run-sql`:
   13 migraciones aplicadas, `inmuebles` = 0 (pendiente Fase 5). Warning E17 (bind en clave
   `volumes`) cosmético; fixes post-build aplicados, `Runtime OK`.
3. Salud PÚBLICA: bloqueada al principio (sin DNS → rollback automático inofensivo).
   Tras el DNS del usuario (Dynadot DNS, A `@`+`www` → `66.94.100.241`; el dominio estaba en
   parking `dyna-ns.net`): Traefik servía `TRAEFIK DEFAULT CERT` (~10 min, backoff LE tras
   intentar emitir contra el parking). `restart` rechaza sitios Rust; el camino es
   `deploy-service --name inmobiliaria --skip-build --skip-backup` (recrea con la imagen
   existente, sin recompilar) → `Deploy exitoso! .../api/health respondiendo (status=200)`.
   Verificado desde local: `HEALTH:200 {"status":"ok","version":"0.1.0"}`,
   `HOME:200 text/html 1042 bytes` con cert válido.
4. E2E público COMPLETADO 2026-09-24: 11 publicados (`/api/public/inmuebles`, suma fotos
   24+14+16+20+12+18+12+30+12+32+28 = 218), foto HD 200 (116 KB reales), login
   `admin@admin.com` con password nueva, receta PUT (ojo: PATCH da 405; `None` en PUT
   conserva, no borra) → visible en pública → revertida a NULL vía `run-sql`, solicitud
   POST pública → visible en admin → borrada vía `run-sql` (no hay DELETE de solicitud),
   WhatsApp horneado en el front (`NUMERO_WHATSAPP = '584249208855'`). Estado final
   prod: 11 inmuebles, 218 fotos, 0 solicitudes, 1 user, `agent_config` con flags IA.

### Fase 7 — DNS (usuario) — COMPLETADA 2026-09-24
1. Dynadot → `Dynadot DNS`; A `@` → `66.94.100.241` (la sección "Registro de dominio" ES el
   apex, no pide host) + A `www` → `66.94.100.241`, TTL 5 min. (No hizo falta Cloudflare ni
   panel Contabo; con proxy habría que dejarlo en "DNS only".)
2. Propagación verificada desde local (`Resolve-DnsName` → `.241` en ambos); Traefik emitió
   el cert letsencrypt tras el `deploy-service` de Fase 6.
3. `health` + home pública por https verificados (ver Fase 6.3).

## Sincronización local ↔ prod (regla: prod es la única fuente de verdad)

No hay fusión bidireccional de base de datos (evita conflictos de UUID, `publicado`,
recetas y fotos). Cada entorno tiene un único escritor:

- **Prod = operativa real.** Altas, ediciones, publicados, recetas y fotos finales se hacen
  en `https://mn-inmobiliaria.com`. Lo que nace en prod vive en prod.
- **Local = sandbox + herramienta de mejora.** Lo que nace en local (pruebas, drafts) se
  queda en local y nunca sube solo. El `pull` lo puede pisar sin aviso.

### Flujos
1. **Pull prod→local (script `scripts/sync-pull.mjs`, solo lectura contra prod):** login
   JWT admin prod → lista admin + públicas → vuelca a la DB local y descarga `/uploads/*`
   al `UPLOAD_DIR` local. Sirve para traer a local un inmueble nuevo de prod y mejorarlo.
   Flag de confirmación; nunca escribe en prod.
2. **Local→prod solo ficheros, vía UI admin de prod:** la foto mejorada se sube con el modal
   de fotos normal (autenticado, al `id` del inmueble de prod). Nada de SQL directo ni push.
3. **Metadatos no se sincronizan:** recetas, `publicado`, copy y precios se editan donde se
   usan (prod). Si algo se prueba en local, se re-guardar a mano en prod.
4. **Solicitudes/suscriptores de visitantes** solo existen en prod; el pull no los toca
   (solo escribe en local) y el backup externo del manager protege prod.

### Conflictos: no se resuelven, se evitan
| Caso | Resolución |
|---|---|
| Inmueble nuevo en prod | Pull lo trae a local; la mejora se hace en local y el fichero sube a prod vía UI |
| Inmueble nuevo en local | Se queda en local (prueba/dev); en prod se crea de nuevo si hace falta real |
| Mismo inmueble editado en ambos | Manda prod; el pull sobrescribe local |
| Receta guardada en local | No viaja; se re-guardar en prod |

### Automatización (Fase 1.7): un comando ida-vuelta, cero pasos manualesFlujo diario objetivo: `npm run mejora:prod -- --inmueble <slug> [--todas] [--forzar]` y listo.
El script `scripts/mejora-prod.mjs` (solo Node, sin dependencias nuevas) hace todo solo:

1. **Preflight que no deja fallar a medias:** mini-backend local `:3122` vivo →
   sonda `POST /api/probar` (valida cookies sin gastar foto; si están muertas aborta con
   "ejecuta `npm run renovar-cookies`") → login prod (JWT solo en memoria) → el inmueble
   y la foto existen en prod. Cualquier fallo = exit 2 + mensaje accionable, sin efectos.
2. **Descarga** el original desde `{PROD}/uploads/<clave>` (público, sin auth) a `C:\tmp`
   (nunca al repo).
3. **Encola en la cola local existente** (`POST /api/mejora` de mejora.mjs; reutiliza su
   dedupe, backoff y topes) y **espera por poll** con timeout (exit 3 si la mejora falla).
4. **Sube el resultado a prod** vía `POST /api/admin/fotos/upload?inmueble_id=&filename=
   &origen=mejorada[&orden=]` (bytes crudos + JWT; máx 10 MiB) y **verifica** releyendo
   las fotos del inmueble (exit 4 si no aparece).
5. **Idempotente por defecto:** si prod ya tiene `mejorada` para esa foto/orden, lo salta
   (solo `--forzar` la rehace). `--todas` recorre las originales del inmueble.
6. **Secretos:** `scripts/.env.prod.local` (gitignored: `PROD_API`, `ADMIN_EMAIL`,
   `ADMIN_PASSWORD`); nunca se imprimen, nunca van a git. Logs en `logs/`, temporales en
   `C:\tmp` que se borran solos.

`scripts/sync-pull.mjs` (espejo prod→local vía APIs, destructivo en local con `--si`):
solo para traer snapshot a local cuando se quiera probar/dev; la mejora-prod NO lo necesita
(descarga ella misma el original de prod). Los UUID pueden diferir entre entornos: todo se
resuelve por `slug` + `orden`, nunca por id hardcodeado.

### Resolución de mejoradas (Fase 1.8): diagnóstico verificado 2026-09-23
Medido en prod-local (inmueble `2a273566…`): original orden 0 = 896×1195 → mejorada = 896×1195;
original orden 1 = 810×1080 → mejorada = 896×1195. Cadena verificada sin culpables en
guardado ni visualización (`urlADataUrl` = fetch→base64 sin canvas; worker guarda bytes tal
cual con intento full-size + `=s2048`; subida = bytes crudos). Causas reales, en orden:
1. **Entrada ya reducida:** `comprimirImagen` topa en 1280px lo que se sube desde disco
   (los originales 896/810px lo confirman). Gemini nunca ve los 4000px del teléfono.
2. **Salida nativa del modelo:** Gemini renderiza a resolución propia (~896×1195 en
   retrato 3:4) ignore el tamaño de entrada; el sufijo `=s2048` no inventa píxeles. En la
   web sale más grande porque pides upscale explícito y el botón Descargar trae el render
   full-size, mientras el worker toma `images[0]` (render inline).
Fix en orden (probar B primero, es gratis): **B.** prompt explícito de upscale
("devuelve mínimo 2048px en el lado mayor, doble de resolución") y comparar medidas;
**A.** subir `ladoMax` de subida (1280→2048 o full-res) aceptando más MB en cola/IndexedDB;
**descartado C.** reescalado local (Lanczos) = nitidez falsa. Mejorar la fuente también
endereza la publi HD 1080 (hoy reescala desde 896px).

**Validado 2026-09-23 (fix A1, descarga):** `server/gemini_worker.py::_bajar_mejorada`
ahora descarga 3 variantes (tal cual / `=s0` / `=s2048`) y se queda con la de más
píxeles; además el RPC full-size vacío u omitido queda registrado en el log (antes era
silencioso) y se eliminó el apéndice ciego de sufijos (podía producir `=s512=s2048-rj`
malformado). Prueba real con original 1280×1280: talcual=512×512, s0=1024×1024,
s2048=1024×1024 → elegida s0, imagen verificada visualmente. Conclusión: `=s0`
garantiza el render completo (4× píxeles frente al preview pelado), pero el modelo
genera a ~tamaño de entrada: para superar la entrada sigue pendiente **B** (prompt de
upscale). **B probado mismo día y DESCARTADO como palanca de tamaño:** con instrucción
explícita ("ultra HD, mínimo 2048px lado mayor, doble de resolución") sobre el mismo
original 1280×1280, salida = 1024×1024 idéntica (calidad correcta, sin artefactos). El
modelo ignora la demanda de tamaño: techo nativo ~1MP (1024×1024 / 896×1195) por esta
vía. 2048px+ reales exigirían otro endpoint (p. ej. Imagen en alta res o upscaler local
dedicado), fuera del alcance de este fix. Sin reinicio del daemon: el worker se re-generaba solo (`vivo:false`, cola
vacía) y el próximo Reintentar ya usa el código nuevo. Cookies renovadas mismo día
(`npm run renovar-cookies` OK). Hallazgo lateral RESUELTO mismo día: `.env` apuntaba a
`inmobiliaria_db` (5 inmuebles); corregido a `glory_backend_inmobiliaria` (11/218) y
validado con `cargo check --tests` + `cargo test` (15 passed, 0 failed, target en
`C:\tmp`).

## Rollback
- E11 automático del manager ante health fallido + backup pre-write de Fase 5.
- `restart --all` PROHIBIDO (mata workloads Rust). Solo `deploy-service --name inmobiliaria`.

## Riesgos / gotchas anotados
- `VITE_API_URL=""` cae al fallback localhost: hornear dominio exacto con `https://`.
- `422 base64` = bytes >127 en compose (plantillas ya ASCII; no tocar).
- Reglas Traefik `Host()` con backticks (ya en template).
- `frontend/dist`, `uploads/`, `.env` no van a git (verificar `.gitignore`).
- `glory-rs` es submódulo: el clone de Coolify/Dockerfile debe traerlo (`--recursive`/init).
