//! [28AE-2] `POST /api/admin/ia/ficha`: la única puerta para que la IA ordene la
//! descripción de un inmueble (panel, completar al publicar y CLI).
//!
//! [por que] Antes el prompt vivía en el front y la IA se llamaba a pelo: ni el
//! CLI lo usaba ni quedaba copia de lo que ella escribió. Aquí el original se
//! guarda en `descripciones_originales` (privada) ANTES de llamar a la IA, sea
//! cual sea la ruta; si el respaldo falla, no se llama a la IA (falla cerrado).
//! La IA es siempre `OpenCode` Go (`DeepSeek` Flash), sin caer a otro proveedor.

use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::ia_proveedores::completar_opencode;
use super::{habilitado, leer_clave, leer_env, CLAVE_HAB_OPENCODE, MAX_CUERPO_FOTOS, MAX_FOTOS, MAX_TEXTO};
use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::repositories::DescripcionOriginalRepository;
use crate::AppState;

/// Orígenes admitidos del respaldo (lista cerrada: columna `origen`).
const ORIGENES: [&str; 3] = ["panel", "publicar", "cli"];

#[derive(Debug, Clone, Deserialize)]
pub(super) struct FichaEntrada {
    texto: String,
    #[serde(default)]
    fotos: Vec<String>,
    #[serde(default)]
    inmueble_id: Option<Uuid>,
    #[serde(default)]
    origen: Option<String>,
    /// Solo guarda el original (sin llamar a la IA): para respaldar en otra base.
    #[serde(default)]
    solo_respaldo: bool,
}

#[derive(Debug, Serialize)]
pub(super) struct FichaSalida {
    ok: bool,
    texto: Option<String>,
    proveedor: Option<String>,
    modelo: Option<String>,
    motivos: Vec<String>,
    /// Id del respaldo guardado (`None` si solo había fotos, sin texto).
    respaldo_id: Option<Uuid>,
}

/// Valida en el boundary y devuelve el origen normalizado.
fn validar(input: &FichaEntrada) -> Result<&'static str, AppError> {
    if input.texto.len() > MAX_TEXTO {
        return Err(AppError::Validation("texto supera 8000 caracteres".to_string()));
    }
    if input.fotos.len() > MAX_FOTOS {
        return Err(AppError::BadRequest(format!("maximo {MAX_FOTOS} fotos por llamada")));
    }
    if input.texto.trim().is_empty() && input.fotos.is_empty() {
        return Err(AppError::BadRequest("texto o fotos requeridos".to_string()));
    }
    if input.fotos.iter().map(String::len).sum::<usize>() > MAX_CUERPO_FOTOS {
        return Err(AppError::PayloadMuyGrande);
    }
    let pedido = input.origen.as_deref().unwrap_or("panel");
    ORIGENES
        .iter()
        .find(|o| **o == pedido)
        .copied()
        .ok_or_else(|| AppError::Validation(format!("origen invalido (usa {ORIGENES:?})")))
}

pub(super) async fn ficha(
    _auth: AuthUser,
    State(state): State<AppState>,
    Json(input): Json<FichaEntrada>,
) -> Result<Json<FichaSalida>, AppError> {
    let origen = validar(&input)?;
    let texto = input.texto.trim();
    /* Respaldo primero: la IA no se llama si el original no quedó guardado. */
    let respaldo_id = if texto.is_empty() {
        None
    } else {
        let id = DescripcionOriginalRepository::guardar(&state.pool, texto, origen, input.inmueble_id)
            .await
            .map_err(|e| AppError::Internal(format!("respaldo de la descripcion: {e}")))?;
        Some(id)
    };
    let mut salida = FichaSalida {
        ok: false,
        texto: None,
        proveedor: None,
        modelo: None,
        motivos: Vec::new(),
        respaldo_id,
    };
    if !habilitado(&leer_clave(&state.pool, CLAVE_HAB_OPENCODE, "on").await?) {
        salida.motivos.push("opencode-go: deshabilitado en Configuracion".to_string());
        return Ok(Json(salida));
    }
    if leer_env("OPENCODE_GO_API_KEY").is_empty() {
        salida.motivos.push("opencode-go: sin clave en .env".to_string());
        return Ok(Json(salida));
    }
    if input.solo_respaldo {
        salida.ok = true;
        return Ok(Json(salida));
    }
    let a_enviar = if texto.is_empty() {
        "Ordena la ficha de este inmueble a partir de las fotos."
    } else {
        texto
    };
    match completar_opencode(crate::config::ia::SISTEMA_FICHA, a_enviar, &input.fotos, "ficha").await {
        Ok((salida_ia, modelo)) => {
            if let Some(id) = respaldo_id {
                /* Anotar proveedor/modelo es informativo: si falla no se pierde la ficha. */
                if let Err(e) =
                    DescripcionOriginalRepository::anotar_modelo(&state.pool, id, "opencode-go", &modelo).await
                {
                    tracing::warn!("no se pudo anotar el modelo del respaldo {id}: {e}");
                }
            }
            salida.ok = true;
            salida.texto = Some(salida_ia);
            salida.proveedor = Some("opencode-go".to_string());
            salida.modelo = Some(modelo);
        }
        Err(motivo) => {
            tracing::warn!("IA ficha fallo: {motivo}");
            salida.motivos.push(format!("opencode-go: {motivo}"));
        }
    }
    Ok(Json(salida))
}

#[cfg(test)]
mod pruebas {
    use super::{validar, FichaEntrada};
    use crate::config::ia::SISTEMA_FICHA;

    fn base() -> FichaEntrada {
        FichaEntrada { texto: "casa linda".to_string(), fotos: vec![], inmueble_id: None, origen: None, solo_respaldo: false }
    }

    /* [28AE-2] Validación del boundary y lista cerrada de orígenes. */
    #[test]
    fn validar_acota_texto_fotos_y_origen() {
        assert_eq!(validar(&base()).unwrap(), "panel");
        let cli = FichaEntrada { origen: Some("cli".to_string()), ..base() };
        assert_eq!(validar(&cli).unwrap(), "cli");
        let raro = FichaEntrada { origen: Some("otro".to_string()), ..base() };
        assert!(validar(&raro).is_err());
        let larga = FichaEntrada { texto: "x".repeat(8001), ..base() };
        assert!(validar(&larga).is_err());
        let muchas = FichaEntrada { fotos: vec!["f".to_string(); 5], ..base() };
        assert!(validar(&muchas).is_err());
        let vacia = FichaEntrada { texto: "  ".to_string(), ..base() };
        assert!(validar(&vacia).is_err());
        let solo_fotos = FichaEntrada { texto: String::new(), fotos: vec!["f".to_string()], ..base() };
        assert!(validar(&solo_fotos).is_ok());
    }

    /* El prompt único pide JSON y no inventar datos (regla de la ficha). */
    #[test]
    fn el_prompt_de_ficha_pide_json_y_cabe_en_el_limite() {
        assert!(SISTEMA_FICHA.contains("JSON"));
        assert!(SISTEMA_FICHA.len() < 8000);
    }
}
