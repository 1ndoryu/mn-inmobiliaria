//! Operaciones de escritura entre entornos (publicar y traer un inmueble) [F5b].
//!
//! Un solo turno por slug: dos clics seguidos no crean dos inmuebles. No hay
//! deshacer; si una operación falla a mitad, el informe dice qué quedó escrito.

use std::collections::HashSet;
use std::fmt::Display;
use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError};

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::errors::AppError;
use crate::services::sync::remoto::ErrorRemoto;

/// Fallo del servidor antes de escribir nada: se devuelve como conflicto con su motivo.
pub fn conflicto(e: ErrorRemoto) -> AppError {
    AppError::Conflict(e.to_string())
}

/// Texto de un fallo a mitad de operación, con el paso donde ocurrió.
pub fn paso(nombre: &str, e: impl Display) -> String {
    format!("{nombre}: {e}")
}

/// Lo hecho por una operación y, si falló a mitad, por qué. Es el único
/// registro de lo escrito en el otro entorno: decide el siguiente paso.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InformeSync {
    pub slug: String,
    /// Pasos completados, en orden.
    pub hecho: Vec<String>,
    /// Paso que falló y motivo. `None` = la operación terminó.
    pub fallo: Option<String>,
}

fn en_curso() -> MutexGuard<'static, HashSet<String>> {
    static EN_CURSO: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    EN_CURSO
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
}

/// Turno exclusivo sobre un slug. Se libera al soltarlo, también si la operación falla.
pub struct Turno(String);

impl Turno {
    pub fn tomar(slug: &str) -> Result<Self, AppError> {
        if en_curso().insert(slug.to_string()) {
            Ok(Self(slug.to_string()))
        } else {
            Err(AppError::Conflict(format!(
                "ya hay una operación en curso sobre «{slug}»"
            )))
        }
    }
}

impl Drop for Turno {
    fn drop(&mut self) {
        en_curso().remove(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segundo_turno_del_mismo_slug_se_rechaza_hasta_soltar_el_primero() {
        let primero = Turno::tomar("turno-prueba-14aa").expect("primer turno");
        assert!(matches!(
            Turno::tomar("turno-prueba-14aa"),
            Err(AppError::Conflict(_))
        ));
        drop(primero);
        assert!(Turno::tomar("turno-prueba-14aa").is_ok());
    }

    #[test]
    fn turnos_de_slugs_distintos_no_se_bloquean() {
        let _a = Turno::tomar("turno-a-14aa").expect("turno a");
        assert!(Turno::tomar("turno-b-14aa").is_ok());
    }
}
