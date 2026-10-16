# Plan 16AA-12 — Lectura del perfil de Marketplace a petición (manual)

Fecha: 2026-10-16. Estado: **hecha** (F1–F4).
Cambia la decisión de `plan-vinculo-marketplace-2026-10-13.md` §3 (lectura automática cada 24 h).

## 1. Objetivo

Que la lectura del perfil de Facebook la dispare la usuaria con un botón en el modal «Vincular con Facebook», en vez de un temporizador. Sin lectura al arrancar la lab y sin repetición periódica.

## 2. Decisión de la usuaria (2026-10-16)

«Quítalo para que no sea automático, que sea manual.»

## 3. Diseño (técnico, reversible)

El canal ya existe y no hay que inventar ninguno: **la lab sondea MN cada 5 s** con el latido `GET /api/admin/marketplace/borradores/version` (de él sale el indicador «Lab · sin señal»). El botón deja una marca en MN, el latido la devuelve, y la lab —al ver un valor que no había atendido— lee el perfil y manda los avisos con el `POST` que ya usa. La marca se cierra cuando los avisos entran.

- **MN** guarda `solicitada_en` en memoria de proceso (`services/marketplace/perfil_lectura.rs`, un `AtomicI64`), igual que `version_borradores_borrados`: sin migración y con la misma vida que el proceso. El botón la escribe; el latido la devuelve; `registrar` (avisos) la marca atendida. Un reinicio del backend pierde la petición pendiente y la usuaria la vuelve a pulsar.
- **La lab** pierde el temporizador de 24 h y la comprobación de arranque. Solo lee cuando la marca cambia de valor, y recuerda localmente la última atendida para no repetir la lectura en cada latido si algo falla.
- **El modal** enseña el botón, avisa de si la lab está conectada y espera a que lleguen los avisos.
- Si la lab está cerrada no se puede leer (no hay quien tenga la sesión de Facebook): el modal lo dice en vez de quedarse esperando.

## 4. Fases

**F1 — Backend MN.** Marca en memoria (`perfil_lectura.rs`: `pedir_lectura`, `lectura_pedida`, `marcar_atendida`); `POST /api/admin/marketplace/perfil/leer` (staff); `GET perfil/avisos` devuelve `solicitada_en`; el latido devuelve `perfil_solicitado_en`; `registrar` cierra la marca. Prueba unitaria del módulo. **Hecha:** `cargo check --tests` limpio.

**F2 — Lab.** Fuera `setInterval`/`setTimeout` y el tope de 24 h (`debeLeer`); la lectura se dispara desde el sondeo del latido cuando la marca es nueva; estado local con la última atendida (`ultimaSolicitud`). Pruebas puras actualizadas. **Hecha:** 53 pruebas puras pasan; `tsgo -b` sin errores en `src/main`.

**F3 — Front.** Botón «Leer el perfil ahora» en el modal, con el estado de la lab y espera activa de los avisos. **Hecha:** `tsc -b` y `oxlint` limpios (0 errores) en la 89.

**F4 — Verificación y cierre.** Lectura real disparada desde el botón, con los avisos apareciendo en el modal sin recargar; type-check del front; gate del proyecto; cierre en `Agente/completados/tareas-2026-10-16.md`. **Hecha:** probado en el navegador integrado contra `mn-dev` (127.0.0.1:5199, el túnel del dev en la 89), con la lab abierta y conectada. Antes de pulsar, `~/.mn-inmobiliaria-lab/perfil-marketplace.json` seguía con la fecha del 15 oct (la lab no lee sola). Al pulsar «Leer el perfil ahora», la lab leyó el perfil en el siguiente latido (registro: `perfil leído { avisos: 13, recibidos: 13 }`) y los avisos aparecieron en el modal **sin recargar la página**, con la marca ya cerrada (el botón vuelve a «Leer el perfil ahora» y «Última lectura: 16 oct 2026, 16:28»). La usuaria vinculó después los 13 avisos desde su navegador. Gate `--cierre`: 0 nuevos (base 6).

## 5. Definición de hecho

- La lab no lee sola: con la marca limpia y la lab abierta, `perfil-marketplace.json` no se toca en horas.
- Pulsando el botón, los avisos nuevos aparecen en el modal; la marca queda cerrada.
- Sin la lab conectada, el modal lo dice claro y no deja la sensación de que «no funciona».

Relacionado: `plan-vinculo-marketplace-2026-10-13.md` (13AA-4, F1–F4 hechas).
