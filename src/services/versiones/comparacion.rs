//! [30AA-14] Revisión de todas las versiones de las fotos de un inmueble.
//!
//! Cruza las fotos por `orden` (la original, su mejorada y la variante de
//! cada versión) y mide cuánto se aparta cada una de su original con un
//! histograma en gris de 64 cubos: 0 % = idéntica, 100 % = nada que ver.
//!
//! Es la misma métrica para todos (tabla y revisión): a diferencia del SSIM de
//! `medir.py`, no castiga el desplazamiento de encuadre, que es justo lo que
//! las variantes buscan. Una variante demasiado fiel a su original se ve como
//! «poca diferencia», que es lo que hay que revisar a ojo.
//!
//! El histograma se calcula en una muestra de 256 puntos: medir 400 fotos no
//! puede costar más que servir las miniaturas que ya sirve el admin.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Serialize;
use sqlx::PgPool;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::url_publica_de_foto;
use crate::repositories::{InmuebleRepository, VersionRepository};

/// De dónde sale una foto: el `origen` de `fotos` o «variante».
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct VersionDeFoto {
    /// `original`, `mejorada` (v1) o `variante` (v2+).
    pub origen: String,
    /// 0 en la original y la mejorada; 2, 3… en las variantes.
    pub version: i16,
    pub url: String,
    /// Distancia de su histograma al de la original (`None` si no hay original
    /// que comparar, o si la foto no se pudo leer).
    pub diferencia_pct: Option<f32>,
}

/// Todas las versiones de una foto: la original primero.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FotoComparada {
    pub orden: i32,
    pub versiones: Vec<VersionDeFoto>,
}

/// Revisión completa de un inmueble.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RevisionFotos {
    pub inmueble_id: Uuid,
    pub titulo: String,
    pub fotos: Vec<FotoComparada>,
}

/// Clave del archivo en disco + de dónde sale (no necesita `orden`: ya es la
/// clave del mapa).
struct ClaveFoto {
    storage_key: String,
    origen: String,
}

pub struct RevisionService;

impl RevisionService {
    pub async fn comparar(pool: &PgPool, upload: &Path, id: Uuid) -> Result<RevisionFotos, AppError> {
        let inmueble = InmuebleRepository::find_by_id(pool, id)
            .await?
            .ok_or_else(|| AppError::NotFound("Inmueble no encontrado".into()))?;
        let fotos = InmuebleRepository::fotos_por_inmuebles(pool, &[id])
            .await?
            .remove(&id)
            .unwrap_or_default();
        let variantes = VersionRepository::fotos_de(pool, id).await?;

        /* Una sola entrada por `orden`: una mejorada por hueco y una variante
         * por (versión, hueco). Su clave la da la BD, sin repetidos. */
        let mut por_orden: BTreeMap<i32, Vec<ClaveFoto>> = BTreeMap::new();
        for f in fotos {
            por_orden.entry(f.orden).or_default().push(ClaveFoto {
                storage_key: f.storage_key,
                origen: f.origen,
            });
        }
        for f in variantes {
            por_orden.entry(f.orden).or_default().push(ClaveFoto {
                storage_key: f.storage_key,
                origen: formato_variante(f.version),
            });
        }

        let mut grupos = Vec::with_capacity(por_orden.len());
        for (orden, claves) in por_orden {
            /* La original manda; sin ella no hay contra qué medir. */
            let base = claves.iter().find(|c| c.origen == "original");
            let base_hist = base
                .and_then(|c| leer_histograma(&upload.join(&c.storage_key)));
            let mut versiones: Vec<VersionDeFoto> = claves
                .iter()
                .map(|c| VersionDeFoto {
                    origen: c.origen.clone(),
                    version: version_de_origen(&c.origen),
                    url: url_publica_de_foto(&c.storage_key),
                    diferencia_pct: if c.origen == "original" {
                        None
                    } else {
                        base_hist.as_ref().and_then(|b| {
                            leer_histograma(&upload.join(&c.storage_key)).map(|h| distancia_pct(b, &h))
                        })
                    },
                })
                .collect();
            /* Original, mejorada y después v2, v3…: el orden de lectura. Por
             * `version` sola la mejorada (0) empataría con la original y el
             * desempate alfabético pondría «mejorada» delante. */
            versiones.sort_by_key(|v| (rango_de_origen(&v.origen), v.version));
            grupos.push(FotoComparada { orden, versiones });
        }

        Ok(RevisionFotos {
            inmueble_id: inmueble.id,
            titulo: inmueble.titulo,
            fotos: grupos,
        })
    }
}

