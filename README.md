# Glory RS

Template para sitios web con **Rust (Axum) + React (TypeScript) + OpenAPI** en un solo repositorio.

Pensado para máxima velocidad de desarrollo, seguridad por defecto y escalabilidad.

## Stack

| Capa                 | Herramienta                  | Para qué                                |
| -------------------- | ---------------------------- | --------------------------------------- |
| Framework web        | Axum                         | HTTP, routing, middleware               |
| OpenAPI              | utoipa + utoipa-swagger-ui   | Genera schema OpenAPI desde código      |
| Serialización        | serde                        | JSON ↔ Structs                          |
| Base de datos        | SQLx (PostgreSQL)            | Queries SQL con verificación            |
| Migraciones          | SQLx migrate                 | Control de schema DB                    |
| Validación           | validator                    | Validar inputs del usuario              |
| Variables de entorno | dotenvy                      | Cargar .env                             |
| Logging              | tracing + tracing-subscriber | Logs estructurados                      |
| Errores              | thiserror                    | Errores tipados                         |
| Auth (JWT)           | HS256 propio                 | Tokens (HS256, `src/services/jwt.rs`)   |
| Hashing              | argon2                       | Hashing seguro de contraseñas           |
| CORS                 | tower-http                   | Middleware CORS                         |
| Linter               | clippy (paranoia)            | Código limpio                           |
| Frontend             | React + TypeScript + Vite    | UI                                      |
| State management     | React Query + Zustand        | Server state + client state             |
| Codegen              | Orval                        | Genera cliente TypeScript desde OpenAPI |

## Requisitos

- Rust (stable, 1.75+)
- Node.js (18+) y npm
- PostgreSQL corriendo localmente

## Inicio rápido

```bash
# 1. Clonar el template con el framework fijado
git clone --recurse-submodules --branch main https://github.com/1ndoryu/glory-rs-template.git nuevo-proyecto
cd nuevo-proyecto
git submodule update --init --recursive
cp .env.example .env
# Editar .env con tus credenciales de PostgreSQL

# 2. Crear la base de datos
psql -U postgres -c "CREATE DATABASE glory_db;"

# 3. Backend
cargo run
# El servidor inicia en http://localhost:3110 ([03AA-1]: el 3000 lo ocupa glory-pulse)
# Swagger UI en http://localhost:3110/swagger-ui/

# 4. Frontend (en otra terminal)
cd frontend
npm install
npm run dev
# Frontend en http://localhost:5173

# 5. Generar cliente API (con backend corriendo)
npm run codegen
```

## Estructura del proyecto

```
├── Cargo.toml              # Dependencias del backend
├── src/
│   ├── main.rs             # Entry point del servidor
│   ├── lib.rs              # Re-exports y AppState
│   ├── config/             # Configuración desde env vars
│   ├── errors/             # Tipos de error → HTTP status codes
│   ├── handlers/           # Capa HTTP (routing, request/response)
│   ├── middleware/          # Auth middleware (JWT extractor)
│   ├── models/             # Structs de dominio y DTOs
│   ├── repositories/       # Capa de base de datos (queries)
│   └── services/           # Lógica de negocio
├── migrations/             # Migraciones SQL (SQLx)
├── frontend/
│   ├── src/
│   │   ├── api/            # Cliente API generado por Orval
│   │   ├── App.tsx         # Componente raíz
│   │   └── main.tsx        # Entry point React
│   ├── orval.config.ts     # Configuración de codegen
│   └── vite.config.ts      # Configuración de Vite + proxy
├── .env.example            # Variables de entorno de ejemplo
└── .gitignore
```

## Arquitectura

El backend sigue separación en capas:

