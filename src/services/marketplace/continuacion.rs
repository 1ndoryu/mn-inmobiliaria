//! [10AA-25] Modo «continuación» del borrador: cuando la dueña ya contestó y el
//! último mensaje es del cliente, la IA responde solo a ese mensaje siguiendo
//! el hilo, en una o dos frases. El modo de apertura (saludo + molde + contacto
//! fijo) sigue siendo el de siempre para el primer mensaje.

use super::{formatear_parrafos, BorradorRequest, Tono, CONTACTO_TEL, CONTACTO_WA};

/// Respuesta de reserva en continuación (la IA falló): neutra, sin apertura ni
/// contacto, para no pegar el molde de primer mensaje en mitad de un hilo.
pub const FALLBACK_CONTINUACION: &str = "Lo reviso y te escribo por aquí";

/// Tope de la respuesta de continuación; se corta en el último fin de frase.
const CONTINUACION_MAX_CHARS: usize = 320;

/// `true` si la dueña ya habló en el hilo y el último mensaje marcado es del
/// cliente. Sin marcas (texto plano sin lados) o con la dueña al final no es
/// continuación: se queda el modo de apertura de siempre.
#[must_use]
pub fn es_continuacion(excerpt: &str) -> bool {
    let mut hay_duena = false;
    let mut ultima_es_cliente = false;
    for linea in excerpt.lines() {
        let l = linea.trim_start();
        if l.starts_with("Cliente:") {
            ultima_es_cliente = true;
        } else if l.starts_with("Dueña:") {
            hay_duena = true;
            ultima_es_cliente = false;
        }
    }
    hay_duena && ultima_es_cliente
}

/// [10AA-28] `true` si la dueña ya escribió en el hilo (alguna marca `Dueña:`).
/// La plantilla compartida del inmueble es de apertura: un hilo con respuesta
/// de la dueña no la consulta ni la publica, aunque su primer mensaje del
/// cliente coincida con el de otro hilo.
#[must_use]
pub fn es_apertura(excerpt: &str) -> bool {
    !excerpt
        .lines()
        .any(|l| l.trim_start().starts_with("Dueña:"))
}

/// Tono pedido por el puente (`amable` por defecto).
#[must_use]
pub fn tono_de(r: &BorradorRequest) -> &'static str {
    r.extras.as_ref().map_or("amable", |e| match e.tono {
        Tono::Corto => "corto",
        Tono::Amable => "amable",
        Tono::Formal => "formal",
    })
}

/// Prompt de sistema de la continuación. A propósito sin molde de apertura, sin
/// contacto fijo y sin la hora: el hilo ya empezó y solo hay que seguirlo.
#[must_use]
pub fn sistema_continuacion(tono: &str, aviso: &str, datos: &str, ya_dicho: &str) -> String {
    format!(
        "Eres el asistente de MN Inmobiliaria respondiendo en Marketplace, en medio \
         de una conversación que ya empezó (la dueña ya contestó). Tono {tono}, MUY \
         BREVE: una o dos frases cortas en un solo párrafo, como un mensaje de chat. \
         La conversación trae marcas: `Cliente:` es el comprador y `Dueña:` es la \
         dueña (tú no eres la dueña: no repitas lo que ella ya dijo). Responde SOLO \
         al último mensaje del Cliente, entendiendo lo ya hablado para continuar el \
         hilo con naturalidad. Sin saludo de apertura (nada de «Hola» ni «buenos \
         días»), sin despedida, sin teléfono ni enlaces. \
         Datos del inmueble: {datos}. Aviso en Facebook: {aviso}. Usa solo esos \
         datos: si falta lo que pregunta, no lo inventes y di que lo consultas y le \
         avisas. No prometas visitas, horarios, descuentos ni disponibilidad que no \
         consten. Da el precio solo si está en los datos o en el aviso, con la cifra \
         y el $ al final (35.000$). El inmueble puede estar en venta y en \
         alquiler a la vez: si la descripción de los datos trae la cifra de la \
         otra operación (p. ej. «Alquiler: $2.800») y el Cliente pregunta por \
         ella, da esa. \
         Ya le dijiste (no lo repitas igual): {ya_dicho}"
    )
}

