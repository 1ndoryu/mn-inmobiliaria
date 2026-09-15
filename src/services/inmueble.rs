use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{
    AddFotoRequest, CreateInmuebleRequest, FiltrosPublicos, Foto, Inmueble, InmuebleRow,
    PaginatedInmuebles, UpdateInmuebleRequest, ESTADOS, OPERACIONES, ORIGENES_FOTO, TIPOS,
};
use crate::repositories::InmuebleRepository;

/* [159A-1] Lógica del catálogo: normalización de enums, slug único con
 * reintento ante carrera (UNIQUE 23505) y ensamblado fila+fotos sin N+1. */

pub struct InmuebleService;

impl InmuebleService {
    /// Slug desde el título: minúsculas, ASCII, guiones; `inmueble` si queda vacío
    #[must_use]
    pub fn slugify(titulo: &str) -> String {
        let mut slug = String::new();
        let mut guion_pendiente = false;
        for ch in titulo.to_lowercase().chars() {
            if ch.is_ascii_alphanumeric() {
                if guion_pendiente && !slug.is_empty() {
                    slug.push('-');
                }
                guion_pendiente = false;
                slug.push(ch);
            } else if ch.is_whitespace() || ch == '-' || ch == '_' {
                guion_pendiente = true;
            }
            if slug.len() >= 60 {
                break;
            }
        }
        if slug.is_empty() {
            "inmueble".to_string()
        } else {
            slug
        }
    }

    fn normalizar(valor: &str, permitidos: &[&str], campo: &str) -> Result<String, AppError> {
        let normalizado = valor.trim().to_lowercase();
        let base = if normalizado.is_empty() {
            permitidos[0]
        } else {
            normalizado.as_str()
        };
        if permitidos.contains(&base) {
            Ok(base.to_string())
        } else {
            Err(AppError::Validation(format!(
                "Valor inválido para {campo}: {valor}"
            )))
        }
    }

    async fn con_fotos(pool: &PgPool, rows: Vec<InmuebleRow>) -> Result<Vec<Inmueble>, AppError> {
        let ids: Vec<Uuid> = rows.iter().map(|r| r.id).collect();
        let mapa = InmuebleRepository::fotos_por_inmuebles(pool, &ids).await?;
        Ok(rows
            .into_iter()
            .map(|row| {
                let fotos = mapa.get(&row.id).cloned().unwrap_or_default();
                Inmueble::from_row(row, fotos)
            })
            .collect())
    }

    fn paginacion(page: i64, per_page: i64) -> (i64, i64) {
        (page.max(1), per_page.clamp(1, 100))
    }

    pub async fn create(pool: &PgPool, req: CreateInmuebleRequest) -> Result<Inmueble, AppError> {
        let tipo = Self::normalizar(&req.tipo, TIPOS, "tipo")?;
        let operacion = Self::normalizar(&req.operacion, OPERACIONES, "operacion")?;
        let estado = Self::normalizar(&req.estado, ESTADOS, "estado")?;
        let base_slug = Self::slugify(&req.titulo);

        let mut intento = 0;
        loop {
            let slug = if intento == 0 {
                base_slug.clone()
            } else {
                format!("{base_slug}-{}", intento + 1)
            };
            let nuevo = crate::repositories::NuevoInmueble {
                titulo: &req.titulo,
                descripcion: &req.descripcion,
                ubicacion: &req.ubicacion,
                precio: req.precio,
                tipo: &tipo,
                operacion: &operacion,
                habitaciones: req.habitaciones,
                banos: req.banos,
                metros: req.metros,
                metros_terreno: req.metros_terreno,
                estado: &estado,
                slug,
            };
            match InmuebleRepository::create(pool, &nuevo).await {
                Ok(row) => return Ok(Inmueble::from_row(row, Vec::new())),
                Err(e) if InmuebleRepository::es_conflicto_slug(&e) && intento < 3 => {
                    intento += 1;
                }
                Err(e) => return Err(e.into()),
            }
        }
    }

    pub async fn get_admin(pool: &PgPool, id: Uuid) -> Result<Inmueble, AppError> {
        let row = InmuebleRepository::find_by_id(pool, id)
            .await?
            .ok_or_else(|| AppError::NotFound("Inmueble no encontrado".into()))?;
        Ok(Self::con_fotos(pool, vec![row])
            .await?
            .pop()
            .expect("una fila produce un inmueble"))
    }

    pub async fn get_public(pool: &PgPool, slug: &str) -> Result<Inmueble, AppError> {
        let row = InmuebleRepository::find_public_by_slug(pool, slug)
            .await?
            .ok_or_else(|| AppError::NotFound("Inmueble no encontrado".into()))?;
        Ok(Self::con_fotos(pool, vec![row])
            .await?
            .pop()
            .expect("una fila produce un inmueble"))
    }

