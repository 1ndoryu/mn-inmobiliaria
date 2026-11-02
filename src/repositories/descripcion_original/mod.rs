use sqlx::PgPool;
use uuid::Uuid;

/* [28AE-2] Respaldo privado de la descripción original: se escribe ANTES de que
 * la IA la procese. Ningún DTO público lee esta tabla. */
pub struct DescripcionOriginalRepository;

impl DescripcionOriginalRepository {
    /// Guarda el texto tal cual llegó y devuelve el id del respaldo.
    pub async fn guardar(
        pool: &PgPool,
        texto: &str,
        origen: &str,
        inmueble_id: Option<Uuid>,
    ) -> Result<Uuid, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query!(
            "INSERT INTO descripciones_originales (id, inmueble_id, texto, origen) VALUES ($1, $2, $3, $4)",
            id,
            inmueble_id,
            texto,
            origen,
        )
        .execute(pool)
        .await?;
        Ok(id)
    }

    /// Anota qué proveedor y modelo procesaron el respaldo (tras la llamada).
    pub async fn anotar_modelo(
        pool: &PgPool,
        id: Uuid,
        proveedor: &str,
        modelo: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE descripciones_originales SET proveedor = $2, modelo = $3 WHERE id = $1",
            id,
            proveedor,
            modelo,
        )
        .execute(pool)
        .await?;
        Ok(())
    }
}
