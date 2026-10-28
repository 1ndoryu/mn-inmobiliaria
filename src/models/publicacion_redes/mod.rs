use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

/* [28AA-1] Calendario de publicaciones en redes: un inmueble publicado en uno
 * o varios días, con piezas propias (imágenes) y el checklist de destinos.
 * Las reglas de cola y de estado viven en el repositorio y el servicio; aquí
 * solo están los tipos, las constantes y el detector de formato. */

/// Estados del ciclo de una publicación (CHECK en BD)
pub const ESTADOS_PUBLICACION: &[&str] = &["borrador", "lista", "publicada", "saltada"];

/// Destinos del checklist [D1]. Mismo orden que las columnas de la BD.
pub const DESTINOS: &[&str] = &["ig_post", "fanpage", "ig_historia", "fb_perfil"];

/// Formatos de pieza que se detectan por proporción (CHECK en BD)
pub const FORMATOS_PIEZA: &[&str] = &["1:1", "3:4", "4:5", "9:16"];

/// Límite de la descripción (CHECK en BD); el texto de redes no pasa de esto
pub const MAX_DESCRIPCION: usize = 2200;
/// Número máximo de tags por publicación
pub const MAX_TAGS: usize = 30;
/// Longitud máxima de un tag, sin la almohadilla
pub const MAX_TAG_LEN: usize = 60;

/// Fila de `publicaciones_redes` tal cual la devuelve Postgres
#[derive(Debug, Clone, FromRow)]
pub struct PublicacionRow {
    pub id: Uuid,
    pub inmueble_id: Uuid,
    pub estado: String,
    pub descripcion: String,
    pub tags: Vec<String>,
    pub prompt_usado: String,
    pub modelo: String,
    pub ig_post: bool,
    pub fanpage: bool,
    pub ig_historia: bool,
    pub fb_perfil: bool,
    pub programada_para: Option<NaiveDate>,
    pub publicada_en: Option<DateTime<Utc>>,
    pub creada_en: DateTime<Utc>,
}

/// Fila de `publicacion_piezas`
#[derive(Debug, Clone, FromRow)]
pub struct PiezaRow {
    pub id: Uuid,
    pub publicacion_id: Uuid,
    pub formato: String,
    pub storage_key: String,
    pub ancho: i32,
    pub alto: i32,
    pub bytes: i32,
    pub creada_en: DateTime<Utc>,
}

/// Pieza tal como la expone la API admin (la ruta pública sale de `storage_key`)
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Pieza {
    pub id: Uuid,
    pub formato: String,
    pub url: String,
    pub ancho: i32,
    pub alto: i32,
    pub bytes: i32,
}

impl Pieza {
    #[must_use]
    pub fn from_row(row: PiezaRow) -> Self {
        Self {
            id: row.id,
            formato: row.formato,
            url: format!("/uploads/{}", row.storage_key),
            ancho: row.ancho,
            alto: row.alto,
            bytes: row.bytes,
        }
    }
}

/// Publicación con sus piezas, lista para el calendario y el historial
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Publicacion {
    pub id: Uuid,
    pub inmueble_id: Uuid,
    pub titulo_inmueble: String,
    pub estado: String,
    pub descripcion: String,
    pub tags: Vec<String>,
    pub modelo: String,
    pub destinos: Destinos,
    pub programada_para: Option<NaiveDate>,
    pub publicada_en: Option<DateTime<Utc>>,
    pub piezas: Vec<Pieza>,
    pub creada_en: DateTime<Utc>,
}

/// Checklist de destinos [D1]: cada casilla es un sí/no independiente
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, ToSchema)]
pub struct Destinos {
    pub ig_post: bool,
    pub fanpage: bool,
    pub ig_historia: bool,
    pub fb_perfil: bool,
}

impl Destinos {
    /// `true` cuando los cuatro destinos están marcados: la publicación pasa a `publicada`
    #[must_use]
    pub fn completos(&self) -> bool {
        self.ig_post && self.fanpage && self.ig_historia && self.fb_perfil
    }
}

impl From<&PublicacionRow> for Destinos {
    fn from(p: &PublicacionRow) -> Self {
        Self {
            ig_post: p.ig_post,
            fanpage: p.fanpage,
            ig_historia: p.ig_historia,
            fb_perfil: p.fb_perfil,
        }
    }
}

