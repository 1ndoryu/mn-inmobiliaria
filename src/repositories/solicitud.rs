use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::SolicitudRow;

/* [169A-2] Acceso a `solicitudes` con prepared statements.
 * El admin lista con filtro opcional de estado; el alta siempre entra
 * en `pendiente` (el estado lo mueve solo la revisión).
 * [169A-4+7] La fila incluye trazabilidad (`ip_origen`, `user_agent`,
 * `origen_contacto`) y `precio_estimado` nullable: toda SELECT/RETURNING
 * debe listarlas o `FromRow` falla en runtime.
 * [229A-1] Todas usan `{COLUMNAS}` (constante única). */

const COLUMNAS: &str = "id, nombre, telefono, email, descripcion, ubicacion, \
    puestos, residencia, precio_estimado, operacion, estado, fotos, ip_origen, \
    user_agent, origen_contacto, created_at, updated_at";

/// Valores ya normalizados listos para insertar (el estado lo fija la BD)
pub struct NuevaSolicitud<'a> {
    pub nombre: &'a str,
    pub telefono: &'a str,
    pub email: &'a str,
    pub descripcion: &'a str,
    pub ubicacion: &'a str,
    pub puestos: i32,
    pub residencia: &'a str,
    pub precio_estimado: Option<f64>,
    pub operacion: &'a str,
    pub fotos: &'a Vec<String>,
    pub ip_origen: Option<&'a str>,
    pub user_agent: Option<&'a str>,
    pub origen_contacto: &'a str,
}

/// Solicitud llegada de producción: conserva su id y sus fechas originales [28AE-1]
pub struct SolicitudSincronizada<'a> {
    pub id: Uuid,
    pub nueva: NuevaSolicitud<'a>,
    pub estado: &'a str,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// `true` en la primera carga: lo que ya existía en producción no debe avisar.
    pub aviso_visto: bool,
}

pub struct SolicitudRepository;

impl SolicitudRepository {
    pub async fn create(
        pool: &PgPool,
        nueva: &NuevaSolicitud<'_>,
    ) -> Result<SolicitudRow, sqlx::Error> {
        let id = Uuid::new_v4();
        // sentinel-disable-next-line sqlx-query-as-sin-macro -- SQL dinámico (columnas de constante COLUMNAS); la macro exige literal
        sqlx::query_as::<_, SolicitudRow>(&format!(
            "INSERT INTO solicitudes (id, nombre, telefono, email, descripcion, \
              ubicacion, puestos, residencia, precio_estimado, operacion, fotos, \
              ip_origen, user_agent, origen_contacto) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14) \
             RETURNING {COLUMNAS}",
        ))
        .bind(id)
        .bind(nueva.nombre)
        .bind(nueva.telefono)
        .bind(nueva.email)
        .bind(nueva.descripcion)
        .bind(nueva.ubicacion)
        .bind(nueva.puestos)
        .bind(nueva.residencia)
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
        // sentinel-disable-next-line sqlx-query-as-sin-macro -- SQL dinámico (columnas de constante COLUMNAS); la macro exige literal
        sqlx::query_as::<_, SolicitudRow>(&format!(
            "SELECT {COLUMNAS} FROM solicitudes WHERE id = $1",
        ))
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
        // sentinel-disable-next-line sqlx-query-as-sin-macro -- SQL dinámico (base concatenado con ORDER/LIMIT); la macro exige literal
        let rows = sqlx::query_as::<_, SolicitudRow>(&format!(
            "{base} ORDER BY created_at DESC LIMIT $2 OFFSET $3"
        ))
        .bind(estado)
        .bind(per_page)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        let (total,): (i64,) = sqlx::query!(
            "SELECT COUNT(*) AS \"total!\" FROM solicitudes WHERE ($1::TEXT IS NULL OR estado = $1)",
            estado
        )
        .fetch_one(pool)
        .await
        .map(|r| (r.total,))?;

        Ok((rows, total))
    }

