use chrono::NaiveDate;
use sqlx::{PgExecutor, PgPool};
use uuid::Uuid;

use crate::models::{PiezaRow, PublicadaRow, PublicacionRow};

/* [28AA-2] Persistencia del calendario de redes. Aquí solo hay SQL: qué
 * estado sigue a qué lo decide el servicio. La cola de inmuebles (regla 1
 * del plan) se resuelve en una sola consulta. Todo literal para que
 * `query!` valide contra la BD (.sqlx). */

/// Pieza nueva, ya validada (formato y bytes) por el servicio
pub struct NuevaPieza {
    pub publicacion_id: Uuid,
    pub formato: String,
    pub storage_key: String,
    pub ancho: i32,
    pub alto: i32,
    pub bytes: i32,
}

pub struct PublicacionRepository;

impl PublicacionRepository {
    /// Siguiente inmueble a proponer. Primero los que nunca se publicaron; después
    /// el día de publicación más antiguo; dentro de ese día, sorteo aleatorio.
    /// `excluir` evita volver a proponer el que se acaba de saltar.
    /// Acepta un ejecutor genérico para que las pruebas lo corran dentro de una
    /// transacción que se deshace (no toca el resto de la BD de pruebas).
    pub async fn siguiente_inmueble<'c>(
        executor: impl PgExecutor<'c>,
        excluir: Option<Uuid>,
    ) -> Result<Option<Uuid>, sqlx::Error> {
        sqlx::query_scalar!(
            r#"SELECT i.id
               FROM inmuebles i
               LEFT JOIN LATERAL (
                   SELECT MAX(p.publicada_en) AS ultima
                   FROM publicaciones_redes p
                   WHERE p.inmueble_id = i.id AND p.estado = 'publicada'
               ) u ON TRUE
               WHERE i.publicado = TRUE
                 AND i.estado = 'disponible'
                 AND ($1::UUID IS NULL OR i.id <> $1)
               ORDER BY (u.ultima)::DATE ASC NULLS FIRST, random()
               LIMIT 1"#,
            excluir
        )
        .fetch_optional(executor)
        .await
    }

    /// Borrador nuevo para un inmueble propuesto
    pub async fn crear_borrador(
        pool: &PgPool,
        inmueble_id: Uuid,
        programada_para: Option<NaiveDate>,
    ) -> Result<PublicacionRow, sqlx::Error> {
        sqlx::query_as!(
            PublicacionRow,
            r#"INSERT INTO publicaciones_redes (inmueble_id, programada_para)
               VALUES ($1, $2)
               RETURNING id, inmueble_id, estado, descripcion, tags, prompt_usado, modelo,
                         ig_post, fanpage, ig_historia, fb_perfil, programada_para,
                         publicada_en, creada_en"#,
            inmueble_id,
            programada_para
        )
        .fetch_one(pool)
        .await
    }

    /// Registro de «saltado»: cuenta como visto, no como publicado
    pub async fn saltar(pool: &PgPool, inmueble_id: Uuid) -> Result<PublicacionRow, sqlx::Error> {
        sqlx::query_as!(
            PublicacionRow,
            r#"INSERT INTO publicaciones_redes (inmueble_id, estado)
               VALUES ($1, 'saltada')
               RETURNING id, inmueble_id, estado, descripcion, tags, prompt_usado, modelo,
                         ig_post, fanpage, ig_historia, fb_perfil, programada_para,
                         publicada_en, creada_en"#,
            inmueble_id
        )
        .fetch_one(pool)
        .await
    }

    pub async fn find(pool: &PgPool, id: Uuid) -> Result<Option<PublicacionRow>, sqlx::Error> {
        sqlx::query_as!(
            PublicacionRow,
            r#"SELECT id, inmueble_id, estado, descripcion, tags, prompt_usado, modelo,
                      ig_post, fanpage, ig_historia, fb_perfil, programada_para,
                      publicada_en, creada_en
               FROM publicaciones_redes WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    /// Escribe el estado completo de la publicación. El servicio carga, cambia
    /// lo pedido y guarda; así las reglas de estado no se repiten en SQL.
    pub async fn guardar(
        pool: &PgPool,
        p: PublicacionRow,
    ) -> Result<Option<PublicacionRow>, sqlx::Error> {
        sqlx::query_as!(
            PublicacionRow,
            r#"UPDATE publicaciones_redes
               SET estado = $2, descripcion = $3, tags = $4, prompt_usado = $5,
                   modelo = $6, ig_post = $7, fanpage = $8, ig_historia = $9,
                   fb_perfil = $10, programada_para = $11, publicada_en = $12
               WHERE id = $1
               RETURNING id, inmueble_id, estado, descripcion, tags, prompt_usado, modelo,
                         ig_post, fanpage, ig_historia, fb_perfil, programada_para,
                         publicada_en, creada_en"#,
            p.id,
            p.estado,
            p.descripcion,
            &p.tags,
            p.prompt_usado,
            p.modelo,
            p.ig_post,
            p.fanpage,
            p.ig_historia,
            p.fb_perfil,
            p.programada_para,
            p.publicada_en
        )
        .fetch_optional(pool)
        .await
    }

    /// Publicaciones que caen en un rango de días del calendario: el día previsto
    /// o, si ya salieron, el día real. Las saltadas no se muestran.
    pub async fn listar_entre(
        pool: &PgPool,
        desde: NaiveDate,
        hasta: NaiveDate,
    ) -> Result<Vec<PublicacionRow>, sqlx::Error> {
        sqlx::query_as!(
            PublicacionRow,
            r#"SELECT id, inmueble_id, estado, descripcion, tags, prompt_usado, modelo,
                      ig_post, fanpage, ig_historia, fb_perfil, programada_para,
                      publicada_en, creada_en
               FROM publicaciones_redes
               WHERE estado <> 'saltada'
                 AND COALESCE(programada_para, publicada_en::DATE) BETWEEN $1 AND $2
               ORDER BY creada_en"#,
            desde,
            hasta
        )
        .fetch_all(pool)
        .await
    }

    /// Historial de un inmueble: cada vez que salió publicado, más reciente primero
    /// Acepta un ejecutor genérico, igual que `siguiente_inmueble`, para probarlo en transacción
    pub async fn historial_de<'c>(
        executor: impl PgExecutor<'c>,
        inmueble_id: Uuid,
    ) -> Result<Vec<PublicacionRow>, sqlx::Error> {
        sqlx::query_as!(
            PublicacionRow,
            r#"SELECT id, inmueble_id, estado, descripcion, tags, prompt_usado, modelo,
                      ig_post, fanpage, ig_historia, fb_perfil, programada_para,
                      publicada_en, creada_en
               FROM publicaciones_redes
               WHERE inmueble_id = $1 AND estado = 'publicada'
               ORDER BY publicada_en DESC NULLS LAST, creada_en DESC"#,
            inmueble_id
        )
        .fetch_all(executor)
        .await
    }

    /* [28AA-7] Una publicación por inmueble visible: la más reciente que salió.
     * `DISTINCT ON` toma la primera fila de cada inmueble con el orden dado; el
     * orden final (más reciente primero) lo pone el servicio. Acepta ejecutor
     * genérico para probarlo dentro de una transacción. `publicada_en!`: el
     * WHERE garantiza que no es nulo, aunque la columna lo admita. */
    pub async fn listar_publicadas<'c>(
        executor: impl PgExecutor<'c>,
    ) -> Result<Vec<PublicadaRow>, sqlx::Error> {
        sqlx::query_as!(
            PublicadaRow,
            r#"SELECT DISTINCT ON (p.inmueble_id)
                      p.id AS publicacion_id, p.inmueble_id, i.titulo, i.slug,
                      p.descripcion, p.tags, p.publicada_en AS "publicada_en!"
               FROM publicaciones_redes p
               JOIN inmuebles i ON i.id = p.inmueble_id
               WHERE p.estado = 'publicada' AND p.publicada_en IS NOT NULL
                 AND i.publicado = TRUE
               ORDER BY p.inmueble_id, p.publicada_en DESC"#
        )
        .fetch_all(executor)
        .await
    }

    /// Títulos de los inmuebles de una lista, para completar la vista sin N+1
    pub async fn titulos(pool: &PgPool, ids: Vec<Uuid>) -> Result<Vec<(Uuid, String)>, sqlx::Error> {
        let filas = sqlx::query!(
            "SELECT id, titulo FROM inmuebles WHERE id = ANY($1)",
            &ids
        )
        .fetch_all(pool)
        .await?;
        Ok(filas.into_iter().map(|f| (f.id, f.titulo)).collect())
    }

    pub async fn piezas_de(pool: &PgPool, publicacion_ids: Vec<Uuid>) -> Result<Vec<PiezaRow>, sqlx::Error> {
        sqlx::query_as!(
            PiezaRow,
            r#"SELECT id, publicacion_id, formato, storage_key, ancho, alto, bytes, creada_en
               FROM publicacion_piezas
               WHERE publicacion_id = ANY($1)
               ORDER BY creada_en"#,
            &publicacion_ids
        )
        .fetch_all(pool)
        .await
    }

    pub async fn anadir_pieza(pool: &PgPool, nueva: NuevaPieza) -> Result<PiezaRow, sqlx::Error> {
        sqlx::query_as!(
            PiezaRow,
            r#"INSERT INTO publicacion_piezas (publicacion_id, formato, storage_key, ancho, alto, bytes)
               VALUES ($1, $2, $3, $4, $5, $6)
               RETURNING id, publicacion_id, formato, storage_key, ancho, alto, bytes, creada_en"#,
            nueva.publicacion_id,
            nueva.formato,
            nueva.storage_key,
            nueva.ancho,
            nueva.alto,
            nueva.bytes
        )
        .fetch_one(pool)
        .await
    }

    pub async fn find_pieza(pool: &PgPool, id: Uuid) -> Result<Option<PiezaRow>, sqlx::Error> {
        sqlx::query_as!(
            PiezaRow,
            r#"SELECT id, publicacion_id, formato, storage_key, ancho, alto, bytes, creada_en
               FROM publicacion_piezas WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    /// Borra la publicación; sus piezas caen con ella (ON DELETE CASCADE).
    /// Devuelve `true` si existía
    pub async fn borrar(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
        let r = sqlx::query!("DELETE FROM publicaciones_redes WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(r.rows_affected() > 0)
    }

    /// Devuelve `true` si había una pieza con ese id
    pub async fn borrar_pieza(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
        let r = sqlx::query!("DELETE FROM publicacion_piezas WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(r.rows_affected() > 0)
    }
}

#[cfg(test)]
mod pruebas;
