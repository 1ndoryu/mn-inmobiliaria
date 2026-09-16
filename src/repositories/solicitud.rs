use sqlx::PgPool;
use uuid::Uuid;

use crate::models::SolicitudRow;

/* [169A-2] Acceso a `solicitudes` con prepared statements.
 * El admin lista con filtro opcional de estado; el alta siempre entra
 * en `pendiente` (el estado lo mueve solo la revisión).
 * [169A-4+7] La fila incluye trazabilidad (`ip_origen`, `user_agent`,
 * `origen_contacto`) y `precio_estimado` nullable: toda SELECT/RETURNING
 * debe listarlas o `FromRow` falla en runtime. */

const COLUMNAS: &str = "id, nombre, telefono, email, descripcion, ubicacion, \
    precio_estimado, operacion, estado, fotos, ip_origen, user_agent, \
    origen_contacto, created_at, updated_at";

/// Valores ya normalizados listos para insertar (el estado lo fija la BD)
pub struct NuevaSolicitud<'a> {
    pub nombre: &'a str,
    pub telefono: &'a str,
    pub email: &'a str,
    pub descripcion: &'a str,
    pub ubicacion: &'a str,
    pub precio_estimado: Option<f64>,
    pub operacion: &'a str,
    pub fotos: &'a Vec<String>,
    pub ip_origen: Option<&'a str>,
    pub user_agent: Option<&'a str>,
    pub origen_contacto: &'a str,
}

pub struct SolicitudRepository;

impl SolicitudRepository {
    pub async fn create(
        pool: &PgPool,
        nueva: &NuevaSolicitud<'_>,
    ) -> Result<SolicitudRow, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as::<_, SolicitudRow>(
            "INSERT INTO solicitudes (id, nombre, telefono, email, descripcion, \
              ubicacion, precio_estimado, operacion, fotos, ip_origen, user_agent, \
              origen_contacto) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) \
             RETURNING id, nombre, telefono, email, descripcion, ubicacion, \
              precio_estimado, operacion, estado, fotos, ip_origen, user_agent, \
              origen_contacto, created_at, updated_at",
        )
        .bind(id)
        .bind(nueva.nombre)
        .bind(nueva.telefono)
        .bind(nueva.email)
        .bind(nueva.descripcion)
        .bind(nueva.ubicacion)
        .bind(nueva.precio_estimado)
        .bind(nueva.operacion)
        .bind(nueva.fotos)
        .bind(nueva.ip_origen)
        .bind(nueva.user_agent)
        .bind(nueva.origen_contacto)
        .fetch_one(pool)
        .await
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<SolicitudRow>, sqlx::Error> {
        sqlx::query_as::<_, SolicitudRow>(
            "SELECT id, nombre, telefono, email, descripcion, ubicacion, \
              precio_estimado, operacion, estado, fotos, ip_origen, user_agent, \
              origen_contacto, created_at, updated_at \
             FROM solicitudes WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    /// Lista de revisión: filtro opcional por estado, paginada
    pub async fn list_admin(
        pool: &PgPool,
        estado: Option<&str>,
        page: i64,
        per_page: i64,
    ) -> Result<(Vec<SolicitudRow>, i64), sqlx::Error> {
        let offset = (page - 1) * per_page;
        let base =
            format!("SELECT {COLUMNAS} FROM solicitudes WHERE ($1::TEXT IS NULL OR estado = $1)");
        let rows = sqlx::query_as::<_, SolicitudRow>(&format!(
            "{base} ORDER BY created_at DESC LIMIT $2 OFFSET $3"
        ))
        .bind(estado)
        .bind(per_page)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        let (total,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM solicitudes WHERE ($1::TEXT IS NULL OR estado = $1)",
        )
        .bind(estado)
        .fetch_one(pool)
        .await?;

        Ok((rows, total))
    }

    /// Mueve el estado de revisión; devuelve la fila o `None` si no existe
    pub async fn update_estado(
        pool: &PgPool,
        id: Uuid,
        estado: &str,
    ) -> Result<Option<SolicitudRow>, sqlx::Error> {
        sqlx::query_as::<_, SolicitudRow>(
            "UPDATE solicitudes SET estado = $2, updated_at = NOW() WHERE id = $1 \
             RETURNING id, nombre, telefono, email, descripcion, ubicacion, \
              precio_estimado, operacion, estado, fotos, ip_origen, user_agent, \
              origen_contacto, created_at, updated_at",
        )
        .bind(id)
        .bind(estado)
        .fetch_optional(pool)
        .await
    }
}
