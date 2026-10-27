use std::path::Path;

use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{
    sugerir_nombre_corto, ActualizacionInmueble, CreateInmuebleRequest, FiltrosPublicos, Inmueble,
    InmuebleRow, PaginatedInmuebles, UpdateInmuebleRequest, ESTADOS, FORMATOS_RECETA, OPERACIONES,
    TIPOS,
};
use crate::repositories::InmuebleRepository;
use crate::services::sync::vigilancia::notificar;

pub(crate) mod alias;
mod fotos;
pub(crate) mod slug;
pub(crate) mod vinculo;

/* [159A-1] Lógica del catálogo: normalización de enums, slug único con
 * reintento ante carrera (UNIQUE 23505) y ensamblado fila+fotos sin N+1.
 * [159A-2] Subida a disco (`UPLOAD_DIR/<inmueble>/<uuid>.<ext>`) con validación
 * de magic-bytes + extensión; al borrar se limpia el archivo (best-effort). */

pub struct InmuebleService;

impl InmuebleService {
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

    /* [09AA-21] Lógica en `vinculo::normalizar_marketplace_id`; se
     * conserva el delegador porque `handlers/marketplace.rs` lo usa. */
    pub fn normalizar_marketplace_id(valor: Option<&str>) -> Result<Option<String>, AppError> {
        vinculo::normalizar_marketplace_id(valor)
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
        let base_slug = slug::slugify(&req.titulo);
        Self::insertar(pool, req, base_slug, true).await
    }

    /// Alta con el slug que ya tiene el inmueble en el servidor (traer, D14). Sin
    /// reintentos: si el slug está ocupado es conflicto; no se inventa otro.
    pub async fn crear_con_slug(
        pool: &PgPool,
        req: CreateInmuebleRequest,
        slug: String,
    ) -> Result<Inmueble, AppError> {
        Self::insertar(pool, req, slug, false).await
    }

