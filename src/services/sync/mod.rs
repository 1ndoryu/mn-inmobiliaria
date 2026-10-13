//! Estado de sincronización de inmuebles local ↔ servidor [13AA-12].
//!
//! Cada entorno calcula una huella determinista por inmueble (`huella`); el
//! entorno local la compara con la del servidor y con la última vez que
//! coincidieron (`comparar`), de modo que se sabe QUÉ lado va por delante.
//! `remoto` habla con el servidor, `vigilancia` mantiene el estado en segundo
//! plano y `detalle` arma las diferencias con los valores de cada lado.

pub mod comparar;
pub mod detalle;
pub mod huella;
pub mod manifiesto;
pub mod remoto;
pub mod vigilancia;
