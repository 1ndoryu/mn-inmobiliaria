use std::path::Path;

use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

use crate::errors::AppError;
use crate::models::{
    CreateSolicitudRequest, FotoSolicitudSubida, PaginatedSolicitudes, Solicitud,
    SolicitudesAdminParams, UpdateEstadoSolicitud, ESTADOS_SOLICITUD, OPERACIONES,
    ORIGENES_CONTACTO,
};
use crate::repositories::{NuevaSolicitud, SolicitudRepository};

use super::inmueble::InmuebleService;

/* [169A-2] Lógica de solicitudes: el alta entra siempre en `pendiente`;
 * la foto pública reutiliza `guardar_archivo` (misma validación que el
 * admin) bajo `solicitudes/<uuid-sesion>/`; el estado solo lo mueve el
 * admin entre los valores del CHECK. */

pub struct SolicitudService;

impl SolicitudService {
    fn paginacion(page: i64, per_page: i64) -> (i64, i64) {
        (page.max(1), per_page.clamp(1, 100))
    }

    /// Alta pública: valida, normaliza y guarda en `pendiente`.
    /// `ip`/`user_agent` vienen de cabeceras (auditoría anti-spam, nunca
    /// se exponen en la API pública); `origen_contacto` se valida contra
    /// la allowlist y cae a `web`.
    pub async fn create(
        pool: &PgPool,
        req: CreateSolicitudRequest,
        ip: Option<String>,
        user_agent: Option<String>,
    ) -> Result<Solicitud, AppError> {
        req.validate()
            .map_err(|e| AppError::Validation(e.to_string()))?;

        let operacion = req.operacion.trim().to_lowercase();
        if !OPERACIONES.contains(&operacion.as_str()) {
            return Err(AppError::Validation(
                "La operación debe ser venta o alquiler".into(),
            ));
        }

        /* Email opcional: ausente pasa; informado lo valida el derive (`email`
         * salta `None`). Aquí solo se normaliza. */
        let email = req.email.as_deref().unwrap_or("").trim();

        /* Origen con allowlist cerrada: lo no reconocido cae a `web` en vez
         * de rechazar el alta (el canal no debe perder el contacto). */
        let origen = req
            .origen_contacto
            .as_deref()
            .map(str::trim)
            .map(str::to_lowercase)
            .filter(|o| ORIGENES_CONTACTO.contains(&o.as_str()))
            .unwrap_or_else(|| "web".to_string());

        /* User-Agent truncado a 500 caracteres (trazabilidad, no fingerprint).
         * `take` por chars: seguro con UTF-8, sin partir codepoints. */
        let user_agent_recortado = user_agent.map(|ua| ua.chars().take(500).collect::<String>());

        let nueva = NuevaSolicitud {
            nombre: req.nombre.trim(),
            telefono: req.telefono.trim(),
            email,
            descripcion: req.descripcion.trim(),
            ubicacion: req.ubicacion.trim(),
            precio_estimado: req.precio_estimado.map(|p| p.max(0.0)),
            operacion: &operacion,
            fotos: &req.fotos,
            ip_origen: ip.as_deref(),
            user_agent: user_agent_recortado.as_deref(),
            origen_contacto: &origen,
        };
        let row = SolicitudRepository::create(pool, &nueva).await?;
        Ok(Solicitud::from_row(row))
    }

    /// Subida pública de una foto: cada archivo estrena carpeta de sesión
    /// (`solicitudes/<uuid>/`) para que la clave tenga el formato validado
    pub async fn subir_foto_publica(
        upload_dir: &Path,
        filename: &str,
        bytes: &[u8],
    ) -> Result<FotoSolicitudSubida, AppError> {
        let carpeta = format!("solicitudes/{}", Uuid::new_v4());
        let storage_key =
            InmuebleService::guardar_archivo(upload_dir, &carpeta, filename, bytes).await?;
        let url = format!("/uploads/{storage_key}");
        Ok(FotoSolicitudSubida { storage_key, url })
    }

    /// Lista de revisión con filtro opcional de estado ya normalizado
    pub async fn list_admin(
        pool: &PgPool,
        params: SolicitudesAdminParams,
    ) -> Result<PaginatedSolicitudes, AppError> {
        let estado = params
            .estado
            .as_deref()
            .map(str::trim)
            .filter(|e| !e.is_empty())
            .map(str::to_lowercase);
        if let Some(ref e) = estado {
            if !ESTADOS_SOLICITUD.contains(&e.as_str()) {
                return Err(AppError::Validation("Estado de revisión no válido".into()));
            }
        }
        let (page, per_page) = Self::paginacion(params.page, params.per_page);
        let (rows, total) =
            SolicitudRepository::list_admin(pool, estado.as_deref(), page, per_page).await?;
        Ok(PaginatedSolicitudes {
            items: rows.into_iter().map(Solicitud::from_row).collect(),
            total,
            page,
            per_page,
        })
    }

    /// Revisión: mueve el estado; `None` si la solicitud no existe
    pub async fn update_estado(
        pool: &PgPool,
        id: Uuid,
        req: UpdateEstadoSolicitud,
    ) -> Result<Solicitud, AppError> {
        req.validate()
            .map_err(|e| AppError::Validation(e.to_string()))?;
        let row = SolicitudRepository::update_estado(pool, id, &req.estado).await?;
        row.map(Solicitud::from_row)
            .ok_or_else(|| AppError::NotFound("Solicitud no encontrada".into()))
    }
}
