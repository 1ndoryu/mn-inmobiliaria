//! Consultas de la sincronización local ↔ servidor [13AA-12].
//!
//! Usan `sqlx::query*` sin macro a propósito: así las columnas nuevas
//! (`fotos.sha256`, `fotos.bytes`) y la tabla `sync_base` no obligan a
//! regenerar `.sqlx`. Los tipos de dominio (`Base`, `FotoHuella`) viven en
//! `services::sync`; aquí solo viaja JSON crudo para no invertir capas.

use serde_json::Value;
use sqlx::types::Json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::InmuebleRow;
use crate::repositories::inmueble::COLUMNAS;

/// Foto con su huella (puede estar aún sin calcular).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct FotoSync {
    pub inmueble_id: Uuid,
    pub orden: i32,
    pub origen: String,
    pub sha256: Option<String>,
    pub bytes: Option<i32>,
}

/// Fila de `sync_base`: contenido de la última vez que coincidieron.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BaseFila {
    pub slug: String,
    pub campos: Json<Value>,
    pub fotos: Json<Value>,
}

pub struct SyncRepository;

impl SyncRepository {
    /* Catálogo completo (publicado o no) para el manifiesto; `limite` acota la memoria. */
    pub async fn todos(pool: &PgPool, limite: i64) -> Result<Vec<InmuebleRow>, sqlx::Error> {
        // sentinel-disable-next-line sqlx-query-as-sin-macro -- SQL dinámico (columnas de constante COLUMNAS); la macro exige literal
        sqlx::query_as::<_, InmuebleRow>(&format!(
            "SELECT {COLUMNAS} FROM inmuebles ORDER BY slug LIMIT $1"
        ))
        .bind(limite)
        .fetch_all(pool)
        .await
    }

    /* Inmueble por slug sin filtrar `publicado`: detalle de sincronización. */
    pub async fn find_admin_by_slug(
        pool: &PgPool,
        slug: &str,
    ) -> Result<Option<InmuebleRow>, sqlx::Error> {
        // sentinel-disable-next-line sqlx-query-as-sin-macro -- SQL dinámico (columnas de constante COLUMNAS); la macro exige literal
        sqlx::query_as::<_, InmuebleRow>(&format!(
            "SELECT {COLUMNAS} FROM inmuebles WHERE slug = $1"
        ))
        .bind(slug)
        .fetch_optional(pool)
        .await
    }

    pub async fn fotos_todas(pool: &PgPool) -> Result<Vec<FotoSync>, sqlx::Error> {
        // sentinel-disable-next-line sqlx-query-as-sin-macro -- columnas nuevas fuera de `.sqlx`; ver cabecera del módulo
        sqlx::query_as::<_, FotoSync>(
            "SELECT inmueble_id, orden, origen, sha256, bytes FROM fotos \
             ORDER BY inmueble_id, orden, origen",
        )
        .fetch_all(pool)
        .await
    }

    /// Fotos de un inmueble con su huella, en el orden de la galería.
    pub async fn fotos_de(pool: &PgPool, inmueble_id: Uuid) -> Result<Vec<FotoSync>, sqlx::Error> {
        // sentinel-disable-next-line sqlx-query-as-sin-macro -- columnas nuevas fuera de `.sqlx`; ver cabecera del módulo
        sqlx::query_as::<_, FotoSync>(
            "SELECT inmueble_id, orden, origen, sha256, bytes FROM fotos \
             WHERE inmueble_id = $1 ORDER BY orden, origen",
        )
        .bind(inmueble_id)
        .fetch_all(pool)
        .await
    }

