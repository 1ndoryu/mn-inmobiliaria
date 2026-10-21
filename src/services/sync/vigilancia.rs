//! Vigilancia de la sincronización [13AA-12]: tarea de fondo que mantiene el
//! estado local ↔ servidor. Un cambio local recalcula al instante (`notificar`,
//! sin tocar la red); el servidor se consulta cada `SYNC_INTERVALO_SEG` (20 s
//! por defecto, mínimo 5) o cuando se pide (`comprobar_ya`). El panel lee el
//! último estado publicado, con long-poll por `revision`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tokio::sync::{watch, Notify};
use tokio::time::{sleep_until, timeout, Instant};
use utoipa::ToSchema;

use super::comparar::{comparar_todo, Base, Comparacion, Estado};
use super::huella::{InmuebleHuella, Manifiesto};
use super::manifiesto::{leer_local, rellenar_huellas};
use super::remoto::{self, Credenciales, ErrorRemoto};
use crate::errors::AppError;
use crate::repositories::sync::SyncRepository;

const INTERVALO_DEFECTO_SEG: u64 = 20;
const INTERVALO_MINIMO_SEG: u64 = 5;
/// Espera máxima de un long-poll antes de devolver el estado vigente.
const ESPERA_LONG_POLL: Duration = Duration::from_secs(25);
/// Pausa entre pasadas mientras queden huellas por calcular.
const REINTENTO_HUELLAS: Duration = Duration::from_secs(2);

/// Cambio local: recalcular con el último manifiesto del servidor.
static CAMBIOS: Notify = Notify::const_new();
/// `POST /sync/comprobar`: consultar al servidor sin esperar al intervalo.
static FORZAR: Notify = Notify::const_new();
/// Último estado publicado. `revision` sube solo si cambia el contenido.
static ESTADO: LazyLock<watch::Sender<Arc<EstadoSync>>> =
    LazyLock::new(|| watch::channel(Arc::new(EstadoSync::inicial())).0);

/// Situación frente al servidor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "estado", rename_all = "snake_case")]
pub enum EstadoRemoto {
    /// Todavía no se ha hecho la primera consulta.
    Iniciando,
    /// Sin archivo de credenciales: no hay comparación.
    Desactivado,
    SinCredenciales,
    SinConexion {
        motivo: String,
    },
    SinSoporte {
        motivo: String,
    },
    /// Último manifiesto del servidor leído con éxito.
    Activo,
}

/// Estado completo que consume el panel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct EstadoSync {
    pub revision: u64,
    pub generado_en: DateTime<Utc>,
    pub remoto: EstadoRemoto,
    /// Fotos locales aún sin huella (se calculan en segundo plano).
    pub fotos_pendientes_local: i64,
    /// Presente solo con el servidor disponible.
    pub comparacion: Option<Comparacion>,
}

impl EstadoSync {
    fn inicial() -> Self {
        Self {
            revision: 0,
            generado_en: Utc::now(),
            remoto: EstadoRemoto::Iniciando,
            fotos_pendientes_local: 0,
            comparacion: None,
        }
    }

    /// Compara el contenido sin `revision` ni `generado_en`: un refresco sin
    /// cambios no despierta a los long-poll.
    fn mismo_contenido(&self, otro: &Self) -> bool {
        self.remoto == otro.remoto
            && self.fotos_pendientes_local == otro.fotos_pendientes_local
            && self.comparacion == otro.comparacion
    }
}

/// Un cambio en el catálogo. No bloquea: la pasada corre en la tarea de fondo.
pub fn notificar() {
    CAMBIOS.notify_one();
}

/// Pide consultar al servidor ya (`POST /sync/comprobar`).
pub fn comprobar_ya() {
    FORZAR.notify_one();
}

/// Estado publicado en este momento.
pub fn actual() -> Arc<EstadoSync> {
    ESTADO.borrow().clone()
}

/// Espera hasta que `revision` difiera de `desde` (o hasta `ESPERA_LONG_POLL`)
/// y devuelve el estado vigente.
pub async fn esperar_cambio(desde: u64) -> Arc<EstadoSync> {
    let mut rx = ESTADO.subscribe();
    let _ = timeout(ESPERA_LONG_POLL, rx.wait_for(|e| e.revision != desde)).await;
    let vigente = rx.borrow().clone();
    vigente
}

/// Tarea de fondo: se lanza una vez al arrancar el backend.
pub async fn vigilar(pool: PgPool, upload_dir: PathBuf) {
    let intervalo = intervalo_desde_entorno();
    let mut remoto: Result<Manifiesto, ErrorRemoto> = Err(ErrorRemoto::Desactivado);
    let mut proxima = Instant::now();
    loop {
        if Instant::now() >= proxima {
            remoto = consultar().await;
            let espera = match &remoto {
                Ok(m) if m.fotos_pendientes > 0 => REINTENTO_HUELLAS,
                _ => intervalo,
            };
            proxima = Instant::now() + espera;
        }
        let pendientes = match recalcular(&pool, &upload_dir, &remoto).await {
            Ok(n) => n,
            Err(e) => {
                tracing::warn!("sync: recálculo fallido: {e:?}");
                0
            }
        };
        let despertar = if pendientes > 0 {
            proxima.min(Instant::now() + REINTENTO_HUELLAS)
        } else {
            proxima
        };
        tokio::select! {
            () = sleep_until(despertar) => {}
            () = CAMBIOS.notified() => {}
            () = FORZAR.notified() => proxima = Instant::now(),
        }
    }
}

