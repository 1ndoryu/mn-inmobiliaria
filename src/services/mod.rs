mod auth;
mod comercial;
mod inmueble;
pub mod marketplace;
mod mensajeria;
pub mod politica;
pub mod sesion;
mod tope_uso;
pub mod triage;
pub mod turno;

pub use auth::{AuthService, Claims};
pub use comercial::note::NoteService;
pub use comercial::solicitud::SolicitudService;
pub use comercial::suscriptor::SuscriptorService;
pub use inmueble::InmuebleService;
pub use mensajeria::alerta_whatsapp::vigilar as vigilar_alertas_whatsapp;
pub use mensajeria::canal_resolver::{modo_por_canal, CanalResolver};
pub use mensajeria::outbox_idempotency::{
    clave_idempotencia, corte_cubre, debe_usar_clave, encolar as encolar_outbox_idem,
    marcar as marcar_outbox, purgar_resueltos, reencolar_fallidos, Encolado,
};
pub use mensajeria::transporte;
pub use politica::{
    codigo_trato, leer_autorizados, prefijo_contexto, resolver, texto_para_turno, DecisionPolitica,
    Rol, CLAVE_AUTORIZADOS, REGLA_FRONTERA,
};
pub use tope_uso::{revisar_tope, vigilar as vigilar_tope_uso};
pub use triage::{
    decidir, decidir_para, evaluar_trato, Contenido, Decision, Duplicidad, EntradaSinResolver,
    EvaluacionTrato, EventoTriage, MarcaTransporte, Motivo, Procedencia, RegistroDuplicados, Trato,
    VENTANA_RETRASO_MIN_DEFAULT,
};

/* Alias de compatibilidad con los nombres previos a la reorganización por
 * dominio. `allow(unused_imports)`: sin llamadores fuera de `services` avisan
 * como no usados; el de `outbox_idempotency` sí lo usa `tope_uso.rs`. */
#[allow(unused_imports)]
pub(crate) use mensajeria::alerta_whatsapp;
#[allow(unused_imports)]
pub(crate) use mensajeria::canal_resolver;
#[allow(unused_imports)]
pub(crate) use mensajeria::outbox_idempotency;
#[allow(unused_imports)]
pub(crate) use comercial::note;
#[allow(unused_imports)]
pub(crate) use comercial::solicitud;
#[allow(unused_imports)]
pub(crate) use comercial::suscriptor;
#[allow(unused_imports)]
pub(crate) use inmueble::alias as inmueble_alias;
#[allow(unused_imports)]
pub(crate) use inmueble::slug as inmueble_slug;
#[allow(unused_imports)]
pub(crate) use inmueble::vinculo as inmueble_vinculo;
#[allow(unused_imports)]
pub(crate) use marketplace::burbujas as marketplace_burbujas;
#[allow(unused_imports)]
pub(crate) use marketplace::cabecera as marketplace_cabecera;
#[allow(unused_imports)]
pub(crate) use marketplace::compartida as marketplace_compartida;
#[allow(unused_imports)]
pub(crate) use marketplace::texto as marketplace_texto;
#[allow(unused_imports)]
pub(crate) use marketplace::vuelo as marketplace_vuelo;
