/* [03AA-3 M3] Lógica pura del asistente Marketplace (sin HTTP): strip de ficha
 * por allowlist, validación del schema M3 v1, matriz negativa versionada,
 * claims del JWT mp y cubo de tasa por minuto. Verificable sin BD. */

use std::sync::atomic::{AtomicU64, Ordering};

use chrono::DateTime;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{url_publica_de_foto, InmuebleRow};
use crate::repositories::InmuebleRepository;

/* [08AA-7] Singleflight vive en su dominio (`marketplace_vuelo`); se
 * re-exporta para no mover sus usos externos (`lib.rs`, handlers, sombra). */
pub use super::marketplace_vuelo::{Generado, Singleflight};

/* [09AA-20] Burbujas estructuradas F0: tipos+validador+firma-v2 viven en su
 * dominio (`marketplace_burbujas`); se re-exporta para no mover sus usos
 * externos (handlers, utoipa, tests). */
pub use super::marketplace_burbujas::{
    estructuradas_apagadas, llave_esperada, texto_para_prompt, validar_conversacion,
    validar_idempotency_key, BurbujaIn, BurbujaUtil, ConversacionEstructurada,
    ConversacionValidada, ErrorEstructurado, Lado, LadoUtil, CODIGO_ESQUEMA, CODIGO_IDEMPOTENCIA,
    CODIGO_PAYLOAD_GIGANTE, CODIGO_REINTENTO_FOREGROUND, CODIGO_VERSION_DESCONOCIDA,
    ENV_KILL_SWITCH, FIRMA_VERSION_V2, MAX_BURBUJAS, MAX_HINT_CARACTERES, MAX_IDEMPOTENCY_CHARS,
    MAX_POR_BURBUJA, MAX_TOTAL_CARACTERES, VERSION_ESTRUCTURADA,
};
/* [09AA-30 F2] Caché compartida por inmueble vive en su dominio
 * (`marketplace_compartida`); se re-exporta para los handlers. */
pub use super::marketplace_compartida::{
    buscar_compartida, corregir_compartida, enlazar_compartida, mensaje_clave_de,
    ocurrencias_nombre, plantilla_de_nombre, purgar_compartida, rellenar_nombre,
    vincular_hilo_compartido, vinculo_compartido, ClaveCompartida, MARCADOR_NOMBRE,
};
/* [08AA-8] Texto puro (schema, excerpt, precio) vive en su dominio
 * (`marketplace_texto`); se re-exporta para no mover sus usos externos
 * (handlers, utoipa, sombra, tests). */
pub use super::marketplace_texto::{
    normalizar_excerpt, normalizar_excerpt_con_hilo, precio_del_aviso, precio_publico,
    validar_borrador, BorradorRequest, ExcerptIn, ExtrasIn, Largo, Tono,
};
/* Solo tests (`super::es_hex64` en `pruebas`): fuera de `cfg(test)` sería
 * import sin uso y rompería `clippy -D warnings` (mismo patrón que `ia.rs`). */
#[cfg(test)]
use super::marketplace_texto::es_hex64;

/// Versión del strip aceptada (`strip_vN` del plan).
/// [08AA-25] v2 suma `operacion` al allowlist: sin ella la IA presentaba
/// los alquileres como ventas (testigo: Townhouse Arivana, hilo cristo).
pub const STRIP_VERSION: &str = "v2";
/// Fallback exacto cuando no hay ficha o falla la IA.
/// [09AA-2] Sin «confirmo»: el prompt prohíbe anunciar confirmaciones y el
/// texto anterior («te confirmo precio/entrega») minaba ese veto.
pub const FALLBACK_BORRADOR: &str = "Lo reviso y te escribo el precio por aquí";
/// [07AA-8] Contacto fijo de los borradores (decisión de ella 2026-10-07):
/// la IA no lo inventa, el prompt lo exige literal y `asegurar_contacto`
/// lo agrega si falta. [08AA-14] Sin matriz negativa por decisión de ella
/// 2026-10-08: el texto (propio o de la IA) pasa tal cual.
pub const CONTACTO_TEL: &str = "04249208855";
pub const CONTACTO_WA: &str = "https://wa.me/584249208855";

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

/// [09AA-2] Fin del bucle del párrafo de relleno (08AA-36/37/38 burlados con
/// sinónimos): el prompt mismo ORDENABA el relleno («avanza la conversación:
/// ofrece fotos o pregunta qué busca») y esa orden positiva siempre le ganó
/// al veto. Ahora la invariante la impone Rust, no el wording: el borrador
/// de IA sale como P1 + [una línea de dato útil] + CTA canónico + wa.
/// Cualquier párrafo intermedio con pregunta, oferta de fotos o reafirmación
/// se poda; el final se reconstruye literal (nunca se conserva el de la IA).
/// No toca texto manual de la dueña ni la rama `reserva`.
pub const CTA_FIJO: &str = "Cuéntame qué estás buscando y con gusto te ayudo.";

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
fn p1_valido(p1: &str) -> bool {
    p1.contains("disponible") || p1.contains('$') || p1.contains(&FALLBACK_BORRADOR[..10])
}

