//! [30AA-1] Acceso a `inmueble_versiones`, `inmueble_texto_historial` y
//! `fotos_variantes`. Sentencias en runtime (`sqlx::query_as` sin macro) por
//! el mismo motivo que `repositories/facebook/`: son nuevas y la caché
//! `.sqlx` solo se regenera con la BD delante.

use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{FotoVarianteRow, InmuebleVersion, TextoHistorial};

/// Clave de `agent_config` donde vive «cuántas versiones por inmueble».
pub const CLAVE_VERSIONES: &str = "inmueble_versiones";
/// Valor por defecto si la clave no existe (base antigua).
pub const VERSIONES_DEFECTO: i16 = 2;

/// Un cambio de texto que se apunta en el historial: qué se pisó, por qué y con
/// qué modelo. Agrupa los campos para no pasarlos sueltos (todos son `&str` y
/// cruzar `anterior` con `nuevo` no daría error de compilación).
pub struct EntradaHistorial<'a> {
    pub inmueble_id: Uuid,
    pub version: i16,
    pub campo: &'a str,
    pub anterior: &'a str,
    pub nuevo: &'a str,
    pub origen: &'a str,
    pub modelo: &'a str,
}

pub struct VersionRepository;

impl VersionRepository {
    /// Cuántas versiones pide la configuración (1..=5, 2 por defecto).
    pub async fn configuradas(pool: &PgPool) -> Result<i16, sqlx::Error> {
        let valor: Option<String> =
            // sentinel-disable-next-line sqlx-query-sin-macro -- sentencia nueva; ver cabecera del módulo
            sqlx::query_scalar("SELECT value FROM agent_config WHERE key = $1")
                .bind(CLAVE_VERSIONES)
                .fetch_optional(pool)
                .await?;
        let n = valor
            .and_then(|v| v.trim().parse::<i16>().ok())
            .unwrap_or(VERSIONES_DEFECTO);
        Ok(n.clamp(1, 5))
    }

