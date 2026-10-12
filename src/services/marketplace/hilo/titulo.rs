/// [08AA-10] El piloto no trae `avisoId`, pero el título del hilo sí nombra
/// el aviso y el catálogo tiene la ficha con el precio real: se empareja
/// en el backend (fuente de verdad) en vez de fiarse del DOM. Sin
/// `regex` en el árbol: normalización manual (caja, tildes, ruido).
#[must_use]
pub fn normalizar_titulo(s: &str) -> String {
    let mut fuera = String::with_capacity(s.len());
    for c in s.to_lowercase().chars() {
        if c.is_alphanumeric() {
            fuera.push(quitar_tilde(c));
        } else if !fuera.ends_with(' ') {
            fuera.push(' ');
        }
    }
    fuera.trim().to_string()
}

/// Minúsculas ya aplicadas por quien llama.
pub(in crate::services::marketplace) fn quitar_tilde(c: char) -> char {
    match c {
        'á' | 'à' | 'ä' | 'â' => 'a',
        'é' | 'è' | 'ë' | 'ê' => 'e',
        'í' | 'ì' | 'ï' | 'î' => 'i',
        'ó' | 'ò' | 'ö' | 'ô' => 'o',
        'ú' | 'ù' | 'ü' | 'û' => 'u',
        'ñ' => 'n',
        'ç' => 'c',
        _ => c,
    }
}

/// Palabras que no identifican un aviso (operación, tipología, zonas
/// grandes, conectores): dos hilos distintos las comparten.
pub(in crate::services::marketplace) const PALABRAS_GENERICAS_TITULO: &[&str] = &[
    "casa",
    "venta",
    "alquiler",
    "alquilo",
    "vendo",
    "vende",
    "apto",
    "apartamento",
    "terreno",
    "local",
    "galpon",
    "oficina",
    "town",
    "house",
    "townhouse",
    "quinta",
    "villa",
    "edificio",
    "piso",
    "anexo",
    "habitacion",
    "habitaciones",
    "urb",
    "urbanizacion",
    "residencia",
    "residencias",
    "conjunto",
    "sector",
    "zona",
    "centro",
    "norte",
    "sur",
    "este",
    "oeste",
    "puerto",
    "ordaz",
    "ciudad",
    "guayana",
    "bolivar",
    "san",
    "felix",
    "en",
    "de",
    "del",
    "la",
    "el",
    "los",
    "las",
    "con",
    "por",
    "para",
    "negociable",
];

/// Palabra sin valor identificativo: corta, cifra, precio (`vef0`,
/// `usd120000`) o genérica del negocio.
pub(in crate::services::marketplace) fn es_generica(palabra: &str) -> bool {
    palabra.len() <= 2
        || palabra.chars().all(|c| c.is_ascii_digit())
        || palabra.starts_with("vef")
        || palabra.starts_with("usd")
        || palabra.starts_with('$')
        || PALABRAS_GENERICAS_TITULO.contains(&palabra)
}

/// `(directo, solape, distintivo)`: `directo` si un título normalizado
/// contiene al otro (títulos de ≥12 caracteres: un "apto" suelto no vale);
/// si no, conteo de palabras compartidas y cuántas son distintivas.
#[must_use]
pub fn puntaje_titulo(fb: &str, titulo: &str) -> (bool, usize, usize) {
    let fb_n = normalizar_titulo(fb);
    let titulo_n = normalizar_titulo(titulo);
    if fb_n.is_empty() || titulo_n.is_empty() {
        return (false, 0, 0);
    }
    let directo = (fb_n.contains(&titulo_n) || titulo_n.contains(&fb_n))
        && fb_n.len() >= 12
        && titulo_n.len() >= 12;
    let en_fb: std::collections::HashSet<&str> = fb_n.split(' ').collect();
    let mut solape = 0;
    let mut distintivo = 0;
    for p in titulo_n.split(' ') {
        if p.len() > 2 && en_fb.contains(p) {
            solape += 1;
            if !es_generica(p) {
                distintivo += 1;
            }
        }
    }
    (directo, solape, distintivo)
}

/// [09AA-24] Mejor puntaje entre el título canónico y sus alias: el mismo
/// inmueble puede publicarse con otro nombre (Caroní Plaza = Río Aro Plaza)
/// y el hilo nombra cualquiera de los dos. Orden del tuple: directo manda,
/// luego solape, luego distintivo.
#[must_use]
pub fn mejor_puntaje_con_alias(fb: &str, titulo: &str, alias: &[String]) -> (bool, usize, usize) {
    let mut mejor = puntaje_titulo(fb, titulo);
    for nombre in alias {
        let puntos = puntaje_titulo(fb, nombre);
        if puntos > mejor {
            mejor = puntos;
        }
    }
    mejor
}
