//! [30AA-1] Manifiesto de exportación para la app de escritorio.
//!
//! No se genera un ZIP: la app ya proxya y cachea `/uploads/*`, así que basta
//! con decirle qué archivos hay y dónde van. Ventaja: sin temporales en el
//! servidor, reanudable y reusa la caché existente.
//!
//! Estructura en disco (la decide el escritorio, se documenta aquí):
//!   inmuebles/<cuenta>/v1/<slug>/01.jpg … + descripcion.txt
//!   inmuebles/<cuenta>/v2/<slug>/01.jpg … + descripcion.txt

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::url_publica_de_foto;
use crate::repositories::{FacebookRepository, InmuebleRepository, VersionRepository};

/// Una foto del manifiesto
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FotoExport {
    /// Posición dentro de la carpeta del inmueble (`01`, `02`…).
    pub orden: i32,
    pub url: String,
    /// `original` o `mejorada` (lo que la ficha muestra hoy).
    pub origen: String,
}

/// Un inmueble exportable
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct InmuebleExport {
    pub slug: String,
    pub titulo: String,
    pub descripcion: String,
    pub fotos: Vec<FotoExport>,
    /// [30AA-13] Publicado en la cuenta dueña de ESTA versión (v1 = Maye,
    /// v2 = Wan): tiene fila en `inmueble_facebook` con esa cuenta y el ID del
    /// aviso puesto. Vincular sin ID no cuenta: el aviso todavía no existe en
    /// Marketplace. El escritorio lo usa para separar `publicados/` de
    /// `pendientes/`.
    pub publicado: bool,
}

/// Un grupo de carpetas: v1 (ficha pública) o v2 (variante)
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GrupoExport {
    pub version: i16,
    /// Texto de `descripcion.txt` por inmueble; en v2 es la variante.
    pub inmuebles: Vec<InmuebleExport>,
}

/// Manifiesto completo de una cuenta
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ManifiestoExport {
    pub cuenta: Uuid,
    pub cuenta_nombre: String,
    pub generado_en: DateTime<Utc>,
    /// Solo las versiones con contenido: v1 siempre, y cada v2, v3… que tenga
    /// fotos variantes o texto.
    pub grupos: Vec<GrupoExport>,
}

/// Fila mínima de fotos para el manifiesto
#[derive(Debug, sqlx::FromRow)]
struct FotoManifiesto {
    inmueble_id: Uuid,
    storage_key: String,
    orden: i32,
    origen: String,
}

pub struct ExportService;

