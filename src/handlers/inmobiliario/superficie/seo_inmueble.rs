/* [13AA-6] SEO por inmueble en el servidor. `/inmueble/<slug>` devuelve el
 * `index.html` del SPA con título, descripción, canónica, Open Graph, Twitter
 * y JSON-LD del inmueble ya escritos: los crawlers que no ejecutan JS (la
 * mayoría) los leen tal cual. El cliente (`fijarEtiquetasPagina` en
 * `platform/documento.ts`) repite lo mismo al navegar; ambos deben generar lo
 * mismo. Slug inexistente o no publicado: 404 con el mismo HTML, para que el
 * SPA muestre su aviso sin que el buscador indexe una página vacía. */

use std::fmt::Write as _;
use std::path::Path;

use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use sqlx::PgPool;

use crate::errors::AppError;
use crate::models::{FiltrosPublicos, Inmueble};
use crate::services::InmuebleService;

/* Base canónica de producción (la misma que `index.html`): no se toma del
 * `Host` de la petición para que nadie pueda inyectar otro origen. */
const ORIGEN: &str = "https://mn-inmobiliaria.com";
const IMAGEN_POR_DEFECTO: &str = "/img/presentacion.jpg";
const LARGO_DESCRIPCION: usize = 160;
const POR_PAGINA_SITEMAP: i64 = 100;
/* Tope de lotes del sitemap (5 000 inmuebles): acota el tiempo de respuesta. */
const MAX_LOTES_SITEMAP: i64 = 50;
/* Etiquetas de la portada que se reemplazan por las del inmueble. El JSON-LD
 * de la organización (`RealEstateAgent`) se conserva. */
const CLAVES_PORTADA: [&str; 9] = [
    "rel=\"canonical\"",
    "name=\"description\"",
    "property=\"og:title\"",
    "property=\"og:description\"",
    "property=\"og:url\"",
    "property=\"og:image\"",
    "name=\"twitter:title\"",
    "name=\"twitter:description\"",
    "name=\"twitter:image\"",
];

/// Slug de una ruta `/inmueble/<slug>`; `None` si la ruta no tiene ese formato.
pub(super) fn slug_de_ruta(ruta: &str) -> Option<&str> {
    let slug = ruta.strip_prefix("/inmueble/")?.trim_end_matches('/');
    (!slug.is_empty() && !slug.contains('/')).then_some(slug)
}

