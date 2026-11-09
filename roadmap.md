# Roadmap — MN-Inmobiliaria

**Contexto.** App de gestión inmobiliaria: backend Rust (`src/`), frontend (`frontend/`) y gateway de WhatsApp (`gateway/`). Se edita en el portátil y se compila, prueba y ejecuta en el VPS 89 (modo `build` por defecto: el backend sirve el front compilado en `:3110`); producción en el VPS 66 vía Coolify con el modo builder de `coolify-manager-rs`. Dev público: `mn-dev.wandori.us` (Cloudflare Access). Stack y comandos: `README.md`. Cada ítem enlaza su plan en su entrada; detalle íntegro del roadmap anterior: `Agente/planes/completados/roadmap-detalle-2026-10-12.md`; cierres por fecha: `Agente/completados/tareas-YYYY-MM-DD.md`.

## Siguiente bloque

- Ninguno.

## Pendientes

- [ ] `30AA-3` — **Extender `push` a versiones y variantes.** Hecho cuando `inmueble.mjs push --slug`
  suba la v2 (`inmueble_versiones`) y sus `fotos_variantes` a prod con el mismo `orden` de su
  original, `--dry-run` informe el diff de ambas y `verificar` las compare por sha256. Antes hay
  que desplegar la 30AA-1 (hoy prod da 404 en `/versiones` y `/configuracion/versiones`).

## Bloqueos

- Ninguno.

## Planes activos

- Ninguno.
