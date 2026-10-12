use crate::services::marketplace::{CONTACTO_TEL, CONTACTO_WA, CTA_FIJO, FALLBACK_BORRADOR};

/// [07AA-8] Garantía determinista del formato: si el texto generado no trae
/// el teléfono o el enlace, se agregan (teléfono como penúltima línea,
/// enlace cerrando). Solo se aplica al borrador de IA, nunca al texto
/// manual de la dueña.
#[must_use]
pub fn asegurar_contacto(texto: &str) -> String {
    use std::fmt::Write as _;
    let mut t = texto.trim_end().to_string();
    if !t.contains(CONTACTO_TEL) {
        let _ = write!(t, "\nCualquier cosa escríbeme al {CONTACTO_TEL}");
    }
    if !t.contains(CONTACTO_WA) {
        t.push('\n');
        t.push_str(CONTACTO_WA);
    }
    t
}

/// Garantía determinista de forma sobre el texto crudo de la IA.
#[must_use]
pub fn imponer_forma_borrador(ia: &str) -> String {
    let norm = formatear_parrafos(ia);
    let ps: Vec<String> = norm
        .split("\n\n")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if ps.is_empty() {
        return borrador_minimo();
    }
    /* Ancla final: último párrafo con contacto o CTA (lo que la IA haya
     * puesto ahí se descarta igual; el final se reconstruye canónico). */
    let fin = ps
        .iter()
        .rposition(|p| {
            p.contains(CONTACTO_WA) || p.contains(CONTACTO_TEL) || p.contains("qué estás buscando")
        })
        .unwrap_or(ps.len() - 1);
    let p1 = sanear_p1(&ps[0]);
    if !p1_valido(&p1) {
        return borrador_minimo();
    }
    /* Medios = párrafos entre P1 y el ancla; se conserva como máximo UNA
     * frase que sea dato útil (respuesta a pregunta concreta no cubierta). */
    let mut medio: Option<String> = None;
    for m in ps.iter().skip(1).take(fin.saturating_sub(1)) {
        for f in partir_frases(m) {
            if es_dato_util(&f) {
                medio = Some(f);
                break;
            }
        }
        if medio.is_some() {
            break;
        }
    }
    let mut out = vec![p1];
    if let Some(m) = medio {
        out.push(m);
    }
    out.push(format!(
        "{CTA_FIJO} Cualquier cosa escríbeme al {CONTACTO_TEL}"
    ));
    out.push(CONTACTO_WA.to_string());
    formatear_parrafos(&out.join("\n\n"))
}

/// P1 trae disponibilidad o precio (o el fallback): si la IA alucinó otro
/// texto, se descarta todo y va el mínimo.
pub(in crate::services::marketplace) fn p1_valido(p1: &str) -> bool {
    p1.contains("disponible") || p1.contains('$') || p1.contains(&FALLBACK_BORRADOR[..10])
}

/// Sanea el primer párrafo a nivel frase: fuera interrogativas (ofertas de
/// fotos / preguntas pegadas) y frases de relleno que no aporten el dato
/// central (el cual trae `$`, «disponible» o el fallback y por eso sobrevive).
pub(in crate::services::marketplace) fn sanear_p1(p1: &str) -> String {
    let frases: Vec<String> = partir_frases(p1)
        .into_iter()
        .filter(|f| {
            f.contains("wa.me")
                || (!f.contains('?')
                    && !f.contains('¿')
                    && (!es_relleno(f)
                        || f.contains('$')
                        || f.contains("disponible")
                        || f.contains(&FALLBACK_BORRADOR[..10])))
        })
        .collect();
    let unido = frases.join(" ");
    unido.chars().take(400).collect()
}

/// Una frase sobrevive en el medio solo si es dato concreto (dígito o
/// sustantivo de ficha), sin preguntas, sin contacto y sin relleno.
pub(in crate::services::marketplace) fn es_dato_util(frase: &str) -> bool {
    let t = frase.trim();
    (3..=140).contains(&t.chars().count())
        && !t.contains('?')
        && !t.contains('¿')
        && !t.contains("wa.me")
        && !t.contains(CONTACTO_TEL)
        && !es_relleno(t)
        && tiene_dato_concreto(t)
}

/// Relleno por intención (actos de habla), no por frases: ofertas, preguntas,
/// reafirmaciones de estado/precio y meta-coordinación. En minúsculas.
pub(in crate::services::marketplace) fn es_relleno(frase: &str) -> bool {
    const RELLENO: &[&str] = &[
        "foto",
        "compart",
        "enví",
        "envi",
        "interesa",
        "dispon",
        "vige",
        "publica",
        "precio",
        "$",
        "mensual",
        "canon",
        "cuesta",
        "vale",
        "estatus",
        "ficha",
        "negociable",
        "visita",
        "coordin",
        "confirm",
        "busca",
        "ayudo",
        "gusto",
        "encanta",
        "oferta",
        "descuento",
        "oportunidad",
        "aprovecha",
        "anímate",
        "animate",
        "escríbeme",
        "escribeme",
        "llámame",
        "llamame",
        "contáctame",
        "contactame",
        "dueña",
        "duena",
    ];
    let min = frase.to_lowercase();
    RELLENO.iter().any(|r| min.contains(r))
}

