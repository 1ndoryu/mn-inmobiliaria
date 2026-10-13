//! Manifiesto local de huellas y relleno perezoso de las huellas de fotos.
//!
//! `leer_local` solo consulta la BD (barato, se llama en cada refresco); el
//! relleno (`rellenar_huellas`) lee archivos y se acota por lote para que la
//! primera carga con fotos antiguas no bloquee nada.

use std::collections::HashMap;
use std::io::ErrorKind;
use std::path::Path;

use chrono::Utc;
use sqlx::PgPool;
use tokio::sync::Mutex;
use uuid::Uuid;

use super::huella::{huella_de, sha256_hex, FotoHuella, Manifiesto, SIN_ARCHIVO, VERSION};
use crate::errors::AppError;
use crate::repositories::sync::SyncRepository;
use crate::services::InmuebleService;

/// Tope de inmuebles por manifiesto; por encima se marca `truncado`.
pub const TOPE_INMUEBLES: usize = 2000;

/// Fotos que se hashean por pasada de relleno.
const LOTE_HUELLAS: i64 = 40;

/// Una sola pasada de relleno a la vez: otra concurrente no aporta nada.
static RELLENANDO: Mutex<()> = Mutex::const_new(());

/// Manifiesto leído solo de la BD (sin tocar archivos).
pub async fn leer_local(pool: &PgPool) -> Result<Manifiesto, AppError> {
    let limite = i64::try_from(TOPE_INMUEBLES + 1).unwrap_or(i64::MAX);
    let mut filas = SyncRepository::todos(pool, limite).await?;
    let truncado = filas.len() > TOPE_INMUEBLES;
    filas.truncate(TOPE_INMUEBLES);

    let mut fotos_por_inmueble: HashMap<Uuid, Vec<FotoHuella>> = HashMap::new();
    for f in SyncRepository::fotos_todas(pool).await? {
        fotos_por_inmueble
            .entry(f.inmueble_id)
            .or_default()
            .push(FotoHuella {
                origen: f.origen,
                orden: f.orden,
                sha256: f.sha256,
                bytes: f.bytes,
            });
    }
    let fotos_pendientes = SyncRepository::contar_sin_huella(pool).await?;
    let inmuebles = filas
        .iter()
        .map(|fila| {
            huella_de(
                fila,
                fotos_por_inmueble.remove(&fila.id).unwrap_or_default(),
            )
        })
        .collect();
    Ok(Manifiesto {
        version: VERSION,
        generado_en: Utc::now(),
        truncado,
        fotos_pendientes,
        inmuebles,
    })
}

/// Hashea hasta `LOTE_HUELLAS` originales sin huella. Devuelve cuántas
/// quedaron resueltas. Un archivo ausente se marca `sin-archivo` (no se
/// reintenta); otro error de lectura deja la foto pendiente para la siguiente
/// pasada.
pub async fn rellenar_huellas(pool: &PgPool, upload_dir: &Path) -> Result<usize, AppError> {
    let Ok(_turno) = RELLENANDO.try_lock() else {
        return Ok(0);
    };
    let pendientes = SyncRepository::fotos_sin_huella(pool, LOTE_HUELLAS).await?;
    let mut hechas = 0;
    for (id, clave) in pendientes {
        let ruta = InmuebleService::ruta_archivo(upload_dir, &clave);
        match tokio::fs::read(&ruta).await {
            Ok(datos) => {
                let largo = i32::try_from(datos.len()).ok();
                let sha = tokio::task::spawn_blocking(move || sha256_hex(&datos))
                    .await
                    .map_err(|e| AppError::Internal(format!("hash de foto: {e}")))?;
                SyncRepository::guardar_huella(pool, id, &sha, largo).await?;
                hechas += 1;
            }
            Err(e) if e.kind() == ErrorKind::NotFound => {
                SyncRepository::guardar_huella(pool, id, SIN_ARCHIVO, None).await?;
                hechas += 1;
            }
            Err(e) => tracing::warn!("sync: no se pudo leer {clave} para su huella: {e}"),
        }
    }
    Ok(hechas)
}

/// Manifiesto completo para `GET /api/admin/sync/manifiesto`: primero rellena
/// un lote de huellas pendientes y después lee.
pub async fn construir(pool: &PgPool, upload_dir: &Path) -> Result<Manifiesto, AppError> {
    rellenar_huellas(pool, upload_dir).await?;
    leer_local(pool).await
}
