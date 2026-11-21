//! [GW-16] Aviso de un inmueble que ya no está disponible: texto
//! determinista, el retoque de la IA sobre él y el filtro que lo acepta. Salía
//! de `forma.rs`, que llegó al tope de tamaño del gate.

use crate::services::marketplace::{formatear_parrafos, PromptSeguro, CIERRE_BORRADOR};

use super::sujeto::{
    alternativa_en_frase, cifra_sin_simbolo, lugar_con_articulo, sujeto_en_frase, sujeto_inicial,
    zona_sin_residencia,
};

/// [GW-16] Arranque fijo de la propuesta. Decisión de ella 2026-11-21: siempre
/// el mismo («tenemos otras opciones»), sin «le ofrecemos», que la IA variaba
/// según el hilo y sonaba a venta.
const MOLDE_ALTERNATIVA: &str = "Como alternativa, tenemos otras opciones: ";

/// [28AD-1] Borrador determinista de un inmueble que ya no está disponible
/// (sin IA ni caché: el precio del similar sale de la ficha, no de un modelo).
/// `similar` = ficha del parecido; sin él, se invita a escribir. Siempre cierra
/// con el contacto canónico.
#[must_use]
pub fn borrador_no_disponible(ficha: &PromptSeguro, similar: Option<&PromptSeguro>) -> String {
    /* [GW-16] Decisiones de ella 2026-11-21:
     * - se dice que no está disponible, sin nombrar el estado («ya fue
     *   vendido» cuenta de más: no hace falta saber si se vendió o se alquiló);
     * - la opción se ofrece sin enlace: el comprador sigue el hilo con
     *   nosotros, no un enlace suelto. */
    let aviso = format!(
        "{} por el momento no está disponible.",
        sujeto_inicial(&ficha.nombre_corto)
    );
    let propuesta = match similar {
        Some(s) => {
            let al_mes = if s.operacion.eq_ignore_ascii_case("alquiler") { " al mes" } else { "" };
            /* [GW-16] Decisión de ella 2026-11-21: la frase se lee seguida. El
             * lugar lleva su artículo y el inmueble va indefinido, para no
             * encadenar dos «la casa…». El lugar solo se cita si aporta algo:
             * cuando la zona es la propia residencia, repetirla aquí la diría
             * dos veces en la misma frase. */
            let lugar = lugar_con_articulo(&zona_sin_residencia(&s.zona, &s.nombre_corto));
            let inmueble = alternativa_en_frase(&s.nombre_corto);
            let precio = cifra_sin_simbolo(&s.precio_publico);
            /* El lugar va delante: detrás quedaban dos «en» seguidos («una
             * casa en Guayana Country Club en la Av. Atlantico»). */
            let cuerpo = if lugar.is_empty() {
                format!("{inmueble}, por {precio}${al_mes}")
            } else {
                format!("en {lugar}, {inmueble}, por {precio}${al_mes}")
            };
            format!("{MOLDE_ALTERNATIVA}{cuerpo}.")
        }
        None => "Si desea, escríbanos y le ayudamos a buscar otra opción que se ajuste a lo que necesita.".to_string(),
    };
    formatear_parrafos(&format!("{aviso}\n\n{propuesta}\n\n{CIERRE_BORRADOR}"))
}

/// [GW-16] Prompt del retoque del aviso «no disponible». El texto de
/// `borrador_no_disponible` ya trae los hechos; la IA solo puede mejorar cómo
/// se leen. Es una rama que también sirve con el relay caído, así que el
/// resultado se valida en Rust: si cambia un dato o se sale del molde, se
/// sirve el texto de partida.
#[must_use]
pub fn sistema_retoque_no_disponible(base: &str) -> String {
    format!(
        "Eres el asistente de MN Inmobiliaria respondiendo en Marketplace. \
         Reescribe este mensaje para que suene natural y amable, en español de \
         Venezuela, sin cambiar ningún dato. Es obligatorio conservar: la misma \
         frase de que el inmueble por el momento no está disponible; el tipo y \
         el nombre del inmueble; el lugar y la cifra tal como vienen, sin \
         redondear ni repetir; el arranque de la propuesta, copiado letra por \
         letra; y el último párrafo completo del contacto, copiado letra por \
         letra. Prohibido añadir precios, enlaces, teléfonos, \
         disponibilidad, visitas, fotos o cualquier dato que no esté en el \
         texto. No nombres el estado (nada de «vendido», «alquilado» ni \
         «reservado»). Mismo número de párrafos, separados por línea en blanco; \
         sin saludo y sin el nombre del cliente (el sistema lo antepone). \
         Devuelve solo el mensaje, sin comillas ni comentarios.\n\n{base}"
    )
}

/// [GW-16] Los nombres tal como se leen en el aviso, para exigírselos al
/// retoque: el del inmueble pedido y el de la alternativa, ya con su artículo
/// y sin abreviaturas. Vacíos se descartan.
#[must_use]
pub fn nombres_del_aviso(ficha: &PromptSeguro, similar: Option<&PromptSeguro>) -> Vec<String> {
    let mut v = Vec::with_capacity(2);
    let mut meter = |s: String| {
        if s.trim().len() > 3 {
            v.push(s);
        }
    };
    meter(sujeto_en_frase(&ficha.nombre_corto));
    if let Some(s) = similar {
        meter(alternativa_en_frase(&s.nombre_corto));
    }
    v
}

/// [GW-16] Acepta el retoque solo si respeta los hechos de `base`: misma
/// cifra, los nombres de `conservar`, el cierre intacto y ningún dato de más.
/// Sin retoque válido se sirve `base`, que ya es correcto.
#[must_use]
pub fn retoque_aceptado(base: &str, candidato: &str, conservar: &[String]) -> bool {
    let c = candidato.trim();
    if c.is_empty() || c.len() > base.len() * 2 {
        return false;
    }
    /* El cierre se copia literal: el modelo no puede reescribir el contacto. */
    if !c.ends_with(CIERRE_BORRADOR) {
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
    /* El nombre del original y el del parecido siguen en el texto: sin ellos
     * la oferta deja de ser la misma. */
    conservar
        .iter()
        .all(|n| bajo.contains(&n.trim().to_lowercase()))
        && bajo.contains("no está disponible")
}
