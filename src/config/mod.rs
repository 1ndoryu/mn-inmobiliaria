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
}

impl AppConfig {
    /// Carga la configuración desde variables de entorno.
    /// Requiere `DATABASE_URL` y `JWT_SECRET`. `HOST`, `PORT` y `UPLOAD_DIR` son opcionales.
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
        })
    }
}