/// Forma final de la continuación: párrafos limpios, sin el contacto fijo que
/// la IA haya arrastrado del molde de apertura y acotada a unas pocas frases.
/// Vacía tras limpiar, cae a la reserva.
#[must_use]
pub fn dar_forma_continuacion(ia: &str) -> String {
    let limpio: Vec<String> = formatear_parrafos(ia)
        .split("\n\n")
        .map(|p| p.trim().trim_matches(['"', '«', '»']).trim().to_string())
        .filter(|p| !p.is_empty() && !p.contains(CONTACTO_WA) && !p.contains(CONTACTO_TEL))
        .collect();
    let texto = acotar(&limpio.join("\n\n"), CONTINUACION_MAX_CHARS);
    if texto.is_empty() {
        FALLBACK_CONTINUACION.to_string()
    } else {
        texto
    }
}

/// Recorta a `max` caracteres en el último fin de frase; sin fin de frase
/// dentro del tope, corta duro en `max`.
fn acotar(texto: &str, max: usize) -> String {
    if texto.chars().count() <= max {
        return texto.to_string();
    }
    let dentro: String = texto.chars().take(max).collect();
    let fin = dentro.rfind(['.', '!', '?']).map(|i| i + 1);
    match fin {
        Some(i) if i > 0 => dentro[..i].trim_end().to_string(),
        _ => dentro.trim_end().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solo_cliente_es_apertura() {
        assert!(!es_continuacion("Cliente: ¿Sigue disponible?"));
        assert!(!es_continuacion("Cliente: Hola\nCliente: ¿precio?"));
    }

    #[test]
    fn duena_y_ultimo_del_cliente_es_continuacion() {
        let ex = "Cliente: ¿Sigue disponible?\nDueña: Hola, sí, sigue disponible\nCliente: ¿Cuántos baños tiene?";
        assert!(es_continuacion(ex));
    }

    #[test]
    fn duena_al_final_no_es_continuacion() {
        assert!(!es_continuacion("Cliente: Hola\nDueña: Hola, buenas"));
    }

    #[test]
    fn mensaje_multilinea_del_cliente_cuenta_como_suyo() {
        let ex = "Dueña: Hola\nCliente: Gracias\nme interesa mucho";
        assert!(es_continuacion(ex));
    }

    #[test]
    fn sin_marcas_no_es_continuacion() {
        assert!(!es_continuacion("hola\n¿sigue disponible?"));
        assert!(!es_continuacion(""));
    }

    /// [10AA-28] Regresión: con respuesta de la dueña el hilo no es de apertura
    /// (no usa ni publica la plantilla compartida), aunque la dueña esté al final.
    #[test]
    fn hilo_con_duena_no_es_apertura() {
        assert!(es_apertura("Cliente: ¿Sigue disponible?"));
        assert!(es_apertura("hola\n¿sigue disponible?"));
        assert!(!es_apertura("Cliente: Hola\nDueña: Hola, buenas"));
        assert!(!es_apertura("Dueña: Hola\nCliente: Gracias"));
        assert!(!es_apertura("Cliente: ¿Sigue disponible?\nDueña: Sí"));
    }

    #[test]
    fn el_prompt_no_trae_molde_de_apertura_ni_contacto() {
        let p = sistema_continuacion("amable", "Casa en venta", "SIN FICHA", "nada todavía");
        assert!(p.contains("SOLO al último mensaje del Cliente"));
        assert!(p.contains("Tono amable"));
        assert!(!p.contains("Formato obligatorio"));
        assert!(!p.contains(CONTACTO_TEL));
        assert!(!p.contains(CONTACTO_WA));
    }

    #[test]
    fn la_forma_quita_contacto_arrastrado() {
        let ia =
            "Tiene 2 baños.\n\nCualquier cosa escríbeme al 04249208855\nhttps://wa.me/584249208855";
        assert_eq!(dar_forma_continuacion(ia), "Tiene 2 baños.");
    }

    #[test]
    fn la_forma_quita_comillas_del_modelo() {
        assert_eq!(dar_forma_continuacion("«Tiene 2 baños.»"), "Tiene 2 baños.");
    }

    #[test]
    fn la_forma_acota_en_fin_de_frase() {
        let frase = "Esta frase llena el espacio disponible. ";
        let largo = frase.repeat(20);
        let out = dar_forma_continuacion(&largo);
        assert!(out.chars().count() <= CONTINUACION_MAX_CHARS);
        assert!(out.ends_with('.'));
    }

    #[test]
    fn la_forma_vacia_cae_a_la_reserva() {
        assert_eq!(dar_forma_continuacion("   "), FALLBACK_CONTINUACION);
        assert_eq!(dar_forma_continuacion(CONTACTO_WA), FALLBACK_CONTINUACION);
    }
}
