use super::super::{CONTACTO_TEL, CONTACTO_WA, CTA_FIJO};
use super::excerpt::{es_etiqueta, sin_tilde_min};

/* [08AA-29] Ruido por CONTENIDO (no por prefijo): el aviso de
 * seguridad de Meta sobrevive al pelado de Enter/cola cuando el pegado
 * del chat lo deja como texto suelto (`fin de detectar y reducir...`)
 * o como eco (`... por Tú: Si te vas a reunir...`). Frases tomadas del
 * aviso real; ninguna puede ser mensaje de un cliente. Se comparan sin
 * tildes y en minúsculas. */
const RUIDO_EXCERPT_CONTENIDO: &[&str] = &[
    "detectar y reducir las estafas",
    "familiares y amigos ad",
    "compartir la ubicaci",
    "consejos de seguridad",
    /* [09AA-16] `Se unió a Facebook en 2010` (testigo edickson en BD):
     * el año varía por comprador, así que casa por contenido y no por
     * línea exacta. Nadie escribe eso como mensaje. */
    "se unio a facebook en",
];

/// Prefijos literales de ruido de Facebook (ES + EN).
/// [08AA-8] +cabeceras ES del hilo wilmery (testigo 554 en BD, texto
/// pegado): `También es miembro de...`, `Detalles del comprador` /
/// `Detalles de la conversación` y `Ver perfil...` nunca son contenido.
pub(super) const RUIDO_EXCERPT_PREFIJOS: &[&str] = &[
    "Si te vas a reunir con alguien",
    "If you're meeting someone",
    "If you are meeting someone",
    "Meta podría usar tecnología",
    "Meta may use technology",
    "También es miembro de",
    "Also a member of",
    "Detalles de",
    "Detalles del",
    "Details of",
    "Ver perfil",
    "View profile",
    "Escribe en ",
    "Write to ",
    "Presionar Enter",
    "Press Enter",
    "Mensaje enviado",
    "Message sent",
    /* [08AA-29] `Enviado hace 1 min` (marca de mensaje propio sin el
     * prefijo `Mensaje enviado`, testigo Yusmelis en BD). */
    "Enviado hace ",
];

/// Líneas completas del chrome del visor (comparación exacta).
/// [08AA-16] +`Mensajes` (cabecera de la columna), la instrucción de las
/// respuestas rápidas y `Enviado` (marca de mensaje propio enviado).
/// [08AA-17] +`Marketplace` suelto, `Cargando...`/`Loading...`
/// (placeholder de hilo aún cargando) del reporte Tina.
pub(super) const RUIDO_EXCERPT_EXACTO: &[&str] = &[
    "Marketplace",
    "Cargando...",
    "Loading...",
    "View buyer",
    "More options",
    "Ver perfil",
    "Ver perfil del comprador",
    "Detalles del comprador",
    "Detalles de la conversación",
    "Buyer details",
    "Mensajes",
    "Enviar mensaje",
    "Escribir mensaje",
    "Aa",
    /* [08AA-29] `wa.me` suelto (línea del chrome junto al `Enviado hace`,
     * testigo Yusmelis en BD) y `En medio de la conversación` (divisor
     * del visor, mismo testigo): nunca son mensajes. */
    "wa.me",
    "En medio de la conversación",
    "Ver más consejos de seguridad",
    "See more safety tips",
    "Toca una respuesta",
    "Toca una respuesta para enviársela al comprador.",
    "Tap a reply",
    "Envía una respuesta rápida",
    "Send a quick reply",
    "Enviado",
    "Visto",
    "Seen",
];

/// Colas huérfanas y etiquetas sueltas del visor (comparación exacta,
/// como `RUIDO_EXCERPT_EXACTO`, pero SIN partir el texto pegado:
/// `segmentar_pegado` usa `EXACTO` como marcadores de corte y un
/// fragmento ahí partiría `Detalles del comprador` dejando `Detalles`
/// huérfano —testigo wilmery— en vez de filtrar la línea entera).
/// [09AA-16] `del comprador` (el float parte `Ver perfil del comprador`
/// en dos líneas y la cabeza `Ver perfil` ya se filtra sola) y
/// `Comprador` (etiqueta de rol; testigo edickson en BD). Ningún mensaje
/// real es solo una de estas líneas.
const RUIDO_EXCERPT_COLA: &[&str] = &["del comprador", "Comprador"];

/// Respuestas rápidas sugeridas por Facebook: solo se filtran sin marca de
/// rol (el chip centrado no trae `Cliente:`/`Dueña:`). Si el cliente las
/// escribe de verdad, llevan marca y se conservan.
/// [08AA-16] +las dos sugeridas ES del hilo Kerley (testigo en BD).
pub(super) const RESPUESTAS_RAPIDAS_FB: &[&str] = &[
    "Sí. ¿Te interesa?",
    "Sí. ¿Sigue disponible?",
    "¿Cuál es el precio?",
    "Lo estoy mirando. Te avisaré.",
    "Lo siento, no está disponible.",
    "Yes. Are you interested?",
    "Yes. Is this still available?",
    "What is the price?",
];

