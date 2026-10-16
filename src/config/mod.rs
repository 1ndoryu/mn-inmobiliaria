pub mod ia;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Variable de entorno requerida no encontrada: {0}")]
    MissingEnvVar(String),
    #[error("Puerto inválido: {0}")]
    InvalidPort(#[from] std::num::ParseIntError),
}

/// Configuración de la aplicación cargada desde variables de entorno
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub jwt_secret: String,
    pub host: String,
    pub port: u16,
    pub upload_dir: String,
    /* [239A-1] Monorepo: `STATIC_DIR` (p. ej. `/app/dist`) activa el front SPA
     * embebido; ausente = solo API (dev). `CORS_ORIGINS` (coma, p. ej.
     * `https://mn-inmobiliaria.com`) restringe orígenes; ausente = abierto. */
    pub static_dir: Option<String>,
    pub cors_origins: Vec<String>,
}

impl AppConfig {
    /// Carga la configuración desde variables de entorno.
    /// Requiere `DATABASE_URL` y `JWT_SECRET`. `HOST`, `PORT`, `UPLOAD_DIR`,
    /// `STATIC_DIR` y `CORS_ORIGINS` son opcionales.
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(Self {
            database_url: std::env::var("DATABASE_URL")
                .map_err(|_| ConfigError::MissingEnvVar("DATABASE_URL".into()))?,
            jwt_secret: std::env::var("JWT_SECRET")
                .map_err(|_| ConfigError::MissingEnvVar("JWT_SECRET".into()))?,
            host: std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            port: std::env::var("PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()?,
            upload_dir: std::env::var("UPLOAD_DIR").unwrap_or_else(|_| "./uploads".to_string()),
            static_dir: std::env::var("STATIC_DIR").ok(),
            cors_origins: std::env::var("CORS_ORIGINS")
                .unwrap_or_default()
                .split(',')
                .map(|o| o.trim().to_owned())
                .filter(|o| !o.is_empty())
                .collect(),
        })
    }
}

/* [10AA-31] Nombre de la BD de desarrollo de la usuaria (rama `main`). */
#[cfg(test)]
const BD_DESARROLLO: &str = "glory_backend_inmobiliaria";

/* [10AA-31] URL de BD para los tests de BD viva. Nunca la de desarrollo: en
 * `main`, `DATABASE_URL` apunta a ella y los tests borran y purgan filas reales
 * (`purgar_cache`, DELETE por hilo). Sin URL o con la de desarrollo, el test se
 * omite, igual que sin `DATABASE_URL`. Los tests solo corren en BD de rama.
 * [09AA-36c] La omisión avisa con el nombre del test (Rust usa el nombre del
 * test como nombre del hilo); se ve con `--nocapture`. */
#[cfg(test)]
pub fn url_bd_pruebas() -> Option<String> {
    let hilo = std::thread::current().name().unwrap_or("?").to_string();
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("OMITIDO {hilo}: sin DATABASE_URL");
        return None;
    };
    let nombre = url.rsplit('/').next()?.split('?').next()?;
    if nombre == BD_DESARROLLO {
        eprintln!("OMITIDO {hilo}: DATABASE_URL apunta a la BD de desarrollo ({BD_DESARROLLO}); los tests de BD viva solo corren en BD de rama");
        return None;
    }
    Some(url)
}