/// Sanea el primer párrafo a nivel frase: fuera interrogativas (ofertas de
/// fotos / preguntas pegadas) y frases de relleno que no aporten el dato
/// central (el cual trae `$`, «disponible» o el fallback y por eso sobrevive).
fn sanear_p1(p1: &str) -> String {
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
fn es_dato_util(frase: &str) -> bool {
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
fn es_relleno(frase: &str) -> bool {
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
fn tiene_dato_concreto(frase: &str) -> bool {
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
fn partir_frases(t: &str) -> Vec<String> {
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
fn borrador_minimo() -> String {
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
fn vaciar_parrafo(actual: &mut String, parrafos: &mut Vec<String>) {
    if !actual.trim().is_empty() {
        parrafos.push(actual.trim().to_string());
        actual.clear();
    }
}

/// Corta `marca` a su propia línea cuando viene pegada a texto previo
/// (con espacio simple). Si ya abre línea se deja intacta.
fn partir_pegado(texto: &str, marca: &str) -> String {
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
fn es_item_lista(linea: &str) -> bool {
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

/// Prompt seguro: solo los 7 campos del allowlist. La frase canónica de la
/// ficha vive en la tabla `inmuebles`; el plugin jamás ve el resto.
/// [08AA-25] `operacion` (`venta`|`alquiler`, tal cual en la fila): sin
/// ella el prompt hablaba siempre en lenguaje de venta.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PromptSeguro {
    pub titulo: String,
    pub precio_publico: String,
    pub operacion: String,
    pub zona: String,
    pub m2: f64,
    pub habitaciones: i32,
    pub descripcion_corta: String,
}

/// Recorta la ficha al allowlist. `strip` distinto de la versión vigente se
/// rechaza para que un despliegue viejo no cuele campos nuevos en silencio.
pub fn strip_ficha_para_prompt(ficha: &InmuebleRow, strip: &str) -> Result<PromptSeguro, AppError> {
    if strip != STRIP_VERSION {
        return Err(AppError::BadRequest(format!(
            "strip desconocido: {strip} (se esperaba {STRIP_VERSION})"
        )));
    }
    let zona = if ficha.residencia.trim().is_empty() {
        ficha.ubicacion.clone()
    } else {
        format!("{}, {}", ficha.ubicacion, ficha.residencia)
    };
    Ok(PromptSeguro {
        titulo: ficha.titulo.clone(),
        precio_publico: precio_publico(ficha.precio),
        operacion: ficha.operacion.clone(),
        zona,
        m2: ficha.metros,
        habitaciones: ficha.habitaciones,
        descripcion_corta: ficha.descripcion.chars().take(500).collect(),
    })
}

/* [08AA-8] `precio_publico`, schema M3 (`BorradorRequest`…),
 * `validar_borrador` y `normalizar_excerpt` viven en `marketplace_texto.rs`
 * (re-export arriba para usos externos e internos). */

/* [08AA-8] `normalizar_excerpt*` vive en
 * `marketplace_texto.rs` (re-export arriba). */

/// [07AA-10] Nombre del cliente desde el hilo (`alejandro|casa en venta...`
/// → `Alejandro`): el borrador lo saluda por su nombre. `sin-hilo` o sin
/// `nombre|` → `None` (saludo sin nombre).
#[must_use]
pub fn nombre_de_thread(thread_id: &str) -> Option<String> {
    let (nombre, _) = thread_id.split_once('|')?;
    if nombre.trim().is_empty() || nombre.trim().eq_ignore_ascii_case("sin-hilo") {
        return None;
    }
    Some(
        nombre
            .split_whitespace()
            .map(|p| {
                let mut c = p.chars();
                match c.next() {
                    Some(i) => i.to_uppercase().to_string() + c.as_str(),
                    None => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join(" "),
    )
}

/* [08AA-8] `precio_del_aviso` vive en `marketplace_texto.rs`
 * (re-export arriba). */

/// [07AA-8] Título del aviso desde el `thread_id` del puente
/// (`comprador|aviso`, minúsculas, tope 120): contexto aproximado para abrir
/// el borrador con la ficha breve cuando no hay `avisoId` (piloto: siempre).
pub fn aviso_fb_de_thread(thread_id: &str) -> Option<String> {
    let aviso = thread_id
        .split('|')
        .nth(1)
        .map(str::trim)
        .unwrap_or_default();
    if aviso.is_empty() {
        return None;
    }
    Some(aviso.to_string())
}

/// [08AA-18] Clave canónica del hilo para guardar y buscar en caché: el
/// puente inyecta la cifra del DOM tras el `|` (`tina|$43.000 vef0
/// casa...`, 07AA-11) y esa cifra parpadea entre llamadas, así que el
/// `thread_id` literal no sirve de clave (el `/borrador` guarda con una
/// forma y el `/releer` busca con otra → `actualizado=false` en
/// silencio). Se deshace un `$CIFRA ` inicial con dígitos (`$43.000`,
/// `US$ 43.000`); sin `|`, sin `$` inicial, sin dígitos en la cifra o
/// si no quedaría aviso, el hilo queda intacto. La cifra sigue viva en
/// el hilo crudo que ven el prompt y `precio_del_aviso`: aquí solo se
/// estabiliza la llave de la BD.
#[must_use]
pub fn clave_hilo(thread_id: &str) -> String {
    let texto = thread_id.trim();
    let Some((nombre, aviso)) = texto.split_once('|') else {
        return texto.to_string();
    };
    /* Moneda inicial (`$`, `US$`, `RD$`): solo letras y `$`, sin
     * espacios ni dígitos. Sin `$` no es inyección del puente
     * (`Casa en venta`, `Casa 3 habs` quedan intactos). */
    let aviso = aviso.trim();
    let tras_moneda = aviso.trim_start_matches(|c: char| c.is_ascii_alphabetic() || c == '$');
    let prefijo = &aviso[..aviso.len() - tras_moneda.len()];
    if !prefijo.contains('$') {
        return texto.to_string();
    }
    /* Cifra: dígitos con `.`/`,` tras un blanco opcional. Sin
     * dígitos no es cifra (`$negociable casa` intacto). */
    let tras_blanco = tras_moneda.trim_start();
    let tras_cifra =
        tras_blanco.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == ',');
    if tras_cifra.len() == tras_blanco.len() || tras_cifra.trim_start().is_empty() {
        return texto.to_string();
    }
    format!("{}|{}", nombre.trim(), tras_cifra.trim_start())
}

/// [08AA-16] Normaliza el excerpt con contexto del hilo: nombre del
/// comprador y título del aviso salen del `thread_id`
/// (`comprador|aviso`). Lo usan `borrador`, `regenerar` y `releer` para
/// que el backend guarde la foto limpia del hilo, no el chrome del visor.
#[must_use]
pub fn normalizar_excerpt_hilo(thread_id: &str, texto: &str) -> String {
    normalizar_excerpt_con_hilo(
        texto,
        nombre_de_thread(thread_id).as_deref(),
        aviso_fb_de_thread(thread_id).as_deref(),
    )
}

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
fn quitar_tilde(c: char) -> char {
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
const PALABRAS_GENERICAS_TITULO: &[&str] = &[
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
fn es_generica(palabra: &str) -> bool {
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

/// Ficha publicada cuyo título mejor empareja con el del hilo: directo, o
/// solape ≥3 con ≥1 palabra distintiva. Empate entre dos avisos o BD
/// caída = `None` (nunca se cita un precio dudoso; quien llama decide si
/// lo registra: el borrador jamás se bloquea por esto).
/// [09AA-24] Cada ficha puntúa con su título + alias (`mejor_puntaje_con_alias`).
/// [09AA-29] `solo_sin_vinculo` (fallback de un ID exacto sin dueño) excluye
/// las fichas ya vinculadas ANTES de puntuar: así no hay falso negativo si la
/// mejor coincidencia es de otro aviso.
pub async fn ficha_por_titulo(
    pool: &sqlx::PgPool,
    titulo_fb: &str,
    solo_sin_vinculo: bool,
) -> Result<Option<InmuebleRow>, AppError> {
    let candidatos = if solo_sin_vinculo {
        InmuebleRepository::titulos_alias_publicados_sin_vinculo(pool).await?
    } else {
        InmuebleRepository::titulos_alias_publicados(pool).await?
    };
    let mut mejor: Option<(uuid::Uuid, usize, usize)> = None;
    let mut empate = false;
    for (id, titulo, alias) in &candidatos {
        let (directo, solape, distintivo) = mejor_puntaje_con_alias(titulo_fb, titulo, alias);
        if !(directo || (solape >= 3 && distintivo >= 1)) {
            continue;
        }
        let clave = (distintivo, solape);
        match mejor {
            Some((_, md, ms)) if (md, ms) == clave => empate = true,
            Some((_, md, ms)) if (md, ms) > clave => {}
            _ => {
                mejor = Some((*id, distintivo, solape));
                empate = false;
            }
        }
    }
    match (mejor, empate) {
        (Some((id, _, _)), false) => Ok(InmuebleRepository::find_by_id(pool, id).await?),
        _ => Ok(None),
    }
}

/// [07AA-8] Últimos borradores del hilo (máx 3, recientes primero): contexto
/// "ya dicho" para que la IA avance la conversación en vez de repetir.
/// [08AA-18] Lee con `clave_hilo()`: la cifra inyectada por el puente
/// (07AA-11) parpadea entre llamadas y el `thread_id` literal no empareja.
pub async fn hilo_previo(pool: &sqlx::PgPool, thread: &str) -> Result<Vec<String>, AppError> {
    let filas: Vec<String> = sqlx::query_scalar(
        "SELECT respuesta FROM mp_respuestas_cache \
         WHERE thread_id = $1 ORDER BY valida_hasta DESC LIMIT 3",
    )
    .bind(clave_hilo(thread))
    .fetch_all(pool)
    .await?;
    Ok(filas)
}

/// Claims del JWT mp (`iss mn-backend`, `aud mp`, `scope mp:borrador`).
/// `mid` (E3, solo CLI): hash hex64 de la máquina atada; `None` = token de
/// panel sin binding. `default` para que los tokens de panel en vuelo (sin
/// `mid`) sigan decodificando.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MpClaims {
    pub iss: String,
    pub sub: String,
    pub aud: String,
    pub scope: String,
    pub exp: usize,
    pub jti: String,
    #[serde(default)]
    pub mid: Option<String>,
}

/* [08AA-20] Vida del token CLI configurable: `MP_CLI_MINUTOS` manda
 * (local: 43200 = 30d en `.env`, gitignored); ausente/inválido → 480
 * (8h, lo que sigue viendo producción). El panel queda fijo en 15min.
 * Por qué env y no quitar la expiración: `jti`+revocación y binding a
 * máquina siguen valiendo; solo se estira el `exp`. */
#[must_use]
pub fn minutos_para_cli(es_cli: bool) -> i64 {
    if !es_cli {
        return 15;
    }
    std::env::var("MP_CLI_MINUTOS")
        .ok()
        .and_then(|v| v.trim().parse::<i64>().ok())
        .filter(|&m| m > 0)
        .unwrap_or(480)
}

/// Hash de máquina válido: 64 hex (igual que `firma`; nunca el id en claro).
#[must_use]
pub fn maquina_valida(mid: &str) -> bool {
    mid.len() == 64 && mid.chars().all(|c| c.is_ascii_hexdigit())
}

/// Binding E3: con `mid` en el token, la petición debe traer la misma máquina
/// en `X-MP-Maquina`; sin `mid` (panel) no se exige nada.
#[must_use]
pub fn maquina_autorizada(claims_mid: Option<&str>, cabecera: Option<&str>) -> bool {
    match (claims_mid, cabecera) {
        (None, _) => true,
        (Some(_), None) => false,
        (Some(a), Some(b)) => a == b,
    }
}

/// `MP_SIN_LIMITE_SUB` (coma-separada): la dueña queda exenta del 429 del
/// borrador; el resto cae al tope de 30/min con `Retry-After`.
#[must_use]
pub fn sub_exento(sub: &str) -> bool {
    std::env::var("MP_SIN_LIMITE_SUB")
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .any(|s| !s.is_empty() && s == sub)
}

/// Cubo por minuto atómico (`clave` + `date_trunc('minute')`): devuelve las
/// peticiones acumuladas incluyendo la actual.
pub async fn consumir_minuto(
    pool: &sqlx::PgPool,
    clave: &str,
    limite: i64,
) -> Result<bool, AppError> {
    let n: i64 = sqlx::query_scalar(
        "INSERT INTO mp_uso_minuto (clave, ventana, n) \
         VALUES ($1, date_trunc('minute', now()), 1) \
         ON CONFLICT (clave, ventana) DO UPDATE SET n = mp_uso_minuto.n + 1 \
         RETURNING n::BIGINT",
    )
    .bind(clave)
    .fetch_one(pool)
    .await?;
    Ok(n <= limite)
}

/// Emite un `jti` fresco y lo registra para poder revocarlo.
pub async fn registrar_token(
    pool: &sqlx::PgPool,
    sub: &str,
    expira_en: &DateTime<chrono::Utc>,
) -> Result<String, AppError> {
    let jti = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO mp_tokens_emitidos (jti, sub, expira_en) VALUES ($1, $2, $3)")
        .bind(&jti)
        .bind(sub)
        .bind(expira_en)
        .execute(pool)
        .await?;
    Ok(jti)
}

/// [08AA-39] Limpieza total del panel: borra TODAS las filas de caché de una
/// vez (la dueña lo pidió como botón al lado de «Recargar» para no depender
/// de limpiezas manuales por SQL). Devuelve cuántas filas cayeron.
pub async fn borrar_todo_cache(pool: &sqlx::PgPool) -> Result<u64, AppError> {
    let r = sqlx::query("DELETE FROM mp_respuestas_cache")
        .execute(pool)
        .await?;
    if r.rows_affected() > 0 {
        VERSION_BORRADORES_BORRADOS.fetch_add(1, Ordering::SeqCst);
    }
    Ok(r.rows_affected())
}

/// [09AA-4] Borrado previo a regenerar: elimina las filas del hilo que NO
/// son correcciones de la dueña (borradores viejos de excerpts anteriores).
/// Las correcciones (`corregida`, puestas por ella con el lápiz) jamás se
/// tocan: son su texto, no caché. Devuelve cuántas filas cayeron.
/// Sin esto, cada excerpt nuevo es una firma nueva y sus filas viejas viven
/// 90 días: el panel lista lo viejo junto a lo fresco y parece «cacheado».
pub async fn borrar_hilo_no_corregidas(
    pool: &sqlx::PgPool,
    thread_clave: &str,
) -> Result<u64, AppError> {
    let r = sqlx::query("DELETE FROM mp_respuestas_cache WHERE thread_id = $1 AND NOT corregida")
        .bind(thread_clave)
        .execute(pool)
        .await?;
    if r.rows_affected() > 0 {
        VERSION_BORRADORES_BORRADOS.fetch_add(1, Ordering::SeqCst);
    }
    Ok(r.rows_affected())
}

/// Archiva un hilo: solo lo oculta de `resumen_chats`. Su caché, sus
/// correcciones y la compartida quedan intactas.
pub async fn archivar_hilo(pool: &sqlx::PgPool, thread_clave: &str) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO mp_chats_archivados (thread_id) VALUES ($1) \
         ON CONFLICT (thread_id) DO NOTHING",
    )
    .bind(thread_clave)
    .execute(pool)
    .await?;
    Ok(())
}

/// Borra la conversación entera (incluidas sus correcciones) y quita su marca
/// de archivado. No toca `mp_respuestas_inmueble`: la compartida es del
/// inmueble y sigue sirviendo a otros hilos.
pub async fn borrar_hilo(pool: &sqlx::PgPool, thread_clave: &str) -> Result<u64, AppError> {
    let mut tx = pool.begin().await?;
    let r = sqlx::query("DELETE FROM mp_respuestas_cache WHERE thread_id = $1")
        .bind(thread_clave)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM mp_chats_archivados WHERE thread_id = $1")
        .bind(thread_clave)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    if r.rows_affected() > 0 {
        VERSION_BORRADORES_BORRADOS.fetch_add(1, Ordering::SeqCst);
    }
    Ok(r.rows_affected())
}

/// [09AA-31] Veces que un borrado de caché ha quitado filas (`borrar_borrador_hilo`,
/// `borrar_hilo`, `borrar_hilo_no_corregidas`, `borrar_todo_cache`). El float de lab
/// lo lee en cada barrido (`GET .../borradores/version`) y, si cambia, vacía su
/// caché en memoria: un borrador borrado no debe seguir sirviéndose desde ahí.
/// Proceso-local: un reinicio lo pone a 0, y el float lo trata como cambio.
static VERSION_BORRADORES_BORRADOS: AtomicU64 = AtomicU64::new(0);

pub fn version_borradores_borrados() -> u64 {
    VERSION_BORRADORES_BORRADOS.load(Ordering::SeqCst)
}

/// Borra solo los borradores no corregidos del hilo. De la compartida borra
/// la fila de cada mensaje que el hilo tenía, siempre que no sea corregida y
/// ningún otro hilo (ni una corrección propia) la siga enlazando.
pub async fn borrar_borrador_hilo(
    pool: &sqlx::PgPool,
    thread_clave: &str,
) -> Result<u64, AppError> {
    let mut tx = pool.begin().await?;
    // Las claves se leen antes de borrar: después ya no hay filas que enlazar.
    let claves: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT DISTINCT catalog_hash, precio_hash, mensaje_clave FROM mp_respuestas_cache \
         WHERE thread_id = $1 AND NOT corregida AND mensaje_clave IS NOT NULL",
    )
    .bind(thread_clave)
    .fetch_all(&mut *tx)
    .await?;
    let r = sqlx::query("DELETE FROM mp_respuestas_cache WHERE thread_id = $1 AND NOT corregida")
        .bind(thread_clave)
        .execute(&mut *tx)
        .await?;
    for (catalogo, precio, mensaje) in &claves {
        sqlx::query(
            "DELETE FROM mp_respuestas_inmueble ci WHERE ci.catalog_hash = $1 \
             AND ci.precio_hash = $2 AND ci.mensaje_clave = $3 AND NOT ci.corregida \
             AND NOT EXISTS (SELECT 1 FROM mp_respuestas_cache c \
             WHERE c.catalog_hash = ci.catalog_hash AND c.precio_hash = ci.precio_hash \
             AND c.mensaje_clave = ci.mensaje_clave)",
        )
        .bind(catalogo.as_str())
        .bind(precio.as_str())
        .bind(mensaje.as_str())
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    if r.rows_affected() > 0 {
        VERSION_BORRADORES_BORRADOS.fetch_add(1, Ordering::SeqCst);
    }
    Ok(r.rows_affected())
}

/// Fila del dashboard M2: conteos por día y evento. Sin PII: el HMAC del hilo
/// jamás sale, solo día + evento + conteo.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct UsoDia {
    pub dia: String,
    pub hit: i64,
    pub miss: i64,
    pub copiar: i64,
    pub regenerar: i64,
    pub emision: i64,
}

/// Agrega `mp_auditoria` en una sola consulta (`GROUP BY` día+evento, sin
/// N+1). `dias` se acota a 1..=90; la ventana es día calendario local del
/// servidor (misma base que `ts_hora` truncada a la hora).
pub async fn resumen_uso(pool: &sqlx::PgPool, dias: i32) -> Result<Vec<UsoDia>, AppError> {
    use sqlx::Row as _;
    let dias = dias.clamp(1, 90);
    let filas = sqlx::query(
        "SELECT ts_hora::date AS dia, evento, COUNT(*) AS n \
         FROM mp_auditoria \
         WHERE ts_hora >= date_trunc('day', now()) - make_interval(days => $1) \
         GROUP BY dia, evento ORDER BY dia",
    )
    .bind(dias)
    .fetch_all(pool)
    .await?;
    let mut orden: Vec<String> = Vec::new();
    let mut por_dia: std::collections::HashMap<String, UsoDia> = std::collections::HashMap::new();
    for f in &filas {
        let dia: chrono::NaiveDate = f.try_get("dia")?;
        let evento: String = f.try_get("evento")?;
        let n: i64 = f.try_get("n")?;
        let clave = dia.format("%Y-%m-%d").to_string();
        let entrada = por_dia.entry(clave.clone()).or_insert_with(|| {
            orden.push(clave.clone());
            UsoDia {
                dia: String::new(),
                hit: 0,
                miss: 0,
                copiar: 0,
                regenerar: 0,
                emision: 0,
            }
        });
        entrada.dia.clone_from(&clave);
        match evento.as_str() {
            "hit" => entrada.hit = n,
            "miss" => entrada.miss = n,
            "copiar" => entrada.copiar = n,
            "regenerar" => entrada.regenerar = n,
            "emision" => entrada.emision = n,
            otro => tracing::warn!("resumen_uso: evento desconocido {otro}"),
        }
    }
    Ok(orden
        .into_iter()
        .filter_map(|d| por_dia.remove(&d))
        .collect())
}

/// [07AA-7] Panel por chat: un chat = un `thread_id` (= clave de ventana
/// del puente, trae nombre+aviso: PII solo-admin por decisión de ella
/// 2026-10-07, misma retención 90d + purga).
/// [09AA-21] `aviso_conocido`: el aviso del hilo empareja con una ficha
/// (ID exacto o título); el front lo usa para la vista de huérfanos.
/// [09AA-23] `inmueble_vinculado`: título de la ficha emparejada (`None` =
/// huérfano); el front lo muestra junto a su miniatura, o «Sin ficha».
/// [09AA-28] `inmueble_foto`: URL pública (`/uploads/…`) de la portada del
/// inmueble vinculado; `None` si no hay vínculo o la ficha no tiene fotos.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ChatResumen {
    pub thread_id: String,
    pub borradores: i64,
    pub usos: i64,
    pub corregidas: i64,
    pub ultimo: String,
    pub aviso_conocido: bool,
    pub inmueble_vinculado: Option<String>,
    pub inmueble_foto: Option<String>,
}

/// [09AA-28] Vínculos de aviso: `marketplace_id` → (id, título, alias).
type VinculosAviso = std::collections::HashMap<String, (uuid::Uuid, String, Vec<String>)>;

/// [07AA-7] Una fila del chat: foto de la conversación + texto guardado.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ChatFila {
    pub excerpt_texto: String,
    pub respuesta: String,
    pub usos: i64,
    pub corregida: bool,
    pub valida_hasta: String,
    /// [09AA-30] `ia` = texto generado; `releer` = solo foto; `None` = fila
    /// anterior a la migración 09AA-30 (origen desconocido).
    pub origen: Option<String>,
    pub coste: Coste,
}

/// [09AA-31] Una página del panel: `hay_mas` indica si queda otra tras
/// `chats`; `total` cuenta todos los hilos con borradores (sin archivados),
/// o solo los huérfanos cuando se pide `solo_huerfanos` (10AA-4).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PaginaResumen {
    pub chats: Vec<ChatResumen>,
    pub total: i64,
    pub hay_mas: bool,
}

/// [09AA-31] Cursor keyset: la página siguiente empieza tras el hilo
/// `(ultimo, thread_id)` de la anterior (orden: más reciente primero).
#[derive(Debug, Clone)]
pub struct CursorChats {
    pub ultimo: chrono::DateTime<chrono::Utc>,
    pub thread_id: String,
}

/// Chats con borradores, ordenados por el más reciente, de `limite` en
/// `limite` (keyset, sin OFFSET). Una consulta para el total y dos para el
/// vínculo (títulos e IDs publicados, una vez, sin N+1): cada hilo resuelve en
/// memoria si su aviso es conocido.
/// [10AA-4] `solo_huerfanos`: solo hilos sin ficha conocida. El vínculo no vive
/// en SQL, así que se recorre por lotes hasta reunir `limite + 1` huérfanos.
/// Si las auxiliares fallan, todo queda `false` (el panel jamás se bloquea).
pub async fn resumen_chats(
    pool: &sqlx::PgPool,
    limite: i64,
    antes: Option<&CursorChats>,
    solo_huerfanos: bool,
) -> Result<PaginaResumen, AppError> {
    let tope = usize::try_from(limite).unwrap_or(usize::MAX);
    let candidatos: Vec<(uuid::Uuid, String, Vec<String>)> =
        match InmuebleRepository::titulos_alias_publicados(pool).await {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("resumen_chats: sin títulos publicados ({e}), avisos no conocidos");
                Vec::new()
            }
        };
    let vinculos: VinculosAviso = match InmuebleRepository::vinculos_publicados(pool).await {
        Ok(v) => v
            .into_iter()
            .map(|(aviso, id, titulo, alias)| (aviso, (id, titulo, alias)))
            .collect(),
        Err(e) => {
            tracing::warn!("resumen_chats: sin vínculos de aviso ({e}), solo título");
            std::collections::HashMap::new()
        }
    };
    /* Sin filtro: una consulta de `limite + 1` filas; la sobrante prueba que hay
     * otra página. Con filtro: lotes hasta llenar la página o agotar la BD. */
    let mut salida: Vec<(FilaChat, Option<(uuid::Uuid, String)>)> = Vec::new();
    let mut hay_mas = false;
    let mut tras: Option<(chrono::DateTime<chrono::Utc>, String)> =
        antes.map(|c| (c.ultimo, c.thread_id.clone()));
    'lotes: loop {
        let lote = if solo_huerfanos {
            LOTE_HUERFANOS
        } else {
            limite.saturating_add(1)
        };
        let leidas = filas_chats_tras(pool, lote, tras.as_ref()).await?;
        let agotado = i64::try_from(leidas.len()).unwrap_or(i64::MAX) < lote;
        for fila in leidas {
            tras = Some((fila.4, fila.0.clone()));
            let vinculado = titulo_vinculado_del_hilo(&fila.0, &candidatos, &vinculos);
            if solo_huerfanos && vinculado.is_some() {
                continue;
            }
            if salida.len() == tope {
                hay_mas = true;
                break 'lotes;
            }
            salida.push((fila, vinculado));
        }
        if agotado {
            break;
        }
    }
    /* [10AA-4] Con `solo_huerfanos` el total cuenta solo los huérfanos: el
     * vínculo se resuelve en memoria igual que en la lista. */
    let total = total_chats(pool, solo_huerfanos, &candidatos, &vinculos).await?;
    /* [09AA-28] Portadas de los inmuebles vinculados en una sola query
     * (únicos); si falla, el panel sigue sin miniaturas (jamás se bloquea). */
    let mut ids_vinculados: Vec<uuid::Uuid> = salida
        .iter()
        .filter_map(|(_, vinculado)| vinculado.as_ref().map(|(id, _)| *id))
        .collect();
    ids_vinculados.sort_unstable();
    ids_vinculados.dedup();
    let portadas = match InmuebleRepository::portadas_por_inmuebles(pool, &ids_vinculados).await {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("resumen_chats: sin portadas ({e}), chats sin miniatura");
            std::collections::HashMap::new()
        }
    };
    let chats = salida
        .into_iter()
        .map(
            |((thread_id, borradores, usos, corregidas, ultimo), vinculado)| {
                let inmueble_foto = vinculado
                    .as_ref()
                    .and_then(|(id, _)| portadas.get(id))
                    .map(|clave| url_publica_de_foto(clave));
                ChatResumen {
                    thread_id,
                    borradores,
                    usos,
                    corregidas,
                    ultimo: ultimo.to_rfc3339(),
                    aviso_conocido: vinculado.is_some(),
                    inmueble_vinculado: vinculado.map(|(_, titulo)| titulo),
                    inmueble_foto,
                }
            },
        )
        .collect();
    Ok(PaginaResumen {
        chats,
        total,
        hay_mas,
    })
}

/// Total de chats con borradores (sin archivados). Con `solo_huerfanos` cuenta
/// solo los hilos sin ficha conocida, con el mismo vínculo que la lista.
async fn total_chats(
    pool: &sqlx::PgPool,
    solo_huerfanos: bool,
    candidatos: &[(uuid::Uuid, String, Vec<String>)],
    vinculos: &VinculosAviso,
) -> Result<i64, AppError> {
    if solo_huerfanos {
        let hilos: Vec<String> = sqlx::query_scalar(
            "SELECT DISTINCT thread_id FROM mp_respuestas_cache \
             WHERE thread_id NOT IN (SELECT thread_id FROM mp_chats_archivados)",
        )
        .fetch_all(pool)
        .await?;
        let sin_ficha = hilos
            .iter()
            .filter(|h| titulo_vinculado_del_hilo(h, candidatos, vinculos).is_none())
            .count();
        return Ok(i64::try_from(sin_ficha).unwrap_or(i64::MAX));
    }
    let total = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT thread_id)::BIGINT FROM mp_respuestas_cache \
         WHERE thread_id NOT IN (SELECT thread_id FROM mp_chats_archivados)",
    )
    .fetch_one(pool)
    .await?;
    Ok(total)
}

