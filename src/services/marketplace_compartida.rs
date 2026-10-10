//! [09AA-30 F2] Caché compartida por inmueble.
//!
//! Dos compradores que escriben el mismo mensaje sobre la misma ficha (mismo
//! precio citado) reciben la misma respuesta sin una llamada nueva a la IA.
//! La clave es `catalog_hash` + `precio_hash` + `mensaje_clave` (los 1-2
//! mensajes del cliente normalizados). Solo entran hilos con ficha conocida y
//! con nombre: el nombre se guarda como `{{nombre}}` y se rellena al servir.
//!
//! La fila por hilo (`mp_respuestas_cache`) guarda la clave en
//! `mensaje_clave`; así una corrección o regeneración de un hilo llega al
//! resto del inmueble. Módulo aparte para no engordar `marketplace.rs`.

use super::marketplace::{clave_hilo, sha_hex, Coste, FotoHilo};
use crate::errors::AppError;

/// Marcador del nombre del comprador dentro de la respuesta compartida.
pub const MARCADOR_NOMBRE: &str = "{{nombre}}";

/// Clave de la caché compartida (sin la firma del hilo).
pub struct ClaveCompartida<'a> {
    pub catalog_hash: &'a str,
    pub precio_hash: &'a str,
    pub mensaje_clave: &'a str,
}

/// Clave de los mensajes del cliente: cada uno normalizado (espacios
/// colapsados, minúsculas) en su línea, con versión. La línea separa los
/// mensajes: `["a b"]` y `["a", "b"]` no colisionan.
#[must_use]
pub fn mensaje_clave_de(textos: &[&str]) -> String {
    let mut canon = String::from("v1\n");
    for (i, texto) in textos.iter().enumerate() {
        if i > 0 {
            canon.push('\n');
        }
        canon.push_str(&texto.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase());
    }
    sha_hex(&canon)
}

/// Minúscula de un `char` (la primera si mapea a varias): mantiene la
/// correspondencia 1:1 de índices con el texto original.
fn bajar(c: char) -> char {
    c.to_lowercase().next().unwrap_or(c)
}