    /// Originales sin huella, las más antiguas primero: `(id, storage_key)`.
    pub async fn fotos_sin_huella(
        pool: &PgPool,
        limite: i64,
    ) -> Result<Vec<(Uuid, String)>, sqlx::Error> {
        // sentinel-disable-next-line sqlx-query-as-sin-macro -- columnas nuevas fuera de `.sqlx`; ver cabecera del módulo
        sqlx::query_as::<_, (Uuid, String)>(
            "SELECT id, storage_key FROM fotos \
             WHERE origen = 'original' AND sha256 IS NULL \
             ORDER BY created_at, id LIMIT $1",
        )
        .bind(limite)
        .fetch_all(pool)
        .await
    }

    pub async fn contar_sin_huella(pool: &PgPool) -> Result<i64, sqlx::Error> {
        // sentinel-disable-next-line sqlx-query-as-sin-macro -- columnas nuevas fuera de `.sqlx`; ver cabecera del módulo
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM fotos WHERE origen = 'original' AND sha256 IS NULL",
        )
        .fetch_one(pool)
        .await
    }

    pub async fn guardar_huella(
        pool: &PgPool,
        foto_id: Uuid,
        sha256: &str,
        bytes: Option<i32>,
    ) -> Result<(), sqlx::Error> {
        // sentinel-disable-next-line sqlx-query-sin-macro -- columnas nuevas fuera de `.sqlx`; ver cabecera del módulo
        sqlx::query("UPDATE fotos SET sha256 = $2, bytes = $3 WHERE id = $1")
            .bind(foto_id)
            .bind(sha256)
            .bind(bytes)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Igual que `guardar_huella`, para la foto recién insertada (se conoce
    /// la clave, no el id, en el flujo de subida).
    pub async fn guardar_huella_por_clave(
        pool: &PgPool,
        storage_key: &str,
        sha256: &str,
        bytes: Option<i32>,
    ) -> Result<(), sqlx::Error> {
        // sentinel-disable-next-line sqlx-query-sin-macro -- columnas nuevas fuera de `.sqlx`; ver cabecera del módulo
        sqlx::query("UPDATE fotos SET sha256 = $2, bytes = $3 WHERE storage_key = $1")
            .bind(storage_key)
            .bind(sha256)
            .bind(bytes)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn bases(pool: &PgPool) -> Result<Vec<BaseFila>, sqlx::Error> {
        // sentinel-disable-next-line sqlx-query-as-sin-macro -- tabla nueva fuera de `.sqlx`; ver cabecera del módulo
        sqlx::query_as::<_, BaseFila>("SELECT slug, campos, fotos FROM sync_base")
            .fetch_all(pool)
            .await
    }

    /// Upsert idempotente de las bases acordadas, en una transacción.
    pub async fn guardar_bases(
        pool: &PgPool,
        filas: &[(String, Value, Value)],
    ) -> Result<(), sqlx::Error> {
        if filas.is_empty() {
            return Ok(());
        }
        let mut tx = pool.begin().await?;
        for (slug, campos, fotos) in filas {
            // sentinel-disable-next-line sqlx-query-sin-macro -- tabla nueva fuera de `.sqlx`; ver cabecera del módulo
            sqlx::query(
                "INSERT INTO sync_base (slug, campos, fotos, acordado_en) \
                 VALUES ($1, $2, $3, now()) \
                 ON CONFLICT (slug) DO UPDATE SET campos = EXCLUDED.campos, \
                 fotos = EXCLUDED.fotos, acordado_en = now()",
            )
            .bind(slug)
            .bind(Json(campos))
            .bind(Json(fotos))
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await
    }

    /// Borra las bases de slugs que ya no existen en ningún lado.
    pub async fn podar_bases(pool: &PgPool, vivos: &[String]) -> Result<u64, sqlx::Error> {
        // sentinel-disable-next-line sqlx-query-sin-macro -- tabla nueva fuera de `.sqlx`; ver cabecera del módulo
        let r = sqlx::query("DELETE FROM sync_base WHERE NOT (slug = ANY($1))")
            .bind(vivos)
            .execute(pool)
            .await?;
        Ok(r.rows_affected())
    }
}
