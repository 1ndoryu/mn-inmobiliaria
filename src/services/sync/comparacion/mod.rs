//! Comparación de inmuebles local ↔ servidor [13AA-12]: huella, comparación,
//! diferencias y plan de resolución. `sync/` conserva el transporte (`remoto`,
//! `operacion`, `traer`, `publicar`, `vigilancia`). Va en su propio directorio
//! para respetar el tope de archivos por directorio del gate (`directorio-abarrotado`).

pub mod comparar;
pub mod detalle;
pub mod huella;
pub mod manifiesto;
pub mod orden;
pub mod resolver;
