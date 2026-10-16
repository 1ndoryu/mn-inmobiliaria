//! Acceso a `fotos` [15AA-8]: alta, reordenado y borrado en cascada.
//!
//! Separado de `repositories::inmueble` por dominio (el fichero padre pasaba
//! del límite de líneas). Las funciones siguen siendo métodos de
//! [`InmuebleRepository`]: se llaman igual desde los servicios.
//!
//! Las sentencias que trabajan con arrays (`unnest($1::uuid[], $2::int[])`)
//! o con borrados encadenados van en runtime (`sqlx::query`), sin macro: la
//! caché `.sqlx` no las cubre y no queremos obligar a regenerarla.

use std::collections::HashMap;

use sqlx::PgPool;
use uuid::Uuid;

use crate::models::Foto;

use super::InmuebleRepository;

impl InmuebleRepository {
    /// Fotos de varios inmuebles en una sola query (evita N+1)
    pub async fn fotos_por_inmuebles(
        pool: &PgPool,
        ids: &[Uuid],
    ) -> Result<HashMap<Uuid, Vec<Foto>>, sqlx::Error> {
        let mut mapa: HashMap<Uuid, Vec<Foto>> = HashMap::new();
        if ids.is_empty() {
            return Ok(mapa);
        }

        let fotos = sqlx::query_as!(
            Foto,
            "SELECT id, inmueble_id, storage_key, orden, origen, created_at \
             FROM fotos WHERE inmueble_id = ANY($1) ORDER BY orden ASC",
            ids,
        )
        .fetch_all(pool)
        .await?;

        for foto in fotos {
            mapa.entry(foto.inmueble_id).or_default().push(foto);
        }
        Ok(mapa)
    }

