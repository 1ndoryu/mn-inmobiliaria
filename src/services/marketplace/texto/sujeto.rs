//! [GW-16] Cómo se nombra un inmueble dentro de una frase. El `nombre_corto`
//! guardado («Casa en Las Orquídeas», «Apart. en Resd. Kama Merú») es una
//! etiqueta; aquí se convierte en sujeto con artículo y tipo completo, y el
//! lugar del parecido en una ubicación con artículo. Estaba en `forma.rs`, que
//! llegó al tope de tamaño del gate.

/// [GW-16] El `nombre_corto` guardado como sujeto de una frase: con artículo y
/// el tipo entero («La casa en…», «El apartamento en…»). Sin nombre guardado,
/// «Este inmueble». El `nombre_corto` en sí no se toca: sigue siendo la
/// etiqueta corta de la ficha y del panel.
#[must_use]
pub fn sujeto_inicial(nombre_corto: &str) -> String {
    let n = nombre_corto.trim();
    if n.is_empty() {
        return "Este inmueble".to_string();
    }
    let Some((palabra, femenino, resto)) = partir_nombre(n) else {
        /* Un tipo que no reconocemos se cita tal cual: mejor literal que
         * inventarle un artículo. */
        return n.to_string();
    };
    if palabra == "inmueble" {
        return "Este inmueble".to_string();
    }
    format!("{} {palabra}{resto}", if femenino { "La" } else { "El" })
}

/// [GW-16] El tipo de `nombre_corto` como palabra completa, su género y el
/// resto del nombre («Casa en Las Orquídeas» → (`casa`, true, ` en Las
/// Orquídeas`)). `None` si el tipo no está en el allowlist: se cita literal.
fn partir_nombre(n: &str) -> Option<(&'static str, bool, &str)> {
    /* El tipo va delante de « en »: es lo único que se traduce. El resto (la
     * residencia) se cita tal cual, que es lo que ella reconoce. */
    let (tipo, resto) = match n.find(" en ") {
        Some(i) => (&n[..i], &n[i..]),
        None => (n, ""),
    };
    let (palabra, femenino) = match tipo.trim().trim_end_matches('.').to_lowercase().as_str() {
        "apart" | "apartamento" => ("apartamento", false),
        "casa" => ("casa", true),
        "townhouse" => ("townhouse", false),
        "local" => ("local", false),
        "terreno" => ("terreno", false),
        "inmueble" => ("inmueble", false),
        _ => return None,
    };
    Some((palabra, femenino, resto))
}

/// [GW-16] El inmueble del parecido como alternativa: indefinido, porque no es
/// el que se pidió («una casa en Guayana Country Club», «un apartamento en
/// Resd. San Ignacio»).
#[must_use]
pub(in crate::services::marketplace) fn alternativa_en_frase(nombre_corto: &str) -> String {
    let n = nombre_corto.trim();
    if n.is_empty() {
        return "otro inmueble".to_string();
    }
    let Some((palabra, femenino, resto)) = partir_nombre(n) else {
        return n.to_string();
    };
    if palabra == "inmueble" {
        return "otro inmueble".to_string();
    }
    format!("{} {palabra}{resto}", if femenino { "una" } else { "un" })
}

/// [GW-16] Una ubicación con su artículo, para que la frase no diga «en Av.
/// Atlantico»: «la Av. Atlantico», «la Urb. Riberas del Caroní», «el sector
/// UD-146». Lo que ya lleva artículo propio («Las Peonías», «El Tiamo») y los
/// nombres que no son vía ni reparto se quedan como están.
#[must_use]
pub(in crate::services::marketplace) fn lugar_con_articulo(lugar: &str) -> String {
    let l = lugar.trim();
    if l.is_empty() {
        return String::new();
    }
    let primera = l
        .split(|c: char| c.is_whitespace() || c == '.')
        .next()
        .unwrap_or("")
        .to_lowercase();
    let articulo = match primera.as_str() {
        "av" | "avda" | "avenida" | "calle" | "urb" | "urbanizacion" | "resd" | "residencia"
        | "residencias" | "carretera" | "prolongacion" | "prol" | "final" => "la",
        "sector" | "edif" | "edificio" | "conjunto" | "barrio" | "centro" | "parque" => "el",
        /* Ya trae su propio artículo, o es un nombre propio: tal cual. */
        _ => return l.to_string(),
    };
    format!("{articulo} {l}")
}

/// [GW-16] El mismo sujeto a mitad de frase («para el apartamento en…»): el
/// artículo va en minúscula cuando no abre la oración.
#[must_use]
pub(in crate::services::marketplace) fn sujeto_en_frase(nombre_corto: &str) -> String {
    en_minuscula(sujeto_inicial(nombre_corto))
}