    pub async fn guardar_configuradas(pool: &PgPool, n: i16) -> Result<(), sqlx::Error> {
        // sentinel-disable-next-line sqlx-query-sin-macro -- sentencia nueva; ver cabecera del módulo
        sqlx::query(
            "INSERT INTO agent_config (key, value) VALUES ($1, $2) \
             ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated_at = NOW()",
        )
        .bind(CLAVE_VERSIONES)
        .bind(n.to_string())
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn listar(
        pool: &PgPool,
        inmueble_id: Uuid,
    ) -> Result<Vec<InmuebleVersion>, sqlx::Error> {
        // sentinel-disable-next-line sqlx-query-as-sin-macro -- sentencia nueva; ver cabecera
        sqlx::query_as::<_, InmuebleVersion>(
            "SELECT id, inmueble_id, version, titulo, descripcion, estado, modelo, \
                    prompt_usado, creada_en, actualizada_en \
             FROM inmueble_versiones WHERE inmueble_id = $1 ORDER BY version ASC",
        )
        .bind(inmueble_id)
        .fetch_all(pool)
        .await
    }

    pub async fn find(
        pool: &PgPool,
        inmueble_id: Uuid,
        version: i16,
    ) -> Result<Option<InmuebleVersion>, sqlx::Error> {
        // sentinel-disable-next-line sqlx-query-as-sin-macro -- sentencia nueva; ver cabecera
        sqlx::query_as::<_, InmuebleVersion>(
            "SELECT id, inmueble_id, version, titulo, descripcion, estado, modelo, \
                    prompt_usado, creada_en, actualizada_en \
             FROM inmueble_versiones WHERE inmueble_id = $1 AND version = $2",
        )
        .bind(inmueble_id)
        .bind(version)
        .fetch_optional(pool)
        .await
    }

    /// Guarda (o reemplaza) una variante generada. El historial se escribe
    /// aparte para dejar rastro de lo anterior si ya existía.
    pub async fn upsert(
        pool: &PgPool,
        inmueble_id: Uuid,
        version: i16,
        titulo: &str,
        descripcion: &str,
        modelo: &str,
        prompt_usado: &str,
    ) -> Result<InmuebleVersion, sqlx::Error> {
        let id = Uuid::new_v4();
        // sentinel-disable-next-line sqlx-query-as-sin-macro -- sentencia nueva; ver cabecera
        sqlx::query_as::<_, InmuebleVersion>(
            "INSERT INTO inmueble_versiones \
                (id, inmueble_id, version, titulo, descripcion, estado, modelo, prompt_usado) \
             VALUES ($1, $2, $3, $4, $5, 'borrador', $6, $7) \
             ON CONFLICT (inmueble_id, version) DO UPDATE SET \
                titulo = EXCLUDED.titulo, descripcion = EXCLUDED.descripcion, \
                estado = 'borrador', modelo = EXCLUDED.modelo, \
                prompt_usado = EXCLUDED.prompt_usado, actualizada_en = NOW() \
             RETURNING id, inmueble_id, version, titulo, descripcion, estado, modelo, \
                       prompt_usado, creada_en, actualizada_en",
        )
        .bind(id)
        .bind(inmueble_id)
        .bind(version)
        .bind(titulo)
        .bind(descripcion)
        .bind(modelo)
        .bind(prompt_usado)
        .fetch_one(pool)
        .await
    }

    pub async fn actualizar(
        pool: &PgPool,
        inmueble_id: Uuid,
        version: i16,
        titulo: Option<&str>,
        descripcion: Option<&str>,
        estado: Option<&str>,
    ) -> Result<Option<InmuebleVersion>, sqlx::Error> {
        // sentinel-disable-next-line sqlx-query-as-sin-macro -- sentencia nueva; ver cabecera
        sqlx::query_as::<_, InmuebleVersion>(
            "UPDATE inmueble_versiones SET \
                titulo = COALESCE($3, titulo), \
                descripcion = COALESCE($4, descripcion), \
                estado = COALESCE($5, estado), \
                actualizada_en = NOW() \
             WHERE inmueble_id = $1 AND version = $2 \
             RETURNING id, inmueble_id, version, titulo, descripcion, estado, modelo, \
                       prompt_usado, creada_en, actualizada_en",
        )
        .bind(inmueble_id)
        .bind(version)
        .bind(titulo)
        .bind(descripcion)
        .bind(estado)
        .fetch_optional(pool)
        .await
    }

    pub async fn borrar(
        pool: &PgPool,
        inmueble_id: Uuid,
        version: i16,
    ) -> Result<bool, sqlx::Error> {
        // sentinel-disable-next-line sqlx-query-sin-macro -- sentencia nueva; ver cabecera del módulo
        let r = sqlx::query("DELETE FROM inmueble_versiones WHERE inmueble_id = $1 AND version = $2")
            .bind(inmueble_id)
            .bind(version)
            .execute(pool)
            .await?;
        Ok(r.rows_affected() > 0)
    }

    /// Apunta en el historial un cambio de texto. Los campos van agrupados
    /// (`EntradaHistorial`) porque son muchos y todos cadenas: pasarlos sueltos
    /// invitaba a cruzar `anterior` con `nuevo` sin que el compilador dijera nada.
    pub async fn anotar_historial(
        pool: &PgPool,
        entrada: &EntradaHistorial<'_>,
    ) -> Result<(), sqlx::Error> {
        // sentinel-disable-next-line sqlx-query-sin-macro -- sentencia nueva; ver cabecera del módulo
        sqlx::query(
            "INSERT INTO inmueble_texto_historial \
                (inmueble_id, version, campo, valor_anterior, valor_nuevo, origen, modelo) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(entrada.inmueble_id)
        .bind(entrada.version)
        .bind(entrada.campo)
        .bind(entrada.anterior)
        .bind(entrada.nuevo)
        .bind(entrada.origen)
        .bind(entrada.modelo)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Variantes de texto de varios inmuebles, indexadas por inmueble (export).
    pub async fn variantes_por_inmuebles(
        pool: &PgPool,
        ids: &[Uuid],
    ) -> Result<std::collections::HashMap<Uuid, InmuebleVersion>, sqlx::Error> {
        let mut mapa = std::collections::HashMap::new();
        if ids.is_empty() {
            return Ok(mapa);
        }
        // sentinel-disable-next-line sqlx-query-as-sin-macro -- sentencia nueva; ver cabecera
        let filas = sqlx::query_as::<_, InmuebleVersion>(
            "SELECT id, inmueble_id, version, titulo, descripcion, estado, modelo,                     prompt_usado, creada_en, actualizada_en              FROM inmueble_versiones              WHERE inmueble_id = ANY($1) AND version >= 2              ORDER BY inmueble_id, version ASC",
        )
        .bind(ids)
        .fetch_all(pool)
        .await?;
        /* La primera (v2) es la que se publica como variante. */
        for fila in filas {
            mapa.entry(fila.inmueble_id).or_insert(fila);
        }
        Ok(mapa)
    }

    pub async fn historial(
        pool: &PgPool,
        inmueble_id: Uuid,
    ) -> Result<Vec<TextoHistorial>, sqlx::Error> {
        // sentinel-disable-next-line sqlx-query-as-sin-macro -- sentencia nueva; ver cabecera
        sqlx::query_as::<_, TextoHistorial>(
            "SELECT id, version, campo, valor_anterior, valor_nuevo, origen, modelo, creado_en \
             FROM inmueble_texto_historial WHERE inmueble_id = $1 \
             ORDER BY creado_en DESC LIMIT 200",
        )
        .bind(inmueble_id)
        .fetch_all(pool)
        .await
    }

    /* ---- Fotos variante ---- */

    pub async fn fotos_de(
        pool: &PgPool,
        inmueble_id: Uuid,
    ) -> Result<Vec<FotoVarianteRow>, sqlx::Error> {
        // sentinel-disable-next-line sqlx-query-as-sin-macro -- sentencia nueva; ver cabecera
        sqlx::query_as::<_, FotoVarianteRow>(
            "SELECT id, inmueble_id, orden, storage_key, version, modelo, prompt_usado, creada_en \
             FROM fotos_variantes WHERE inmueble_id = $1 ORDER BY version ASC, orden ASC",
        )
        .bind(inmueble_id)
        .fetch_all(pool)
        .await
    }

    /// Fotos variante de varios inmuebles (exportación y listas).
    pub async fn fotos_por_inmuebles(
        pool: &PgPool,
        ids: &[Uuid],
        version: Option<i16>,
    ) -> Result<Vec<FotoVarianteRow>, sqlx::Error> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        // sentinel-disable-next-line sqlx-query-as-sin-macro -- sentencia nueva; ver cabecera
        sqlx::query_as::<_, FotoVarianteRow>(
            "SELECT id, inmueble_id, orden, storage_key, version, modelo, prompt_usado, creada_en \
             FROM fotos_variantes \
             WHERE inmueble_id = ANY($1) AND ($2::SMALLINT IS NULL OR version = $2) \
             ORDER BY inmueble_id, version ASC, orden ASC",
        )
        .bind(ids)
        .bind(version)
        .fetch_all(pool)
        .await
    }

    /// Guarda la variante de una foto. Reemplaza la que hubiera en ese
    /// `orden` para la misma versión y devuelve la clave anterior para que el
    /// llamador borre el archivo (fuera de la transacción).
    pub async fn upsert_foto(
        pool: &PgPool,
        inmueble_id: Uuid,
        orden: i32,
        version: i16,
        storage_key: &str,
        modelo: &str,
        prompt_usado: &str,
    ) -> Result<Option<String>, sqlx::Error> {
        // sentinel-disable-next-line sqlx-query-sin-macro -- sentencia nueva; ver cabecera del módulo
        let anterior: Option<String> = sqlx::query_scalar(
            "DELETE FROM fotos_variantes WHERE inmueble_id = $1 AND orden = $2 AND version = $3 \
             RETURNING storage_key",
        )
        .bind(inmueble_id)
        .bind(orden)
        .bind(version)
        .fetch_optional(pool)
        .await?;
        // sentinel-disable-next-line sqlx-query-sin-macro -- sentencia nueva; ver cabecera del módulo
        sqlx::query(
            "INSERT INTO fotos_variantes (inmueble_id, orden, version, storage_key, modelo, prompt_usado) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(inmueble_id)
        .bind(orden)
        .bind(version)
        .bind(storage_key)
        .bind(modelo)
        .bind(prompt_usado)
        .execute(pool)
        .await?;
        Ok(anterior)
    }

    pub async fn borrar_foto(
        pool: &PgPool,
        inmueble_id: Uuid,
        orden: i32,
        version: i16,
    ) -> Result<Option<String>, sqlx::Error> {
        // sentinel-disable-next-line sqlx-query-sin-macro -- sentencia nueva; ver cabecera del módulo
        sqlx::query_scalar(
            "DELETE FROM fotos_variantes WHERE inmueble_id = $1 AND orden = $2 AND version = $3 \
             RETURNING storage_key",
        )
        .bind(inmueble_id)
        .bind(orden)
        .bind(version)
        .fetch_optional(pool)
        .await
    }

    /// Claves de todas las variantes de un inmueble (limpieza al borrarlo).
    pub async fn borrar_todas_fotos(
        pool: &PgPool,
        inmueble_id: Uuid,
    ) -> Result<Vec<String>, sqlx::Error> {
        // sentinel-disable-next-line sqlx-query-sin-macro -- sentencia nueva; ver cabecera del módulo
        sqlx::query_scalar(
            "DELETE FROM fotos_variantes WHERE inmueble_id = $1 RETURNING storage_key",
        )
        .bind(inmueble_id)
        .fetch_all(pool)
        .await
    }
}