impl ExportService {
    pub async fn manifiesto(
        pool: &PgPool,
        facebook_id: Option<Uuid>,
        solo_version: Option<i16>,
    ) -> Result<ManifiestoExport, AppError> {
        /* Sin cuenta indicada se usa la principal: es la que tiene los avisos
         * históricos y lo que la dueña espera al exportar «todo». */
        let cuenta = match facebook_id {
            Some(id) => FacebookRepository::find_by_id(pool, id)
                .await?
                .ok_or_else(|| AppError::NotFound("Cuenta de Facebook no encontrada".into()))?,
            None => FacebookRepository::principal(pool)
                .await?
                .ok_or_else(|| AppError::NotFound("No hay cuenta de Facebook principal".into()))?,
        };

        /* [30AA-15] El inmueble sin cuenta solo lo ve la cuenta principal:
         * sigue siendo una ficha con su aviso pendiente (filtrarlo lo hacía
         * desaparecer en vez de caer en `pendientes`), pero si lo vieran todas
         * saldría repetido en cada carpeta. */
        let inmuebles = InmuebleRepository::por_facebook(pool, cuenta.id, cuenta.principal).await?;
        let ids: Vec<Uuid> = inmuebles.iter().map(|i| i.id).collect();
        let fotos = fotos_de(pool, &ids).await?;
        /* [30AA-13] Cada versión se publica en UNA cuenta (v1 = Maye, v2 = Wan):
         * «publicado» se mide en la cuenta dueña de esa versión, no en la
         * cuenta que pidió el export. Dos consultas para todo el manifiesto. */
        let dueno = FacebookRepository::cuentas_por_version(pool).await?;
        let avisos = FacebookRepository::avisos_en_la_cuenta_de_su_version(pool, &ids).await?;
        let publicado = |id: Uuid, version: i16| avisos.contains(&(id, version));

        let mut grupos = Vec::with_capacity(5);
        /* Sin cuenta dueña no hay dónde publicar: esa versión no se exporta
         * (y `publicado` sería siempre falso, que es una mentira útil). */
        let incluir = |v: i16| solo_version.is_none_or(|s| s == v) && dueno.contains_key(&v);

        if incluir(1) {
            grupos.push(GrupoExport {
                version: 1,
                inmuebles: inmuebles
                    .iter()
                    .map(|i| {
                        /* La mejorada sustituye a su original si existe (la
                         * ficha ya enseña esa). Se empareja por `orden`. */
                        let fs = preferir_mejoradas(fotos_de_la_ficha(&fotos, i.id));
                        InmuebleExport {
                            slug: i.slug.clone(),
                            titulo: i.titulo.clone(),
                            descripcion: i.descripcion.clone(),
                            fotos: fs,
                            publicado: publicado(i.id, 1),
                        }
                    })
                    .collect(),
            });
        }

        /* Una carpeta por cada versión que tenga contenido (fotos o texto), no
         * solo la 2: si alguien crea la 3, el export la tiene que ver. Las
         * versiones salen de los datos, así que no hay lista que mantener. */
        let mut variantes = VersionRepository::fotos_por_inmuebles(pool, &ids, None).await?;
        variantes.retain(|v| v.version >= 2);
        let textos = VersionRepository::variantes_por_inmuebles(pool, &ids).await?;

        if solo_version.is_none_or(|s| s == 0) {
            /* [30AA-18] La v0 son las fotos ORIGINALES (sin Gemini) con su
             * texto propio. No exige cuenta dueña (no la tiene asignada
             * ninguna) y su aviso vale en cualquier cuenta: `publicado` aquí
             * no usa `publicado()` sino el aviso de v0. */
            let avisos_v0 = FacebookRepository::avisos_de_version(pool, &ids, 0).await?;
            let filas = grupo_originales(&inmuebles, FotoFiltro { fotos: &fotos, textos: &textos }, &avisos_v0);
            /* Sin fichas con v0 hecha no se crea la carpeta: a diferencia de
             * las demás, aquí la lista sale de los textos, no de los datos. */
            if !filas.is_empty() {
                grupos.push(GrupoExport {
                    version: 0,
                    inmuebles: filas,
                });
            }
        }
        let mut numeros: Vec<i16> = variantes
            .iter()
            .map(|v| v.version)
            .chain(textos.values().map(|t| t.version))
            /* La v0 ya se armó arriba con las originales. */
            .filter(|v| *v >= 2)
            .collect();
        numeros.sort_unstable();
        numeros.dedup();

        for numero in numeros.into_iter().filter(|v| incluir(*v)) {
            let filas = grupo_variante(&inmuebles, &variantes, &textos, numero, &publicado);
            grupos.push(GrupoExport {
                version: numero,
                inmuebles: filas,
            });
        }

        /* La v0 se arma aparte de la 1 (que va primera por ser la ficha), así
         * que el orden natural queda 1, 0, 2…: se ordena para que el escritorio
         * cree las carpetas en el orden que se lee. */
        grupos.sort_by_key(|g| g.version);

        Ok(ManifiestoExport {
            cuenta: cuenta.id,
            cuenta_nombre: cuenta.nombre,
            generado_en: Utc::now(),
            grupos,
        })
    }
}

impl From<&FotoManifiesto> for FotoExport {
    fn from(f: &FotoManifiesto) -> Self {
        Self {
            orden: f.orden,
            url: url_publica_de_foto(&f.storage_key),
            origen: f.origen.clone(),
        }
    }
}

/// Lo que necesita el grupo de originales: las fotos de la ficha y los textos
/// por `(inmueble, versión)`.
struct FotoFiltro<'a> {
    fotos: &'a [FotoManifiesto],
    textos: &'a std::collections::HashMap<(Uuid, i16), crate::models::InmuebleVersion>,
}

