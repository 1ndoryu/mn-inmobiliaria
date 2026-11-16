//! [30AA-1] Cuentas de Facebook y vínculo por inmueble.
//!
//! Dos reglas que se cumplen aquí y en ningún otro sitio:
//!  1. La cuenta principal es única (`principal` con índice parcial).
//!  2. `inmuebles.marketplace_id` es un espejo del aviso de la principal: se
//!     recalcula tras cada cambio de vínculo, nunca se escribe a mano.

use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{Facebook, FacebookRequest, VinculoFacebook};
use crate::repositories::FacebookRepository;
use crate::services::sync::vigilancia::notificar;

pub struct FacebookService;

impl FacebookService {
    pub async fn listar(pool: &PgPool) -> Result<Vec<Facebook>, AppError> {
        let cuentas = FacebookRepository::listar(pool).await?;
        let conteo = FacebookRepository::conteo_por_cuenta(pool).await?;
        Ok(cuentas
            .into_iter()
            .map(|c| {
                let n = conteo.get(&c.id).copied().unwrap_or(0);
                Facebook::from_row(c, n)
            })
            .collect())
    }

    /// Crea la cuenta. La primera que exista es principal aunque no se pida:
    /// sin principal, el espejo de `inmuebles.marketplace_id` no tendría a
    /// quién apuntar y los avisos históricos dejarían de resolverse.
    pub async fn crear(pool: &PgPool, req: FacebookRequest) -> Result<Facebook, AppError> {
        let nombre = req.nombre.trim();
        if nombre.is_empty() {
            return Err(AppError::Validation("El nombre es obligatorio".into()));
        }
        let principal = req.principal || FacebookRepository::principal(pool).await?.is_none();
        let fila = FacebookRepository::crear(
            pool,
            nombre,
            req.url.trim(),
            req.notas.trim(),
            principal,
        )
        .await
        .map_err(conflicto)?;
        notificar();
        Ok(Facebook::from_row(fila, 0))
    }

    pub async fn actualizar(
        pool: &PgPool,
        id: Uuid,
        req: FacebookRequest,
    ) -> Result<Facebook, AppError> {
        let nombre = req.nombre.trim();
        if nombre.is_empty() {
            return Err(AppError::Validation("El nombre es obligatorio".into()));
        }
        let fila = FacebookRepository::actualizar(
            pool,
            id,
            nombre,
            req.url.trim(),
            req.notas.trim(),
            req.principal,
        )
        .await
        .map_err(conflicto)?
        .ok_or_else(|| AppError::NotFound("Cuenta de Facebook no encontrada".into()))?;
        /* Cambiar de principal cambia qué aviso espeja cada inmueble. */
        if req.principal {
            Self::resincronizar_todos(pool).await?;
        }
        notificar();
        Ok(Facebook::from_row(fila, 0))
    }

    /// Borrar una cuenta desvincula sus inmuebles (cascada) y vuelve a
    /// calcular el espejo: los avisos que solo estaban en ella se pierden,
    /// que es lo que la dueña pidió al borrarla.
    pub async fn borrar(pool: &PgPool, id: Uuid) -> Result<(), AppError> {
        if !FacebookRepository::borrar(pool, id).await? {
            return Err(AppError::NotFound("Cuenta de Facebook no encontrada".into()));
        }
        Self::resincronizar_todos(pool).await?;
        notificar();
        Ok(())
    }

    pub async fn vinculos(pool: &PgPool, inmueble_id: Uuid) -> Result<Vec<VinculoFacebook>, AppError> {
        Ok(FacebookRepository::vinculos_de(pool, inmueble_id).await?)
    }