- **handlers/** → Reciben HTTP requests, extraen datos, llaman services, retornan responses
- **services/** → Lógica de negocio, orquestan repositories
- **repositories/** → Queries a PostgreSQL via SQLx
- **models/** → Structs de dominio, DTOs de request/response, schemas OpenAPI
- **errors/** → Enum de errores que mapean a HTTP status codes
- **middleware/** → Extractores de Axum (auth JWT)

## API de ejemplo

El template incluye un CRUD de notas con autenticación:

| Método | Ruta               | Descripción             | Auth |
| ------ | ------------------ | ----------------------- | ---- |
| POST   | /api/auth/register | Registrar usuario       | No   |
| POST   | /api/auth/login    | Iniciar sesión          | No   |
| GET    | /api/health        | Health check            | No   |
| POST   | /api/notes         | Crear nota              | Sí   |
| GET    | /api/notes         | Listar notas (paginado) | Sí   |
| GET    | /api/notes/:id     | Obtener nota            | Sí   |
| PUT    | /api/notes/:id     | Actualizar nota         | Sí   |
| DELETE | /api/notes/:id     | Eliminar nota           | Sí   |

## Sincronización con el servidor (panel de inmuebles)

La columna **Sync** de la tabla de inmuebles compara cada inmueble local con el servidor (fotos, descripción y detalles) y dice qué lado va por delante: Igual, Local adelante, Servidor adelante, Ambos cambiaron, Solo local; «Solo en el servidor» es un aviso sobre la tabla. Es de solo lectura: no escribe en el servidor. Tarea 13AA-12.

- **Activación:** un fichero dotenv con `PROD_BASE`, `PROD_EMAIL` y `PROD_PASSWORD`. Por defecto es `scripts/.env.prod.local` (el que ya usan los scripts de producción); `SYNC_REMOTO_ENV`, si está definida, apunta a otro. Sin fichero, el estado es «desactivado». Se relee en cada consulta: crear o cambiar el fichero no exige reiniciar. Las credenciales nunca se muestran ni se registran.
- **Intervalo:** `SYNC_INTERVALO_SEG` (20 s por defecto, mínimo 5) para consultar el servidor. Los cambios locales se reflejan en ~1 s.
- **Base de tres vías:** tabla `sync_base`, escrita solo por la tarea de fondo al observar un inmueble igual. Sin base, el estado sale como «estimado» (usa `updated_at`).
- **Fotos:** se comparan por `(origen, orden)` y sha256 de los bytes. Las `mejorada` solo por presencia, porque el servidor las recodifica.
- **Endpoints admin (JWT):** `GET /api/admin/sync/estado?desde=<rev>` (long-poll, 25 s), `POST /api/admin/sync/comprobar` (202), `GET /api/admin/sync/manifiesto`, `GET /api/admin/sync/inmuebles/{slug}`, `GET /api/admin/sync/diferencias/{slug}`.

## Ramas por sitio

Este template está diseñado para usar **una rama por sitio/proyecto**:

```bash
git checkout -b mi-sitio-web
# Desarrollar en la rama
# Cambiar a otro sitio:
git checkout otro-sitio
```

La estructura es idéntica en cada rama. Solo cambia el contenido específico del sitio.

## Base de datos por rama

Cada rama usa su propia base de datos: `<base>_<rama>`. `main` usa la base de `DATABASE_URL`. Los comandos `node scripts/run-with-db.mjs ...` eligen la base de la rama actual (`scripts/branch-db.mjs`).

Consecuencia: los tokens que emite MN para el laboratorio (`opencode-propio-dev`) están en la base de `main`. Una rama nueva empieza sin ellos, y el laboratorio recibe 401 y aparece como «no conectado» ([13AA-8]).

- **Automático:** al arrancar el backend con `run` en una rama distinta de `main`, `scripts/run-with-db.mjs` copia a la base de la rama los tokens de `main` (`copiarTokensLabABdRama`). Es idempotente y nunca escribe en `main`.
- **Se copian también los revocados**, para que una revocación en `main` llegue a la rama.
- **Inmuebles y fotos ([13AA-9]):** al arrancar con `run` también se copian de `main` los inmuebles y fotos que la rama no tenga (`copiarInmueblesABdRama`). No se copian las visitas (datos personales). Un cambio hecho en `main` a un inmueble ya copiado no llega a la rama.
- **Tokens nuevos:** los emitidos después del arranque se copian en el siguiente arranque del backend en esa rama.
- **Comprobar:** en `Arranque logs "MN-Inmobiliaria"` debe aparecer `[db] Tokens del laboratorio vigentes en esta rama: N`.
- **Si falla:** sale un aviso `[db]` y el backend sigue; el laboratorio no se conectará en esa rama hasta que se resuelva la causa (suele ser `psql` no disponible o la base de `main` inaccesible).

## Comandos útiles

```bash
# Comando unificado — verifica todo el proyecto (backend + frontend)
npm run check

# Backend
cargo run                    # Iniciar servidor
cargo check                  # Verificar compilación
cargo clippy                 # Linter (nivel paranoia)
cargo test                   # Tests
cargo fmt                    # Formatear código
npm run check:back           # cargo check + clippy

# Frontend
npm run dev:front            # Dev server con HMR
npm run check:front          # Type-check TypeScript
npm run codegen              # Regenerar cliente API desde OpenAPI

# O directamente desde frontend/
cd frontend
npm run dev                  # Dev server con HMR
npm run build                # Build producción
npm run type-check           # Verificar tipos TypeScript
npm run codegen              # Regenerar cliente API desde OpenAPI
```

## Lock de calidad (Sentinel y VarSense)

`sentinel.lock.json` fija versión, commit y sha256 de las herramientas declaradas en `quality-tools.json`. No se edita a mano. Desde la raíz de MN, con el checkout de las herramientas instalado:

```bash
# Validar (sin escribir): sale con 0 si el lock coincide
node "<area-trabajo>/workspace-manager/scripts/quality/lock-generator.mjs" --check

# Regenerar tras cambiar la versión fijada (escribe con backup previo)
node "<area-trabajo>/workspace-manager/scripts/quality/lock-generator.mjs" --write
```

`varsense.config.json` exime por patrón las clases que VarSense no puede seguir desde constantes TS (`texto-publica`, `fases-presentacion`, `parpadeo-cursor`; ver `frontend/src/features/publica/disenno.ts`).

## Clippy nivel paranoia

El proyecto tiene configurado clippy en modo estricto (`[lints.clippy]` en Cargo.toml):

- `clippy::all` → **deny** (error en cualquier warning estándar)
- `clippy::pedantic` → **warn** (warnings extra para código idiomático)

Antes de cada commit: `cargo fmt --check && cargo clippy && cargo test`
