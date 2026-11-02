//! Lectura de las solicitudes «Publicar mi inmueble» de producción [28AE-1].
//! Solo GET: nada se escribe en el servidor. Vive dentro de `remoto` para que
//! `http` no salga de este módulo.

use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

use super::http::{enviar, leer, Envio};
use super::{Credenciales, ErrorRemoto, SESION};

/// Foto de una solicitud remota: clave (`solicitudes/<uuid>/<uuid>.<ext>`) y URL servible
#[derive(Debug, Deserialize)]
pub struct FotoSolicitudRemota {
    pub storage_key: String,
    pub url: String,
}

/// Solicitud tal como la devuelve `GET /api/admin/solicitudes` del servidor
#[derive(Debug, Deserialize)]
pub struct SolicitudRemota {
    pub id: Uuid,
    pub nombre: String,
    pub telefono: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub descripcion: String,
    #[serde(default)]
    pub ubicacion: String,
    #[serde(default)]
    pub puestos: i32,
    #[serde(default)]
    pub residencia: String,
    pub precio_estimado: Option<f64>,
    pub operacion: String,
    pub estado: String,
    pub fotos: Vec<FotoSolicitudRemota>,
    pub ip_origen: Option<String>,
    pub user_agent: Option<String>,
    pub origen_contacto: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Una página del listado remoto (`PaginatedSolicitudes` del servidor)
#[derive(Debug, Deserialize)]
pub struct PaginaSolicitudesRemota {
    pub items: Vec<SolicitudRemota>,
}

/// Página `page` de solicitudes de producción, de la más reciente a la más antigua.
/// Un 404 (servidor sin esta ruta) sale como «sin soporte».
pub async fn pagina_solicitudes(
    cred: &Credenciales,
    page: i64,
    per_page: i64,
) -> Result<PaginaSolicitudesRemota, ErrorRemoto> {
    let mut envio = Envio::lectura("/api/admin/solicitudes".into());
    envio.consulta = vec![("page", page.to_string()), ("per_page", per_page.to_string())];
    leer(enviar(cred, &SESION, &envio).await?)
        .await?
        .ok_or_else(|| ErrorRemoto::SinSoporte("falta /api/admin/solicitudes en el servidor".into()))
}
