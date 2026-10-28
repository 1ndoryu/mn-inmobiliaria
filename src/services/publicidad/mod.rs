use std::collections::HashMap;

use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{url_publica_de_foto, Foto, Pieza, PublicidadPublica};
use crate::repositories::{InmuebleRepository, PublicacionRepository};

/* [28AA-7] Página pública `/publicidad`: una publicación por inmueble visible
 * (la más reciente que salió), con sus fotos en su mejor versión y sus piezas.
 * Solo lectura; la visibilidad la decide `publicado = TRUE` en el repositorio. */

pub struct PublicidadService;

impl PublicidadService {
    /// Lista para la página pública, la publicación más reciente primero
    pub async fn listar(pool: &PgPool) -> Result<Vec<PublicidadPublica>, AppError> {
        let mut filas = PublicacionRepository::listar_publicadas(pool).await?;
        filas.sort_by(|a, b| b.publicada_en.cmp(&a.publicada_en));

        let inmuebles: Vec<Uuid> = filas.iter().map(|f| f.inmueble_id).collect();
        let fotos = InmuebleRepository::fotos_por_inmuebles(pool, &inmuebles).await?;

        let publicaciones: Vec<Uuid> = filas.iter().map(|f| f.publicacion_id).collect();
        let mut piezas: HashMap<Uuid, Vec<Pieza>> = HashMap::new();
        for fila in PublicacionRepository::piezas_de(pool, publicaciones).await? {
            // Se separa el id antes de mover la fila: `from_row` la consume
            let publicacion = fila.publicacion_id;
            piezas.entry(publicacion).or_default().push(Pieza::from_row(fila));
        }

        Ok(filas
            .into_iter()
            .map(|f| PublicidadPublica {
                fotos: fotos.get(&f.inmueble_id).map(|v| fotos_visibles(v)).unwrap_or_default(),
                piezas: piezas.remove(&f.publicacion_id).unwrap_or_default(),
                inmueble_id: f.inmueble_id,
                titulo: f.titulo,
                slug: f.slug,
                descripcion: f.descripcion,
                tags: f.tags,
                publicada_en: f.publicada_en,
            })
            .collect())
    }
}

/// Fotos en su mejor versión, igual que `fotosVisiblesDe` del panel: cada
/// original se sustituye por la mejorada de su mismo `orden`, si existe.
#[must_use]
pub fn fotos_visibles(fotos: &[Foto]) -> Vec<String> {
    let mut originales: Vec<&Foto> = fotos.iter().filter(|f| f.origen == "original").collect();
    originales.sort_by_key(|f| f.orden);
    originales
        .into_iter()
        .map(|original| {
            let mejor = fotos
                .iter()
                .find(|f| f.origen == "mejorada" && f.orden == original.orden)
                .unwrap_or(original);
            url_publica_de_foto(&mejor.storage_key)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;

    fn foto(orden: i32, origen: &str, clave: &str) -> Foto {
        Foto {
            id: Uuid::new_v4(),
            inmueble_id: Uuid::nil(),
            storage_key: clave.to_string(),
            orden,
            origen: origen.to_string(),
            created_at: Utc::now(),
        }
    }

    #[test]
    fn mejorada_sustituye_a_su_original_por_orden() {
        let fotos = vec![
            foto(0, "original", "a.jpg"),
            foto(1, "original", "b.jpg"),
            foto(0, "mejorada", "a-m.jpg"),
        ];
        assert_eq!(
            fotos_visibles(&fotos),
            vec!["/uploads/a-m.jpg".to_string(), "/uploads/b.jpg".to_string()]
        );
    }

    #[test]
    fn ignora_mejoradas_sin_original_y_respeta_el_orden() {
        let fotos = vec![
            foto(2, "original", "c.jpg"),
            foto(1, "original", "b.jpg"),
            foto(5, "mejorada", "huerfana.jpg"),
        ];
        assert_eq!(
            fotos_visibles(&fotos),
            vec!["/uploads/b.jpg".to_string(), "/uploads/c.jpg".to_string()]
        );
    }
}