impl Publicacion {
    /// Ensambla la vista con el título y las piezas ya resueltos por el handler
    #[must_use]
    pub fn from_row(row: PublicacionRow, titulo_inmueble: String, piezas: Vec<PiezaRow>) -> Self {
        Self {
            destinos: Destinos::from(&row),
            id: row.id,
            inmueble_id: row.inmueble_id,
            titulo_inmueble,
            estado: row.estado,
            descripcion: row.descripcion,
            tags: row.tags,
            modelo: row.modelo,
            programada_para: row.programada_para,
            publicada_en: row.publicada_en,
            piezas: piezas.into_iter().map(Pieza::from_row).collect(),
            creada_en: row.creada_en,
        }
    }
}

/// Pide la siguiente propuesta; `excluir` evita repetir la que se acaba de saltar
#[derive(Debug, Deserialize, ToSchema)]
pub struct ProponerRequest {
    pub excluir: Option<Uuid>,
}

/// Inmueble que toca en el calendario, para elegir o saltar
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Propuesta {
    pub inmueble_id: Uuid,
    pub titulo: String,
}

/// Cuerpo para crear un borrador de publicación de un inmueble propuesto
#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct CrearPublicacionRequest {
    pub inmueble_id: Uuid,
    /// Día previsto en el calendario; opcional, se puede fijar luego
    pub programada_para: Option<NaiveDate>,
}

/// Cuerpo para saltar la propuesta de un inmueble (no cuenta como publicado)
#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct SaltarPropuestaRequest {
    pub inmueble_id: Uuid,
}

/// Edición del texto, los tags, el día y el checklist. Cada campo es opcional:
/// solo se cambia lo que llega.
#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct ActualizarPublicacionRequest {
    #[validate(length(max = 2200))]
    pub descripcion: Option<String>,
    pub tags: Option<Vec<String>>,
    pub programada_para: Option<NaiveDate>,
    pub destinos: Option<Destinos>,
}

/// Detecta el formato de una pieza por su proporción ancho/alto. Acepta un
/// margen de 2 % para no rechazar capturas casi cuadradas o recortadas a mano.
#[must_use]
pub fn formato_de_proporcion(ancho: u32, alto: u32) -> Option<&'static str> {
    if ancho == 0 || alto == 0 {
        return None;
    }
    let r = f64::from(ancho) / f64::from(alto);
    FORMATOS_PIEZA
        .iter()
        .find(|f| {
            let (a, b) = f.split_once(':').unwrap_or(("1", "1"));
            let objetivo: f64 = a.parse::<f64>().unwrap_or(1.0) / b.parse::<f64>().unwrap_or(1.0);
            (r - objetivo).abs() / objetivo <= 0.02
        })
        .copied()
}

/// Normaliza los tags: sin almohadilla, minúsculas, sin duplicados ni vacíos
#[must_use]
pub fn normalizar_tags(entrada: &[String]) -> Vec<String> {
    let mut salida: Vec<String> = Vec::new();
    for t in entrada {
        let limpio: String = t
            .trim()
            .trim_start_matches('#')
            .chars()
            .filter(|c| !c.is_whitespace())
            .take(MAX_TAG_LEN)
            .collect::<String>()
            .to_lowercase();
        if !limpio.is_empty() && !salida.contains(&limpio) && salida.len() < MAX_TAGS {
            salida.push(limpio);
        }
    }
    salida
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detecta_formatos_por_proporcion() {
        assert_eq!(formato_de_proporcion(1080, 1080), Some("1:1"));
        assert_eq!(formato_de_proporcion(1080, 1350), Some("4:5"));
        assert_eq!(formato_de_proporcion(1080, 1440), Some("3:4"));
        assert_eq!(formato_de_proporcion(1080, 1920), Some("9:16"));
        assert_eq!(formato_de_proporcion(1600, 900), None);
        assert_eq!(formato_de_proporcion(0, 10), None);
    }

    #[test]
    fn normaliza_tags() {
        let entrada = vec![
            " #Casa ".to_string(),
            "casa".to_string(),
            "Playa del Este".to_string(),
            "   ".to_string(),
        ];
        assert_eq!(normalizar_tags(&entrada), vec!["casa", "playadeleste"]);
    }

    #[test]
    fn destinos_completos_solo_con_los_cuatro() {
        let mut d = Destinos::default();
        assert!(!d.completos());
        d = Destinos { ig_post: true, fanpage: true, ig_historia: true, fb_perfil: true };
        assert!(d.completos());
    }
}