/// [10AA-4] Hilos por lote al filtrar huérfanos: cuántos hilos se leen de la
/// caché por vuelta antes de volver a mirar el vínculo en memoria.
const LOTE_HUERFANOS: i64 = 100;

/// [10AA-4] Fila agregada por hilo: (thread_id, borradores, usos, corregidas,
/// último `valida_hasta`).
type FilaChat = (String, i64, i64, i64, chrono::DateTime<chrono::Utc>);

/// [10AA-4] Hasta `lote` hilos tras el cursor `(ultimo, thread_id)`, en el mismo
/// orden que el panel. Excluye los archivados, igual que `total`.
async fn filas_chats_tras(
    pool: &sqlx::PgPool,
    lote: i64,
    tras: Option<&(chrono::DateTime<chrono::Utc>, String)>,
) -> Result<Vec<FilaChat>, AppError> {
    let (ultimo, hilo) = match tras {
        Some((u, h)) => (Some(*u), Some(h.as_str())),
        None => (None, None),
    };
    let filas: Vec<FilaChat> = sqlx::query_as(
        "SELECT thread_id, COUNT(*)::BIGINT, COALESCE(SUM(usos), 0)::BIGINT, \
         SUM(CASE WHEN corregida THEN 1 ELSE 0 END)::BIGINT, MAX(valida_hasta) \
         FROM mp_respuestas_cache \
         WHERE thread_id NOT IN (SELECT thread_id FROM mp_chats_archivados) \
         GROUP BY thread_id \
         HAVING $1::TIMESTAMPTZ IS NULL OR (MAX(valida_hasta), thread_id) < ($1, $2) \
         ORDER BY MAX(valida_hasta) DESC, thread_id DESC \
         LIMIT $3",
    )
    .bind(ultimo)
    .bind(hilo)
    .bind(lote)
    .fetch_all(pool)
    .await?;
    Ok(filas)
}

