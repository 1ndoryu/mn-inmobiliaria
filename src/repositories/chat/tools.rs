//! Consultas SQL de las tools del chat IA (movidas desde `handlers/chat_tools*.rs`).

use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

/// Tarjeta breve de un inmueble para `buscar_inmuebles` (struct en vez de
/// tupla de 9: legible y evita el lint de tipos complejos). Incluye
/// `puestos`, `residencia` y `habitaciones` para que el agente responda con
/// esos datos exactos (Fase3-H3: sin este campo filtraba a ojo).
#[derive(Debug, sqlx::FromRow)]
pub(crate) struct Tarjeta {
    pub(crate) id: Uuid,
    pub(crate) titulo: String,
    pub(crate) tipo: String,
    pub(crate) operacion: String,
    pub(crate) precio: f64,
    pub(crate) ubicacion: String,
    pub(crate) slug: String,
    pub(crate) puestos: i32,
    pub(crate) residencia: String,
    pub(crate) habitaciones: i32,
}

/// Ficha completa de un inmueble para `detalle_inmueble` (struct en vez de
/// tupla de 12: legible y evita el lint de tipos complejos). Tipos alineados
/// con `20260915000002_inmuebles.up.sql` (NOT NULL salvo `copy_corta`).
/* [279A-8] La IA ve cada campo rellenable: `extras` (respuestas /ask, tal cual,
 * incluidos `no_se`/`a_veces`: saber lo que falta también informa) y si el
 * precio tiene margen (`margen_negociable`, calculado en SQL). La cifra del
 * mínimo jamás sale (frontera 279A-3: la IA insinúa sin cifras). */
#[derive(Debug, sqlx::FromRow)]
pub(crate) struct Ficha {
    pub(crate) titulo: String,
    pub(crate) descripcion: String,
    pub(crate) ubicacion: String,
    pub(crate) puestos: i32,
    pub(crate) residencia: String,
    pub(crate) precio: f64,
    pub(crate) tipo: String,
    pub(crate) operacion: String,
    pub(crate) habitaciones: i32,
    pub(crate) banos: i32,
    pub(crate) metros: f64,
    pub(crate) metros_terreno: f64,
    pub(crate) estado: String,
    pub(crate) copy_corta: Option<String>,
    pub(crate) extras: Value,
    pub(crate) margen_negociable: bool,
}

/// Inmuebles publicados y disponibles para `buscar_inmuebles`.
/// Binds por orden: $1 texto, $2 tipo, $3 operacion, $4 precio_max,
/// $5 limite, $6 habitaciones, $7 zona.
pub(crate) async fn tarjetas_inmuebles(
    pool: &PgPool,
    texto: Option<&str>,
    tipo: Option<&str>,
    operacion: Option<&str>,
    precio_max: Option<f64>,
    limite: i64,
    habitaciones: Option<i64>,
    zona: Option<&str>,
) -> Result<Vec<Tarjeta>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, titulo, tipo, operacion, precio, ubicacion, slug, puestos, residencia, habitaciones \
             FROM inmuebles \
             WHERE publicado AND estado = 'disponible' \
             AND ($1::TEXT IS NULL OR sencilla(titulo) LIKE '%' || sencilla($1) || '%' OR sencilla(ubicacion) LIKE '%' || sencilla($1) || '%') \
             AND ($2::TEXT IS NULL OR tipo = $2) \
             AND ($3::TEXT IS NULL OR operacion = $3) \
             AND ($4::FLOAT8 IS NULL OR precio <= $4) \
             AND ($6::BIGINT IS NULL OR habitaciones = $6) \
             AND ($7::TEXT IS NULL OR sencilla(ubicacion) LIKE '%' || sencilla($7) || '%') \
             ORDER BY updated_at DESC LIMIT $5",
    )
    .bind(texto)
    .bind(tipo)
    .bind(operacion)
    .bind(precio_max)
    .bind(limite)
    .bind(habitaciones)
    .bind(zona)
    .fetch_all(pool)
    .await
}

/// Ficha de `detalle_inmueble`: `None` si el inmueble no existe o no está publicado.
pub(crate) async fn ficha_inmueble(pool: &PgPool, id: Uuid) -> Result<Option<Ficha>, sqlx::Error> {
    sqlx::query_as(
        "SELECT titulo, descripcion, ubicacion, puestos, residencia, precio, tipo, operacion, \
              habitaciones, banos, metros, metros_terreno, estado, copy_corta, extras, \
              (precio_minimo IS NOT NULL AND precio_minimo > 0) AS margen_negociable \
              FROM inmuebles WHERE id = $1 AND publicado",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

/// Título de un inmueble publicado (`None` si no existe o no está publicado).
/// Compartida por `enviar_fotos` (chat_tools) y `agendar` (chat_tools_captacion).
pub(crate) async fn titulo_inmueble_publicado(
    pool: &PgPool,
    id: Uuid,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT titulo FROM inmuebles WHERE id = $1 AND publicado")
        .bind(id)
        .fetch_optional(pool)
        .await
}

/// Claves de almacenamiento de las fotos de un inmueble, en orden, hasta `max`.
pub(crate) async fn claves_fotos_inmueble(
    pool: &PgPool,
    id: Uuid,
    max: i64,
) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT storage_key FROM fotos WHERE inmueble_id = $1 ORDER BY orden LIMIT $2",
    )
    .bind(id)
    .bind(max)
    .fetch_all(pool)
    .await
}