    pub async fn list_admin(
        pool: &PgPool,
        page: i64,
        per_page: i64,
    ) -> Result<PaginatedInmuebles, AppError> {
        let (page, per_page) = Self::paginacion(page, per_page);
        let (rows, total) = InmuebleRepository::list_admin(pool, page, per_page).await?;
        Ok(PaginatedInmuebles {
            items: Self::con_fotos(pool, rows).await?,
            total,
            page,
            per_page,
        })
    }

    pub async fn list_public(
        pool: &PgPool,
        mut filtros: FiltrosPublicos,
    ) -> Result<PaginatedInmuebles, AppError> {
        if let Some(tipo) = &filtros.tipo {
            filtros.tipo = Some(Self::normalizar(tipo, TIPOS, "tipo")?);
        }
        if let Some(operacion) = &filtros.operacion {
            filtros.operacion = Some(Self::normalizar(operacion, OPERACIONES, "operacion")?);
        }
        let (page, per_page) = Self::paginacion(filtros.page, filtros.per_page);
        filtros.page = page;
        filtros.per_page = per_page;
        let (rows, total) = InmuebleRepository::list_public(pool, &filtros).await?;
        Ok(PaginatedInmuebles {
            items: Self::con_fotos(pool, rows).await?,
            total,
            page,
            per_page,
        })
    }

    pub async fn update(
        pool: &PgPool,
        id: Uuid,
        req: UpdateInmuebleRequest,
    ) -> Result<Inmueble, AppError> {
        let tipo = req
            .tipo
            .as_deref()
            .map(|v| Self::normalizar(v, TIPOS, "tipo"))
            .transpose()?;
        let operacion = req
            .operacion
            .as_deref()
            .map(|v| Self::normalizar(v, OPERACIONES, "operacion"))
            .transpose()?;
        let estado = req
            .estado
            .as_deref()
            .map(|v| Self::normalizar(v, ESTADOS, "estado"))
            .transpose()?;

        let row = InmuebleRepository::update(
            pool,
            id,
            req.titulo.as_deref(),
            req.descripcion.as_deref(),
            req.ubicacion.as_deref(),
            req.precio,
            tipo.as_deref(),
            operacion.as_deref(),
            req.habitaciones,
            req.banos,
            req.metros,
            req.metros_terreno,
            estado.as_deref(),
        )
        .await?
        .ok_or_else(|| AppError::NotFound("Inmueble no encontrado".into()))?;
        Ok(Self::con_fotos(pool, vec![row])
            .await?
            .pop()
            .expect("una fila produce un inmueble"))
    }

    pub async fn set_publicado(
        pool: &PgPool,
        id: Uuid,
        publicado: bool,
    ) -> Result<Inmueble, AppError> {
        let row = InmuebleRepository::set_publicado(pool, id, publicado)
            .await?
            .ok_or_else(|| AppError::NotFound("Inmueble no encontrado".into()))?;
        Ok(Self::con_fotos(pool, vec![row])
            .await?
            .pop()
            .expect("una fila produce un inmueble"))
    }

    pub async fn delete(pool: &PgPool, id: Uuid) -> Result<(), AppError> {
        if !InmuebleRepository::delete(pool, id).await? {
            return Err(AppError::NotFound("Inmueble no encontrado".into()));
        }
        Ok(())
    }

    pub async fn add_foto(
        pool: &PgPool,
        inmueble_id: Uuid,
        req: AddFotoRequest,
    ) -> Result<Foto, AppError> {
        if InmuebleRepository::find_by_id(pool, inmueble_id)
            .await?
            .is_none()
        {
            return Err(AppError::NotFound("Inmueble no encontrado".into()));
        }
        let origen = req
            .origen
            .as_deref()
            .map(|v| Self::normalizar(v, ORIGENES_FOTO, "origen"))
            .transpose()?
            .unwrap_or_else(|| "original".to_string());
        Ok(
            InmuebleRepository::add_foto(pool, inmueble_id, &req.storage_key, req.orden, &origen)
                .await?,
        )
    }

    pub async fn delete_foto(pool: &PgPool, foto_id: Uuid) -> Result<(), AppError> {
        if !InmuebleRepository::delete_foto(pool, foto_id).await? {
            return Err(AppError::NotFound("Foto no encontrada".into()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::InmuebleService;

    #[test]
    fn slugify_basicos() {
        assert_eq!(InmuebleService::slugify("Piso en Centro"), "piso-en-centro");
        assert_eq!(
            InmuebleService::slugify("  Casa -- Grande__  "),
            "casa-grande"
        );
        assert_eq!(InmuebleService::slugify(""), "inmueble");
        assert_eq!(InmuebleService::slugify("---"), "inmueble");
    }
}
