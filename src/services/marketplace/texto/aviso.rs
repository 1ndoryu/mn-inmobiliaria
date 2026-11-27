//! [GW-16] Aviso de un inmueble que ya no está disponible: texto
//! determinista, el retoque de la IA sobre él y el filtro que lo acepta. Salía
//! de `forma.rs`, que llegó al tope de tamaño del gate.

use crate::services::marketplace::{formatear_parrafos, PromptSeguro, CIERRE_BORRADOR_SIN_CTA};

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
fn cifra_por_operacion(s: &PromptSeguro, pedida: &str) -> String {
    let venta = cifra_sin_simbolo(&s.precio_publico);
    let canon = s.precio_alquiler_publico.as_deref().map(cifra_sin_simbolo);
    let es = |o: &str| s.operacion.eq_ignore_ascii_case(o);
    if es("alquiler") {
        return format!("{venta}$ al mes");
    }
    match canon {
        Some(alq) if es("ambas") => {
            if pedida.eq_ignore_ascii_case("alquiler") {
                format!("{alq}$ al mes o {venta}$ en venta")
            } else {
                format!("{venta}$ en venta o {alq}$ al mes")
            }
        }
        _ => format!("{venta}$"),
    }
}

/// [28AD-1] Borrador determinista de un inmueble que ya no está disponible
/// (sin IA ni caché: el precio del similar sale de la ficha, no de un modelo).
/// `ficha` solo aporta su operación (qué se pidió, para ordenar la cifra de un
/// similar «ambas»); `similar` = ficha del parecido; sin él, se invita a
/// escribir. Siempre cierra con el contacto canónico.
#[must_use]
pub fn borrador_no_disponible(ficha: &PromptSeguro, similar: Option<&PromptSeguro>) -> String {
    /* [GW-16] Decisiones de ella 2026-11-21:
     * - se dice que no está disponible, sin nombrar el estado («ya fue
     *   vendido» cuenta de más: no hace falta saber si se vendió o se alquiló);
     * - la opción se ofrece sin enlace: el comprador sigue el hilo con
     *   nosotros, no un enlace suelto.
     * [GW-18] Decisiones de ella 2026-11-27: el aviso no nombra el inmueble
     * («El inmueble…») y el cierre va sin la invitación a contar qué busca. */
    let propuesta = match similar {
        Some(s) => {
            /* El lugar solo se cita si aporta algo: cuando la zona es la propia
             * residencia, repetirla aquí la diría dos veces en la misma frase. */
            let lugar = lugar_con_articulo(&zona_sin_residencia(&s.zona, &s.nombre_corto));
            let inmueble = alternativa_en_frase(&s.nombre_corto);
            let cifra = cifra_por_operacion(s, &ficha.operacion);
            /* El lugar va delante: detrás quedaban dos «en» seguidos («una
             * casa en Guayana Country Club en la Av. Atlantico»). */
            let cuerpo = if lugar.is_empty() {
                format!("{inmueble}, por {cifra}")
            } else {
                format!("en {lugar}, {inmueble}, por {cifra}")
            };
            format!("{MOLDE_ALTERNATIVA}{cuerpo}.")
        }
        None => "Si desea, escríbanos y le ayudamos a buscar otra opción.".to_string(),
    };
    formatear_parrafos(&format!("{AVISO_GENERICO}\n\n{propuesta}\n\n{CIERRE_BORRADOR_SIN_CTA}"))
}