fn es_palabra(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Inicios (en `char`s) donde `nombre` aparece como palabra en `texto`, sin
/// distinguir mayúsculas. Ambos ya vienen en minúsculas.
fn inicios_palabra(texto: &[char], nombre: &[char]) -> Vec<usize> {
    if nombre.is_empty() || nombre.len() > texto.len() {
        return Vec::new();
    }
    let fin = nombre.len();
    (0..=texto.len() - fin)
        .filter(|&i| {
            let coincide = nombre.iter().enumerate().all(|(j, &c)| bajar(texto[i + j]) == c);
            let antes = i == 0 || !es_palabra(texto[i - 1]);
            let despues = i + fin == texto.len() || !es_palabra(texto[i + fin]);
            coincide && antes && despues
        })
        .collect()
}

/// Nombre del hilo en minúsculas, listo para comparar.
fn nombre_bajado(nombre: &str) -> Vec<char> {
    nombre.trim().chars().map(bajar).collect()
}

/// Cuántas veces aparece `nombre` como palabra en `texto`. Más de una vez es
/// ambiguo (el nombre puede ser una palabra común): ese texto no se comparte.
#[must_use]
pub fn ocurrencias_nombre(texto: &str, nombre: &str) -> usize {
    let bajo: Vec<char> = texto.chars().map(bajar).collect();
    inicios_palabra(&bajo, &nombre_bajado(nombre)).len()
}

/// Reemplaza el nombre (como palabra, sin mayúsculas) por `{{nombre}}` y deja
/// el resto del texto tal cual.
#[must_use]
pub fn plantilla_de_nombre(texto: &str, nombre: &str) -> String {
    let orig: Vec<char> = texto.chars().collect();
    let bajo: Vec<char> = orig.iter().map(|&c| bajar(c)).collect();
    let nombre = nombre_bajado(nombre);
    let mut salida = String::new();
    let mut desde = 0;
    for inicio in inicios_palabra(&bajo, &nombre) {
        salida.extend(orig[desde..inicio].iter());
        salida.push_str(MARCADOR_NOMBRE);
        desde = inicio + nombre.len();
    }
    salida.extend(orig[desde..].iter());
    salida
}

/// Rellena `{{nombre}}` con el nombre del hilo. `None` si la plantilla lo pide
/// y el hilo no tiene nombre: no se sirve y el hilo genera su propia respuesta.
#[must_use]
pub fn rellenar_nombre(plantilla: &str, nombre: Option<&str>) -> Option<String> {
    if !plantilla.contains(MARCADOR_NOMBRE) {
        return Some(plantilla.to_string());
    }
    nombre
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map(|n| plantilla.replace(MARCADOR_NOMBRE, n))
}

/// Hit compartido: `UPDATE` atómico que cuenta el uso (como `buscar_cache`).
pub async fn buscar_compartida(
    pool: &sqlx::PgPool,
    clave: &ClaveCompartida<'_>,
) -> Result<Option<(String, bool)>, AppError> {
    let fila: Option<(String, bool)> = sqlx::query_as(
        "UPDATE mp_respuestas_inmueble SET usos = usos + 1 \
         WHERE catalog_hash = $1 AND precio_hash = $2 AND mensaje_clave = $3 \
         AND valida_hasta > now() \
         RETURNING respuesta, corregida",
    )
    .bind(clave.catalog_hash)
    .bind(clave.precio_hash)
    .bind(clave.mensaje_clave)
    .fetch_optional(pool)
    .await?;
    Ok(fila)
}

/// Publica la respuesta del hilo como compartida y enlaza su fila
/// (`mp_respuestas_cache.mensaje_clave`) en una transacción.
/// `pisar=false` (alta normal): respeta una compartida que ya exista (puede ser
/// corrección de la dueña). `pisar=true` (regenerar explícito): la reemplaza
/// para todo el inmueble.
pub async fn enlazar_compartida(
    pool: &sqlx::PgPool,
    firma: &str,
    clave: &ClaveCompartida<'_>,
    plantilla: &str,
    coste: Coste,
    pisar: bool,
) -> Result<(), AppError> {
    const ALTA: &str = "INSERT INTO mp_respuestas_inmueble \
         (catalog_hash, precio_hash, mensaje_clave, respuesta, origen, tokens_entrada, tokens_salida, ms_generacion) \
         VALUES ($1, $2, $3, $4, 'ia', $5, $6, $7) \
         ON CONFLICT (catalog_hash, precio_hash, mensaje_clave) DO NOTHING";
    const PISAR: &str = "INSERT INTO mp_respuestas_inmueble \
         (catalog_hash, precio_hash, mensaje_clave, respuesta, origen, tokens_entrada, tokens_salida, ms_generacion) \
         VALUES ($1, $2, $3, $4, 'ia', $5, $6, $7) \
         ON CONFLICT (catalog_hash, precio_hash, mensaje_clave) DO UPDATE SET \
         respuesta = EXCLUDED.respuesta, valida_hasta = now() + INTERVAL '90 days', \
         corregida = FALSE, usos = 0, origen = EXCLUDED.origen, \
         tokens_entrada = EXCLUDED.tokens_entrada, tokens_salida = EXCLUDED.tokens_salida, \
         ms_generacion = EXCLUDED.ms_generacion";
    let mut tx = pool.begin().await?;
    let alta = if pisar { PISAR } else { ALTA };
    sqlx::query(alta)
        .bind(clave.catalog_hash)
        .bind(clave.precio_hash)
        .bind(clave.mensaje_clave)
        .bind(plantilla)
        .bind(coste.tokens_entrada)
        .bind(coste.tokens_salida)
        .bind(coste.ms)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE mp_respuestas_cache SET mensaje_clave = $4 \
         WHERE firma = $1 AND precio_hash = $2 AND catalog_hash = $3",
    )
    .bind(firma)
    .bind(clave.precio_hash)
    .bind(clave.catalog_hash)
    .bind(clave.mensaje_clave)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

/// Hilo enlazado a una clave compartida: `(thread_id, mensaje_clave)` de su
/// fila por hilo. `None` si nunca se compartió (sin enlace, sin propagación).
pub async fn vinculo_compartido(
    pool: &sqlx::PgPool,
    firma: &str,
    precio_hash: &str,
    catalog_hash: &str,
) -> Result<Option<(String, String)>, AppError> {
    let fila: Option<(String, String)> = sqlx::query_as(
        "SELECT thread_id, mensaje_clave FROM mp_respuestas_cache \
         WHERE firma = $1 AND precio_hash = $2 AND catalog_hash = $3 \
         AND mensaje_clave IS NOT NULL AND thread_id IS NOT NULL",
    )
    .bind(firma)
    .bind(precio_hash)
    .bind(catalog_hash)
    .fetch_optional(pool)
    .await?;
    Ok(fila)
}

/// Hilo que recibe una respuesta compartida ya servida (acierto de la clave):
/// su fila por hilo copia origen y coste de la compartida y queda enlazada,
/// así una corrección posterior le llega también. Si la fila ya existía solo
/// se enlaza; su texto puede ser anterior, pero la lectura compartida manda.
pub async fn vincular_hilo_compartido(
    pool: &sqlx::PgPool,
    firma: &str,
    clave: &ClaveCompartida<'_>,
    texto: &str,
    corregida: bool,
    foto: &FotoHilo<'_>,
) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        "INSERT INTO mp_respuestas_cache (firma, precio_hash, catalog_hash, respuesta, thread_id, excerpt_texto, excerpt_crudo, origen, tokens_entrada, tokens_salida, ms_generacion, corregida, mensaje_clave) \
         SELECT $1, $2, $3, $4, $5, $6, $7, origen, tokens_entrada, tokens_salida, ms_generacion, $8, $9 \
         FROM mp_respuestas_inmueble \
         WHERE catalog_hash = $3 AND precio_hash = $2 AND mensaje_clave = $9 \
         ON CONFLICT (firma, precio_hash, catalog_hash) DO NOTHING",
    )
    .bind(firma)
    .bind(clave.precio_hash)
    .bind(clave.catalog_hash)
    .bind(texto)
    .bind(clave_hilo(foto.thread_id))
    .bind(foto.excerpt)
    .bind(foto.excerpt_crudo)
    .bind(corregida)
    .bind(clave.mensaje_clave)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE mp_respuestas_cache SET mensaje_clave = $4 \
         WHERE firma = $1 AND precio_hash = $2 AND catalog_hash = $3",
    )
    .bind(firma)
    .bind(clave.precio_hash)
    .bind(clave.catalog_hash)
    .bind(clave.mensaje_clave)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

