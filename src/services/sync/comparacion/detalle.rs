//! Detalle de un inmueble para la comparación [13AA-12]: huella y valores
//! canónicos en claro. Cada entorno lo sirve en `GET /sync/inmuebles/{slug}`;
//! el panel local combina ambos lados en `diferencias`.

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use utoipa::ToSchema;

use super::comparar::{clave_foto, comparar, Base, Estado, Lado, Resultado};
use super::huella::{huella_de, valores_canonicos, FotoHuella, InmuebleHuella};
use crate::errors::AppError;
use crate::models::url_publica_de_foto;
use crate::repositories::sync::SyncRepository;
use crate::services::sync::operacion::conflicto;
use crate::services::sync::remoto::{self, Credenciales};

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DetalleSync {
    pub huella: InmuebleHuella,
    /// Valor canónico de cada campo, en claro. Solo lo ve el admin: el
    /// manifiesto expone únicamente la huella de `precio_minimo` y `extras`.
    pub valores: BTreeMap<String, String>,
}

/// Una diferencia con el valor de cada lado, para el modal.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FilaDiferencia {
    pub clave: String,
    pub lado: Lado,
    /// `None` = el elemento no existe en ese lado.
    pub local: Option<String>,
    pub servidor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DiferenciasSync {
    pub slug: String,
    pub resultado: Resultado,
    pub filas: Vec<FilaDiferencia>,
}

/// Detalle del inmueble de este entorno (incluye no publicados). `None` si no existe.
pub async fn detalle_local(pool: &PgPool, slug: &str) -> Result<Option<DetalleSync>, AppError> {
    let Some(fila) = SyncRepository::find_admin_by_slug(pool, slug).await? else {
        return Ok(None);
    };
    let fotos: Vec<FotoHuella> = SyncRepository::fotos_de(pool, fila.id)
        .await?
        .into_iter()
        .map(|f| FotoHuella {
            origen: f.origen,
            orden: f.orden,
            sha256: f.sha256,
            bytes: f.bytes,
        })
        .collect();
    let valores = valores_canonicos(&fila)
        .into_iter()
        .map(|(clave, valor)| (clave.to_string(), valor))
        .collect();
    Ok(Some(DetalleSync {
        huella: huella_de(&fila, fotos),
        valores,
    }))
}

/// Diferencias entre el local y el servidor, con el valor de cada lado.
/// `remoto = None` = el inmueble solo existe aquí.
pub async fn diferencias(
    pool: &PgPool,
    local: &DetalleSync,
    remoto: Option<&DetalleSync>,
) -> Result<DiferenciasSync, AppError> {
    let slug = local.huella.slug.clone();
    let Some(remoto) = remoto else {
        return Ok(DiferenciasSync {
            slug,
            resultado: Resultado {
                estado: Estado::SoloLocal,
                estimado: false,
                diferencias: Vec::new(),
            },
            filas: Vec::new(),
        });
    };
    let base: Option<Base> = SyncRepository::bases(pool)
        .await?
        .into_iter()
        .find(|f| f.slug == slug)
        .and_then(|f| Base::desde_json(f.campos.0, f.fotos.0));
    let resultado = comparar(&local.huella, &remoto.huella, base.as_ref());
    let filas = resultado
        .diferencias
        .iter()
        .map(|d| FilaDiferencia {
            clave: d.clave.clone(),
            lado: d.lado,
            local: valor_de(local, &d.clave),
            servidor: valor_de(remoto, &d.clave),
        })
        .collect();
    Ok(DiferenciasSync {
        slug,
        resultado,
        filas,
    })
}

/// Miniaturas por clave de foto (`foto:<origen>:<orden>`): la local es relativa
/// (`/uploads`); la del servidor, absoluta. Entran en el `plan_id`, así que el
/// plan y su aplicación tienen que calcularlas igual.
pub async fn miniaturas(
    pool: &PgPool,
    cred: &Credenciales,
    local: &DetalleSync,
    servidor: &DetalleSync,
) -> Result<(HashMap<String, String>, HashMap<String, String>), AppError> {
    let mini_local = SyncRepository::claves_de_fotos(pool, local.huella.id)
        .await?
        .into_iter()
        .map(|f| (clave_foto(&f.origen, f.orden), url_publica_de_foto(&f.storage_key)))
        .collect();
    let mini_servidor = remoto::fotos(cred, servidor.huella.id)
        .await
        .map_err(conflicto)?
        .into_iter()
        .map(|f| (clave_foto(&f.origen, f.orden), f.url))
        .collect();
    Ok((mini_local, mini_servidor))
}

/// Valor legible de una clave: el campo en claro, o `presente (<8 hex>)` para una
/// foto. `None` = no existe en ese lado.
fn valor_de(detalle: &DetalleSync, clave: &str) -> Option<String> {
    let Some(resto) = clave.strip_prefix("foto:") else {
        return detalle.valores.get(clave).cloned();
    };
    let (origen, orden) = resto.split_once(':')?;
    let orden: i32 = orden.parse().ok()?;
    let foto = detalle
        .huella
        .fotos
        .iter()
        .find(|f| f.origen == origen && f.orden == orden)?;
    Some(match &foto.sha256 {
        Some(sha) => format!("presente ({})", sha.get(..8).unwrap_or(sha.as_str())),
        None => "presente (sin calcular)".to_string(),
    })
}
