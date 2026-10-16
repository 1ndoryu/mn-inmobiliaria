//! Plan de resolución de un inmueble [14AA-2]. Un solo sitio lo calcula, para
//! que el plan que el panel muestra y el que `aplicar` exige sean el mismo
//! (`plan_id`): si el handler y la aplicación lo armaran por separado, bastaría
//! una diferencia mínima para que el botón diera 409 siempre.

use std::collections::BTreeSet;

use sqlx::PgPool;

use crate::errors::AppError;
use crate::services::sync::comparacion::detalle::{self, DetalleSync};
use crate::services::sync::comparacion::resolver::{self, PlanResolucion};
use crate::services::sync::operacion::conflicto;
use crate::services::sync::remoto::{self, Credenciales};

/// Plan del inmueble `slug`, leyendo los dos lados. `conservar_viejo` = claves
/// cuyo lado contrario gana.
pub async fn plan_de(
    pool: &PgPool,
    cred: &Credenciales,
    slug: &str,
    conservar_viejo: &BTreeSet<String>,
) -> Result<PlanResolucion, AppError> {
    let local = detalle::detalle_local(pool, slug)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("inmueble {slug}")))?;
    let servidor = remoto::detalle(cred, slug)
        .await
        .map_err(conflicto)?
        .ok_or_else(|| AppError::Conflict(format!("{slug} no existe en el servidor")))?;
    plan_con(pool, cred, &local, &servidor, conservar_viejo).await
}

/// Igual que `plan_de`, con los dos detalles ya leídos: `aplicar` los necesita
/// de todos modos, y así no los pide dos veces.
pub async fn plan_con(
    pool: &PgPool,
    cred: &Credenciales,
    local: &DetalleSync,
    servidor: &DetalleSync,
    conservar_viejo: &BTreeSet<String>,
) -> Result<PlanResolucion, AppError> {
    let dif = detalle::diferencias(pool, local, Some(servidor)).await?;
    // Las miniaturas solo hacen falta si hay fotos que comparar (y entran en el `plan_id`).
    let (mini_local, mini_servidor) = if dif.filas.iter().any(|f| f.clave.starts_with("foto:")) {
        detalle::miniaturas(pool, cred, local, servidor).await?
    } else {
        Default::default()
    };
    resolver::planificar(
        &dif,
        &local.huella,
        &servidor.huella,
        conservar_viejo,
        &mini_local,
        &mini_servidor,
    )
    .map_err(|e| AppError::Internal(e.to_string()))
}
