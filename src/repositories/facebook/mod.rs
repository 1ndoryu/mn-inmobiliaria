//! [30AA-1] Acceso a `facebook` e `inmueble_facebook`.
//!
//! Sentencias en runtime sin macro (`sqlx::query_as`): son nuevas y la caché
//! `.sqlx` solo se regenera con la BD delante; así el `cargo check` del
//! portátil no depende de tener Postgres levantado.

use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{FacebookRow, VinculoFacebook};

/// Fila cruda del JOIN `inmueble_facebook` + `facebook` (tipo con nombre por
/// la regla `type_complexity` de clippy).
type FilaVinculo = (
    Uuid,
    Uuid,
    String,
    String,
    bool,
    Option<String>,
    Option<chrono::DateTime<chrono::Utc>>,
);

pub struct FacebookRepository;

impl FacebookRepository {
    pub async fn listar(pool: &PgPool) -> Result<Vec<FacebookRow>, sqlx::Error> {
        // sentinel-disable-next-line sqlx-query-as-sin-macro -- sentencia nueva; ver cabecera
        sqlx::query_as::<_, FacebookRow>(
            "SELECT id, nombre, url, notas, principal, creado_en \
             FROM facebook ORDER BY principal DESC, nombre ASC",
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<FacebookRow>, sqlx::Error> {
        // sentinel-disable-next-line sqlx-query-as-sin-macro -- sentencia nueva; ver cabecera
        sqlx::query_as::<_, FacebookRow>(
            "SELECT id, nombre, url, notas, principal, creado_en FROM facebook WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    /// Cuántos inmuebles hay vinculados a cada cuenta (una sola query).
    pub async fn conteo_por_cuenta(
        pool: &PgPool,
    ) -> Result<std::collections::HashMap<Uuid, i64>, sqlx::Error> {
        // sentinel-disable-next-line sqlx-query-as-sin-macro -- sentencia nueva; ver cabecera
        let filas: Vec<(Uuid, i64)> = sqlx::query_as(
            "SELECT facebook_id, COUNT(*)::BIGINT FROM inmueble_facebook GROUP BY facebook_id",
        )
        .fetch_all(pool)
        .await?;
        Ok(filas.into_iter().collect())
    }

    pub async fn crear(
        pool: &PgPool,
        nombre: &str,
        url: &str,
        notas: &str,
        principal: bool,
    ) -> Result<FacebookRow, sqlx::Error> {
        let id = Uuid::new_v4();
        let mut tx = pool.begin().await?;
        if principal {
            Self::quitar_principal_tx(&mut tx).await?;
        }
        // sentinel-disable-next-line sqlx-query-as-sin-macro -- sentencia nueva; ver cabecera
        let fila = sqlx::query_as::<_, FacebookRow>(
            "INSERT INTO facebook (id, nombre, url, notas, principal) VALUES ($1, $2, $3, $4, $5) \
             RETURNING id, nombre, url, notas, principal, creado_en",
        )
        .bind(id)
        .bind(nombre)
        .bind(url)
        .bind(notas)
        .bind(principal)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(fila)
    }

    pub async fn actualizar(
        pool: &PgPool,
        id: Uuid,
        nombre: &str,
        url: &str,
        notas: &str,
        principal: bool,
    ) -> Result<Option<FacebookRow>, sqlx::Error> {
        let mut tx = pool.begin().await?;
        if principal {
            Self::quitar_principal_tx(&mut tx).await?;
        }
        // sentinel-disable-next-line sqlx-query-as-sin-macro -- sentencia nueva; ver cabecera
        let fila = sqlx::query_as::<_, FacebookRow>(
            "UPDATE facebook SET nombre = $2, url = $3, notas = $4, principal = $5 WHERE id = $1 \
             RETURNING id, nombre, url, notas, principal, creado_en",
        )
        .bind(id)
        .bind(nombre)
        .bind(url)
        .bind(notas)
        .bind(principal)
        .fetch_optional(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(fila)
    }

    pub async fn borrar(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
        // sentinel-disable-next-line sqlx-query-sin-macro -- sentencia nueva; ver cabecera del módulo
        let r = sqlx::query("DELETE FROM facebook WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(r.rows_affected() > 0)
    }

    /// El índice único parcial exige un solo `principal`; se limpia antes.
    async fn quitar_principal_tx(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), sqlx::Error> {
        // sentinel-disable-next-line sqlx-query-sin-macro -- sentencia nueva; ver cabecera del módulo
        sqlx::query("UPDATE facebook SET principal = FALSE WHERE principal")
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    /// Cuenta principal (la que espeja `inmuebles.marketplace_id`).
    pub async fn principal(pool: &PgPool) -> Result<Option<FacebookRow>, sqlx::Error> {
        // sentinel-disable-next-line sqlx-query-as-sin-macro -- sentencia nueva; ver cabecera
        sqlx::query_as::<_, FacebookRow>(
            "SELECT id, nombre, url, notas, principal, creado_en \
             FROM facebook WHERE principal LIMIT 1",
        )
        .fetch_optional(pool)
        .await
    }

    /* ---- Vínculos inmueble ↔ cuenta ---- */

    pub async fn vinculos_de(
        pool: &PgPool,
        inmueble_id: Uuid,
    ) -> Result<Vec<VinculoFacebook>, sqlx::Error> {
        // sentinel-disable-next-line sqlx-query-as-sin-macro -- sentencia nueva; ver cabecera
        sqlx::query_as::<_, VinculoFacebook>(
            "SELECT f.id AS facebook_id, f.nombre, f.url, f.principal, \
                    v.marketplace_id, v.publicado_en \
             FROM inmueble_facebook v JOIN facebook f ON f.id = v.facebook_id \
             WHERE v.inmueble_id = $1 \
             ORDER BY f.principal DESC, f.nombre ASC",
        )
        .bind(inmueble_id)
        .fetch_all(pool)
        .await
    }

    /// Vínculos de varios inmuebles en una sola query (evita N+1 en la lista).
    pub async fn vinculos_por_inmuebles(
        pool: &PgPool,
        ids: &[Uuid],
    ) -> Result<std::collections::HashMap<Uuid, Vec<VinculoFacebook>>, sqlx::Error> {
        let mut mapa: std::collections::HashMap<Uuid, Vec<VinculoFacebook>> =
            std::collections::HashMap::new();
        if ids.is_empty() {
            return Ok(mapa);
        }
        // sentinel-disable-next-line sqlx-query-as-sin-macro -- sentencia nueva; ver cabecera
        let filas: Vec<FilaVinculo> =
            // sentinel-disable-next-line sqlx-query-as-sin-macro -- sentencia nueva; ver cabecera del módulo
            sqlx::query_as(
                "SELECT v.inmueble_id, f.id, f.nombre, f.url, f.principal, v.marketplace_id, v.publicado_en \
                 FROM inmueble_facebook v JOIN facebook f ON f.id = v.facebook_id \
                 WHERE v.inmueble_id = ANY($1) \
                 ORDER BY f.principal DESC, f.nombre ASC",
            )
            .bind(ids)
            .fetch_all(pool)
            .await?;
        for (inmueble_id, facebook_id, nombre, url, principal, marketplace_id, publicado_en) in filas
        {
            mapa.entry(inmueble_id).or_default().push(VinculoFacebook {
                facebook_id,
                nombre,
                url,
                principal,
                marketplace_id,
                publicado_en,
            });
        }
        Ok(mapa)
    }

    /// Fija (o quita, con `None`) el aviso de una ficha en una cuenta.
    /// Devuelve `false` si esa cuenta no estaba vinculada.
    pub async fn fijar_aviso(
        pool: &PgPool,
        inmueble_id: Uuid,
        facebook_id: Uuid,
        marketplace_id: Option<&str>,
    ) -> Result<bool, sqlx::Error> {
        // sentinel-disable-next-line sqlx-query-sin-macro -- sentencia nueva; ver cabecera del módulo
        let r = sqlx::query(
            "UPDATE inmueble_facebook SET marketplace_id = $3, \
                publicado_en = CASE WHEN $3::TEXT IS NULL THEN NULL ELSE COALESCE(publicado_en, NOW()) END \
             WHERE inmueble_id = $1 AND facebook_id = $2",
        )
        .bind(inmueble_id)
        .bind(facebook_id)
        .bind(marketplace_id)
        .execute(pool)
        .await?;
        Ok(r.rows_affected() > 0)
    }

    /// Vincula (idempotente) un inmueble a una cuenta.
    pub async fn vincular(
        pool: &PgPool,
        inmueble_id: Uuid,
        facebook_id: Uuid,
        marketplace_id: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        // sentinel-disable-next-line sqlx-query-sin-macro -- sentencia nueva; ver cabecera del módulo
        sqlx::query(
            "INSERT INTO inmueble_facebook (inmueble_id, facebook_id, marketplace_id) \
             VALUES ($1, $2, $3) \
             ON CONFLICT (inmueble_id, facebook_id) DO UPDATE SET marketplace_id = EXCLUDED.marketplace_id",
        )
        .bind(inmueble_id)
        .bind(facebook_id)
        .bind(marketplace_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn desvincular(
        pool: &PgPool,
        inmueble_id: Uuid,
        facebook_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        // sentinel-disable-next-line sqlx-query-sin-macro -- sentencia nueva; ver cabecera del módulo
        let r = sqlx::query(
            "DELETE FROM inmueble_facebook WHERE inmueble_id = $1 AND facebook_id = $2",
        )
        .bind(inmueble_id)
        .bind(facebook_id)
        .execute(pool)
        .await?;
        Ok(r.rows_affected() > 0)
    }

    /// Espeja en `inmuebles.marketplace_id` el aviso de la cuenta principal
    /// (`NULL` si la ficha no está vinculada a ella o no tiene aviso).
    pub async fn sincronizar_espejo_principal(
        pool: &PgPool,
        inmueble_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        // sentinel-disable-next-line sqlx-query-sin-macro -- sentencia nueva; ver cabecera del módulo
        sqlx::query(
            "UPDATE inmuebles i SET marketplace_id = ( \
                SELECT v.marketplace_id FROM inmueble_facebook v \
                JOIN facebook f ON f.id = v.facebook_id \
                WHERE v.inmueble_id = i.id AND f.principal LIMIT 1) \
             WHERE i.id = $1",
        )
        .bind(inmueble_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Ques es el aviso de esta cuenta el que ocupa `marketplace_id` y lo
    /// reclama otra ficha (para el 422 previo al INSERT).
    pub async fn aviso_ocupado(
        pool: &PgPool,
        facebook_id: Uuid,
        marketplace_id: &str,
        inmueble_id: Option<Uuid>,
    ) -> Result<bool, sqlx::Error> {
        // sentinel-disable-next-line sqlx-query-as-sin-macro -- sentencia nueva; ver cabecera
        let fila: Option<(Uuid,)> = sqlx::query_as(
            "SELECT inmueble_id FROM inmueble_facebook \
             WHERE facebook_id = $1 AND marketplace_id = $2",
        )
        .bind(facebook_id)
        .bind(marketplace_id)
        .fetch_optional(pool)
        .await?;
        Ok(matches!(fila, Some((id,)) if Some(id) != inmueble_id))
    }
}