/// [30AA-1] Fichas de un grupo v2+: sus fotos variante y su texto. Sin fotos
/// ni texto en esa versión no hay carpeta que crear. El texto cae a la v1 si
/// la variante aún no tiene el suyo.
fn grupo_variante(
    inmuebles: &[crate::models::InmuebleRow],
    variantes: &[crate::models::FotoVarianteRow],
    textos: &std::collections::HashMap<(Uuid, i16), crate::models::InmuebleVersion>,
    numero: i16,
    publicado: &impl Fn(Uuid, i16) -> bool,
) -> Vec<InmuebleExport> {
    inmuebles
        .iter()
        .filter_map(|i| {
            let propias: Vec<FotoExport> = variantes
                .iter()
                .filter(|v| v.inmueble_id == i.id && v.version == numero)
                .map(|v| FotoExport {
                    orden: v.orden,
                    url: url_publica_de_foto(&v.storage_key),
                    origen: "variante".to_string(),
                })
                .collect();
            let texto = textos.get(&(i.id, numero));
            if propias.is_empty() && texto.is_none() {
                return None;
            }
            let (titulo, descripcion) = texto.map_or_else(
                || (i.titulo.clone(), i.descripcion.clone()),
                |v| (v.titulo.clone(), v.descripcion.clone()),
            );
            Some(InmuebleExport {
                slug: i.slug.clone(),
                titulo,
                descripcion,
                fotos: propias,
                publicado: publicado(i.id, numero),
            })
        })
        .collect()
}

/// Todas las fotos de la ficha (originales y mejoradas), por `orden`.
fn fotos_de_la_ficha(fotos: &[FotoManifiesto], inmueble_id: Uuid) -> Vec<FotoExport> {
    fotos
        .iter()
        .filter(|f| f.inmueble_id == inmueble_id)
        .map(FotoExport::from)
        .collect()
}

/// [30AA-18] Fichas del grupo v0: solo las que ya tienen texto de v0 y sus
/// fotos **originales**, sin sustituirlas por las mejoradas.
fn grupo_originales(
    inmuebles: &[crate::models::InmuebleRow],
    datos: FotoFiltro<'_>,
    avisos_v0: &std::collections::HashSet<Uuid>,
) -> Vec<InmuebleExport> {
    inmuebles
        .iter()
        .filter_map(|i| {
            let texto = datos.textos.get(&(i.id, 0))?;
            let mut fs: Vec<FotoExport> = datos
                .fotos
                .iter()
                .filter(|f| f.inmueble_id == i.id && f.origen == "original")
                .map(FotoExport::from)
                .collect();
            fs.sort_by_key(|f| f.orden);
            if fs.is_empty() {
                return None;
            }
            Some(InmuebleExport {
                slug: i.slug.clone(),
                titulo: texto.titulo.clone(),
                descripcion: texto.descripcion.clone(),
                fotos: fs,
                publicado: avisos_v0.contains(&i.id),
            })
        })
        .collect()
}

/// Quita el original de un hueco donde hay mejorada: evita bajar dos veces
/// la misma foto, una de ellas sin los retoques.
fn preferir_mejoradas(mut fotos: Vec<FotoExport>) -> Vec<FotoExport> {
    let con_mejorada: std::collections::HashSet<i32> = fotos
        .iter()
        .filter(|f| f.origen == "mejorada")
        .map(|f| f.orden)
        .collect();
    fotos.retain(|f| f.origen != "original" || !con_mejorada.contains(&f.orden));
    fotos.sort_by_key(|f| f.orden);
    fotos
}

async fn fotos_de(pool: &PgPool, ids: &[Uuid]) -> Result<Vec<FotoManifiesto>, AppError> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    // sentinel-disable-next-line sqlx-query-as-sin-macro -- sentencia nueva; ver cabecera
    let filas = sqlx::query_as::<_, FotoManifiesto>(
        "SELECT inmueble_id, storage_key, orden, origen FROM fotos \
         WHERE inmueble_id = ANY($1) ORDER BY inmueble_id, orden, origen",
    )
    .bind(ids)
    .fetch_all(pool)
    .await?;
    Ok(filas)
}

#[cfg(test)]
mod pruebas {
    use super::{preferir_mejoradas, FotoExport};

    #[test]
    fn la_mejorada_sustituye_a_su_original() {
        let f = |orden, origen: &str| FotoExport {
            orden,
            url: format!("/uploads/x/{origen}-{orden}.jpg"),
            origen: origen.to_string(),
        };
        let salida = preferir_mejoradas(vec![f(0, "original"), f(0, "mejorada"), f(1, "original")]);
        assert_eq!(salida.len(), 2);
        assert!(salida.iter().any(|x| x.origen == "mejorada" && x.orden == 0));
        assert!(salida.iter().any(|x| x.orden == 1));
        assert!(!salida.iter().any(|x| x.origen == "original" && x.orden == 0));
    }
}
