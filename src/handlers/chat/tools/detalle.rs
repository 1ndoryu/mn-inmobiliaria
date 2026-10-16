/* [16AA-9 F3] `detalle_inmueble`: la ficha completa de un inmueble.
 * Salio de `chat/tools.rs`. */

use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use crate::repositories::chat::tools::ficha_inmueble;
use glory_agent::errors::AgentError;

pub(super) async fn detalle(pool: &PgPool, args: &Value) -> Result<Value, AgentError> {
    let id: Uuid = args
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .parse()
        .map_err(|_| AgentError::BadRequest("id de inmueble invalido".to_string()))?;
    let fila = ficha_inmueble(pool, id)
        .await
        .map_err(|e| AgentError::Db(e.to_string()))?;
    let Some(f) = fila else {
        return Ok(json!({"error": "inmueble no disponible"}));
    };
    let descripcion: String = f.descripcion.chars().take(600).collect();
    Ok(
        json!({"id": id, "titulo": f.titulo, "descripcion": descripcion, "ubicacion": f.ubicacion,
              "puestos": f.puestos, "residencia": f.residencia,
              "precio": f.precio, "tipo": f.tipo, "operacion": f.operacion, "habitaciones": f.habitaciones,
              "banos": f.banos, "metros": f.metros, "metros_terreno": f.metros_terreno, "estado": f.estado,
              "resumen": f.copy_corta.unwrap_or_default(),
              "extras": f.extras, "margen_negociable": f.margen_negociable}),
    )
}
