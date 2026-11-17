# Roadmap — MN-Inmobiliaria

**Contexto.** App de gestión inmobiliaria: backend Rust (`src/`), frontend (`frontend/`) y gateway de WhatsApp (`gateway/`). Se edita en el portátil y se compila, prueba y ejecuta en el VPS 89 (modo `build` por defecto: el backend sirve el front compilado en `:3110`); producción en el VPS 66 vía Coolify con el modo builder de `coolify-manager-rs`. Dev público: `mn-dev.wandori.us` (Cloudflare Access). Stack y comandos: `README.md`. Cada ítem enlaza su plan en su entrada; detalle íntegro del roadmap anterior: `Agente/planes/completados/roadmap-detalle-2026-10-12.md`; cierres por fecha: `Agente/completados/tareas-YYYY-MM-DD.md`.

## Siguiente bloque

- Ninguno.

## Pendientes

- [ ] `30AA-15` — **La v1 del export incluye los inmuebles sin cuenta.** Hoy
  `InmuebleRepository::por_facebook` filtraba por `inmueble_facebook` y cuatro inmuebles sin
  vínculo (Villa Betania, Propiedad Industrial, Calas Suites, Orquídea) desaparecían del
  manifiesto en vez de caer en `pendientes`. Hecho cuando `version 1/pendientes` del exe los
  contenga a los cuatro.
- [ ] `30AA-11` — **Cerrar Lechera y la v2 de los 4 inmuebles pequeños.** Hecho cuando
  `apartamento-amoblado-en-villa-sol-suites-lechera` (5/19), Villa Tocoma, Vista Hermosa, Coronado
  Plaza piso 4 y Las Peoñas estén con su v2 completa, medida con `medir.py`, con el texto de cada
  v2 generado (`POST /versiones` + `estado: lista`) y subido a la carpeta `Downloads\MN Inmobiliaria`
  por el exe.
- [ ] `30AA-5` — **Pestaña propia por versión en el modal de inmueble.** Hoy todas las pestañas
  v2+ comparten `PestanaFotos`: cambiar de pestaña no cambia lo que ya se generó y no hay forma de
  decir «esta foto tiene v2 y v3» desde la UI del modal. Hecho cuando cada versión tenga su propio
  estado y el botón de generar sepa a cuál escribe.
- [ ] `30AA-3` — **Extender `push` a versiones y variantes.** Hecho cuando `inmueble.mjs push --slug`
  suba la v2 (`inmueble_versiones`) y sus `fotos_variantes` a prod con el mismo `orden` de su
  original, `--dry-run` informe el diff de ambas y `verificar` las compare por sha256. Antes hay
  que desplegar la 30AA-1 (hoy prod da 404 en `/versiones` y `/configuracion/versiones`).

## Bloqueos

- Ninguno.

## Planes activos

- Ninguno.