/// [GW-18] Prompt del retoque del aviso «no disponible». El texto de
/// `borrador_no_disponible` ya trae los hechos; la IA solo puede mejorar cómo
/// se leen. Es una rama que también sirve con el relay caído, así que el
/// resultado se valida en Rust: si cambia un dato o se sale del molde, se
/// sirve el texto de partida.
#[must_use]
pub fn sistema_retoque_no_disponible(base: &str) -> String {
    format!(
        "Eres el asistente de MN Inmobiliaria respondiendo en Marketplace. \
         Reescribe este mensaje para que suene natural, en español de \
         Venezuela, sin cambiar ningún dato. Es obligatorio conservar: la misma \
         frase de que el inmueble por el momento no está disponible (tal como \
         viene, sin añadirle el nombre ni el tipo del inmueble); el lugar y la \
         cifra tal como vienen, sin redondear ni repetir; el arranque de la \
         propuesta, copiado letra por letra; y el último párrafo completo del \
         contacto, copiado letra por letra. Prohibido añadir precios, enlaces, \
         teléfonos, disponibilidad, visitas, fotos o cualquier dato que no \
         esté en el texto. No nombres el estado (nada de «vendido», \
         «alquilado» ni «reservado») ni el inmueble que se pidió. No añadas \
         invitaciones ni ofertas de ayuda («Cuéntenos qué está buscando», «con \
         gusto le ayudamos»): el cierre ya está completo. Mismo número de \
         párrafos, separados por línea en blanco; \
         sin saludo y sin el nombre del cliente (el sistema lo antepone). \
         Devuelve solo el mensaje, sin comillas ni comentarios.\n\n{base}"
    )
}

/// [GW-18] El nombre de la alternativa tal como se lee en el aviso, para
/// exigírselo al retoque (el inmueble pedido ya no se nombra). Vacío se
/// descarta.
#[must_use]
pub fn nombres_del_aviso(similar: Option<&PromptSeguro>) -> Vec<String> {
    let mut v = Vec::with_capacity(1);
    if let Some(s) = similar {
        let a = alternativa_en_frase(&s.nombre_corto);
        if a.trim().len() > 3 {
            v.push(a);
        }
    }
    v
}

/// [GW-18] Acepta el retoque solo si respeta los hechos de `base`: misma
/// cifra, los nombres de `conservar`, el cierre intacto, el aviso sin nombre
/// y ningún dato de más. Sin retoque válido se sirve `base`, que ya es
/// correcto.
#[must_use]
pub fn retoque_aceptado(base: &str, candidato: &str, conservar: &[String]) -> bool {
    let c = candidato.trim();
    if c.is_empty() || c.len() > base.len() * 2 {
        return false;
    }
    /* El cierre se copia literal: el modelo no puede reescribir el contacto. */
    if !c.ends_with(CIERRE_BORRADOR_SIN_CTA) {
        return false;
    }
    /* [GW-18] El aviso va literal y sin nombre: si la IA le cuelga el inmueble
     * que se pidió («El apartamento en…»), deja de ser el aviso corto. */
    if c.split("\n\n").next().map(str::trim) != Some(AVISO_GENERICO) {
        return false;
    }
    /* Todos los números de la base siguen ahí, y ninguno nuevo. */
    let cifras = |t: &str| {
        let mut v: Vec<String> = t
            .split(|c: char| !(c.is_ascii_digit() || c == '.'))
            .filter(|s| s.chars().filter(char::is_ascii_digit).count() >= 3)
            .map(ToString::to_string)
            .collect();
        v.sort();
        v
    };
    if cifras(base) != cifras(c) {
        return false;
    }
    /* Ni estados ni enlaces. */
    let bajo = c.to_lowercase();
    if ["vendid", "alquilad", "reservad", "http", "wa.me"]
        .iter()
        .any(|p| bajo.contains(p))
    {
        return false;
    }
    /* Decisión de ella 2026-11-21: la propuesta siempre arranca igual. El
     * modelo la variaba («le ofrecemos», «contamos con»), así que el molde es
     * parte de lo que se exige, no una sugerencia del prompt. */
    if base.contains(MOLDE_ALTERNATIVA) && !c.contains(MOLDE_ALTERNATIVA) {
        return false;
    }
    /* [GW-18] Decisión de ella 2026-11-27: el cierre va sin invitación, así
     * que ninguna variante de esa invitación puede aparecer, ni antes ni
     * después del teléfono. Antes se adelantaba al primer párrafo; ahora el
     * mensaje es corto y el aviso no ofrece ayuda. */
    if ["con gusto le ayudamos", "cuéntenos qué", "cuentanos que", "qué está buscando"]
        .iter()
        .any(|p| bajo.contains(p))
    {
        return false;
    }
    /* El nombre de la alternativa sigue en el texto: sin él la oferta deja de
     * ser la misma. */
    conservar
        .iter()
        .all(|n| bajo.contains(&n.trim().to_lowercase()))
        && bajo.contains("no está disponible")
}