fn intervalo_desde_entorno() -> Duration {
    let seg = std::env::var("SYNC_INTERVALO_SEG")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(INTERVALO_DEFECTO_SEG)
        .max(INTERVALO_MINIMO_SEG);
    Duration::from_secs(seg)
}

async fn consultar() -> Result<Manifiesto, ErrorRemoto> {
    let cred = Credenciales::cargar()?;
    remoto::manifiesto(&cred).await
}

/// Recalcula con el último manifiesto del servidor y publica si cambió.
/// Devuelve las fotos locales pendientes de huella.
async fn recalcular(
    pool: &PgPool,
    upload_dir: &Path,
    remoto: &Result<Manifiesto, ErrorRemoto>,
) -> Result<i64, AppError> {
    let remoto = match remoto {
        Ok(m) => m,
        Err(e) => {
            publicar(EstadoSync {
                revision: 0,
                generado_en: Utc::now(),
                remoto: estado_de_error(e),
                fotos_pendientes_local: 0,
                comparacion: None,
            });
            return Ok(0);
        }
    };
    rellenar_huellas(pool, upload_dir).await?;
    let local = leer_local(pool).await?;
    let comparacion = comparar_y_guardar(pool, &local.inmuebles, &remoto.inmuebles).await?;
    publicar(EstadoSync {
        revision: 0,
        generado_en: Utc::now(),
        remoto: EstadoRemoto::Activo,
        fotos_pendientes_local: local.fotos_pendientes,
        comparacion: Some(comparacion),
    });
    Ok(local.fotos_pendientes)
}

fn estado_de_error(e: &ErrorRemoto) -> EstadoRemoto {
    match e {
        ErrorRemoto::Desactivado => EstadoRemoto::Desactivado,
        ErrorRemoto::SinCredenciales => EstadoRemoto::SinCredenciales,
        ErrorRemoto::SinConexion(motivo) => EstadoRemoto::SinConexion {
            motivo: motivo.clone(),
        },
        // Una operación rechazada no cambia el estado de la vigilancia; es el motivo de una escritura.
        ErrorRemoto::SinSoporte(motivo) | ErrorRemoto::Rechazado(motivo) => {
            EstadoRemoto::SinSoporte {
                motivo: motivo.clone(),
            }
        }
    }
}

/// Compara y, donde los dos lados coinciden por completo, fija la base (`sync_base`)
/// para que las próximas diferencias sepan quién cambió. Poda las bases de
/// slugs que ya no existen en ningún lado.
async fn comparar_y_guardar(
    pool: &PgPool,
    locales: &[InmuebleHuella],
    remotos: &[InmuebleHuella],
) -> Result<Comparacion, AppError> {
    let bases: HashMap<String, Base> = SyncRepository::bases(pool)
        .await?
        .into_iter()
        .filter_map(|f| Base::desde_json(f.campos.0, f.fotos.0).map(|b| (f.slug, b)))
        .collect();
    let comparacion = comparar_todo(locales, remotos, &bases);

    let por_slug: HashMap<&str, &InmuebleHuella> =
        locales.iter().map(|h| (h.slug.as_str(), h)).collect();
    let mut acordadas = Vec::new();
    for entrada in comparacion
        .inmuebles
        .iter()
        .filter(|e| e.resultado.estado == Estado::Igual)
    {
        let Some(local) = por_slug.get(entrada.slug.as_str()) else {
            continue;
        };
        let base = Base::de(local);
        if bases.get(&entrada.slug) != Some(&base) {
            let campos = serde_json::to_value(&base.campos).unwrap_or_default();
            let fotos = serde_json::to_value(&base.fotos).unwrap_or_default();
            acordadas.push((entrada.slug.clone(), campos, fotos));
        }
    }
    SyncRepository::guardar_bases(pool, &acordadas).await?;

    let mut vivos: Vec<String> = locales.iter().map(|h| h.slug.clone()).collect();
    vivos.extend(remotos.iter().map(|h| h.slug.clone()));
    SyncRepository::podar_bases(pool, &vivos).await?;
    Ok(comparacion)
}

/// Publica el estado nuevo. `revision` sube solo si el contenido cambió.
fn publicar(nuevo: EstadoSync) {
    ESTADO.send_if_modified(|actual| {
        if actual.mismo_contenido(&nuevo) {
            return false;
        }
        let revision = actual.revision + 1;
        *actual = Arc::new(EstadoSync { revision, ..nuevo });
        true
    });
}
