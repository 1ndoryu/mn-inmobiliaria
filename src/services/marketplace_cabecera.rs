//! [09AA-29] Cola de la cabecera del hilo que el corte del float deja al
//! inicio del excerpt.
//!
//! El float recorta el texto plano con `slice(-1200)` por carácter: el
//! crudo puede empezar a mitad del título del aviso o del nombre del
//! comprador (testigo `الله معي|VEF0 casa en venta en urbanización villa
//! icabarú, puerto ordaz.`: el crudo abría con `erto Ordaz.` y el panel lo
//! mostraba como mensaje del Cliente). `es_cola_truncada` solo ve colas sin
//! espacios; aquí se descartan las que son sufijo propio de la cabecera.
//! Módulo aparte para no engordar `marketplace_texto.rs` (09AA-18).

use super::marketplace_texto::sin_tilde_min;

/// Por debajo de este largo un sufijo coincide por azar (`la`, `ar.`).
const MIN_COLA: usize = 4;

/// `true` si `linea` (primera del excerpt) es un sufijo propio del título del
/// aviso o del nombre del comprador. Compara sin tildes ni mayúsculas y sin
/// la puntuación final. Un mensaje real que reproduzca el final exacto del
/// título sería descartado, pero solo se evalúa en la primera línea del
/// crudo, donde un mensaje completo casi nunca cae tras un corte por carácter.
pub(super) fn es_cola_de_cabecera(linea: &str, nombre: Option<&str>, aviso: Option<&str>) -> bool {
    let fragmento = canonico(linea);
    if fragmento.chars().count() < MIN_COLA {
        return false;
    }
    [aviso, nombre]
        .into_iter()
        .flatten()
        .map(canonico)
        .any(|cabecera| cabecera.len() > fragmento.len() && cabecera.ends_with(&fragmento))
}

fn canonico(s: &str) -> String {
    sin_tilde_min(s.trim())
        .trim_end_matches(['.', ',', ';', ':'])
        .trim()
        .to_string()
}

#[cfg(test)]
mod pruebas {
    use super::es_cola_de_cabecera;

    const AVISO: &str = "VEF0 casa en venta en urbanización villa icabarú, puerto ordaz.";

    #[test]
    fn cola_del_titulo_con_corte_a_mitad_de_palabra() {
        assert!(es_cola_de_cabecera("erto Ordaz.", None, Some(AVISO)));
        assert!(es_cola_de_cabecera(
            "abarú, Puerto Ordaz",
            None,
            Some(AVISO)
        ));
    }

    #[test]
    fn cola_del_nombre_del_comprador() {
        assert!(es_cola_de_cabecera("erley", Some("Kerley"), None));
    }

    #[test]
    fn mensajes_reales_no_son_cola() {
        for msg in [
            "¿Sigue disponible?",
            "Hola. ¿Sigue estando disponible?",
            "Sí",
            "Hola",
        ] {
            assert!(
                !es_cola_de_cabecera(msg, Some("Kerley"), Some(AVISO)),
                "{msg}"
            );
        }
    }

    #[test]
    fn titulo_completo_o_demasiado_corto_no_cuenta() {
        assert!(!es_cola_de_cabecera(AVISO, None, Some(AVISO)));
        assert!(!es_cola_de_cabecera("az.", None, Some(AVISO)));
        assert!(!es_cola_de_cabecera("erto Ordaz.", None, None));
    }
}
