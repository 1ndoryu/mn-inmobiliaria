/* [03AA-3 M3] Lógica pura del asistente Marketplace (sin HTTP): strip de ficha
 * por allowlist, validación del schema M3 v1, matriz negativa versionada,
 * claims del JWT mp y cubo de tasa por minuto. Verificable sin BD. */

/* Solo tests (`pruebas`): el resto de imports viven en cada submódulo; aquí
 * quedarían sin uso en la build normal y romperían `clippy -D warnings`. */
#[cfg(test)]
use crate::models::InmuebleRow;
#[cfg(test)]
use uuid::Uuid;

pub(crate) mod burbujas;
pub(crate) mod cabecera;
pub(crate) mod cambios;
mod chat;
pub(crate) mod compartida;
pub(crate) mod continuacion;
mod hilo;
pub(crate) mod perfil;
pub(crate) mod texto;
pub(crate) mod vuelo;
pub use self::chat::acciones::*;
pub use self::chat::panel::*;
pub use self::chat::tokens::*;
pub use self::hilo::cache::*;
pub use self::perfil::*;
pub use self::hilo::ficha::*;
pub use self::hilo::titulo::*;
pub use self::texto::constantes::*;
pub use self::texto::forma::*;
pub use hilo::*;

/* [08AA-7] Singleflight vive en su dominio (`marketplace/vuelo.rs`); se
 * re-exporta para no mover sus usos externos (`lib.rs`, handlers, sombra). */
pub use self::vuelo::{Generado, Singleflight};

/* [09AA-20] Burbujas estructuradas F0: tipos+validador+firma-v2 viven en su
 * dominio (`marketplace/burbujas.rs`); se re-exporta para no mover sus usos
 * externos (handlers, utoipa, tests). */
pub use self::burbujas::{
    estructuradas_apagadas, llave_esperada, texto_para_prompt, validar_conversacion,
    validar_idempotency_key, BurbujaIn, BurbujaUtil, ConversacionEstructurada,
    ConversacionValidada, ErrorEstructurado, Lado, LadoUtil, CODIGO_ESQUEMA, CODIGO_IDEMPOTENCIA,
    CODIGO_PAYLOAD_GIGANTE, CODIGO_REINTENTO_FOREGROUND, CODIGO_VERSION_DESCONOCIDA,
    ENV_KILL_SWITCH, FIRMA_VERSION_V2, MAX_BURBUJAS, MAX_HINT_CARACTERES, MAX_IDEMPOTENCY_CHARS,
    MAX_POR_BURBUJA, MAX_TOTAL_CARACTERES, VERSION_ESTRUCTURADA,
};
/* [09AA-30 F2] Caché compartida por inmueble vive en su dominio
 * (`marketplace/compartida.rs`); se re-exporta para los handlers. */
pub use self::compartida::{
    buscar_compartida, corregir_compartida, enlazar_compartida, mensaje_clave_de,
    ocurrencias_nombre, plantilla_de_nombre, purgar_compartida, rellenar_nombre,
    vincular_hilo_compartido, vinculo_compartido, ClaveCompartida, MARCADOR_NOMBRE,
};
/* [08AA-8] Texto puro (schema, excerpt, precio) vive en su dominio
 * (`marketplace/texto.rs`); se re-exporta para no mover sus usos externos
 * (handlers, utoipa, sombra, tests). */
pub use self::texto::{
    normalizar_excerpt, normalizar_excerpt_con_hilo, precio_del_aviso, precio_publico,
    validar_borrador, BorradorRequest, ExcerptIn, ExtrasIn, Largo, Tono,
};
/* Solo tests (`super::es_hex64` en `pruebas`): fuera de `cfg(test)` sería
 * import sin uso y rompería `clippy -D warnings` (mismo patrón que `ia.rs`). */
#[cfg(test)]
use self::texto::es_hex64;
#[cfg(test)]
mod pruebas;