    pub async fn add_foto(
        pool: &PgPool,
        inmueble_id: Uuid,
        storage_key: &str,
        orden: Option<i32>,
        origen: &str,
    ) -> Result<Foto, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as!(
            Foto,
            "INSERT INTO fotos (id, inmueble_id, storage_key, orden, origen) \
             VALUES ($1, $2, $3, \
              COALESCE($4, (SELECT COALESCE(MAX(orden), -1) + 1 FROM fotos WHERE inmueble_id = $2)), \
              $5) \
             RETURNING id, inmueble_id, storage_key, orden, origen, created_at",
            id,
            inmueble_id,
            storage_key,
            orden,
            origen,
        )
        .fetch_one(pool)
        .await
    }

    /* [F3 de 14AA-2] Sustituye el archivo de una foto sin tocar su fila ni su
     * `orden`. Borrar y volver a subir en el mismo `orden` no sirve: el borrado
     * renumera el álbum ([08AA-4]) y la nueva chocaría con las de arriba. Va en
     * runtime porque la caché `.sqlx` no cubre esta sentencia. */
    pub async fn cambiar_archivo_foto(
        pool: &PgPool,
        foto_id: Uuid,
        storage_key: &str,
    ) -> Result<Foto, sqlx::Error> {
        // sentinel-disable-next-line sqlx-query-as-sin-macro -- sentencia nueva; ver cabecera del módulo
        sqlx::query_as::<_, Foto>(
            "UPDATE fotos SET storage_key = $1 WHERE id = $2 \
             RETURNING id, inmueble_id, storage_key, orden, origen, created_at",
        )
        .bind(storage_key)
        .bind(foto_id)
        .fetch_one(pool)
        .await
    }

    /* [15AA-8] Reordena los huecos de fotos sin borrar ni subir nada: un
     * hueco es una `original` y, si la tiene, su `mejorada` hermana (mismo
     * `orden`). `ids` son los ids de las originales en el orden deseado y
     * debe ser una permutación exacta de las que hay; si no, devuelve
     * `false` y la transacción no toca nada. Reordenar no puede perder
     * fotos: antes el front las borraba todas y las resubía, y un corte a
     * mitad las dejaba sin fila ni archivo. */
    pub async fn reordenar_fotos(
        pool: &PgPool,
        inmueble_id: Uuid,
        ids: &[Uuid],
    ) -> Result<bool, sqlx::Error> {
        let mut tx = pool.begin().await?;
        // sentinel-disable-next-line sqlx-query-as-sin-macro -- lectura de los huecos actuales; ver cabecera del módulo
        let actuales: Vec<(Uuid, i32)> = sqlx::query_as(
            "SELECT id, orden FROM fotos WHERE inmueble_id = $1 AND origen = 'original'",
        )
        .bind(inmueble_id)
        .fetch_all(&mut *tx)
        .await?;
        let mut orden_de: HashMap<Uuid, i32> = HashMap::with_capacity(actuales.len());
        for (id, orden) in actuales {
            orden_de.insert(id, orden);
        }
        /* Huecos: (orden viejo, orden nuevo). Se mueve por `orden` viejo
         * para que la mejorada hermana viaje con su original. */
        let mut huecos: Vec<(i32, i32)> = Vec::with_capacity(ids.len());
        let mut vistos: HashMap<Uuid, ()> = HashMap::with_capacity(ids.len());
        for (nuevo, id) in ids.iter().enumerate() {
            let (Some(viejo), Ok(nuevo), None) =
                (orden_de.get(id), i32::try_from(nuevo), vistos.insert(*id, ()))
            else {
                return Ok(false);
            };
            huecos.push((*viejo, nuevo));
        }
        if orden_de.len() != huecos.len() {
            return Ok(false);
        }
        /* Sin fotos no hay nada que reordenar (y un `VALUES` vacío no es SQL). */
        if huecos.is_empty() {
            return Ok(true);
        }
        /* Un solo UPDATE por tabla, con las parejas como arrays (`unnest`):
         * dentro de una sentencia Postgres compara todas las filas con la
         * imagen previa, así que un intercambio (0↔1) no se pisa. Las
         * mejoradas van después y siguen leyendo su `orden` viejo, que
         * nadie tocó. Consultas en runtime: la caché `.sqlx` no las cubre. */
        let (ids_ordenados, nuevos): (Vec<Uuid>, Vec<i32>) = ids
            .iter()
            .zip(huecos.iter())
            .map(|(id, (_, nuevo))| (*id, *nuevo))
            .unzip();
        // sentinel-disable-next-line sqlx-query-sin-macro -- `unnest` de arrays; ver cabecera del módulo
        sqlx::query(
            "UPDATE fotos SET orden = m.nuevo \
             FROM unnest($1::uuid[], $2::int[]) AS m(id, nuevo) \
             WHERE fotos.inmueble_id = $3 AND fotos.origen = 'original' AND fotos.id = m.id",
        )
        .bind(&ids_ordenados)
        .bind(&nuevos)
        .bind(inmueble_id)
        .execute(&mut *tx)
        .await?;

        let (viejos, nuevos_mejora): (Vec<i32>, Vec<i32>) = huecos.iter().copied().unzip();
        // sentinel-disable-next-line sqlx-query-sin-macro -- `unnest` de arrays; ver cabecera del módulo
        sqlx::query(
            "UPDATE fotos SET orden = m.nuevo \
             FROM unnest($1::int[], $2::int[]) AS m(viejo, nuevo) \
             WHERE fotos.inmueble_id = $3 AND fotos.origen = 'mejorada' AND fotos.orden = m.viejo",
        )
        .bind(&viejos)
        .bind(&nuevos_mejora)
        .bind(inmueble_id)
        .execute(&mut *tx)
        .await?;
        /* [249A-1] Invalida la caché de fotos (`?v=<updated_at>`). */
        Self::tocar_inmueble_tx(&mut tx, inmueble_id).await?;
        tx.commit().await?;
        Ok(true)
    }

    /* [08AA-4] Borrado de foto sin desfase: si cae una `original`, su
     * `mejorada` hermana (mismo `inmueble_id` + `orden`) cae en la misma
     * transacción y el resto se renumera para mantener `orden` denso. Sin
     * esto el pareo original↔mejorada por `orden` se cruza (Altos del
     * Caroní: 15 mejoradas para 12 originales + 3 duplicadas). Borrar solo
     * la mejorada no toca el original (caso "regenerar"). Devuelve las
     * `storage_key` eliminadas para limpiar disco fuera de la transacción. */
    pub async fn delete_foto_en_cascada(
        pool: &PgPool,
        foto_id: Uuid,
    ) -> Result<Option<Vec<String>>, sqlx::Error> {
        let Some(foto) = Self::find_foto(pool, foto_id).await? else {
            return Ok(None);
        };
        let mut tx = pool.begin().await?;
        let claves: Vec<String> = if foto.origen == "original" {
            /* [13AA-12] Solo la original (por id) y su mejorada hermana. Borrar por
             * `orden` a secas arrastraba otra original con el mismo orden: al
             * sincronizar se perdían fotos sin que nadie las borrara. Consultas en
             * runtime: la caché `.sqlx` no cubre estas sentencias. */
            // sentinel-disable-next-line sqlx-query-sin-macro -- `query_scalar` con RETURNING; ver cabecera del módulo
            let mut claves = sqlx::query_scalar::<_, String>(
                "DELETE FROM fotos WHERE id = $1 RETURNING storage_key",
            )
            .bind(foto_id)
            .fetch_all(&mut *tx)
            .await?;
            claves.extend(
                // sentinel-disable-next-line sqlx-query-sin-macro -- `query_scalar` con RETURNING; ver cabecera del módulo
                sqlx::query_scalar::<_, String>(
                    "DELETE FROM fotos WHERE inmueble_id = $1 AND orden = $2 \
                     AND origen = 'mejorada' RETURNING storage_key",
                )
                .bind(foto.inmueble_id)
                .bind(foto.orden)
                .fetch_all(&mut *tx)
                .await?,
            );
            sqlx::query!(
                "UPDATE fotos SET orden = orden - 1 \
                 WHERE inmueble_id = $1 AND orden > $2",
                foto.inmueble_id,
                foto.orden,
            )
            .execute(&mut *tx)
            .await?;
            claves
        } else {
            sqlx::query!("DELETE FROM fotos WHERE id = $1", foto_id)
                .execute(&mut *tx)
                .await?;
            vec![foto.storage_key]
        };
        /* [249A-1] Invalida la caché de fotos (`?v=<updated_at>`). */
        Self::tocar_inmueble_tx(&mut tx, foto.inmueble_id).await?;
        tx.commit().await?;
        Ok(Some(claves))
    }
}
