/* [16AA-9 F4] Lectura del estado de los proveedores de IA: el diagnostico
 * guardado (`ok|<epoch>|<ms>|<modelo>` o `error|<epoch>|<motivo>`) y la ficha
 * que pinta Configuracion. Extraido de `rutas.rs`. */

use axum::extract::State;
use axum::Json;
use serde::Serialize;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::AppState;

use super::{
    fail, habilitado, leer_clave, leer_env, CLAVE_ACTIVO, CLAVE_HAB_GLORY, CLAVE_HAB_OPENCODE,
    ID_GLORY, ID_OPENCODE,
};

/// Diagnostico guardado `ok|<epoch>|<ms>|<modelo>` o `error|<epoch>|<motivo>`.
#[derive(Debug, Default)]
struct Diagnostico {
    estado: String,
    comprobado_en: Option<i64>,
    latencia_ms: Option<u64>,
    ultimo_modelo: Option<String>,
    ultimo_error: Option<String>,
}

fn parsear_diagnostico(crudo: Option<String>) -> Diagnostico {
    let Some(texto) = crudo else {
        return Diagnostico {
            estado: "no-probado".to_string(),
            ..Diagnostico::default()
        };
    };
    let partes: Vec<&str> = texto.splitn(4, '|').collect();
    let (kind, resto) = (partes.first(), &partes[1..]);
    match (kind, resto) {
        (Some(&"ok"), [epoch, ms, modelo]) => Diagnostico {
            estado: "ok".to_string(),
            comprobado_en: epoch.parse().ok(),
            latencia_ms: ms.parse().ok(),
            ultimo_modelo: Some((*modelo).to_string()),
            ..Diagnostico::default()
        },
        (Some(&"error"), [epoch, motivo]) => Diagnostico {
            estado: "error".to_string(),
            comprobado_en: epoch.parse().ok(),
            ultimo_error: Some((*motivo).to_string()),
            ..Diagnostico::default()
        },
        _ => Diagnostico {
            estado: "no-probado".to_string(),
            ..Diagnostico::default()
        },
    }
}

#[derive(Debug, Serialize)]
pub(super) struct EstadoProveedor {
    id: String,
    nombre: String,
    modelo: String,
    habilitado: bool,
    configurado: bool,
    estado: String,
    comprobado_en: Option<i64>,
    latencia_ms: Option<u64>,
    ultimo_modelo: Option<String>,
    ultimo_error: Option<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct EstadoIA {
    activo: String,
    proveedores: Vec<EstadoProveedor>,
}

async fn estado_proveedor(
    pool: &sqlx::PgPool,
    id: &'static str,
    nombre: &str,
    modelo: String,
    clave_hab: &str,
    clave_check: &str,
    api_key: &str,
) -> Result<EstadoProveedor, AppError> {
    let hab = habilitado(&leer_clave(pool, clave_hab, "on").await?);
    let configurado = !api_key.trim().is_empty();
    let diag = parsear_diagnostico(
        glory_agent::persistence::get_config(pool, clave_check)
            .await
            .map_err(|e| fail(&e))?,
    );
    let estado = if !hab {
        "deshabilitado".to_string()
    } else if !configurado {
        "sin-clave".to_string()
    } else {
        diag.estado.clone()
    };
    Ok(EstadoProveedor {
        id: id.to_string(),
        nombre: nombre.to_string(),
        modelo,
        habilitado: hab,
        configurado,
        estado,
        comprobado_en: diag.comprobado_en,
        latencia_ms: diag.latencia_ms,
        ultimo_modelo: diag.ultimo_modelo,
        ultimo_error: diag.ultimo_error,
    })
}

pub(super) async fn leer_estado(
    _auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<EstadoIA>, AppError> {
    let activo = leer_clave(&state.pool, CLAVE_ACTIVO, ID_GLORY).await?;
    let activo = if activo == ID_OPENCODE {
        ID_OPENCODE.to_string()
    } else {
        ID_GLORY.to_string()
    };
    let proveedores = vec![
        estado_proveedor(
            &state.pool,
            ID_GLORY,
            "GloryAPI",
            "auto".to_string(),
            CLAVE_HAB_GLORY,
            "ia_check_gloryapi",
            &leer_env("GLORY_API_KEY"),
        )
        .await?,
        estado_proveedor(
            &state.pool,
            ID_OPENCODE,
            "OpenCode Go",
            crate::config::ia::MODELO_OPENCODE.to_string(),
            CLAVE_HAB_OPENCODE,
            "ia_check_opencode_go",
            &leer_env("OPENCODE_GO_API_KEY"),
        )
        .await?,
    ];
    Ok(Json(EstadoIA {
        activo,
        proveedores,
    }))
}

#[cfg(test)]
mod pruebas {
    use super::parsear_diagnostico;

    /* [199A-1] El diagnostico `ok|epoch|ms|modelo` se rehidrata; el error no. */
    #[test]
    fn parsear_diagnostico_redondea() {
        let d = parsear_diagnostico(Some("ok|1726233600|420|muse-spark-1.3".to_string()));
        assert_eq!(d.estado, "ok");
        assert_eq!(d.comprobado_en, Some(1_726_233_600));
        assert_eq!(d.latencia_ms, Some(420));
        assert_eq!(d.ultimo_modelo.as_deref(), Some("muse-spark-1.3"));
        let e = parsear_diagnostico(Some("error|1726233600|sin clave".to_string()));
        assert_eq!(e.estado, "error");
        assert_eq!(
            e.ultimo_error.as_deref(),
            Some("sin clave"),
            "el motivo se conserva para Configuracion"
        );
        assert!(e.ultimo_modelo.is_none());
        assert_eq!(parsear_diagnostico(None).estado, "no-probado");
    }
}