/// Cuerpo de la línea sin la marca de rol (`Cliente:`/`Dueña:`), si la trae.
pub(super) fn cuerpo_sin_marca(linea: &str) -> &str {
    linea
        .strip_prefix("Cliente:")
        .or_else(|| linea.strip_prefix("Dueña:"))
        .map_or(linea, str::trim_start)
}

pub(super) fn es_ruido_excerpt(linea: &str) -> bool {
    let cuerpo = cuerpo_sin_marca(linea);
    if cuerpo.contains("inició este chat") || cuerpo.contains("started this chat") {
        return true;
    }
    if RUIDO_EXCERPT_PREFIJOS.iter().any(|p| cuerpo.starts_with(p)) {
        return true;
    }
    if RUIDO_EXCERPT_EXACTO.contains(&cuerpo) {
        return true;
    }
    if RUIDO_EXCERPT_COLA.contains(&cuerpo) {
        return true;
    }
    /* [08AA-29] Ruido por contenido (aviso de seguridad de Meta dejado
     * como texto suelto por el pegado del chat): se compara sin tildes
     * y en minúsculas. Ninguna de estas frases puede ser mensaje real. */
    let canon = sin_tilde_min(cuerpo);
    if RUIDO_EXCERPT_CONTENIDO
        .iter()
        .any(|f| canon.contains(&sin_tilde_min(f)))
    {
        return true;
    }
    let sin_marca = cuerpo.len() == linea.len();
    sin_marca && RESPUESTAS_RAPIDAS_FB.contains(&cuerpo)
}

/* [09AA-17] El aria del visor pega el fragmento `wa.me` tras la URL
 * completa sin separador (`...855wa.mewa.me`, testigo edgarluis en BD):
 * si tras la URL solo quedan repeticiones de `wa.me`, se truncan para
 * recuperar el cierre canónico. Con cualquier otra cola no se toca. */
pub(super) fn despegar_url_wa(linea: &str) -> &str {
    const ESQUEMA: &str = "https://wa.me/";
    let Some(pos) = linea.find(ESQUEMA) else {
        return linea;
    };
    let mut fin = pos + ESQUEMA.len();
    let bytes = linea.as_bytes();
    while fin < bytes.len() && bytes[fin].is_ascii_digit() {
        fin += 1;
    }
    if fin == pos + ESQUEMA.len() {
        return linea;
    }
    let mut resto = &linea[fin..];
    while let Some(siguiente) = resto.strip_prefix("wa.me") {
        resto = siguiente;
    }
    if resto.is_empty() {
        linea[..fin].trim_end()
    } else {
        linea
    }
}

/// Cierre canónico del borrador propio (`CTA_FIJO` + contacto de
/// `imponer_forma_borrador`): igualdad exacta; el cliente nunca escribe
/// estas líneas tal cual.
fn es_cierre_propio(linea: &str) -> bool {
    let cuerpo = linea.trim();
    if cuerpo == CTA_FIJO || cuerpo == CONTACTO_WA {
        return true;
    }
    cuerpo
        .strip_prefix("Cualquier cosa escríbeme al")
        .is_some_and(|resto| resto.trim().trim_end_matches('.').trim() == CONTACTO_TEL)
}

/* [09AA-17] Eco del borrador propio como líneas sin marca (testigo
 * edgarluis en BD: la burbuja de las 12:59am trae `por Edgarluis:` + el
 * tip + NUESTRO borrador, y el pelado de la atribución lo deja como
 * supuesto Cliente). Sin marca de rol, los cierres canónicos y el
 * saludo `Hola, {nombre},` (fórmula de `saludo_y_regla`: el comprador
 * nunca se saluda a sí mismo por su nombre) solo pueden ser eco propio.
 * Lo marcado (`Cliente:`/`Dueña:`/`Tú:`) se conserva siempre: la marca
 * es atribución explícita del visor. */
pub(super) fn es_eco_propio_sin_marca(linea: &str, nombre: Option<&str>) -> bool {
    if linea.starts_with("Cliente:") || linea.starts_with("Dueña:") || es_etiqueta(linea) {
        return false;
    }
    let cuerpo = despegar_url_wa(linea.trim());
    if es_cierre_propio(cuerpo) {
        return true;
    }
    if let Some(nom) = nombre.map(str::trim).filter(|n| !n.is_empty()) {
        let saludo = format!("hola, {},", sin_tilde_min(nom));
        if sin_tilde_min(cuerpo).starts_with(&saludo) {
            return true;
        }
    }
    false
}

/* [09AA-17] Del eco propio (`por Tú:`) se guardan solo las líneas con
 * contenido real: los cierres son boilerplate que `imponer_forma`
 * re-agrega al generar, y sueltos sin marca el panel los muestra como
 * Cliente (la segunda copia del testigo edgarluis). */
pub(super) fn sin_cierres_propios(mensaje: &str) -> String {
    mensaje
        .lines()
        .map(|linea| despegar_url_wa(linea.trim()))
        .filter(|linea| !linea.is_empty() && !es_cierre_propio(linea))
        .collect::<Vec<_>>()
        .join("\n")
}