/// [GW-16] La primera letra en minúscula, sin tocar el resto.
fn en_minuscula(s: String) -> String {
    let mut s = s;
    if let Some(p) = s.get(0..1) {
        s.replace_range(0..p.len(), &p.to_lowercase());
    }
    s
}

/// [GW-16] La zona sin la parte que el sujeto ya dice. `zona` es
/// «`ubicacion`, `residencia`» y `nombre_corto` es «`Tipo` en `residencia`»,
/// así que citar las dos repite el mismo nombre en una frase («Av. Atlantico,
/// Guayana Country Club: la casa en Guayana Country Club»). Devuelve solo lo
/// que el sujeto no nombra; vacío si no queda nada.
#[must_use]
pub(in crate::services::marketplace) fn zona_sin_residencia(
    zona: &str,
    nombre_corto: &str,
) -> String {
    let z = zona.trim().trim_end_matches('.').trim();
    let Some((_, residencia)) = nombre_corto.trim().split_once(" en ") else {
        return z.to_string();
    };
    let objetivo = palabras(residencia.trim());
    if objetivo.is_empty() {
        return z.to_string();
    }
    /* La residencia cuelga del final de la zona, pero cada fila la escribe a
     * su manera: «Urb. Riberas del Caroní» en la zona y «Urbanización
     * Riberas del Caroní» en el nombre. Se compara por palabra, con las
     * abreviaturas de dirección expandidas, y se corta donde empieza. */
    let suyas = palabras(z);
    let corte_en = suyas.len().checked_sub(objetivo.len());
    let Some(desde) = corte_en else {
        return z.to_string();
    };
    if !suyas[desde..]
        .iter()
        .zip(&objetivo)
        .all(|(a, b)| a.0 == b.0)
    {
        return z.to_string();
    }
    /* Del prefijo se cae también el separador que queda colgando. */
    z[..suyas[desde].1]
        .trim()
        .trim_end_matches([',', '-', '–'])
        .trim()
        .to_string()
}

/// Palabras de un texto con su byte de inicio y la abreviatura expandida.
fn palabras(t: &str) -> Vec<(String, usize)> {
    let mut out = Vec::new();
    let mut inicio = None;
    for (i, c) in t.char_indices() {
        if c.is_whitespace() {
            if let Some(s) = inicio.take() {
                out.push((clave(&t[s..i]), s));
            }
        } else if inicio.is_none() {
            inicio = Some(i);
        }
    }
    if let Some(s) = inicio {
        out.push((clave(&t[s..]), s));
    }
    out
}

/// La palabra sin puntuación periférica, sin tildes y con las abreviaturas
/// de dirección expandidas. Las dos formas se normalizan igual porque cada
/// fila escribe lo mismo a su manera («Urb.» frente a «Urbanización»).
fn clave(p: &str) -> String {
    let limpia = p.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase();
    let plana = super::sin_tilde_min(&limpia);
    match plana.as_str() {
        "urb" | "urbanizacion" => "urbanizacion",
        "resd" | "residencias" | "residencia" => "residencia",
        "av" | "avda" | "avenida" => "avenida",
        "edif" | "edificio" => "edificio",
        "prol" | "prolongacion" => "prolongacion",
        _ => return plana,
    }
    .to_string()
}

/// [GW-16] Reescribe el `nombre_corto` que la IA (o una fila de caché ya
/// guardada) usa como sujeto: «Casa en Las Orquídeas sigue disponible…» pasa a
/// «La casa en Las Orquídeas sigue disponible…». [GW-15] El cierre no lo lleva,
/// así que el resto del texto no se toca. Case-sensitive a propósito: el texto
/// ya reescrito lleva el tipo en minúscula y no vuelve a coincidir, de modo que
/// aplicarlo dos veces da lo mismo.
#[must_use]
pub fn normalizar_sujeto(texto: &str, nombre_corto: &str) -> String {
    let n = nombre_corto.trim();
    if n.is_empty() || !texto.contains(n) {
        return texto.to_string();
    }
    let abre = texto.starts_with(n);
    let mut t = texto.replace(n, &sujeto_en_frase(n));
    /* El sujeto que abre la frase lleva el artículo en mayúscula. */
    if abre {
        if let Some(p) = t.get(0..1) {
            t.replace_range(0..p.len(), &p.to_uppercase());
        }
    }
    t
}

/// [GW-16] Cifra publicada sin el `$` ni espacios (`"$86.000"` → `"86.000"`).
#[must_use]
pub(in crate::services::marketplace) fn cifra_sin_simbolo(precio: &str) -> String {
    precio.trim().trim_start_matches('$').trim().to_string()
}
