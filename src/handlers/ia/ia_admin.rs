/* [16AA-9 F4] Escritura de la configuracion de IA: proveedor activo y
 * habilitacion, y prueba de un proveedor con su diagnostico persistido.
 * Extraido de `rutas.rs`. */

use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::AppState;

use super::ia_proveedores::{ping_glory, ping_opencode};
use super::{
    fail, glory_base, leer_env, CLAVE_ACTIVO, CLAVE_HAB_GLORY, CLAVE_HAB_OPENCODE, ID_GLORY,
    ID_OPENCODE,
};

#[derive(Debug, Deserialize)]
pub(super) struct ConfigIAEntrada {
    activo: String,
    gloryapi_habilitado: bool,
    opencode_go_habilitado: bool,
}

pub(super) async fn guardar_config_ia(
    _auth: AuthUser,
    State(state): State<AppState>,
    Json(input): Json<ConfigIAEntrada>,
) -> Result<Json<serde_json::Value>, AppError> {
    if input.activo != ID_GLORY && input.activo != ID_OPENCODE {
        return Err(AppError::BadRequest(
            "activo debe ser gloryapi|opencode-go".to_string(),
        ));
    }
    let ConfigIAEntrada {
        activo,
        gloryapi_habilitado,
        opencode_go_habilitado,
    } = input;
    for (clave, valor) in [
        (CLAVE_ACTIVO, activo),
        (
            CLAVE_HAB_GLORY,
            if gloryapi_habilitado { "on" } else { "off" }.to_string(),
        ),
        (
            CLAVE_HAB_OPENCODE,
            if opencode_go_habilitado { "on" } else { "off" }.to_string(),
        ),
    ] {
        glory_agent::persistence::set_config(&state.pool, clave, &valor)
            .await
            .map_err(|e| fail(&e))?;
    }
    Ok(Json(serde_json::json!({"ok": true})))
}

#[derive(Debug, Deserialize)]
pub(super) struct ProbarEntrada {
    proveedor: String,
}

#[derive(Debug, Serialize)]
pub(super) struct ProbarSalida {
    ok: bool,
    latencia_ms: u64,
    modelo: Option<String>,
    error: Option<String>,
}

async fn guardar_diagnostico(
    pool: &sqlx::PgPool,
    id: &str,
    resultado: &Result<String, String>,
    latencia_ms: u64,
) -> Result<(), AppError> {
    let clave = if id == ID_OPENCODE {
        "ia_check_opencode_go"
    } else {
        "ia_check_gloryapi"
    };
    let epoch = chrono::Utc::now().timestamp();
    let valor = match resultado {
        Ok(modelo) => format!("ok|{epoch}|{latencia_ms}|{modelo}"),
        Err(motivo) => {
            let corto: String = motivo.chars().take(120).collect();
            format!("error|{epoch}|{corto}")
        }
    };
    glory_agent::persistence::set_config(pool, clave, &valor)
        .await
        .map_err(|e| fail(&e))?;
    Ok(())
}

pub(super) async fn probar(
    _auth: AuthUser,
    State(state): State<AppState>,
    Json(input): Json<ProbarEntrada>,
) -> Result<Json<ProbarSalida>, AppError> {
    let inicio = Instant::now();
    let resultado = match input.proveedor.as_str() {
        ID_GLORY => {
            let key = leer_env("GLORY_API_KEY");
            if key.is_empty() {
                Err("Sin GLORY_API_KEY en .env".to_string())
            } else {
                ping_glory(&glory_base(), &key).await
            }
        }
        ID_OPENCODE => {
            let key = leer_env("OPENCODE_GO_API_KEY");
            if key.is_empty() {
                Err("Sin OPENCODE_GO_API_KEY en .env".to_string())
            } else {
                ping_opencode(&key).await
            }
        }
        otro => {
            return Err(AppError::BadRequest(format!(
                "proveedor desconocido: {otro}"
            )));
        }
    };
    let latencia_ms = u64::try_from(inicio.elapsed().as_millis()).unwrap_or(u64::MAX);
    guardar_diagnostico(&state.pool, &input.proveedor, &resultado, latencia_ms).await?;
    Ok(Json(match resultado {
        Ok(modelo) => ProbarSalida {
            ok: true,
            latencia_ms,
            modelo: Some(modelo),
            error: None,
        },
        Err(motivo) => ProbarSalida {
            ok: false,
            latencia_ms,
            modelo: None,
            error: Some(motivo),
        },
    }))
}
