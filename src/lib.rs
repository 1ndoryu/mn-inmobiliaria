#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

pub mod config;
pub mod errors;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod repositories;
pub mod services;

use std::path::PathBuf;

use sqlx::PgPool;

/// Estado compartido de la aplicación — accesible desde handlers y middleware
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub jwt_secret: String,
    pub upload_dir: PathBuf,
    /* [169A-4] Mismo hub que el router visitante: el staff emite al WS del
     * visitante (`ChatHub` es `Clone` con interiores `Arc`). */
    pub hub: glory_agent::session::ChatHub,
}
