# Roadmap — MN-Inmobiliaria

**Contexto.** App de gestión inmobiliaria: backend Rust (`src/`), frontend (`frontend/`) y gateway de WhatsApp (`gateway/`). Se edita en el portátil y se compila, prueba y ejecuta en el VPS 89 (modo `build` por defecto: el backend sirve el front compilado en `:3110`); producción en el VPS 66 vía Coolify con el modo builder de `coolify-manager-rs`. Dev público: `mn-dev.wandori.us` (Cloudflare Access). Stack y comandos: `README.md`. Cada ítem enlaza su plan en su entrada; detalle íntegro del roadmap anterior: `Agente/planes/completados/roadmap-detalle-2026-10-12.md`; cierres por fecha: `Agente/completados/tareas-YYYY-MM-DD.md`.

## Siguiente bloque

- Ninguno.

## Siguiente bloque

- [ ] `30AA-6` — **El guard de datos inventados no conoce las columnas de la ficha.**
  `numero_inventado()` sólo mira los dígitos de `titulo`/`descripcion`, así que un precio real
  que vive en `inmuebles.precio` se rechaza como inventado (caso: San Ignacio, 29.000, el
  2026-11-12). Hecho cuando el guard acepte los números de precio, m², habitaciones, baños y
  puestos, con los separadores de miles normalizados, y la generación pueda citar el precio.

## Pendientes

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
