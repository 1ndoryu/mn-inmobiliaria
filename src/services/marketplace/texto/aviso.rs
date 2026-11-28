//! [GW-16] Aviso de un inmueble que ya no está disponible: texto
//! determinista, el retoque de la IA sobre él y el filtro que lo acepta. Salía
//! de `forma.rs`, que llegó al tope de tamaño del gate.

use crate::services::marketplace::{
    formatear_parrafos, PromptSeguro, CIERRE_AVISO, CIERRE_BORRADOR_SIN_CTA,
};

use super::sujeto::{
    alternativa_en_frase, cifra_sin_simbolo, lugar_con_articulo, zona_sin_residencia,
};

/// [GW-18] Arranque fijo de la propuesta. Decisión de ella 2026-11-27: el
/// aviso es corto y no nombra el inmueble pedido; la alternativa va en
/// singular porque solo se ofrece una.
const MOLDE_ALTERNATIVA: &str = "Tenemos otra opción disponible: ";

/// [GW-18] Aviso sin nombre: el inmueble pedido no se cita (lo que se guarda
/// es el estado, no la ficha). Decisión de ella 2026-11-27.
const AVISO_GENERICO: &str = "El inmueble por el momento no está disponible.";

/// [GW-18] La cifra del parecido según SU operación: en alquiler se cita el
/// canon mensual, en venta la cifra de venta y en «ambas» las dos. Decisión de
/// ella 2026-11-27: quien busca alquiler no quiere una cifra de venta, así que
/// en una alternativa «ambas» el modo que se pidió va delante.
///
/// [GW-19] Devuelve también con qué se enlaza: `en` con una sola cifra
/// (decisión de ella 2026-11-28: «en 70.000$», no «por»); con dos cifras, la
/// frase ya trae su propio «en venta» y un segundo «en» sobra, así que va
/// tras dos puntos.
fn cifra_por_operacion(s: &PromptSeguro, pedida: &str) -> (String, &'static str) {
    let venta = cifra_sin_simbolo(&s.precio_publico);
    let canon = s.precio_alquiler_publico.as_deref().map(cifra_sin_simbolo);
    let es = |o: &str| s.operacion.eq_ignore_ascii_case(o);
    if es("alquiler") {
        return (format!("{venta}$ al mes"), "en ");
    }
    match canon {
        Some(alq) if es("ambas") => {
            let cifra = if pedida.eq_ignore_ascii_case("alquiler") {
                format!("{alq}$ al mes o {venta}$ en venta")
            } else {
                format!("{venta}$ en venta o {alq}$ al mes")
            };
            (cifra, ": ")
        }
        _ => (format!("{venta}$"), "en "),
    }
}

/// [28AD-1] Borrador determinista de un inmueble que ya no está disponible
/// (sin IA ni caché: el precio del similar sale de la ficha, no de un modelo).
/// `ficha` solo aporta su operación (qué se pidió, para ordenar la cifra de un
/// similar «ambas»); `similar` = ficha del parecido; sin él, se invita a
/// escribir. Cierra con el aviso propio ([`CIERRE_AVISO`]) cuando hay algo que
/// mandar, y con el contacto canónico cuando no hay alternativa.
#[must_use]
pub fn borrador_no_disponible(ficha: &PromptSeguro, similar: Option<&PromptSeguro>) -> String {
    /* [GW-16] Decisiones de ella 2026-11-21:
     * - se dice que no está disponible, sin nombrar el estado («ya fue
     *   vendido» cuenta de más: no hace falta saber si se vendió o se alquiló);
     * - la opción se ofrece sin enlace: el comprador sigue el hilo con
     *   nosotros, no un enlace suelto.
     * [GW-18] Decisiones de ella 2026-11-27: el aviso no nombra el inmueble
     * («El inmueble…») y el cierre va sin la invitación a contar qué busca.
     * [GW-19] Decisión de ella 2026-11-28: la cifra se enlaza con «en» (no
     * con «por») y el cierre propio del aviso ofrece mandar la ficha y
     * pregunta el monto; sin alternativa no hay ficha que mandar, así que va
     * el cierre con el teléfono. */
    let propuesta = match similar {
        Some(s) => {
            /* El lugar solo se cita si aporta algo: cuando la zona es la propia
             * residencia, repetirla aquí la diría dos veces en la misma frase. */
            let lugar = lugar_con_articulo(&zona_sin_residencia(&s.zona, &s.nombre_corto));
            let inmueble = alternativa_en_frase(&s.nombre_corto);
            let (cifra, nexo) = cifra_por_operacion(s, &ficha.operacion);
            /* El lugar va delante: detrás quedaban dos «en» seguidos («una
             * casa en Guayana Country Club en la Av. Atlantico»). */
            let cuerpo = if lugar.is_empty() {
                format!("{inmueble}, {nexo}{cifra}")
            } else {
                format!("en {lugar}, {inmueble}, {nexo}{cifra}")
            };
            format!("{MOLDE_ALTERNATIVA}{cuerpo}.")
        }
        None => "Si desea, escríbanos y le ayudamos a buscar otra opción.".to_string(),
    };
    let cierre = if similar.is_some() {
        CIERRE_AVISO
    } else {
        CIERRE_BORRADOR_SIN_CTA
    };
    formatear_parrafos(&format!("{AVISO_GENERICO}\n\n{propuesta}\n\n{cierre}"))
}
