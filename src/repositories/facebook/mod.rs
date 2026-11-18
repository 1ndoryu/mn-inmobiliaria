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
    i16,
    Option<String>,
    Option<chrono::DateTime<chrono::Utc>>,
);

/// Columnas del JOIN, en el orden de `VinculoFacebook`: una sola lista para
/// que `vinculos_de` y los dos `vinculos_por_inmuebles` no se desalineen.
const COLUMNAS_VINCULO: &str = "f.id AS facebook_id, f.nombre, f.url, f.principal, \
                                v.version, v.marketplace_id, v.publicado_en";

pub struct FacebookRepository;

impl FacebookRepository {
    pub async fn listar(pool: &PgPool) -> Result<Vec<FacebookRow>, sqlx::Error> {
        // sentinel-disable-next-line sqlx-query-as-sin-macro -- sentencia nueva; ver cabecera
        sqlx::query_as::<_, FacebookRow>(
            "SELECT id, nombre, url, notas, principal, version, creado_en \
             FROM facebook ORDER BY version ASC NULLS LAST, nombre ASC",
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<FacebookRow>, sqlx::Error> {
        // sentinel-disable-next-line sqlx-query-as-sin-macro -- sentencia nueva; ver cabecera
        sqlx::query_as::<_, FacebookRow>(
            "SELECT id, nombre, url, notas, principal, version, creado_en FROM facebook WHERE id = $1",
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
        version: Option<i16>,
    ) -> Result<FacebookRow, sqlx::Error> {
        let id = Uuid::new_v4();
        let mut tx = pool.begin().await?;
        if principal {
            Self::quitar_principal_tx(&mut tx).await?;
        }
        // sentinel-disable-next-line sqlx-query-as-sin-macro -- sentencia nueva; ver cabecera
        let fila = sqlx::query_as::<_, FacebookRow>(
            "INSERT INTO facebook (id, nombre, url, notas, principal, version) \
             VALUES ($1, $2, $3, $4, $5, $6) \
             RETURNING id, nombre, url, notas, principal, version, creado_en",
        )
        .bind(id)
        .bind(nombre)
        .bind(url)
        .bind(notas)
        .bind(principal)
        .bind(version)
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
        version: Option<i16>,
    ) -> Result<Option<FacebookRow>, sqlx::Error> {
        let mut tx = pool.begin().await?;
        if principal {
            Self::quitar_principal_tx(&mut tx).await?;
        }
        // sentinel-disable-next-line sqlx-query-as-sin-macro -- sentencia nueva; ver cabecera
        let fila = sqlx::query_as::<_, FacebookRow>(
            "UPDATE facebook SET nombre = $2, url = $3, notas = $4, principal = $5, version = $6 \
             WHERE id = $1 \
             RETURNING id, nombre, url, notas, principal, version, creado_en",
        )
        .bind(id)
        .bind(nombre)
        .bind(url)
        .bind(notas)
        .bind(principal)
        .bind(version)
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
            "SELECT id, nombre, url, notas, principal, version, creado_en \
             FROM facebook WHERE principal LIMIT 1",
        )
        .fetch_optional(pool)
        .await
    }

    /// [30AA-13] La cuenta dueña de una versión (`None` = ninguna cuenta la
    /// tiene asignada, así que esa versión no se exporta).
    pub async fn cuenta_de_version(
        pool: &PgPool,
        version: i16,
    ) -> Result<Option<FacebookRow>, sqlx::Error> {
        // sentinel-disable-next-line sqlx-query-as-sin-macro -- sentencia nueva; ver cabecera
        sqlx::query_as::<_, FacebookRow>(
            "SELECT id, nombre, url, notas, principal, version, creado_en \
             FROM facebook WHERE version = $1 LIMIT 1",
        )
        .bind(version)
        .fetch_optional(pool)
        .await
    }

    /// [30AA-13] Versión → cuenta, de una sola consulta (el export lo usa para
    /// decidir en qué cuenta se mide cada grupo).
    pub async fn cuentas_por_version(
        pool: &PgPool,
    ) -> Result<std::collections::HashMap<i16, Uuid>, sqlx::Error> {
        // sentinel-disable-next-line sqlx-query-as-sin-macro -- sentencia nueva; ver cabecera
        let filas: Vec<(i16, Uuid)> = sqlx::query_as(
            "SELECT version, id FROM facebook WHERE version IS NOT NULL",
        )
        .fetch_all(pool)
        .await?;
        Ok(filas.into_iter().collect())
    }

    /// [30AA-13] ¿Qué versiones tienen aviso en SU cuenta propia? Clave
    /// `(inmueble, version)`; solo las que tienen fila en la cuenta de esa
    /// versión con el ID del aviso puesto. Es lo que separa publicados de
    /// pendientes en el export.
    pub async fn avisos_en_la_cuenta_de_su_version(
        pool: &PgPool,
        ids: &[Uuid],
    ) -> Result<std::collections::HashSet<(Uuid, i16)>, sqlx::Error> {
        if ids.is_empty() {
            return Ok(std::collections::HashSet::new());
        }
        // sentinel-disable-next-line sqlx-query-as-sin-macro -- sentencia nueva; ver cabecera
        let filas: Vec<(Uuid, i16)> = sqlx::query_as(
            "SELECT v.inmueble_id, v.version FROM inmueble_facebook v \
             JOIN facebook f ON f.id = v.facebook_id \
             WHERE v.inmueble_id = ANY($1) AND f.version = v.version \
               AND v.marketplace_id IS NOT NULL AND btrim(v.marketplace_id) <> ''",
        )
        .bind(ids)
        .fetch_all(pool)
        .await?;
        Ok(filas.into_iter().collect())
    }

    /// [30AA-18] Inmuebles con aviso puesto en CUALQUIER cuenta para esa
    /// versión. La v0 (originales) no tiene cuenta dueña: su aviso vale el
    /// que se publicó, sin importar desde qué cuenta. Las demás siguen
    /// midiéndose en la suya (`avisos_en_la_cuenta_de_su_version`).
    pub async fn avisos_de_version(
        pool: &PgPool,
        ids: &[Uuid],
        version: i16,
    ) -> Result<std::collections::HashSet<Uuid>, sqlx::Error> {
        if ids.is_empty() {
            return Ok(std::collections::HashSet::new());
        }
        // sentinel-disable-next-line sqlx-query-as-sin-macro -- sentencia nueva; ver cabecera
        let filas: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT DISTINCT inmueble_id FROM inmueble_facebook \
             WHERE inmueble_id = ANY($1) AND version = $2 \
               AND marketplace_id IS NOT NULL AND btrim(marketplace_id) <> ''",
        )
        .bind(ids)
        .bind(version)
        .fetch_all(pool)
        .await?;
        Ok(filas.into_iter().map(|(id,)| id).collect())
    }

    /* ---- Vínculos inmueble ↔ cuenta ---- */

    pub async fn vinculos_de(
        pool: &PgPool,
        inmueble_id: Uuid,
    ) -> Result<Vec<VinculoFacebook>, sqlx::Error> {
        // sentinel-disable-next-line sqlx-query-as-sin-macro -- sentencia nueva; ver cabecera
        sqlx::query_as::<_, VinculoFacebook>(&format!(
            "SELECT {COLUMNAS_VINCULO} \
             FROM inmueble_facebook v JOIN facebook f ON f.id = v.facebook_id \
             WHERE v.inmueble_id = $1 \
             ORDER BY v.version ASC, f.principal DESC, f.nombre ASC"
        ))
        .bind(inmueble_id)
        .fetch_all(pool)
        .await
    }

    /// Versiones **que existen en la ficha** sin ningún aviso vinculado.
    ///
    /// Una versión cuenta como cubierta si tiene fila en `inmueble_facebook`,
    /// aunque todavía no tenga el ID del anuncio: el ID se pega después de
    /// publicar y la cuenta ya está decidida.
    ///
    /// Solo se echan en falta las versiones que la ficha tiene: la 1 siempre, y
    /// de la 2 en adelante las que tengan texto o fotos variante (con el tope
    /// de `configuradas`, por si la configuración bajó). Un inmueble sin v2 no
    /// «debe» v2.
    pub async fn faltan_avisos(
        pool: &PgPool,
        inmueble_id: Uuid,
        configuradas: i16,
    ) -> Result<Vec<i16>, sqlx::Error> {
        // sentinel-disable-next-line sqlx-query-as-sin-macro -- sentencia nueva; ver cabecera
        /* [30AA-13] Solo cubre el vínculo con la cuenta dueña de esa versión:
         * la v1 va a Maye y la v2 a Wan, así que tener la v2 apuntada a Maye
         * no publica nada. */
        let cubiertas: Vec<(i16,)> = sqlx::query_as(
            "SELECT DISTINCT v.version FROM inmueble_facebook v \
             JOIN facebook f ON f.id = v.facebook_id \
             WHERE v.inmueble_id = $1 \
               AND (f.version = v.version OR v.version = 0)",
        )
        .bind(inmueble_id)
        .fetch_all(pool)
        .await?;
        let mut propias = propias_por_inmueble(pool, &[inmueble_id]).await?;
        let propias = propias.remove(&inmueble_id).unwrap_or_default();
        Ok(sin_aviso(configuradas, &propias, &cubiertas))
    }

    /// Lo mismo para muchos inmuebles en una sola query (la tabla los pinta
    /// todos de golpe; sin esto serían N peticiones).
    pub async fn faltan_avisos_por_inmuebles(
        pool: &PgPool,
        ids: &[Uuid],
        configuradas: i16,
    ) -> Result<std::collections::HashMap<Uuid, Vec<i16>>, sqlx::Error> {
        let mut mapa: std::collections::HashMap<Uuid, Vec<i16>> =
            std::collections::HashMap::new();
        if ids.is_empty() {
            return Ok(mapa);
        }
        // sentinel-disable-next-line sqlx-query-as-sin-macro -- sentencia nueva; ver cabecera
        /* [30AA-13] Igual que `faltan_avisos`: cubre solo la cuenta dueña de
         * la propia versión. */
        let cubiertas_filas: Vec<(Uuid, i16)> = sqlx::query_as(
            "SELECT DISTINCT v.inmueble_id, v.version FROM inmueble_facebook v \
             JOIN facebook f ON f.id = v.facebook_id \
             WHERE v.inmueble_id = ANY($1) \
               AND (f.version = v.version OR v.version = 0)",
        )
        .bind(ids)
        .fetch_all(pool)
        .await?;
        let propias = propias_por_inmueble(pool, ids).await?;
        let mut cubiertas: std::collections::HashMap<Uuid, Vec<(i16,)>> =
            std::collections::HashMap::new();
        for (id, version) in cubiertas_filas {
            cubiertas.entry(id).or_default().push((version,));
        }
        let mut propias = propias;
        for id in ids {
            let de_este_cubiertas = cubiertas.remove(id).unwrap_or_default();
            let de_este_propias = propias.remove(id).unwrap_or_default();
            mapa.insert(*id, sin_aviso(configuradas, &de_este_propias, &de_este_cubiertas));
        }
        Ok(mapa)
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
            sqlx::query_as(&format!(
                "SELECT v.inmueble_id, {COLUMNAS_VINCULO} \
                 FROM inmueble_facebook v JOIN facebook f ON f.id = v.facebook_id \
                 WHERE v.inmueble_id = ANY($1) \
                 ORDER BY v.version ASC, f.principal DESC, f.nombre ASC"
            ))
            .bind(ids)
            .fetch_all(pool)
            .await?;
        for (
            inmueble_id,
            facebook_id,
            nombre,
            url,
            principal,
            version,
            marketplace_id,
            publicado_en,
        ) in filas
        {
            mapa.entry(inmueble_id).or_default().push(VinculoFacebook {
                facebook_id,
                nombre,
                url,
                principal,
                version,
                marketplace_id,
                publicado_en,
            });
        }
        Ok(mapa)
    }

    /// Fija (o quita, con `None`) el aviso de una VERSIÓN en una cuenta.
    /// Devuelve `false` si esa cuenta no estaba vinculada en esa versión.
    pub async fn fijar_aviso(
        pool: &PgPool,
        inmueble_id: Uuid,
        facebook_id: Uuid,
        version: i16,
        marketplace_id: Option<&str>,
    ) -> Result<bool, sqlx::Error> {
        // sentinel-disable-next-line sqlx-query-sin-macro -- sentencia nueva; ver cabecera del módulo
        let r = sqlx::query(
            "UPDATE inmueble_facebook SET marketplace_id = $4, \
                publicado_en = CASE WHEN $4::TEXT IS NULL THEN NULL ELSE COALESCE(publicado_en, NOW()) END \
             WHERE inmueble_id = $1 AND facebook_id = $2 AND version = $3",
        )
        .bind(inmueble_id)
        .bind(facebook_id)
        .bind(version)
        .bind(marketplace_id)
        .execute(pool)
        .await?;
        Ok(r.rows_affected() > 0)
    }

    /// Vincula (idempotente) una versión de un inmueble a una cuenta.
    pub async fn vincular(
        pool: &PgPool,
        inmueble_id: Uuid,
        facebook_id: Uuid,
        version: i16,
        marketplace_id: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        // sentinel-disable-next-line sqlx-query-sin-macro -- sentencia nueva; ver cabecera del módulo
        sqlx::query(
            "INSERT INTO inmueble_facebook (inmueble_id, facebook_id, version, marketplace_id) \
             VALUES ($1, $2, $3, $4) \
             ON CONFLICT (inmueble_id, facebook_id, version) \
             DO UPDATE SET marketplace_id = EXCLUDED.marketplace_id",
        )
        .bind(inmueble_id)
        .bind(facebook_id)
        .bind(version)
        .bind(marketplace_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn desvincular(
        pool: &PgPool,
        inmueble_id: Uuid,
        facebook_id: Uuid,
        version: i16,
    ) -> Result<bool, sqlx::Error> {
        // sentinel-disable-next-line sqlx-query-sin-macro -- sentencia nueva; ver cabecera del módulo
        let r = sqlx::query(
            "DELETE FROM inmueble_facebook \
             WHERE inmueble_id = $1 AND facebook_id = $2 AND version = $3",
        )
        .bind(inmueble_id)
        .bind(facebook_id)
        .bind(version)
        .execute(pool)
        .await?;
        Ok(r.rows_affected() > 0)
    }

    /// Espeja en `inmuebles.marketplace_id` el aviso de la **v1** en la cuenta
    /// principal (`NULL` si no está vinculada o no tiene aviso).
    ///
    /// El espejo es de la v1 a propósito: es lo que leen el laboratorio de
    /// Marketplace y el chat, que emparejan el aviso con la ficha publicada.
    pub async fn sincronizar_espejo_principal(
        pool: &PgPool,
        inmueble_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        // sentinel-disable-next-line sqlx-query-sin-macro -- sentencia nueva; ver cabecera del módulo
        sqlx::query(
            "UPDATE inmuebles i SET marketplace_id = ( \
                SELECT v.marketplace_id FROM inmueble_facebook v \
                JOIN facebook f ON f.id = v.facebook_id \
                WHERE v.inmueble_id = i.id AND v.version = 1 AND f.principal LIMIT 1) \
             WHERE i.id = $1",
        )
        .bind(inmueble_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Si el aviso de esta cuenta ya lo ocupa otra ficha (para el 422 previo
    /// al INSERT). El aviso es único por cuenta, de cualquier versión.
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

/// Versiones que la ficha tiene de verdad, por inmueble.
///
/// [30AA-17] Una versión existe si tiene **texto** (`inmueble_versiones`) **o
/// fotos variante** (`fotos_variantes`). Antes solo contaba el texto, así que
/// un inmueble con la v2 de fotos hecha y sin vínculo salía «al día»: el lote
/// genera fotos, no texto, y la columna Vínculo nunca avisaba. Es la misma
/// regla que `versionesDe()` en `frontend/src/domain/versiones.ts`.
async fn propias_por_inmueble(
    pool: &PgPool,
    ids: &[Uuid],
) -> Result<std::collections::HashMap<Uuid, Vec<(i16,)>>, sqlx::Error> {
    let mut mapa: std::collections::HashMap<Uuid, Vec<(i16,)>> =
        std::collections::HashMap::new();
    if ids.is_empty() {
        return Ok(mapa);
    }
    // sentinel-disable-next-line sqlx-query-as-sin-macro -- sentencia nueva; ver cabecera
    let filas: Vec<(Uuid, i16)> = sqlx::query_as(
        "SELECT inmueble_id, version FROM inmueble_versiones WHERE inmueble_id = ANY($1) \
         UNION \
         SELECT inmueble_id, version FROM fotos_variantes WHERE inmueble_id = ANY($1)",
    )
    .bind(ids)
    .fetch_all(pool)
    .await?;
    for (id, version) in filas {
        mapa.entry(id).or_default().push((version,));
    }
    Ok(mapa)
}

/// Versiones **que la ficha tiene** y que no aparecen en `cubiertas`.
///
/// La 1 siempre cuenta (vive en `inmuebles`); la 0 solo si ya tiene su texto
/// (30AA-18); de la 2 en adelante, solo las que tienen fila en
/// `inmueble_versiones`/`fotos_variantes` y no pasan de `configuradas`. Las
/// versiones que existen pero quedaron fuera de la configuración no se echan
/// en falta.
fn sin_aviso(configuradas: i16, propias: &[(i16,)], cubiertas: &[(i16,)]) -> Vec<i16> {
    let tope = configuradas.max(1);
    let tiene = |v: i16| propias.iter().any(|(p,)| *p == v);
    let existe = |v: i16| {
        if v == 0 {
            /* La v0 es la de las fotos originales: existe en cuanto la ficha
             * tiene su texto propio, y no depende de `configuradas`. */
            return tiene(0);
        }
        v <= tope && (v == 1 || tiene(v))
    };
    let mut faltan: Vec<i16> = (1..=tope)
        .filter(|v| existe(*v) && !cubiertas.iter().any(|(c,)| c == v))
        .collect();
    if existe(0) && !cubiertas.iter().any(|(c,)| *c == 0) {
        faltan.insert(0, 0);
    }
    faltan
}

#[cfg(test)]
mod tests {
    use super::sin_aviso;

    /* [30AA-7] Sin filas, faltan las versiones que la ficha tiene: la 1
     * siempre, la 2 solo si se generó. Un inmueble sin v2 no «debe» v2. */
    #[test]
    fn sin_nada_faltan_las_propias() {
        assert_eq!(sin_aviso(2, &[], &[]), vec![1]);
        assert_eq!(sin_aviso(2, &[(2,)], &[]), vec![1, 2]);
        assert_eq!(sin_aviso(3, &[(2,), (3,)], &[]), vec![1, 2, 3]);
    }

    /* Una versión con fila en CUALQUIER cuenta ya cuenta como cubierta, tenga
     * o no el ID del aviso: el ID se pega después de publicar. */
    #[test]
    fn las_cubiertas_salen_de_la_lista() {
        assert_eq!(sin_aviso(2, &[(2,)], &[(1,)]), vec![2]);
        assert_eq!(sin_aviso(2, &[(2,)], &[(2,)]), vec![1]);
        assert_eq!(sin_aviso(3, &[(2,), (3,)], &[(1,), (3,)]), vec![2]);
    }

    /* Nada que avisar: la tabla pinta «al día». */
    #[test]
    fn todas_cubiertas_no_falta_ninguna() {
        assert!(sin_aviso(2, &[(2,)], &[(1,), (2,)]).is_empty());
        assert!(sin_aviso(1, &[], &[(1,)]).is_empty());
    }

    /* El tope es la configuración: una v3 generada y luego revocada
     * (configuradas = 2) no se echa en falta. */
    #[test]
    fn manda_la_configuracion() {
        assert!(sin_aviso(1, &[(2,), (3,)], &[(1,)]).is_empty());
        assert!(sin_aviso(2, &[(2,), (3,)], &[(1,), (2,)]).is_empty());
    }

    /* [30AA-18] La v0 se echa en falta en cuanto tiene su texto, sin depender
     * de `configuradas`, y sale la primera: la lista se lee «falta v0, v1». */
    #[test]
    fn la_v0_cuenta_en_cuanto_tiene_texto() {
        assert_eq!(sin_aviso(2, &[(0,)], &[(1,)]), vec![0]);
        assert_eq!(sin_aviso(2, &[(0,), (2,)], &[]), vec![0, 1, 2]);
        assert!(sin_aviso(1, &[(0,)], &[(0,), (1,)]).is_empty());
    }
}