    async fn insertar(
        pool: &PgPool,
        req: CreateInmuebleRequest,
        base_slug: String,
        reintentar: bool,
    ) -> Result<Inmueble, AppError> {
        let tipo = Self::normalizar(&req.tipo, TIPOS, "tipo")?;
        let operacion = Self::normalizar(&req.operacion, OPERACIONES, "operacion")?;
        let estado = Self::normalizar(&req.estado, ESTADOS, "estado")?;
        /* [09AA-21] Vínculo en `vinculo::preparar_para_crear`. */
        let marketplace_id =
            vinculo::preparar_para_crear(pool, req.marketplace_id.as_deref()).await?;
        /* [09AA-24] Alias en `alias::preparar_para_crear`. */
        let alias_titulos = alias::preparar_para_crear(req.alias_titulos)?;
        /* [18AA-6] El nombre corto es un dato del inmueble y se rellena aquí:
         * si el alta no lo trae, se sugiere con tipo, residencia y ubicación y
         * queda guardado. Así los inmuebles nuevos se citan en el chat sin
         * deducirlo del título en cada respuesta. */
        let nombre_corto = if req.nombre_corto.trim().is_empty() {
            sugerir_nombre_corto(&tipo, &req.residencia, &req.ubicacion)
        } else {
            req.nombre_corto.trim().to_string()
        };
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
                puestos: req.puestos,
                residencia: &req.residencia,
                precio: req.precio,
                precio_alquiler: req.precio_alquiler,
                tipo: &tipo,
                operacion: &operacion,
                habitaciones: req.habitaciones,
                banos: req.banos,
                metros: req.metros,
                metros_terreno: req.metros_terreno,
                estado: &estado,
                slug,
                copy_corta: req.copy.as_ref().map(|c| c.corta.as_str()),
                copy_larga: req.copy.as_ref().map(|c| c.larga.as_str()),
                copy_modelo: req.copy.as_ref().map(|c| c.modelo.as_str()),
                copy_actualizada_en: req.copy.as_ref().map(|c| c.actualizada_en),
                marketplace_id: marketplace_id.as_deref(),
                /* Se clona por reintento de slug (máx 3, ≤10 textos cortos). */
                alias_titulos: alias_titulos.clone(),
                nombre_corto: &nombre_corto,
            };
            match InmuebleRepository::create(pool, &nuevo).await {
                Ok(row) => {
                    notificar();
                    return Ok(Inmueble::from_row(row, Vec::new()));
                }
                Err(e) if vinculo::es_conflicto(&e) => {
                    return Err(vinculo::error_duplicado());
                }
                Err(e)
                    if reintentar && InmuebleRepository::es_conflicto_slug(&e) && intento < 3 =>
                {
                    intento += 1;
                }
                /* Sin reintentos o agotados: conflicto explícito, no un 500. */
                Err(e) if InmuebleRepository::es_conflicto_slug(&e) => {
                    return Err(AppError::Conflict(format!(
                        "ya existe un inmueble con el slug «{base_slug}»"
                    )));
                }
                Err(e) => return Err(e.into()),
            }
        }
    }

    pub async fn get_admin(pool: &PgPool, id: Uuid) -> Result<Inmueble, AppError> {
        let row = InmuebleRepository::find_by_id(pool, id)
            .await?
            .ok_or_else(|| AppError::NotFound("Inmueble no encontrado".into()))?;
        Self::con_fotos(pool, vec![row])
            .await?
            .pop()
            .ok_or_else(|| AppError::Internal("con_fotos devolvio vacio para una fila".into()))
    }

    pub async fn get_public(pool: &PgPool, slug: &str) -> Result<Inmueble, AppError> {
        let row = InmuebleRepository::find_public_by_slug(pool, slug)
            .await?
            .ok_or_else(|| AppError::NotFound("Inmueble no encontrado".into()))?;
        Self::con_fotos(pool, vec![row])
            .await?
            .pop()
            .ok_or_else(|| AppError::Internal("con_fotos devolvio vacio para una fila".into()))
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
        /* [09AA-21] Tri-estado en `vinculo::preparar_para_update`. */
        let marketplace_id =
            vinculo::preparar_para_update(pool, id, req.marketplace_id.as_ref()).await?;
        /* [09AA-24] Alias en `alias::preparar_para_update`. */
        let alias_titulos = alias::preparar_para_update(req.alias_titulos)?;
        /* El formato de la receta no admite normalización con defecto (vacío
         * no es válido): allowlist directa. Los índices los cubre `range`
         * del validador en el modelo. [229A-2] */
        if let Some(receta) = &req.receta {
            if !FORMATOS_RECETA.contains(&receta.formato.as_str()) {
                return Err(AppError::Validation(format!(
                    "Valor inválido para formato: {}",
                    receta.formato
                )));
            }
        }

        /* [08AA-3] B5: los 18 campos viajan en `ActualizacionInmueble`
         * (presta los `&str` de `req` + normalizados; `receta` se clona).
         * [09AA-21] El vínculo presta vía `vinculo::prestar_para_update`. */
        let cambios = ActualizacionInmueble {
            titulo: req.titulo.as_deref(),
            descripcion: req.descripcion.as_deref(),
            ubicacion: req.ubicacion.as_deref(),
            puestos: req.puestos,
            residencia: req.residencia.as_deref(),
            /* [18AA-6] Ausente o vacío = no tocar el guardado (lo resuelve el
             * `NULLIF` del UPDATE). */
            nombre_corto: req.nombre_corto.as_deref(),
            precio: req.precio,
            precio_alquiler: req.precio_alquiler,
            tipo: tipo.as_deref(),
            operacion: operacion.as_deref(),
            habitaciones: req.habitaciones,
            banos: req.banos,
            metros: req.metros,
            metros_terreno: req.metros_terreno,
            estado: estado.as_deref(),
            copy_corta: req.copy.as_ref().map(|c| c.corta.as_str()),
            copy_larga: req.copy.as_ref().map(|c| c.larga.as_str()),
            copy_modelo: req.copy.as_ref().map(|c| c.modelo.as_str()),
            copy_actualizada_en: req.copy.as_ref().map(|c| c.actualizada_en),
            receta: req.receta.clone().map(sqlx::types::Json),
            marketplace_id: vinculo::prestar_para_update(marketplace_id.as_ref()),
            alias_titulos,
        };
        let fila = InmuebleRepository::update(pool, id, &cambios).await;
        let row = match fila {
            Err(e) if vinculo::es_conflicto(&e) => {
                return Err(vinculo::error_duplicado());
            }
            resto => resto?,
        }
        .ok_or_else(|| AppError::NotFound("Inmueble no encontrado".into()))?;
        notificar();
        Self::con_fotos(pool, vec![row])
            .await?
            .pop()
            .ok_or_else(|| AppError::Internal("con_fotos devolvio vacio para una fila".into()))
    }

    pub async fn set_publicado(
        pool: &PgPool,
        id: Uuid,
        publicado: bool,
    ) -> Result<Inmueble, AppError> {
        let row = InmuebleRepository::set_publicado(pool, id, publicado)
            .await?
            .ok_or_else(|| AppError::NotFound("Inmueble no encontrado".into()))?;
        notificar();
        Self::con_fotos(pool, vec![row])
            .await?
            .pop()
            .ok_or_else(|| AppError::Internal("con_fotos devolvio vacio para una fila".into()))
    }

    /* [08AA-33] Cambio de estado validado contra `ESTADOS` (422 si no
     * existe el valor). `vendido`/`alquilado` despublican en el repo;
     * el front /ask lo usa para "Esta propiedad se vendió". */
    pub async fn set_estado(pool: &PgPool, id: Uuid, estado: &str) -> Result<Inmueble, AppError> {
        let estado = Self::normalizar(estado, ESTADOS, "estado")?;
        let row = InmuebleRepository::set_estado(pool, id, &estado)
            .await?
            .ok_or_else(|| AppError::NotFound("Inmueble no encontrado".into()))?;
        notificar();
        Self::con_fotos(pool, vec![row])
            .await?
            .pop()
            .ok_or_else(|| AppError::Internal("con_fotos devolvio vacio para una fila".into()))
    }

    pub async fn delete(pool: &PgPool, upload_dir: &Path, id: Uuid) -> Result<(), AppError> {
        let fotos = InmuebleRepository::fotos_por_inmuebles(pool, &[id])
            .await?
            .remove(&id)
            .unwrap_or_default();
        if !InmuebleRepository::delete(pool, id).await? {
            return Err(AppError::NotFound("Inmueble no encontrado".into()));
        }
        notificar();
        /* Fila fuera (fotos en cascada); los archivos se limpian best-effort */
        for foto in fotos {
            Self::borrar_archivo(upload_dir, &foto.storage_key).await;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;

/* [229A-2] La receta publicitaria persiste en `inmuebles.receta` (JSONB):
 * crear la deja NULL, `update` la fija y la relectura la trae; un formato
 * fuera del allowlist se rechaza. Humo contra la BD real de rama
 * (`DATABASE_URL`); sin ella se omite como el resto de humos. */
#[cfg(test)]
mod pruebas_receta;

/* [08AA-33] "Esta propiedad se vendió": `vendido`/`alquilado` cambian el
 * estado Y despublican en una sola query; `reservado` no toca la
 * visibilidad; estado inválido es 422 e id inexistente es 404. Humo contra
 * la BD real de rama (`DATABASE_URL`); sin ella se omite como el resto. */
#[cfg(test)]
mod pruebas_estado;

/* [08AA-4] Borrado de foto sin desfase: borrar una `original` arrastra a su
 * `mejorada` hermana y renumera el resto (orden denso); borrar solo la
 * mejorada conserva el original (caso "regenerar"). Humo contra la BD real
 * de rama (`DATABASE_URL`); sin ella se omite como el resto de humos. */
#[cfg(test)]
mod pruebas_borrado_foto;

/* [09AA-21-split] Tests del vínculo en `vinculo::pruebas_marketplace_id`. */

/* [25AA-5] Operación «ambas»: guarda los dos precios, la validan el servicio y
 * los filtros públicos (venta y alquiler la incluyen, cada uno con su precio).
 * Humo contra la BD de pruebas (`DATABASE_URL`); sin ella se omite. */
#[cfg(test)]
mod pruebas_ambas;