/// `index.html` del SPA con las etiquetas del inmueble: 200 si está publicado,
/// 404 con el mismo HTML si no existe. Un fallo de base de datos sirve el SPA
/// sin etiquetas (queda registrado): el cliente pide el dato por la API.
pub(super) async fn responder(pool: &PgPool, index: &Path, slug: &str) -> Response {
    let Ok(html) = tokio::fs::read_to_string(index).await else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let (estado, cuerpo) = match InmuebleService::get_public(pool, slug).await {
        Ok(inmueble) => (StatusCode::OK, etiquetar(&html, &inmueble)),
        Err(AppError::NotFound(_)) => (StatusCode::NOT_FOUND, html),
        Err(e) => {
            tracing::warn!(slug, error = %e, "SEO de inmueble sin datos: se sirve el SPA sin etiquetas");
            (StatusCode::OK, html)
        }
    };
    (
        estado,
        [
            (header::CONTENT_TYPE, "text/html; charset=utf-8"),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        cuerpo,
    )
        .into_response()
}

/// Entradas `<url>` de los inmuebles publicados, en lotes de 100 con tope.
pub(super) async fn urls_sitemap(pool: &PgPool) -> Result<String, AppError> {
    let mut xml = String::new();
    let mut pagina: i64 = 1;
    loop {
        let filtros = FiltrosPublicos {
            tipo: None,
            operacion: None,
            precio_min: None,
            precio_max: None,
            page: pagina,
            per_page: POR_PAGINA_SITEMAP,
        };
        let lote = InmuebleService::list_public(pool, filtros).await?;
        for it in &lote.items {
            let loc = escapar(&format!("{ORIGEN}/inmueble/{}", it.slug));
            let fecha = it.updated_at.format("%Y-%m-%d");
            let _ = writeln!(xml, "<url><loc>{loc}</loc><lastmod>{fecha}</lastmod></url>");
        }
        if lote.items.is_empty() || pagina * POR_PAGINA_SITEMAP >= lote.total {
            break;
        }
        if pagina >= MAX_LOTES_SITEMAP {
            tracing::warn!(total = lote.total, "sitemap: inmuebles truncados por el tope de lotes");
            break;
        }
        pagina += 1;
    }
    Ok(xml)
}

/// `index.html` con las etiquetas de la portada sustituidas por las del inmueble.
fn etiquetar(html: &str, i: &Inmueble) -> String {
    let limpio = quitar_titulo(&quitar_etiquetas(html, &CLAVES_PORTADA));
    let Some(fin_head) = limpio.find("</head>") else {
        return html.to_string();
    };
    let url = format!("{ORIGEN}/inmueble/{}", i.slug);
    let titulo = escapar(&format!("{} - MN Inmobiliaria", i.titulo));
    let descripcion = escapar(&texto_descripcion(i));
    let imagen = escapar(&imagen_de(i));
    let url = escapar(&url);
    let ld = datos_estructurados(i, &url, &imagen, &descripcion);
    let bloque = format!(
        "<title>{titulo}</title>\n\
         <link rel=\"canonical\" href=\"{url}\" />\n\
         <meta name=\"description\" content=\"{descripcion}\" />\n\
         <meta property=\"og:title\" content=\"{titulo}\" />\n\
         <meta property=\"og:description\" content=\"{descripcion}\" />\n\
         <meta property=\"og:url\" content=\"{url}\" />\n\
         <meta property=\"og:image\" content=\"{imagen}\" />\n\
         <meta name=\"twitter:title\" content=\"{titulo}\" />\n\
         <meta name=\"twitter:description\" content=\"{descripcion}\" />\n\
         <meta name=\"twitter:image\" content=\"{imagen}\" />\n\
         <script id=\"datos-pagina\" type=\"application/ld+json\">{ld}</script>\n"
    );
    format!("{}{bloque}{}", &limpio[..fin_head], &limpio[fin_head..])
}

/// Quita cada `<meta>` o `<link>` de `html` cuyo texto contenga alguna de `claves`.
fn quitar_etiquetas(html: &str, claves: &[&str]) -> String {
    let mut salida = String::with_capacity(html.len());
    let mut resto = html;
    while let Some(pos) = resto.find('<') {
        salida.push_str(&resto[..pos]);
        let desde = &resto[pos..];
        let fin = desde.find('>').map_or(desde.len(), |f| f + 1);
        let etiqueta = &desde[..fin];
        let de_portada = (etiqueta.starts_with("<meta") || etiqueta.starts_with("<link"))
            && claves.iter().any(|c| etiqueta.contains(c));
        if !de_portada {
            salida.push_str(etiqueta);
        }
        resto = &desde[fin..];
    }
    salida.push_str(resto);
    salida
}

/// Quita el `<title>` de la portada (el del inmueble va en su lugar).
fn quitar_titulo(html: &str) -> String {
    let (Some(ini), Some(fin)) = (html.find("<title>"), html.find("</title>")) else {
        return html.to_string();
    };
    if fin < ini {
        return html.to_string();
    }
    format!("{}{}", &html[..ini], &html[fin + "</title>".len()..])
}

fn texto_descripcion(i: &Inmueble) -> String {
    let operacion = if i.operacion == "venta" { "venta" } else { "alquiler" };
    let ubicacion = match i.ubicacion.trim() {
        "" => "Puerto Ordaz",
        u => u,
    };
    let generado = format!(
        "{} en {operacion} en {ubicacion}. Contacto directo por WhatsApp.",
        etiqueta_tipo(&i.tipo)
    );
    let texto = match i.descripcion.trim() {
        "" => generado,
        d => d.to_string(),
    };
    if texto.chars().count() <= LARGO_DESCRIPCION {
        return texto;
    }
    let corto: String = texto.chars().take(LARGO_DESCRIPCION - 1).collect();
    format!("{}…", corto.trim_end())
}

/// `apartamento` -> `Apartamento`.
fn etiqueta_tipo(tipo: &str) -> String {
    let mut letras = tipo.chars();
    match letras.next() {
        Some(primera) => primera.to_uppercase().chain(letras).collect(),
        None => String::new(),
    }
}

/// Primera foto del inmueble como URL absoluta; sin fotos, la imagen de marca.
fn imagen_de(i: &Inmueble) -> String {
    match i.fotos.first().map(|f| f.url.as_str()) {
        Some(url) if url.starts_with("http") => url.to_string(),
        Some(url) if url.starts_with('/') => format!("{ORIGEN}{url}"),
        _ => format!("{ORIGEN}{IMAGEN_POR_DEFECTO}"),
    }
}

fn datos_estructurados(i: &Inmueble, url: &str, imagen: &str, descripcion: &str) -> String {
    let mut ld = serde_json::json!({
        "@context": "https://schema.org",
        "@type": "RealEstateListing",
        "name": i.titulo,
        "description": descripcion,
        "url": url,
        "image": imagen,
    });
    if i.precio > 0.0 {
        ld["offers"] = serde_json::json!({
            "@type": "Offer",
            "price": i.precio,
            "priceCurrency": "USD",
            "url": url,
        });
    }
    // Un `Value` siempre serializa. `</` se escapa para que ningún texto cierre el <script> antes de tiempo.
    serde_json::to_string(&ld)
        .unwrap_or_default()
        .replace("</", "<\\/")
}

/// Escapa texto para nodos y atributos HTML.
fn escapar(texto: &str) -> String {
    let mut salida = String::with_capacity(texto.len());
    for c in texto.chars() {
        match c {
            '&' => salida.push_str("&amp;"),
            '<' => salida.push_str("&lt;"),
            '>' => salida.push_str("&gt;"),
            '"' => salida.push_str("&quot;"),
            '\'' => salida.push_str("&#39;"),
            _ => salida.push(c),
        }
    }
    salida
}

#[cfg(test)]
mod pruebas {
    use super::{escapar, quitar_etiquetas, quitar_titulo, slug_de_ruta, CLAVES_PORTADA};

    #[test]
    fn slug_de_ruta_acepta_solo_un_segmento() {
        assert_eq!(slug_de_ruta("/inmueble/casa-1"), Some("casa-1"));
        assert_eq!(slug_de_ruta("/inmueble/casa-1/"), Some("casa-1"));
        assert_eq!(slug_de_ruta("/inmueble/"), None);
        assert_eq!(slug_de_ruta("/inmueble"), None);
        assert_eq!(slug_de_ruta("/inmueble/a/b"), None);
        assert_eq!(slug_de_ruta("/api/public/inmuebles/x"), None);
    }

    #[test]
    fn quitar_etiquetas_conserva_el_resto_de_la_cabecera() {
        let html = "<head><meta name=\"description\" content=\"x\" /><meta charset=\"UTF-8\" /><link rel=\"canonical\" href=\"y\" /></head>";
        assert_eq!(
            quitar_etiquetas(html, &CLAVES_PORTADA),
            "<head><meta charset=\"UTF-8\" /></head>"
        );
    }

    #[test]
    fn quitar_titulo_borra_el_de_la_portada() {
        assert_eq!(quitar_titulo("<head><title>A</title></head>"), "<head></head>");
    }

    #[test]
    fn escapar_neutraliza_comillas_y_etiquetas() {
        assert_eq!(escapar("a\"<b>&'"), "a&quot;&lt;b&gt;&amp;&#39;");
    }
}