/// Dato concreto = dígito o sustantivo de ficha (baños, m2, ubicación...).
pub(in crate::services::marketplace) fn tiene_dato_concreto(frase: &str) -> bool {
    const DATOS: &[&str] = &[
        "bañ",
        "habit",
        "dormitorio",
        "m2",
        "m²",
        "metro",
        "terreno",
        "puesto",
        "amobl",
        "ubic",
        "financ",
        "cuota",
        "cocina",
        "estaciona",
        "piscina",
        "pozo",
        "planta",
        "sala",
        "comedor",
        "vista",
        "colegio",
        "centro",
        "cerca",
        "villa",
        "residenc",
    ];
    if frase.chars().any(|c| c.is_ascii_digit()) {
        return true;
    }
    let min = frase.to_lowercase();
    DATOS.iter().any(|d| min.contains(d))
}

/// Parte en frases por `.`/`?`/`!`/salto; el punto entre dígitos ($43.000)
/// no parte para no romper cifras.
pub(in crate::services::marketplace) fn partir_frases(t: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut act = String::new();
    let mut it = t.chars().peekable();
    while let Some(c) = it.next() {
        act.push(c);
        let corta = match c {
            '?' | '!' | '\n' => true,
            '.' => !matches!(it.peek(), Some(n) if n.is_ascii_digit()),
            _ => false,
        };
        if corta {
            let f = act.trim().to_string();
            if !f.is_empty() {
                out.push(f);
            }
            act.clear();
        }
    }
    let f = act.trim().to_string();
    if !f.is_empty() {
        out.push(f);
    }
    out
}

/// Mínimo servible cuando la IA devolvió basura: fallback + contacto + wa.
pub(in crate::services::marketplace) fn borrador_minimo() -> String {
    formatear_parrafos(&format!(
        "{FALLBACK_BORRADOR}\nCualquier cosa escríbeme al {CONTACTO_TEL}\n{CONTACTO_WA}"
    ))
}

/// [08AA-11] Párrafos coherentes para el borrador que se copia a
/// `WhatsApp`: la IA a veces devuelve líneas sueltas (venía de pedirle
/// "máximo 6 líneas") o pega el contacto al final de la frase anterior.
/// Reglas deterministas: cada salto simple dentro de un párrafo se vuelve
/// espacio; los bloques se separan con una línea en blanco; la invitación
/// de contacto y el enlace wa.me siempre abren su propio párrafo; los
/// ítems de lista (`1. `, `- `, `• `) quedan en párrafo propio.
#[must_use]
pub fn formatear_parrafos(texto: &str) -> String {
    let plano = texto.replace("\r\n", "\n").replace('\r', "\n");
    let marca_contacto = format!("Cualquier cosa escríbeme al {CONTACTO_TEL}");
    let con_contacto = partir_pegado(&plano, &marca_contacto);
    let con_wa = partir_pegado(&con_contacto, CONTACTO_WA);
    let mut parrafos: Vec<String> = Vec::new();
    let mut actual = String::new();
    for linea in con_wa.lines() {
        let l = linea.trim();
        if l.is_empty() {
            vaciar_parrafo(&mut actual, &mut parrafos);
            continue;
        }
        if es_item_lista(l) {
            vaciar_parrafo(&mut actual, &mut parrafos);
            parrafos.push(l.to_string());
            continue;
        }
        if !actual.is_empty() {
            actual.push(' ');
        }
        actual.push_str(l);
    }
    vaciar_parrafo(&mut actual, &mut parrafos);
    parrafos.join("\n\n")
}

/// Vacía el párrafo en curso a la lista (sin el cierre no se puede usar
/// cierre + `push` directo: doble préstamo mutable del acumulador).
pub(in crate::services::marketplace) fn vaciar_parrafo(
    actual: &mut String,
    parrafos: &mut Vec<String>,
) {
    if !actual.trim().is_empty() {
        parrafos.push(actual.trim().to_string());
        actual.clear();
    }
}

/// Corta `marca` a su propia línea cuando viene pegada a texto previo
/// (con espacio simple). Si ya abre línea se deja intacta.
pub(in crate::services::marketplace) fn partir_pegado(texto: &str, marca: &str) -> String {
    let mut fuera = String::with_capacity(texto.len() + 8);
    let mut resto = texto;
    while let Some(pos) = resto.find(marca) {
        let antes = &resto[..pos];
        fuera.push_str(antes);
        if !(antes.is_empty() || antes.ends_with('\n')) {
            fuera.push_str("\n\n");
        }
        fuera.push_str(marca);
        resto = &resto[pos + marca.len()..];
    }
    fuera.push_str(resto);
    fuera
}

/// Ítem de lista al inicio de la línea: `1. `, `2) `, `- ` o `• `.
/// Siempre por `chars` (nunca por bytes: `•` es multibyte).
pub(in crate::services::marketplace) fn es_item_lista(linea: &str) -> bool {
    let mut letras = linea.chars();
    match letras.next() {
        Some('-' | '•') => letras.next() == Some(' '),
        Some(c) if c.is_ascii_digit() => {
            let mut cola = linea.chars();
            cola.next();
            let mut cola = cola.peekable();
            while cola.peek().is_some_and(char::is_ascii_digit) {
                cola.next();
            }
            match cola.next() {
                Some('.' | ')') => matches!(cola.next(), Some(' ') | None),
                _ => false,
            }
        }
        _ => false,
    }
}
