# Plan — Detección fiable del mensaje del cliente + caché por inmueble (09AA-29, 2026-10-09)

## Objetivo
Que el borrador (a) lea SOLO lo que el cliente escribió y (b) cite el precio
de la ficha cuando el hilo pertenece a un inmueble publicado. Con eso como
base, planificar la caché de respuestas a nivel de **inmueble** (no de
comprador+hilo), que exige claves y extracción deterministas.

## Evidencia (retest desde cero, 2026-10-09, hilo `الله معي|VEF0 casa en venta
en urbanización villa icabarú, puerto ordaz.`)
- Fila de caché: `excerpt_texto = "erto Ordaz. ⏎ ¿Sigue disponible?"`,
  `precio_hash = sin-ficha`. El cliente solo escribió «¿Sigue disponible?».
- **Bug 1 (mensaje):** el float corta el texto plano con `slice(-1200)` por
  carácter; el resto de la cabecera del chat (`…erto Ordaz.`) sobrevive.
  `es_cola_truncada` (marketplace_texto.rs) solo descarta colas SIN espacios,
  así que «erto Ordaz.» pasa y el panel lo muestra como «Cliente».
- **Bug 2 (precio):** el inmueble existe, está publicado y es el único con
  ese título (precio 90000, `marketplace_id` vacío). El `avisoId` numérico
  del DOM (`/marketplace/item/<id>`) manda a la rama exacta
  (`claves_por_marketplace_id`); nadie reclama ese ID → `sin-ficha` SIN caer
  al título ([09AA-21] lo decidió para no citar precio dudoso). Resultado: la
  IA va sin ficha y no da precio aunque el título empareja sin ambigüedad.

## Alcance / no alcance
- SÍ: F1 extracción robusta (backend + float lab), F2 fallback de ficha
  (ID sin dueño → título, solo si la ficha no está vinculada a OTRO aviso),
  F3 diseño de la caché por inmueble (este documento), tests con el crudo real.
- NO (hasta cerrar F1+F2 y que ella retestee): implementar la caché por
  inmueble (F4), migración de filas, promoción del lab a su app viva.

## Dependencias
- 09AA-19/20/22 (burbujas estructuradas, firma v2), 09AA-21 (vínculo exacto),
  09AA-24 (alias de título), 07AA-8 (contrato del borrador).
- Lab `opencode-propio-dev` (sin git): se verifica allí; no se promociona.

## Fases
- **F1 — Mensaje del cliente fiable.**
  1. Backend (`marketplace_texto.rs`): en el texto plano, descartar el
     fragmento inicial que sea cola de la cabecera del hilo (sufijo del
     título/aviso o del nombre del comprador, ≥1 palabra, antes de cualquier
     marca de mensaje), además de `es_cola_truncada`. Anclar al patrón
     inequívoco `Mensaje enviado <hora> por <comprador>: <texto>` cuando
     exista.
  2. Float (lab, `marketplace-float.ts`): cortar el excerpt por LÍNEA/burbuja,
     jamás por carácter (`slice(-1200)` → últimas líneas completas).
  3. Verificar por qué el hilo cayó a texto plano (`burbujas` vacías) y que la
     ruta estructurada sea la primaria.
  - Tests: el crudo exacto del caso (cabecera cortada + «¿Sigue disponible?»)
    → un solo mensaje Cliente; regresiones de las familias previas.
- **F2 — Ficha por título cuando el ID no tiene dueño.**
  `claves_por_marketplace_id`: `Ok(None)` → `claves_por_titulo(…, true)` →
  `ficha_por_titulo(…, true)`, que elige candidatos con
  `titulos_alias_publicados_sin_vinculo` (`publicado = TRUE AND
  marketplace_id IS NULL`) ANTES de puntuar. Una ficha vinculada a OTRO aviso
  jamás se cita, ni como mejor coincidencia. Pendiente: alinear
  `titulo_vinculado_del_hilo` (panel) con la misma regla. Tests:
  `id_sin_dueno_no_cita_ficha_vinculada_a_otro_aviso` y matriz de `claves_cache`.
- **F3 — Diseño de la caché por inmueble (sin implementar).**
  - Clave: `inmueble_id` (o `sin-ficha`) + intención normalizada del mensaje
    del cliente + `precio_hash` + `catalog_hash`. Hoy la fila cuelga de
    `firma` (texto del hilo) y `thread_id` (comprador|aviso).
  - Privacidad: los borradores llevan el nombre del comprador («Hola, X»);
    una caché por inmueble debe guardar una PLANTILLA con marcador de nombre
    y rellenarlo al servir. Sin PII en la clave compartida.
  - Contexto «ya dicho» (`hilo_previo`): depende del hilo, no del inmueble;
    decidir si un hit por inmueble solo aplica al primer turno.
  - Invalidación: precio/catálogo cambian → `precio_hash`/`catalog_hash`
    distintos = miss honesto (ya existe). Regenerar explícito pisa.
  - Migración: columna `inmueble_id` nullable + índice; filas viejas
    conviven (hit solo por la clave nueva); purga por TTL 90 d existente.
  - Decisión pendiente de ella: ¿mismo mensaje de dos compradores comparte
    respuesta? (ahorro de IA) vs. personalización por hilo.
- **F4 — Implementar la caché por inmueble** (tras F1–F3 y su visto bueno).

## Estado
F1 y F2 implementadas (2026-10-09). F3 redactado arriba. F4 sin empezar.

- Gate Rust (2026-10-09): `fmt --check`, `check --all-targets`, `clippy --all-targets -D warnings` OK;
  `test --lib` 191 pasados, 0 fallidos. Dos `chat_humo` con `#[ignore]` (necesitan servidor).
- Sentinel (`POST /api/gate/analizar`, forzar): error 0, warning 450 = baseline. Atribuibles
  corregidos: `handler-accede-bd-rs` (consulta en `limpiar_restos`) y `sqlx-query-as-sin-macro`
  (`titulos_publicados`). Restantes en archivos tocados: preexistentes.
- F1 lab: `excerptFor` y mpScan cortan por línea completa (`marketplace-float.ts`); 76/76 y
  `bun run typecheck` exit 0. El lab no tiene git: sin commit.
- **Pendiente:** retest vivo (paso 6). Sin él no hay cierre (tarea sigue abierta en roadmap).

## Próximo paso verificable
Tests rojos con el crudo real → fix F1/F2 → gate → relanzar backend + lab,
vaciar `mp_respuestas_cache` y que ella repita el hilo de Puerto Ordaz.

## Verificación y Definition of Done
- `cargo fmt --check && cargo check && cargo clippy -- -D warnings && cargo test`.
- Re-análisis Sentinel sin hallazgos nuevos atribuibles.
- Retest vivo: el panel muestra un único mensaje «¿Sigue disponible?» y el
  borrador cita el precio de la casa de Villa Icabarú.
- Archivado en `Agente/completados/`, lección registrada, commit + push.
