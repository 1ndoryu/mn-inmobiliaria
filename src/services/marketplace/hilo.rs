use crate::errors::AppError;

use super::normalizar_excerpt_con_hilo;

pub(crate) mod cache;
pub(crate) mod ficha;
pub(crate) mod titulo;

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

/* [08AA-8] `precio_del_aviso` vive en `marketplace/texto.rs`
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
    match separar_id_hilo(texto) {
        Some((base, id)) => format!("{}|{id}", clave_sin_id(base)),
        None => clave_sin_id(texto),
    }
}

/// [13AA-13] Separa el id de perfil del comprador (último segmento,
/// `nombre|aviso|id`, solo dígitos, 5–40). Exige `nombre|aviso` delante:
/// `tina|123456` es un hilo sin id (el número es el aviso). Un id fuera de
/// formato no se separa: queda dentro del aviso, como hoy.
fn separar_id_hilo(texto: &str) -> Option<(&str, &str)> {
    let (base, id) = texto.rsplit_once('|')?;
    let id = id.trim();
    let valido = base.contains('|')
        && (5..=40).contains(&id.len())
        && id.bytes().all(|b| b.is_ascii_digit());
    valido.then_some((base, id))
}

/// Clave sin id: `nombre|aviso` con la cifra inyectada deshecha (ver
/// `clave_hilo`).
fn clave_sin_id(thread_id: &str) -> String {
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

/// [11AA] Quita el saludo que la IA escribió al principio («Hola, Ana, buenas
/// tardes. El…»): el saludo lo añade `servir_cuerpo` al servir y la caché guarda
/// solo el cuerpo. El nombre se quita solo si es el del hilo. Si no hay saludo,
/// o no queda cuerpo, devuelve el texto tal cual.
#[must_use]
pub fn quitar_saludo(texto: &str, nombre: Option<&str>) -> String {
    let Some(mut resto) = quitar_palabra(
        texto.trim_start_matches(|c: char| c.is_whitespace() || c == '¡'),
        "hola",
    ) else {
        return texto.to_string();
    };
    resto = resto.trim_start_matches(es_separador);
    if let Some(n) = nombre.map(str::trim).filter(|n| !n.is_empty()) {
        if let Some(tras_nombre) = quitar_palabra(resto, n) {
            resto = tras_nombre.trim_start_matches(es_separador);
        }
    }
    for franja in ["buenos días", "buenas tardes", "buenas noches"] {
        if let Some(tras_franja) = quitar_palabra(resto, franja) {
            resto = tras_franja.trim_start_matches(es_separador);
            break;
        }
    }
    let mut chars = resto.trim().chars();
    match chars.next() {
        Some(primera) => format!("{}{}", primera.to_uppercase(), chars.as_str()),
        None => texto.to_string(),
    }
}

/// Separadores entre las piezas de un saludo: espacios y `, . ! ;`.
pub(super) fn es_separador(c: char) -> bool {
    c.is_whitespace() || matches!(c, ',' | '.' | '!' | ';')
}

/// Minúsculas y sin tilde, para comparar saludos (`Días` → `dias`).
pub(super) fn plegar(c: char) -> char {
    match c.to_lowercase().next().unwrap_or(c) {
        'á' => 'a',
        'é' => 'e',
        'í' => 'i',
        'ó' => 'o',
        'ú' | 'ü' => 'u',
        otro => otro,
    }
}

/// Quita `palabra` al inicio de `texto` (sin distinguir mayúsculas ni tildes) y
/// devuelve lo que sigue. Exige separador o fin detrás: «Holanda» no es «Hola».
pub(super) fn quitar_palabra<'a>(texto: &'a str, palabra: &str) -> Option<&'a str> {
    let mut fin = 0;
    let mut tramos = texto.char_indices();
    for pc in palabra.chars() {
        let (i, tc) = tramos.next()?;
        if plegar(tc) != plegar(pc) {
            return None;
        }
        fin = i + tc.len_utf8();
    }
    match texto[fin..].chars().next() {
        Some(c) if !es_separador(c) => None,
        _ => Some(&texto[fin..]),
    }
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