    /// Mueve el estado de revisión; devuelve la fila o `None` si no existe
    pub async fn update_estado(
        pool: &PgPool,
        id: Uuid,
        estado: &str,
    ) -> Result<Option<SolicitudRow>, sqlx::Error> {
        // sentinel-disable-next-line sqlx-query-as-sin-macro -- SQL dinámico (columnas de constante COLUMNAS); la macro exige literal
        sqlx::query_as::<_, SolicitudRow>(&format!(
            "UPDATE solicitudes SET estado = $2, updated_at = NOW() WHERE id = $1 \
             RETURNING {COLUMNAS}",
        ))
        .bind(id)
        .bind(estado)
        .fetch_optional(pool)
        .await
    }

    /// Inserta una solicitud de producción con su mismo id. `false` si ya estaba:
    /// repetir la pasada no duplica nada.
    pub async fn insertar_sincronizada(
        pool: &PgPool,
        nueva: &SolicitudSincronizada<'_>,
    ) -> Result<bool, sqlx::Error> {
        let r = &nueva.nueva;
        // RETURNING solo devuelve fila si se insertó: `None` = ya existía (ON CONFLICT).
        // Sin macro `query!`: su cache en `.sqlx/` no recogía esta sentencia.
        let insertada = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO solicitudes (id, nombre, telefono, email, descripcion, ubicacion, \
              puestos, residencia, precio_estimado, operacion, estado, fotos, ip_origen, \
              user_agent, origen_contacto, created_at, updated_at, aviso_visto, sincronizada_en) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, NOW()) \
             ON CONFLICT (id) DO NOTHING RETURNING id",
        )
        .bind(nueva.id)
        .bind(r.nombre)
        .bind(r.telefono)
        .bind(r.email)
        .bind(r.descripcion)
        .bind(r.ubicacion)
        .bind(r.puestos)
        .bind(r.residencia)
        .bind(r.precio_estimado)
        .bind(r.operacion)
        .bind(nueva.estado)
        .bind(r.fotos)
        .bind(r.ip_origen)
        .bind(r.user_agent)
        .bind(r.origen_contacto)
        .bind(nueva.created_at)
        .bind(nueva.updated_at)
        .bind(nueva.aviso_visto)
        .fetch_optional(pool)
        .await?;
        Ok(insertada.is_some())
    }

    /// Ids de `ids` que ya están en la base local
    pub async fn ids_existentes(pool: &PgPool, ids: &[Uuid]) -> Result<Vec<Uuid>, sqlx::Error> {
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM solicitudes WHERE id = ANY($1)")
            .bind(ids)
            .fetch_all(pool)
            .await
    }

    /// ¿Hubo ya una pasada de sincronización? Si no, la siguiente es la primera carga.
    pub async fn primera_pasada_hecha(pool: &PgPool) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar::<_, bool>("SELECT EXISTS (SELECT 1 FROM sync_solicitudes_estado)")
            .fetch_one(pool)
            .await
    }

    /// Anota que ya hubo una pasada (idempotente; la tabla tiene una sola fila)
    pub async fn marcar_primera_pasada(pool: &PgPool) -> Result<(), sqlx::Error> {
        sqlx::query_scalar::<_, bool>(
            "INSERT INTO sync_solicitudes_estado (id) VALUES (TRUE) ON CONFLICT (id) DO NOTHING RETURNING TRUE",
        )
        .fetch_optional(pool)
        .await?;
        Ok(())
    }

    /// Solicitudes que el admin aún no ha visto (aviso de la cabecera)
    pub async fn contar_no_vistas(pool: &PgPool) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM solicitudes WHERE aviso_visto = FALSE")
            .fetch_one(pool)
            .await
    }

    /// Marca todas como vistas; devuelve cuántas cambiaron
    pub async fn marcar_vistas(pool: &PgPool) -> Result<u64, sqlx::Error> {
        let ids = sqlx::query_scalar::<_, Uuid>(
            "UPDATE solicitudes SET aviso_visto = TRUE WHERE aviso_visto = FALSE RETURNING id",
        )
        .fetch_all(pool)
        .await?;
        Ok(ids.len() as u64)
    }
}