/* [09AA-21] ¿El aviso del hilo empareja con una ficha? Rama exacta primero:
 * el texto tras `|` son dígitos 5–32 vinculados; si no, emparejado por título
 * (misma regla que `ficha_por_titulo`: directo o solape ≥3 con distintiva,
 * empate = no conocido). Pura en memoria (sin BD).
 * [09AA-23] Devuelve el título emparejado (rama exacta: el título del
 * vínculo; rama título: el título candidato). `resumen_chats` deriva
 * `aviso_conocido` como `vinculado.is_some()`.
 * [09AA-24] Las ramas por título puntúan título + alias: el hilo puede
 * nombrar cualquiera de los nombres del inmueble, pero el badge muestra
 * siempre el título canónico.
 * [09AA-28] Devuelve (id, título): el id resuelve la portada del panel.
 * [09AA-29] La rama por título compara contra TODOS los candidatos, no solo
 * contra las fichas sin vínculo: el filtro `solo_sin_vinculo` de
 * `ficha_por_titulo` existe para el fallback de un ID de aviso sin dueño (no
 * puede citar la ficha de OTRO aviso). Aquí el texto tras `|` es un título
 * (no un ID numérico), así que el hilo ya lo nombra y la ficha vinculada a su
 * propio aviso sí debe verse. Pendiente de decisión: la rama de dígitos NO cae
 * al título, a diferencia del borrador (F2); el badge puede decir «Sin ficha»
 * donde el borrador cita precio. */
