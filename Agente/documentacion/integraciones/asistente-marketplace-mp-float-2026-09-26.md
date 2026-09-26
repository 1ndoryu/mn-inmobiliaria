# Asistente Marketplace mp-float — cómo funciona y qué se hizo

> **Fecha:** 2026-09-26 · **Tarea:** 269A-1 · **Código vive en:** repo
> `opencode-propio`, rama `propio-identity`
> (`src/packages/desktop/src/main/marketplace-*`).

## Qué es

Borrador flotante dentro de la página de Facebook Marketplace (bandeja del
vendedor) que genera **1 sola opción de respuesta** con el precio del aviso,
lista para copiar a mano al chat. Riesgo 0: cero red hacia Meta, sin
auto-envío, sin login ni credenciales, única escritura = el propio panel
flotante (shadow-DOM). Sin IA: motor local de plantillas, cero tokens.

## Arquitectura (4 piezas)

| Archivo | Rol |
|---|---|
| `marketplace-catalog.ts` | `catalogFromApi`: trae los 11 avisos de `GET /api/public/inmuebles` de **este repo** (fuente única de precio). |
| `marketplace-assistant.ts` | `detectarIntencion` (precio/disponibilidad/visita/negociacion/desconocida), `emparejarAviso` (título FB → aviso), `generarBorradores`, `formatearPrecio`. Contacto fijo `Mayerlin Navarro 04249208855`. |
| `marketplace-float.ts` | Guest precargado en la ventana de Facebook (`FLOAT_VERSION = 3`): detecta chats por ancla `Conversación con el título {comprador} · {aviso}`, reporta `focusedKey` (chat con foco real vía `activeElement` + `document.hasFocus()`), pinta el panel y `__mpPushDrafts(key, drafts, sig, debug)`. |
| `marketplace-service.ts` | Orquesta: excerpt (últimos 1200 chars del chat) + catálogo → borrador → push; `floatDraftCache` (tope 300, sobrevive al reabrir); `autoCopiarSiFoco` (clipboard OS, indetectable por Facebook, con gesto clic como respaldo). |

## Reglas de negocio (decididas con la usuaria)

- **Precio siempre o nada:** sin aviso emparejado no hay borrador (salvo
  `visita`, que es plantilla de coordinación sin precio).
- **1 sola opción,** la mejor según intención.
- **Plantillas 2026-09-26:** precio/negociacion/disponibilidad →
  `…negociable. Mayerlin Navarro 04249208855`; `visita` →
  `Con gusto. Escríbeme al 04249208855 para coordinar la visita…`.
- **Histéresis de precio:** si el caché tiene precio y el nuevo cálculo no,
  se conserva el cacheado (`draftsConPrecio`, `mantenerPrecioCacheado`).
- **Auto-copia solo del chat con foco** (hay ~35 chats y 2+ abiertos a la
  vez): solo borradores recién generados, nunca en `reuse`. Sin hover-copy
  (el clipboard exige gesto).
- **Línea debug visible** en el panel: `d=<intencion> | fb="<título>" |
  aviso=<precio>/sin aviso | cat=N | msg="<extracto>"`.

## Historial (commits OP-33 en `opencode-propio`)

- `44a93d1`: el flotante repinta igual y conserva precio al reabrir.
- `78106ba`: precio-o-nada + línea debug + excepción `visita`.
- `5b2a581`: `desconocida` con aviso sí muestra precio; raíz prefiere
  ancestro con composer (`contenteditable`), fallback por dimensiones.
- `1bcec99`: auto-copia del chat con foco (`FLOAT_VERSION 3`,
  `focusedKey`, `autoCopiarSiFoco`). 160 tests desktop en verde.

## Relación con MN-Inmobiliaria

- **El precio que cita el flotante sale de nuestra API pública.** Cambiar un
  precio aquí (p. ej. 269A-2: Riberas `50000 → 43000` vía PUT parcial
  `UpdateInmuebleRequest`) actualiza automáticamente lo que el flotante
  ofrece. No hay que tocar `opencode-propio`.
- **Antecesor local [259A-5]:** `scripts/respuesta-marketplace.mjs`
  genera un borrador con la **IA del backend** (`POST /api/admin/ia/completar`,
  OpenCode Go) a partir de la ficha pública. Diferencia: mp-float es
  instantáneo, determinista y vive dentro de Facebook; el script es
  con-IA y por terminal. La clave admin viaja solo por `MN_ADMIN_CLAVE`.

## Gotchas

- El ancla del chat depende del `aria-label` de Facebook; si Meta lo
  cambia, el flotante deja de emparejar (los logs `[mp-float] sin aviso`
  lo delatan).
- Alias conocidos: `Rio Ara Plaza = Arivana Plaza` (en `emparejarAviso`).
- `?per_page=50` devuelve `{items, total, page, per_page}`.
