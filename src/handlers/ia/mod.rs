//! Centro de IA de texto: ficha /ask, proveedores, sombra y rutas (`rutas.rs`).
//!
//! [16AA-9 F4] `rutas.rs` partido en tres por tamano: `rutas` (boundary HTTP
//! y `completar`), `ia_estado` (lectura del estado) e `ia_admin` (config y
//! prueba). Constantes y helpers compartidos viven aqui: los submodulos son
//! descendientes, asi que lo privado de este modulo les es visible.

pub mod ask;
mod ia_admin;
mod ia_borrador;
mod ia_estado;
pub mod ia_proveedores;
pub mod rutas;
pub mod sombra;

use crate::errors::AppError;

const ID_GLORY: &str = "gloryapi";
const ID_OPENCODE: &str = "opencode-go";

const CLAVE_ACTIVO: &str = "ia_activo";
const CLAVE_HAB_GLORY: &str = "ia_hab_gloryapi";
const CLAVE_HAB_OPENCODE: &str = "ia_hab_opencode_go";

const GLORY_DEFECTO_URL: &str = "http://127.0.0.1:3101";
/* 4 dataURL JPEG 1280px caben de sobra; coincide con el tope del front. */
const MAX_CUERPO_FOTOS: usize = 12_000_000;
const MAX_TEXTO: usize = 8000;
/* Compartida con `ia_proveedores` (tope de fotos por llamada). */
const MAX_FOTOS: usize = 4;

fn fail(e: &glory_agent::errors::AgentError) -> AppError {
    AppError::Internal(e.to_string())
}

fn leer_env(nombre: &str) -> String {
    std::env::var(nombre).unwrap_or_default().trim().to_string()
}

fn glory_base() -> String {
    let base = leer_env("GLORY_API_URL");
    let base = if base.is_empty() {
        GLORY_DEFECTO_URL.to_string()
    } else {
        base
    };
    base.trim_end_matches('/').to_string()
}

fn ordenar_intento(activo: &str) -> [&'static str; 2] {
    if activo == ID_OPENCODE {
        [ID_OPENCODE, ID_GLORY]
    } else {
        [ID_GLORY, ID_OPENCODE]
    }
}

async fn leer_clave(pool: &sqlx::PgPool, clave: &str, defecto: &str) -> Result<String, AppError> {
    Ok(glory_agent::persistence::get_config(pool, clave)
        .await
        .map_err(|e| fail(&e))?
        .unwrap_or_else(|| defecto.to_string()))
}

fn habilitado(valor: &str) -> bool {
    valor != "off"
}

#[cfg(test)]
mod pruebas {
    use super::ordenar_intento;

    /* [199A-1] Orden de intento: el activo primero, el otro de respaldo. */
    #[test]
    fn activo_primero_y_respaldo_despues() {
        assert_eq!(ordenar_intento("gloryapi"), ["gloryapi", "opencode-go"]);
        assert_eq!(ordenar_intento("opencode-go"), ["opencode-go", "gloryapi"]);
        assert_eq!(ordenar_intento("otro"), ["gloryapi", "opencode-go"]);
    }
}