fn titulo_vinculado_del_hilo(
    thread_id: &str,
    candidatos: &[(uuid::Uuid, String, Vec<String>)],
    vinculos: &VinculosAviso,
) -> Option<(uuid::Uuid, String)> {
    let aviso = aviso_fb_de_thread(thread_id).unwrap_or_default();
    let recortado = aviso.trim();
    if recortado.is_empty() {
        return None;
    }
    if recortado.chars().all(|c| c.is_ascii_digit()) && (5..=32).contains(&recortado.len()) {
        return vinculos
            .get(recortado)
            .map(|(id, titulo, _)| (*id, titulo.clone()));
    }
    let mut mejor: Option<(usize, usize)> = None;
    let mut vinculo_mejor: Option<(uuid::Uuid, String)> = None;
    let mut empate = false;
    for (id, titulo, alias) in candidatos {
        let (directo, solape, distintivo) = mejor_puntaje_con_alias(recortado, titulo, alias);
        if !(directo || (solape >= 3 && distintivo >= 1)) {
            continue;
        }
        let clave = (distintivo, solape);
        match mejor {
            Some(m) if m == clave => empate = true,
            Some(m) if m > clave => {}
            _ => {
                mejor = Some(clave);
                vinculo_mejor = Some((*id, titulo.clone()));
                empate = false;
            }
        }
    }
    if mejor.is_some() && !empate {
        vinculo_mejor
    } else {
        None
    }
}