/// La variante se guarda con `origen = "variante:<n>"` para no perder de qué
/// versión es al aplanar fotos y variantes en la misma lista.
fn formato_variante(version: i16) -> String {
    format!("variante:{version}")
}

fn version_de_origen(origen: &str) -> i16 {
    origen
        .strip_prefix("variante:")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}

/// Puesto en la columna: original (0), mejorada (1), variantes (2+). La
/// mejorada es la mejora de la original en la misma versión, así que va justo
/// después de ella.
fn rango_de_origen(origen: &str) -> u8 {
    match origen {
        "original" => 0,
        "mejorada" => 1,
        _ => 2,
    }
}

/// Histograma en gris de 64 cubos, sobre una muestra fija de puntos. `None`
/// si el archivo no está o no decodifica: una foto rota no cae la revisión.
fn leer_histograma(ruta: &Path) -> Option<[f32; 64]> {
    let img = image::open(ruta).ok()?.to_luma8();
    let (ancho, alto) = img.dimensions();
    let total = u64::from(ancho) * u64::from(alto);
    /* Salto constante por el rayo de píxeles: cubre alto y ancho sin guardar
     * una lista de muestras. Nunca 0 (una foto de 1 píxel repetiría). */
    let paso = (total / 256).max(1);
    let mut hist = [0f32; 64];
    for i in 0..256u64 {
        let plano = (i * paso) % total;
        let (x, y) = ((plano % u64::from(ancho)) as u32, (plano / u64::from(ancho)) as u32);
        hist[usize::from(img.get_pixel(x, y).0[0]) / 4] += 1.0;
    }
    for cubo in &mut hist {
        *cubo /= 256.0;
    }
    Some(hist)
}

/// Distancia de dos histogramas normalizados, en %: media de las diferencias
/// absolutas por cubo (0 = misma distribución de grises, 50 = opuestas).
fn distancia_pct(a: &[f32; 64], b: &[f32; 64]) -> f32 {
    let suma: f32 = a.iter().zip(b.iter()).map(|(x, y)| (x - y).abs()).sum();
    (100.0 * suma / 2.0).clamp(0.0, 100.0)
}

#[cfg(test)]
mod pruebas {
    use super::{distancia_pct, formato_variante, rango_de_origen, version_de_origen, VersionDeFoto};

    fn hist(cubo: usize, valor: f32) -> [f32; 64] {
        let mut h = [0.0; 64];
        h[cubo] = valor;
        h
    }

    #[test]
    fn la_misma_distribucion_no_tiene_diferencia() {
        assert!(distancia_pct(&hist(10, 1.0), &hist(10, 1.0)) < 0.01);
    }

    #[test]
    fn distribuciones_opuestas_llegan_al_maximo() {
        assert!((distancia_pct(&hist(0, 1.0), &hist(63, 1.0)) - 100.0).abs() < 0.01);
    }

    #[test]
    fn la_mitad_de_la_masa_movida_es_50() {
        let a = hist(0, 1.0);
        let mut b = hist(0, 0.5);
        b[1] = 0.5;
        assert!((distancia_pct(&a, &b) - 50.0).abs() < 0.01);
    }

    /// La original va primero y su mejorada justo detrás: ordenar solo por
    /// `version` empataba las dos a 0 y el desempate alfabético ponía la
    /// mejorada delante (visto en el modal, 2026-11-16).
    #[test]
    fn la_original_manda_en_la_columna() {
        assert!(rango_de_origen("original") < rango_de_origen("mejorada"));
        assert!(rango_de_origen("mejorada") < rango_de_origen(&formato_variante(2)));
    }

    #[test]
    fn la_variante_recuerda_su_version() {
        assert_eq!(version_de_origen(&formato_variante(3)), 3);
        assert_eq!(version_de_origen("original"), 0);
        assert_eq!(version_de_origen("variante:x"), 0);
    }

    #[test]
    fn una_version_serializa_su_origen() {
        /* El front distingue «variante:2» y pinta la etiqueta «v2». */
        let v = VersionDeFoto {
            origen: formato_variante(2),
            version: 2,
            url: "/uploads/x.jpg".into(),
            diferencia_pct: Some(12.5),
        };
        assert_eq!(v.origen, "variante:2");
    }
}
