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

### Fase 1 — Fusión monorepo (local, commit en `inmobiliaria`)
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

### Fase 2 — GitHub (acción del usuario)
1. `gh auth login` (o web).
2. Crear `github.com/1ndoryu/mn-inmobiliaria` (privado) + push rama `inmobiliaria`
   (incluye `frontend/` fusionado + `Dockerfile.rust`; el template-remote se conserva).

### Fase 3 — Recompilar manager + preflight (local)
1. `cargo build --release` con `CARGO_TARGET_DIR=C:\tmp\glory-target\coolify-manager`.
2. `cm --version`, `cm list`, `cm new --help` (la ayuda manda; confirmar flags
   `--repo-url/--app-bin/--frontend-dir/--glory-branch`).

### Fase 4 — Crear sitio (REQUIERE autorización explícita del usuario en el momento)
- `cm new --name inmobiliaria --domain "https://mn-inmobiliaria.com" --template rust
  --glory-branch inmobiliaria --repo-url "https://github.com/1ndoryu/mn-inmobiliaria.git"
  --app-bin glory-backend --frontend-dir frontend --skip-theme --skip-cache`
- `sync-env` para `OPENCODE_GO_API_KEY`, `GLORY_API_URL`, `AGENTE_CONTACTO`;
  `VITE_API_URL` como build-arg en panel Coolify. Verificar con `exec printenv`.

### Fase 5 — Migración de datos (con backup pre-write; SIN `--skip-backup`)
1. `pg_dump` data-only de `glory_backend_inmobiliaria` (tablas `users/inmuebles/fotos/
   solicitudes/suscriptores/agent_config/notes` si aplica; sin `_sqlx_migrations`:
   el esquema lo crean las 13 migraciones al primer arranque).
2. `import-database` al postgres del stack (`rust_db`).
3. Transferir `uploads/` (230 ficheros, ~28 MB) al bind `/data/uploads/inmobiliaria`
   vía capacidad del manager (`restore`/`exec`; nunca SCP/SSH directo).
4. Rotar `users.password_hash` de `admin@admin.com` a hash argon2 nuevo (generado local
   con la misma versión del crate, aplicado vía `run-sql`); eliminar `import@example.com`
   (confirmar en el momento).

### Fase 6 — Deploy + verificación (autorización explícita)
1. `deploy --name inmobiliaria --update` (build Rust 8–12 min; 503 intermedio = normal).
2. `health` + `logs --target app --lines 50`.
3. E2E contra `https://mn-inmobiliaria.com`: 11 publicados, fotos HD, login
   `admin@admin.com`, guardar receta, crear solicitud de prueba y borrarla,
   WhatsApp `wa.me/584249208855` (modo prod, sin banner local).

### Fase 7 — DNS Contabo (usuario, paso final)
1. En Contabo: registro A `mn-inmobiliaria.com` (+ `www` si se quiere) → IP del VPS
   (la confirma el preflight del manager al ejecutar).
2. Esperar propagación; Traefik emite cert letsencrypt solo (~1–5 min).
3. Re-verificar `health` + home pública por https.

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
