/* [199A-1] Centro de IA de texto (redaccion de fichas + copy; la mejora
 * de fotos va por su propio servicio). Dos proveedores: GloryAPI (chat
 * completions OpenAI-compatible, `model:auto`) y OpenCode Go (Responses API
 * via `glory-agent`). El activo se elige en Configuracion; si falla, se
 * reintenta con el otro. Las claves viven solo en `.env` y nunca se loguean
 * ni se devuelven (regla del provider de glory-agent).
 *
 * [16AA-9 F4] Aqui queda el boundary HTTP (`routes`) y `completar`. El estado
 * de proveedores vive en `ia_estado.rs`, la config en `ia_admin.rs` y los
 * proveedores en `ia_proveedores.rs`; constantes y helpers, en `mod.rs`. */

use axum::extract::{DefaultBodyLimit, State};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::AppState;

use super::ia_admin::{guardar_config_ia, probar};
use super::ia_estado::leer_estado;
use super::{
    habilitado, leer_clave, leer_env, ordenar_intento, CLAVE_ACTIVO, CLAVE_HAB_GLORY,
    CLAVE_HAB_OPENCODE, ID_GLORY, ID_OPENCODE, MAX_CUERPO_FOTOS, MAX_FOTOS, MAX_TEXTO,
};

/* [08AA-8] Proveedores (ping/completar/transcribir/describir) viven en
 * `ia_proveedores.rs` (split god-object). Re-export `pub(crate)` para
 * `marketplace`/`turno`; el resto para rutas y tests (`super::*`). */
pub(crate) use super::ia_borrador::completar_opencode_rapido;
pub(super) use super::ia_proveedores::completar_glory;
pub(crate) use super::ia_proveedores::{completar_opencode, describir_foto, transcribir_audio};

#[derive(Debug, Clone, Deserialize)]
struct CompletarEntrada {
    system: String,
    texto: String,
    #[serde(default)]
    fotos: Vec<String>,
}

#[derive(Debug, Serialize)]
struct CompletarSalida {
    ok: bool,
    texto: Option<String>,
    proveedor: Option<String>,
    modelo: Option<String>,
    motivos: Vec<String>,
}

fn validar_entrada(input: &CompletarEntrada) -> Result<(), AppError> {
    if input.system.trim().is_empty() || input.system.len() > MAX_TEXTO {
        return Err(AppError::Validation(
            "system requerido (max 8000 caracteres)".to_string(),
        ));
    }
    if input.texto.len() > MAX_TEXTO {
        return Err(AppError::Validation(
            "texto supera 8000 caracteres".to_string(),
        ));
    }
    if input.fotos.len() > MAX_FOTOS {
        return Err(AppError::BadRequest(format!(
            "maximo {MAX_FOTOS} fotos por llamada"
        )));
    }
    if input.texto.trim().is_empty() && input.fotos.is_empty() {
        return Err(AppError::BadRequest("texto o fotos requeridos".to_string()));
    }
    let total: usize = input.fotos.iter().map(String::len).sum();
    if total > MAX_CUERPO_FOTOS {
        return Err(AppError::PayloadMuyGrande);
    }
    Ok(())
}

/* [08AA-8] `texto_glory`→`describir_foto` movidos a `ia_proveedores.rs`
 * (re-export arriba para `completar`, tests y `marketplace`/`turno`). */

async fn completar(
    _auth: AuthUser,
    State(state): State<AppState>,
    Json(input): Json<CompletarEntrada>,
) -> Result<Json<CompletarSalida>, AppError> {
    validar_entrada(&input)?;
    let activo = leer_clave(&state.pool, CLAVE_ACTIVO, ID_GLORY).await?;
    let mut motivos = Vec::new();
    for id in ordenar_intento(&activo) {
        let (clave_hab, tiene_clave) = if id == ID_OPENCODE {
            (
                CLAVE_HAB_OPENCODE,
                !leer_env("OPENCODE_GO_API_KEY").is_empty(),
            )
        } else {
            (CLAVE_HAB_GLORY, !leer_env("GLORY_API_KEY").is_empty())
        };
        if !habilitado(&leer_clave(&state.pool, clave_hab, "on").await?) {
            motivos.push(format!("{id}: deshabilitado en Configuracion"));
            continue;
        }
        if !tiene_clave {
            motivos.push(format!("{id}: sin clave en .env"));
            continue;
        }
        let texto_fallback = input.texto.trim().to_string();
        let texto_enviar = if texto_fallback.is_empty() {
            "Ordena la ficha de este inmueble a partir de las fotos.".to_string()
        } else {
            texto_fallback
        };
        let intento = if id == ID_OPENCODE {
            completar_opencode(&input.system, &texto_enviar, &input.fotos, "centro-ia").await
        } else {
            completar_glory(&input.system, &texto_enviar, &input.fotos).await
        };
        match intento {
            Ok((texto, modelo)) => {
                return Ok(Json(CompletarSalida {
                    ok: true,
                    texto: Some(texto),
                    proveedor: Some(id.to_string()),
                    modelo: Some(modelo),
                    motivos,
                }));
            }
            Err(motivo) => {
                tracing::warn!("IA {id} fallo: {motivo}");
                motivos.push(format!("{id}: {motivo}"));
            }
        }
    }
    Ok(Json(CompletarSalida {
        ok: false,
        texto: None,
        proveedor: None,
        modelo: None,
        motivos,
    }))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/ia/estado", get(leer_estado))
        .route("/ia/config", put(guardar_config_ia))
        .route("/ia/probar", post(probar))
        /* Sin el limite Json de 2 MiB de axum: las fotos base64 lo superan;
         * `validar_entrada` aplica el tope real (413 si excede). */
        .route(
            "/ia/completar",
            post(completar).route_layer(DefaultBodyLimit::disable()),
        )
}

#[cfg(test)]
mod pruebas {
    use super::{validar_entrada, CompletarEntrada};

    /* [199A-1] Validacion en el boundary: texto, conteo y peso de fotos. */
    #[test]
    fn validar_entrada_rechaza_excesos() {
        let base = CompletarEntrada {
            system: "s".to_string(),
            texto: "t".to_string(),
            fotos: vec![],
        };
        assert!(validar_entrada(&base).is_ok());
        let larga = CompletarEntrada {
            texto: "x".repeat(8001),
            ..base.clone()
        };
        assert!(validar_entrada(&larga).is_err());
        let muchas = CompletarEntrada {
            fotos: vec!["f".to_string(); 5],
            ..base.clone()
        };
        assert!(validar_entrada(&muchas).is_err());
    }
}
