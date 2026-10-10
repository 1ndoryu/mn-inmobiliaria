//! [08AA-8] Texto puro del asistente Marketplace (sin HTTP ni BD).
//!
//! Extraído de `marketplace.rs` (límite 700): schema M3 v1
//! (`BorradorRequest` y tipos), validación (`validar_borrador`), limpieza
//! del excerpt (`normalizar_excerpt`), precio del aviso (`precio_del_aviso`)
//! y formato de precio (`precio_publico`). `marketplace.rs` conserva strip de ficha, contacto,
//! párrafos, títulos, caché y tokens, y re-exporta estos nombres para no
//! mover sus usos externos (handlers, utoipa, sombra).

mod excerpt;
mod ruido;
mod schema;

pub(in crate::services) use excerpt::sin_tilde_min;
pub use excerpt::{normalizar_excerpt, normalizar_excerpt_con_hilo};
#[cfg(test)]
pub(in crate::services) use schema::es_hex64;
pub use schema::{
    precio_del_aviso, precio_publico, validar_borrador, BorradorRequest, ExcerptIn, ExtrasIn,
    Largo, Tono,
};