/// Filas de un chat (tope 200, recientes primero).
/// [08AA-18] Busca con `clave_hilo()` (ver `hilo_previo`).
pub async fn detalle_chat(pool: &sqlx::PgPool, thread: &str) -> Result<Vec<ChatFila>, AppError> {
    type FilaBd = (
        String,
        String,
        i64,
        bool,
        chrono::DateTime<chrono::Utc>,
        Option<String>,
        Option<i64>,
        Option<i64>,
        Option<i64>,
    );
    let filas: Vec<FilaBd> = sqlx::query_as(
        "SELECT excerpt_texto, respuesta, usos::BIGINT, corregida, valida_hasta, \
         origen, tokens_entrada, tokens_salida, ms_generacion \
         FROM mp_respuestas_cache WHERE thread_id = $1 \
         ORDER BY valida_hasta DESC LIMIT 200",
    )
    .bind(clave_hilo(thread))
    .fetch_all(pool)
    .await?;
    Ok(filas
        .into_iter()
        .map(
            |(excerpt_texto, respuesta, usos, corregida, valida_hasta, origen, te, ts, ms)| {
                ChatFila {
                    excerpt_texto,
                    respuesta,
                    usos,
                    corregida,
                    valida_hasta: valida_hasta.to_rfc3339(),
                    origen,
                    coste: Coste {
                        tokens_entrada: te,
                        tokens_salida: ts,
                        ms,
                    },
                }
            },
        )
        .collect())
}

/* [03AA-3 M4] Caché de respuestas (`mp_respuestas_cache`): la clave es
 * (firma, precio_hash, catalog_hash). `precio_hash` ata la respuesta al
 * precio citado (si cambia el precio, miss y se regenera: jamás se sirve un
 * precio viejo). `catalog_hash` sale de `hash_ficha` — hash de los bytes que
 * alimentan el prompt (campos del strip + estado) calculados tras el fetch
 * y antes del strip; cualquier cambio ahí invalida. Campos ajenos al prompt
 * (copy, receta, extras) NO invalidan a propósito: no cambian la respuesta.
 * Solo se cachea `fuente=ia`; el fallback nunca (con la IA caída, cachearlo
 * envenenaría 90 días). La corrección humana (`corregida`) gana sobre
 * generaciones futuras (`guardar` usa DO NOTHING; solo `reemplazar`, vía
 * Regenerar explícito, la pisa). Sin ficha: claves literales "sin-ficha"
 * (la `firma` ya diferencia cada excerpt). */

/// Marca sin ficha para `precio_hash`/`catalog_hash` (la firma diferencia).
pub const SIN_FICHA: &str = "sin-ficha";

/// SHA-256 hex con `sha2` (ya dependencia directa). Solo hashes, sin PII.
#[must_use]
pub fn sha_hex(canon: &str) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(canon.as_bytes()))
}