    /// Vincula una cuenta a UNA VERSIÓN (idempotente) y fija su aviso si llega.
    ///
    /// [30AA-7] La versión tiene que existir: la 1 siempre, y las demás solo si
    /// la configuración las pide y ya se generaron. Vincular una v3 vacía
    /// dejaría un aviso apuntando a un texto que no existe.
    pub async fn vincular(
        pool: &PgPool,
        inmueble_id: Uuid,
        facebook_id: Uuid,
        version: i16,
        marketplace_id: Option<&str>,
    ) -> Result<Vec<VinculoFacebook>, AppError> {
        if FacebookRepository::find_by_id(pool, facebook_id)
            .await?
            .is_none()
        {
            return Err(AppError::NotFound("Cuenta de Facebook no encontrada".into()));
        }
        Self::exigir_version(pool, inmueble_id, version).await?;
        let aviso = crate::services::InmuebleService::normalizar_marketplace_id(marketplace_id)?;
        if let Some(a) = &aviso {
            if FacebookRepository::aviso_ocupado(pool, facebook_id, a, Some(inmueble_id)).await? {
                return Err(aviso_duplicado());
            }
        }
        FacebookRepository::vincular(pool, inmueble_id, facebook_id, version, aviso.as_deref())
            .await
            .map_err(conflicto)?;
        FacebookRepository::sincronizar_espejo_principal(pool, inmueble_id)
            .await
            .map_err(conflicto)?;
        notificar();
        Self::vinculos(pool, inmueble_id).await
    }

    pub async fn desvincular(
        pool: &PgPool,
        inmueble_id: Uuid,
        facebook_id: Uuid,
        version: i16,
    ) -> Result<Vec<VinculoFacebook>, AppError> {
        if !FacebookRepository::desvincular(pool, inmueble_id, facebook_id, version).await? {
            return Err(AppError::NotFound(
                "El inmueble no está vinculado a esa cuenta en esa versión".into(),
            ));
        }
        FacebookRepository::sincronizar_espejo_principal(pool, inmueble_id)
            .await
            .map_err(conflicto)?;
        notificar();
        Self::vinculos(pool, inmueble_id).await
    }

    /// Versiones que aún no tienen aviso en ninguna cuenta, más cuántas son.
    pub async fn faltan_avisos(
        pool: &PgPool,
        inmueble_id: Uuid,
        configuradas: i16,
    ) -> Result<crate::models::FaltanAvisos, AppError> {
        let versiones = FacebookRepository::faltan_avisos(pool, inmueble_id, configuradas).await?;
        Ok(crate::models::FaltanAvisos {
            total: versiones.len(),
            versiones,
        })
    }

    /// La versión 1 existe siempre (vive en `inmuebles`); las demás necesitan
    /// fila en `inmueble_versiones` y estar dentro de la configuración.
    async fn exigir_version(pool: &PgPool, inmueble_id: Uuid, version: i16) -> Result<(), AppError> {
        if version < 1 {
            return Err(AppError::Validation("La versión va de 1 en adelante".into()));
        }
        if version == 1 {
            return Ok(());
        }
        let configuradas = crate::repositories::VersionRepository::configuradas(pool).await?;
        if version > configuradas {
            return Err(AppError::Validation(format!(
                "Versión fuera de rango (1..={configuradas})"
            )));
        }
        if crate::repositories::VersionRepository::find(pool, inmueble_id, version)
            .await?
            .is_none()
        {
            return Err(AppError::NotFound(format!(
                "La versión {version} aún no se ha generado"
            )));
        }
        Ok(())
    }

    /// Recalcula el espejo de todos los inmuebles (cambio de principal).
    /// Una sola sentencia: el volumen es de miles de filas, no de millones.
    /// Espeja la v1: es el aviso de la ficha, no el de una variante.
    async fn resincronizar_todos(pool: &PgPool) -> Result<(), AppError> {
        // sentinel-disable-next-line sqlx-query-sin-macro -- sentencia nueva; ver cabecera del módulo
        sqlx::query(
            "UPDATE inmuebles i SET marketplace_id = ( \
                SELECT v.marketplace_id FROM inmueble_facebook v \
                JOIN facebook f ON f.id = v.facebook_id \
                WHERE v.inmueble_id = i.id AND v.version = 1 AND f.principal LIMIT 1)",
        )
        .execute(pool)
        .await?;
        Ok(())
    }
}

fn conflicto(e: sqlx::Error) -> AppError {
    if es_unico(&e) {
        aviso_duplicado()
    } else {
        e.into()
    }
}

fn aviso_duplicado() -> AppError {
    AppError::Validation("marketplace_id ya vinculado a otro inmueble en esa cuenta".into())
}

fn es_unico(e: &sqlx::Error) -> bool {
    matches!(e, sqlx::Error::Database(d) if d.code().as_deref() == Some("23505"))
}
