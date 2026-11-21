# Roadmap — MN-Inmobiliaria

**Contexto.** App de gestión inmobiliaria: backend Rust (`src/`), frontend (`frontend/`) y gateway de WhatsApp (`gateway/`). Se edita en el portátil y se compila, prueba y ejecuta en el VPS 89 (modo `build` por defecto: el backend sirve el front compilado en `:3110`); producción en el VPS 66 vía Coolify con el modo builder de `coolify-manager-rs`. Dev público: `mn-dev.wandori.us` (Cloudflare Access). Stack y comandos: `README.md`. Cada ítem enlaza su plan en su entrada; detalle íntegro del roadmap anterior: `Agente/planes/completados/roadmap-detalle-2026-10-12.md`; cierres por fecha: `Agente/completados/tareas-YYYY-MM-DD.md`.

## Siguiente bloque

- Ninguno.

## Pendientes

- [ ] `GW-14` — **Emparejar el borrador con la ficha en cualquier reparto de versiones.** El
  espejo `inmuebles.marketplace_id` se quedaba vacío con cuentas que no fueran v1 (medido el
  2026-11-21: 0 de 26 con espejo) y el borrador contestaba sin ficha. Hecho cuando el espejo tome
  la versión más baja, el respaldo por `inmueble_facebook` resuelva el aviso de otra cuenta, los
  vínculos v1 estén recuperados desde producción y esté desplegado en prod con un borrador real
  devolviendo `fuente=no-disponible`.
- [ ] `30AA-18` — **Publicar la v0 (fotos originales) en las 26 fichas.** El export ya la
  sirve (grupo 0 con las 314 originales y su texto propio, distinto del de la v1, generado el
  2026-11-18) y el modal tiene su pestaña «Originales». Hecho cuando las 26 estén publicadas en
  Marketplace con su `marketplace_id` pegado (hoy `faltan_avisos` las da todas por pendientes) y
  el escritorio las deje en `version 0/publicados`.
- [ ] `30AA-16` — **Lote continuo de variantes hasta la v5 en todos los inmuebles.** Pase cada dos
  horas (`pase.py`, tarea programada `variantes-mn-cada-2h`) que renueva cookies de Gemini si
  hacen falta y genera solo las variantes que faltan, con el tope de 140/día. Hecho cuando los 26
  inmuebles tengan sus 314 fotos en v2, v3, v4 y v5 (993 pendientes al 2026-11-18), cada versión
  con su propio texto generado (`textos` del pase, 30AA-17), el manifiesto lo refleje por versión
  y la tarea programada se pueda retirar.
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
