//! Estado de sincronización de inmuebles local ↔ servidor [13AA-12].
//!
//! Cada entorno calcula una huella determinista por inmueble (`huella`); el
//! entorno local la compara con la del servidor y con la última vez que
//! coincidieron (`comparar`), de modo que se sabe QUÉ lado va por delante.
//! `remoto` habla con el servidor, `vigilancia` mantiene el estado en segundo
//! plano y `detalle` arma las diferencias con los valores de cada lado.

pub mod aplicar;
pub mod comparacion;
pub mod operacion;
pub mod plan;
pub mod publicar;
pub mod remoto;
pub mod traer;
pub mod vigilancia;

// Re-exportación: handlers y el resto de sync/ siguen usando `sync::huella`, `sync::comparar`, etc.
pub use comparacion::{comparar, detalle, huella, manifiesto, resolver};