/// Corrección de la dueña que llega a todo el inmueble: `corregida=TRUE`,
/// vigencia renovada y contador a cero, igual que `corregir_cache`.
pub async fn corregir_compartida(
    pool: &sqlx::PgPool,
    clave: &ClaveCompartida<'_>,
    plantilla: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO mp_respuestas_inmueble (catalog_hash, precio_hash, mensaje_clave, respuesta, corregida) \
         VALUES ($1, $2, $3, $4, TRUE) \
         ON CONFLICT (catalog_hash, precio_hash, mensaje_clave) DO UPDATE SET \
         respuesta = EXCLUDED.respuesta, valida_hasta = now() + INTERVAL '90 days', \
         corregida = TRUE, usos = 0",
    )
    .bind(clave.catalog_hash)
    .bind(clave.precio_hash)
    .bind(clave.mensaje_clave)
    .bind(plantilla)
    .execute(pool)
    .await?;
    Ok(())
}

/// Purga las compartidas vencidas; la llama `purgar_cache`.
pub async fn purgar_compartida(pool: &sqlx::PgPool) -> Result<u64, sqlx::Error> {
    let r = sqlx::query("DELETE FROM mp_respuestas_inmueble WHERE valida_hasta <= now()")
        .execute(pool)
        .await?;
    Ok(r.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mensaje_clave_normaliza_espacios_y_mayusculas() {
        assert_eq!(
            mensaje_clave_de(&["Hola  ¿Sigue   disponible?"]),
            mensaje_clave_de(&["hola ¿sigue disponible?"])
        );
    }

    #[test]
    fn mensaje_clave_distingue_uno_de_dos_mensajes() {
        assert_ne!(mensaje_clave_de(&["a b"]), mensaje_clave_de(&["a", "b"]));
        assert_ne!(mensaje_clave_de(&["a", "b"]), mensaje_clave_de(&["b", "a"]));
    }

    #[test]
    fn mensaje_clave_es_hex64() {
        let k = mensaje_clave_de(&["hola"]);
        assert_eq!(k.len(), 64);
        assert!(k.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn ocurrencias_cuentan_solo_palabras_completas() {
        assert_eq!(ocurrencias_nombre("Hola Ana, ¿cómo estás?", "Ana"), 1);
        assert_eq!(ocurrencias_nombre("hola ANA y ana", "Ana"), 2);
        assert_eq!(ocurrencias_nombre("Hola Anabel", "Ana"), 0);
        assert_eq!(ocurrencias_nombre("Hola", ""), 0);
    }

    #[test]
    fn plantilla_sustituye_el_nombre_y_conserva_el_resto() {
        assert_eq!(
            plantilla_de_nombre("Hola Ana, sí está disponible.", "Ana"),
            "Hola {{nombre}}, sí está disponible."
        );
        assert_eq!(
            plantilla_de_nombre("hola ana", "Ana"),
            "hola {{nombre}}"
        );
        assert_eq!(plantilla_de_nombre("Hola Anabel", "Ana"), "Hola Anabel");
    }

    #[test]
    fn rellenar_sirve_plantilla_sin_nombre_y_bloquea_sin_nombre() {
        assert_eq!(
            rellenar_nombre("Hola {{nombre}}!", Some("Ana")).as_deref(),
            Some("Hola Ana!")
        );
        assert_eq!(rellenar_nombre("Hola {{nombre}}!", None), None);
        assert_eq!(rellenar_nombre("Hola {{nombre}}!", Some("  ")), None);
        assert_eq!(
            rellenar_nombre("Sin nombre aquí.", None).as_deref(),
            Some("Sin nombre aquí.")
        );
    }

    fn pool_si_hay() -> Option<sqlx::PgPool> {
        let url = std::env::var("DATABASE_URL").ok()?;
        sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy(&url)
            .ok()
    }

    fn clave_azar() -> String {
        format!("{:x}", uuid::Uuid::new_v4().as_simple())
    }

    /* Contra BD viva: enlaza dos hilos del mismo inmueble, corrige desde uno y
     * comprueba la propagación. Sin `DATABASE_URL` se omite. */
    #[tokio::test]
    async fn compartida_enlaza_y_propaga_correccion() {
        let Some(pool) = pool_si_hay() else { return };
        let catalogo = clave_azar();
        let precio = clave_azar();
        let mensaje = mensaje_clave_de(&["¿sigue disponible?"]);
        let clave = ClaveCompartida {
            catalog_hash: &catalogo,
            precio_hash: &precio,
            mensaje_clave: &mensaje,
        };
        let firma_a = clave_azar();
        let foto = super::super::marketplace::FotoHilo {
            thread_id: "Ana|Casa",
            excerpt: "",
            excerpt_crudo: "",
        };
        super::super::marketplace::guardar_cache(
            &pool, &firma_a, &precio, &catalogo, "Hola {{nombre}}.", &foto, Coste::default(),
        )
        .await
        .expect("guardar");
        enlazar_compartida(&pool, &firma_a, &clave, "Hola {{nombre}}.", Coste::default(), false)
            .await
            .expect("enlazar");
        let hit = buscar_compartida(&pool, &clave).await.expect("buscar");
        assert_eq!(hit, Some(("Hola {{nombre}}.".to_string(), false)));
        let vinculo = vinculo_compartido(&pool, &firma_a, &precio, &catalogo)
            .await
            .expect("vinculo");
        assert_eq!(vinculo, Some(("Ana|Casa".to_string(), mensaje.clone())));
        corregir_compartida(&pool, &clave, "Hola {{nombre}}, sí.")
            .await
            .expect("corregir");
        let tras = buscar_compartida(&pool, &clave).await.expect("buscar");
        assert_eq!(tras, Some(("Hola {{nombre}}, sí.".to_string(), true)));
        let firma_b = clave_azar();
        let foto_b = FotoHilo {
            thread_id: "Beto|Casa",
            excerpt: "",
            excerpt_crudo: "",
        };
        vincular_hilo_compartido(&pool, &firma_b, &clave, "Hola Beto, sí.", true, &foto_b)
            .await
            .expect("vincular");
        let vinculo_b = vinculo_compartido(&pool, &firma_b, &precio, &catalogo)
            .await
            .expect("vinculo b");
        assert_eq!(vinculo_b, Some(("Beto|Casa".to_string(), mensaje.clone())));
        sqlx::query("DELETE FROM mp_respuestas_inmueble WHERE catalog_hash = $1")
            .bind(&catalogo)
            .execute(&pool)
            .await
            .expect("limpiar compartida");
        super::super::marketplace::borrar_cache(&pool, &firma_a, &precio, &catalogo)
            .await
            .expect("limpiar hilo");
        super::super::marketplace::borrar_cache(&pool, &firma_b, &precio, &catalogo)
            .await
            .expect("limpiar hilo b");
    }
}