/// Hash del catálogo: serialización canónica de exactamente lo que entra al
/// prompt (los 7 campos del strip + `estado`, que condiciona disponibilidad),
/// calculada sobre la fila recién leída y antes de stripeear. Si la ficha
/// cambia en algo que la respuesta cita → hash distinto → miss → regenera.
#[must_use]
pub fn hash_ficha(ficha: &InmuebleRow) -> String {
    let canon = match strip_ficha_para_prompt(ficha, STRIP_VERSION) {
        Ok(s) => serde_json::json!({
            "titulo": s.titulo,
            "precio": s.precio_publico,
            "operacion": s.operacion,
            "zona": s.zona,
            "m2": s.m2,
            "hab": s.habitaciones,
            "desc": s.descripcion_corta,
            "estado": ficha.estado,
        }),
        /* Inalcanzable con v2 (el handler lo rechazaría antes); clave
         * estable para no romper el flujo si el strip evoluciona. */
        Err(_) => serde_json::json!({"strip": "error"}),
    };
    sha_hex(&canon.to_string())
}

/// Hash del precio citado: el `precio_publico` ya formateado que ve la IA.
#[must_use]
pub fn precio_hash_seguro(seguro: &PromptSeguro) -> String {
    sha_hex(&seguro.precio_publico)
}

/// Hit de caché: el texto listo + si es corrección de la dueña. El `UPDATE`
/// atómico cuenta el uso en la misma sentencia (sin roundtrip ni carrera).
pub async fn buscar_cache(
    pool: &sqlx::PgPool,
    firma: &str,
    precio_hash: &str,
    catalog_hash: &str,
) -> Result<Option<(String, bool)>, AppError> {
    let fila: Option<(String, bool)> = sqlx::query_as(
        "UPDATE mp_respuestas_cache SET usos = usos + 1 \
         WHERE firma = $1 AND precio_hash = $2 AND catalog_hash = $3 \
         AND valida_hasta > now() \
         RETURNING respuesta, corregida",
    )
    .bind(firma)
    .bind(precio_hash)
    .bind(catalog_hash)
    .fetch_optional(pool)
    .await?;
    Ok(fila)
}

/// [08AA-21] Foto del hilo que acompaña a cada fila de caché: clave de la
/// ventana + excerpt limpio (panel) + excerpt crudo tal como llegó del
/// puente (diagnóstico del filtro, 08AA-8). Viaja junta para no engordar
/// la firma de `guardar_cache`/`reemplazar_cache` (clippy: máx 7 args).
pub struct FotoHilo<'a> {
    pub thread_id: &'a str,
    pub excerpt: &'a str,
    pub excerpt_crudo: &'a str,
}

/// [09AA-30] Coste de una generación IA: tokens del `usage` (`None` si el
/// relay no los trajo) y tiempo de la llamada en ms. Viaja con la fila para
/// que el admin muestre lo que costó el texto guardado; un hit no lo repite.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, ToSchema)]
pub struct Coste {
    pub tokens_entrada: Option<i64>,
    pub tokens_salida: Option<i64>,
    pub ms: Option<i64>,
}

/// Guarda una generación fresca; si la dueña ya corrigió esa clave, su texto
/// gana (`DO NOTHING`: la corrección humana no se pisa en silencio).
/// [07AA-7] Anota `thread_id` + `excerpt_texto` para el panel por chat.
/// [08AA-18] Guarda con `clave_hilo()`: la cifra inyectada por el puente
/// (07AA-11) parpadea entre llamadas y el `thread_id` literal no empareja
/// al releer.
/// [08AA-21] Guarda también `excerpt_crudo`: el texto tal como llegó del
/// puente, antes de `normalizar_excerpt`. El filtro por líneas no se puede
/// calibrar a ciegas (el puente aplana el DOM y lo pegado no se ve);
/// con el crudo a la vista se corrige el filtro (08AA-8).
pub async fn guardar_cache(
    pool: &sqlx::PgPool,
    firma: &str,
    precio_hash: &str,
    catalog_hash: &str,
    respuesta: &str,
    foto: &FotoHilo<'_>,
    coste: Coste,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO mp_respuestas_cache (firma, precio_hash, catalog_hash, respuesta, thread_id, excerpt_texto, excerpt_crudo, origen, tokens_entrada, tokens_salida, ms_generacion) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, 'ia', $8, $9, $10) ON CONFLICT DO NOTHING",
    )
    .bind(firma)
    .bind(precio_hash)
    .bind(catalog_hash)
    .bind(respuesta)
    .bind(clave_hilo(foto.thread_id))
    .bind(foto.excerpt)
    .bind(foto.excerpt_crudo)
    .bind(coste.tokens_entrada)
    .bind(coste.tokens_salida)
    .bind(coste.ms)
    .execute(pool)
    .await?;
    Ok(())
}

/// Pisa la fila (Regenerar explícito de la dueña): texto nuevo, vigencia
/// renovada, `corregida=FALSE`, contador a cero (nueva versión).
/// [07AA-7] Refresca también `thread_id` + `excerpt_texto` (foto actual).
/// [08AA-18] Guarda con `clave_hilo()` (ver `guardar_cache`).
/// [08AA-21] Refresca también `excerpt_crudo` (ver `guardar_cache`).
pub async fn reemplazar_cache(
    pool: &sqlx::PgPool,
    firma: &str,
    precio_hash: &str,
    catalog_hash: &str,
    respuesta: &str,
    foto: &FotoHilo<'_>,
    coste: Coste,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO mp_respuestas_cache (firma, precio_hash, catalog_hash, respuesta, thread_id, excerpt_texto, excerpt_crudo, origen, tokens_entrada, tokens_salida, ms_generacion) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, 'ia', $8, $9, $10) \
         ON CONFLICT (firma, precio_hash, catalog_hash) DO UPDATE SET \
         respuesta = EXCLUDED.respuesta, valida_hasta = now() + INTERVAL '90 days', \
         corregida = FALSE, usos = 0, thread_id = EXCLUDED.thread_id, \
         excerpt_texto = EXCLUDED.excerpt_texto, excerpt_crudo = EXCLUDED.excerpt_crudo, \
         origen = EXCLUDED.origen, tokens_entrada = EXCLUDED.tokens_entrada, \
         tokens_salida = EXCLUDED.tokens_salida, ms_generacion = EXCLUDED.ms_generacion",
    )
    .bind(firma)
    .bind(precio_hash)
    .bind(catalog_hash)
    .bind(respuesta)
    .bind(clave_hilo(foto.thread_id))
    .bind(foto.excerpt)
    .bind(foto.excerpt_crudo)
    .bind(coste.tokens_entrada)
    .bind(coste.tokens_salida)
    .bind(coste.ms)
    .execute(pool)
    .await?;
    Ok(())
}

