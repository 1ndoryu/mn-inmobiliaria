# Roadmap — MN-Inmobiliaria

**Contexto.** App de gestión inmobiliaria: backend Rust (`src/`), frontend (`frontend/`) y gateway de WhatsApp (`gateway/`). Se edita en el portátil y se compila, prueba y ejecuta en el VPS 89 (modo `build` por defecto: el backend sirve el front compilado en `:3110`); producción en el VPS 66 vía Coolify con el modo builder de `coolify-manager-rs`. Dev público: `mn-dev.wandori.us` (Cloudflare Access). Stack y comandos: `README.md`. Cada ítem enlaza su plan en su entrada; detalle íntegro del roadmap anterior: `Agente/planes/completados/roadmap-detalle-2026-10-12.md`; cierres por fecha: `Agente/completados/tareas-YYYY-MM-DD.md`.

## Siguiente bloque

- Ninguno.

## Pendientes

- Ninguno.

## Bloqueos

- Bloqueo `239A-1` — deploy, fase 5.4: confirmar la password admin rotada (o pedir otra).
- Bloqueo `199A-1` / `199A-2` — IA central en producción: en dev OpenCode Go es el proveedor activo y GloryAPI está deshabilitado (decidido 2026-11-02); falta decidir el proveedor de producción y, si fuera GloryAPI, su `GLORY_API_KEY` real.
- Ref. `10AA-21` — **Riesgo aceptado (`rsa` 0.9.10, RUSTSEC-2023-0071).** Decidido 2026-10-12. Se reabre al subir `sqlx` sin `sqlx-mysql`; criterio: `rsa` ausente de `Cargo.lock` y `check:back` verde.
- Ref. `08AA-30` — **Filas duplicadas en `mp_respuestas_cache`.** Decidido «Dejarlas» (161 filas, 48 `thread_id` repetidos, 82 sobrantes). No se borran; la unificación de `thread_id` sigue pendiente.
- Ref. `289A-5` — **Sesión `739bb63e` con `devolver_a_ia`.** Decidido «No, pendiente». No se reintenta ahora.
- Ref. `279A-4..8` — **Extras públicos.** Decidido 2026-10-12 (la usuaria): la ficha pública no muestra ningún extra, tampoco el precio negociable (revierte la decisión previa de publicarlo). Si mañana decide mostrar algo, se revisa lista por lista. La lista candidata y el criterio quedan en `Agente/planes/plan-extras-publicos-2026-10-12.md`. La IA sigue leyéndolos (ver `279A`).
- `279A` — **Ficha pública sin extras.** En pausa por decisión de la usuaria (arriba): sin trabajo pendiente. Si decide mostrar alguno, se abre la revisión lista por lista (`Agente/planes/plan-extras-publicos-2026-10-12.md`).
- **Entrada de personal en producción.** Acción de la usuaria (entrar una vez en la app); el agente no entra en producción.

## Planes activos

- `Agente/planes/plan-solicitudes-2026-09-16.md` — `169A-2` — «Publicar mi inmueble» (solicitudes públicas).
- `Agente/planes/plan-ia-central-2026-09-19.md` — `199A-1` — IA central: estado, proveedor y prueba.
- `Agente/planes/plan-deploy-produccion-2026-09-23.md` — `239A-1` — despliegue en producción (fase 5.4 pendiente).
- `Agente/planes/plan-optimizacion-pagespeed-seo-2026-09-24.md` — `279A` — PageSpeed y SEO profundo (medición pendiente).
- `Agente/planes/plan-bateria-escenarios-ia-2026-09-29.md` — `199A-1` — batería de escenarios IA por WhatsApp (bloqueada por la clave).
- `Agente/planes/plan-cache-inmueble-automatica-2026-10-09.md` — `09AA-30` — caché compartida por inmueble y detección automática.
- `Agente/planes/plan-cache-mensajes-inmueble-2026-10-09.md` — `09AA-30` — caché de mensajes por inmueble (parcialmente superado; se archiva al cerrar 09AA-30).
- `Agente/planes/plan-extras-publicos-2026-10-12.md` — `279A` — extras públicos (en pausa; se reabre si la usuaria lo decide).
