# Plan optimización PageSpeed + SEO — 249A-1 (2026-09-24)

> Origen: pedido usuario tras incidencia `Servidor no disponible` cerrada.
> PageSpeed mobile medido en captura: 75 Rendimiento / 100 Accesibilidad / 100
> Recomendaciones / 92 SEO (PSI API devolvió 429, no hay desglose de oportunidades).

## Diagnóstico verificado (no suposiciones)

1. `frontend/src/App.tsx:26-30`: la ruta pública importa TODO el admin de forma
   estática (tabla, mejora IA, canvas-publicidad, modales). El visitante descarga
   el bundle entero (`/assets/index-EZwYRSQX.js`, 612 KB sin comprimir).
2. `frontend/src/features/publica/disenno.ts:96`: héroe como CSS `bg-[url(...)]`
   a `public/img/presentacion.jpg` de **3 MB**, sin preload ni formato moderno.
3. `CajaInmueble` (`lista/caja-inmueble.tsx:46`) carga la foto a resolución
   completa para filas de 100 px (11 fotos por página).
4. `ModalDetallePublico` (`modal-detalle-publico.tsx:116`): la tira de
   miniaturas carga TODAS las fotos a resolución completa sin `loading="lazy"`.
   El contenido principal sí monta solo al abrir (`inmueble && <Contenido>`).
5. Backend sin caché ni compresión: `handlers/mod.rs:fallback_spa` y
   `handlers/uploads.rs:servir_archivo` devuelven solo `Content-Type` (sin
   `Cache-Control`, `ETag` ni `Last-Modified`); sin `CompressionLayer`.
   `robots.txt`/`sitemap.xml` caen al fallback y devuelven `index.html` (1042 b).
6. Subir/borrar fotos NO toca `inmuebles.updated_at` (solo PUT y PATCH
   publicación lo hacen): versionar por `updatedAt` exige tocarlo en uploads.
7. `index.html`: título/descripción genéricos, sin OG/Twitter/JSON-LD/canonical;
   favicon apunta a `/favicon.svg` existiendo logo real en `/img/logo-mn.svg`.
8. Detalle público solo-modal (sin URL con slug): SEO por inmueble = JSON-LD
   `ItemList` + sitemap dinámico. URLs `/inmueble/:slug` quedan como fase futura
   (SPA renderiza en cliente; Google lo indexa, pero es otro bloque).

## Fases

- **F1 Head + favicon + robots**: título `MN Inmobiliaria - Inmuebles en
  Puerto Ordaz`, descripción, OG/Twitter, canonical, JSON-LD, icono al logo,
  `theme-color`; `frontend/public/robots.txt` (apunta al sitemap).
- **F2 Héroe**: `presentacion.jpg` → webp comprimido (~150 KB) + `preload` con
  `fetchpriority="high"`; mantener jpg como reserva si el diseño lo exige.
- **F3 Code-split**: `AppAdmin` a `React.lazy` (chunk aparte tras login); el
  chunk público queda solo con `PaginaPublica`.
- **F4 Modal**: `loading="lazy"` + `decoding="async"` en miniaturas; principal a
  máxima resolución solo al abrir (ya monta bajo condición, se conserva).
- **F5 Backend**: `CompressionLayer` (gzip/br); `Cache-Control immutable` + `ETag`
   en `/assets/*` y `/uploads/*`, `no-cache` en `index.html`; `UPDATE
   inmuebles SET updated_at=NOW()` en subida/borrado de fotos; ruta dinámica
   `/sitemap.xml` (slugs publicados) registrada antes del fallback.
- **F6 Cache-bust**: `portadaDe`/`fotosVisiblesDe` (`domain/inmueble.ts`)
   añaden `?v=<updatedAt>`; con F5, cambiar fotos invalida caché. OJO:
   no versionar en `remotoADominio`/`urlAbsoluta`: `sincronizarFotos` compara
   URLs con `relativa()` y el `?v=` rompería la igualdad (re-subiría siempre).
- **F7 Thumbs**: variante `thumb-<archivo>` generada en subida (crate `image`,
   320 px, jpeg q70) + backfill de las 218 existentes; `CajaInmueble` usa thumb,
   modal sigue a máxima resolución.
- **F8 SEO profundo + cierre**: h1/alt/canonical/JSON-LD, `tsc` + `vite build`,
   `cargo fmt/check/clippy/test`, commit `249A-1`, push, deploy
   (`deploy-service --skip-backup`), verificar bundle/headers/`total=11` y
   re-ejecutar PageSpeed.

## No alcance

- URLs `/inmueble/:slug` ruteables (bloque futuro).
- Chat IA en producción (decisión vigente: solo local).
- `www` con cert self-signed (tema DNS/TLS, fuera de este bloque).