/// [08AA-31] Foto combinada del hilo (pura, sin BD): une la foto vieja con
/// el `limpio` nuevo, línea a línea, con dedup exacto y tope de 30 líneas
/// (las últimas). Sin esto, `releer_foto` pisaba todas las filas con el
/// último snapshot y si ese solo traía lo propio (`Tú:`), el mensaje del
/// cliente desaparecía del panel (hilo angelv). Las líneas del cliente
/// viajan sin etiqueta y las propias como `Tú:` (08AA-29): el dedup es por
/// línea exacta, así que ambos lados conviven.
#[must_use]
pub fn combinar_foto_hilo(vieja: &str, nueva: &str) -> String {
    use std::collections::HashSet as Conjunto;
    let mut vistas: Conjunto<String> = Conjunto::new();
    let mut lineas: Vec<&str> = Vec::new();
    for linea in vieja.lines().chain(nueva.lines()) {
        let t = linea.trim();
        if t.is_empty() || vistas.contains(t) {
            continue;
        }
        vistas.insert(t.to_string());
        lineas.push(t);
    }
    let desde = lineas.len().saturating_sub(30);
    lineas[desde..].join("\n")
}

/// [08AA-28] Releer con creación: refresca la foto del hilo; si no hay
/// fila (caché borrada o hilo nuevo sin borrador), la crea solo con la
/// foto y `respuesta` vacía para que el chat aparezca en el panel sin
/// inventar borrador. La PK sintética (`firma=sha("releer-sin-borrador|hilo")`,
/// `precio/catalog="releer"`) nunca choca con firmas HMAC reales, así un
/// borrador posterior inserta su propia fila y gana por `valida_hasta`.
/// `ON CONFLICT DO UPDATE` lo hace idempotente (doble clic o releers
/// concurrentes convergen; regla 6: upsert atómico, no buscar-crear).
/// [08AA-31] Fusiona con `combinar_foto_hilo` en vez de pisar: la foto
/// vieja aporta las líneas que el último snapshot ya no trae (el mensaje
/// del cliente cuando el eco propio es lo único nuevo).
/// Devuelve `(actualizado, creado)`.
pub async fn releer_foto(
    pool: &sqlx::PgPool,
    hilo: &str,
    limpio: &str,
    crudo: &str,
) -> Result<(bool, bool), AppError> {
    let vieja: Option<(String,)> = sqlx::query_as(
        "SELECT excerpt_texto FROM mp_respuestas_cache WHERE thread_id = $1 \
         ORDER BY valida_hasta DESC LIMIT 1",
    )
    .bind(clave_hilo(hilo))
    .fetch_optional(pool)
    .await?;
    let combinada = match &vieja {
        Some((v,)) => combinar_foto_hilo(v, limpio),
        None => limpio.to_string(),
    };
    let tocadas = sqlx::query(
        "UPDATE mp_respuestas_cache SET excerpt_texto = $1, excerpt_crudo = $2 \
         WHERE thread_id = $3",
    )
    .bind(&combinada)
    .bind(crudo)
    .bind(clave_hilo(hilo))
    .execute(pool)
    .await?
    .rows_affected();
    if tocadas > 0 {
        return Ok((true, false));
    }
    let firma = sha_hex(&format!("releer-sin-borrador|{hilo}"));
    sqlx::query(
        "INSERT INTO mp_respuestas_cache (firma, precio_hash, catalog_hash, respuesta, thread_id, excerpt_texto, excerpt_crudo, origen) \
         VALUES ($1, 'releer', 'releer', '', $2, $3, $4, 'releer') \
         ON CONFLICT (firma, precio_hash, catalog_hash) DO UPDATE SET \
         excerpt_texto = EXCLUDED.excerpt_texto, excerpt_crudo = EXCLUDED.excerpt_crudo",
    )
    .bind(firma)
    .bind(clave_hilo(hilo))
    .bind(&combinada)
    .bind(crudo)
    .execute(pool)
    .await?;
    Ok((true, true))
}

/// Borra la fila (primer paso de Regenerar: la siguiente lectura es miss).
pub async fn borrar_cache(
    pool: &sqlx::PgPool,
    firma: &str,
    precio_hash: &str,
    catalog_hash: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "DELETE FROM mp_respuestas_cache \
         WHERE firma = $1 AND precio_hash = $2 AND catalog_hash = $3",
    )
    .bind(firma)
    .bind(precio_hash)
    .bind(catalog_hash)
    .execute(pool)
    .await?;
    Ok(())
}

/// Guarda la corrección de la dueña (su texto manda tal cual: lo revisa
/// ella a mano al enviar). Vigencia renovada; la fila misma es el registro
/// (sin audit separada: `corregida=TRUE` + `usos` ya lo cuentan).
/// [08AA-14] Sin matriz negativa por decisión de ella 2026-10-08.
pub async fn corregir_cache(
    pool: &sqlx::PgPool,
    firma: &str,
    precio_hash: &str,
    catalog_hash: &str,
    texto: &str,
) -> Result<(), AppError> {
    let n = texto.chars().count();
    if n == 0 || n > 2000 {
        return Err(AppError::Validation("texto 1..2000 caracteres".to_string()));
    }
    sqlx::query(
        "INSERT INTO mp_respuestas_cache (firma, precio_hash, catalog_hash, respuesta, corregida) \
         VALUES ($1, $2, $3, $4, TRUE) \
         ON CONFLICT (firma, precio_hash, catalog_hash) DO UPDATE SET \
         respuesta = EXCLUDED.respuesta, valida_hasta = now() + INTERVAL '90 days', \
         corregida = TRUE, usos = 0",
    )
    .bind(firma)
    .bind(precio_hash)
    .bind(catalog_hash)
    .bind(texto)
    .execute(pool)
    .await?;
    Ok(())
}

/// Purga vencidas; devuelve cuántas cayeron. Se corre al arrancar (siempre) y
/// a diario vía `pg_cron` (solo `DB_24H=true`).
pub async fn purgar_cache(pool: &sqlx::PgPool) -> Result<u64, sqlx::Error> {
    let r = sqlx::query("DELETE FROM mp_respuestas_cache WHERE valida_hasta <= now()")
        .execute(pool)
        .await?;
    Ok(r.rows_affected() + purgar_compartida(pool).await?)
}

/// Programa la purga diaria en `pg_cron` (07:00 UTC = 03:00 Caracas, sin horario
/// de verano). Idempotente (reemplaza el job si existe). Falla si no hay
/// `pg_cron` en el servidor: el llamador lo deja en `warn` y sigue (la purga al
/// arrancar ya cubre; fail-open documentado, nunca tumba el boot).
pub async fn programar_purga_diaria(pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    sqlx::query("CREATE EXTENSION IF NOT EXISTS pg_cron")
        .execute(pool)
        .await?;
    sqlx::query(
        "DO $purga$ BEGIN \
           IF EXISTS (SELECT 1 FROM cron.job WHERE jobname = 'mp-purga-diaria') THEN \
             PERFORM cron.unschedule('mp-purga-diaria'); \
           END IF; \
           PERFORM cron.schedule('mp-purga-diaria', '0 7 * * *', \
             'DELETE FROM mp_respuestas_cache WHERE valida_hasta <= now()'); \
         END $purga$",
    )
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod pruebas;
