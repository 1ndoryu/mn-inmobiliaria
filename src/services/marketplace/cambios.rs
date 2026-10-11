//! Aviso de cambios del panel de chats Marketplace [10AA-24].
//!
//! El admin necesita ver un chat en cuanto el lab lo abre o llega un mensaje,
//! sin recargar. Se usa long-poll (no SSE/WebSocket): pasa por cualquier
//! proxy, reutiliza `apiFetch` (JWT en cabecera) y se aborta con `AbortSignal`.
//! El contador es proceso-local: tras un reinicio vuelve a 0 y el cliente lo
//! trata como cambio (`version != desde`), así nunca se queda esperando un
//! número que ya no existe.
//!
//! Es independiente de `version_borradores_borrados`: aquel contador vacía la
//! caché del float del lab, este solo avisa al panel; mezclarlos haría que cada
//! borrador nuevo vaciara la caché del lab.

use std::sync::OnceLock;
use std::time::Duration;

use tokio::sync::watch;

/// Espera máxima de una petición de long-poll. Por debajo de los tiempos de
/// espera habituales de proxies (30–60 s) para que nunca la corte un tercero.
pub const ESPERA_MAX: Duration = Duration::from_secs(25);

fn canal() -> &'static watch::Sender<u64> {
    static CANAL: OnceLock<watch::Sender<u64>> = OnceLock::new();
    CANAL.get_or_init(|| watch::channel(0).0)
}

/// Marca que el panel cambió (chat creado, texto nuevo, borrado, archivado).
pub fn notificar() {
    // `send_modify` actualiza aunque no haya nadie escuchando.
    canal().send_modify(|v| *v = v.wrapping_add(1));
}

/// Versión actual del panel (solo la usan las pruebas: el handler pasa por `esperar_cambio`).
#[cfg(test)]
fn version() -> u64 {
    *canal().borrow()
}

/// Devuelve la versión en cuanto difiera de `desde`, o la actual al agotarse
/// `max`. Sin `desde` (primera petición) responde al instante.
pub async fn esperar_cambio(desde: Option<u64>, max: Duration) -> u64 {
    let mut rx = canal().subscribe();
    let actual = *rx.borrow_and_update();
    if desde != Some(actual) {
        return actual;
    }
    // Timeout o canal cerrado: en ambos casos se responde con la versión vigente.
    let _ = tokio::time::timeout(max, rx.changed()).await;
    // Se copia antes de devolver: el `Ref` temporal no puede sobrevivir a `rx`.
    let vigente = *rx.borrow();
    vigente
}

#[cfg(test)]
mod pruebas {
    use super::*;

    /* El canal es global al proceso y los tests corren en paralelo: cada test
     * razona solo con diferencias respecto a la versión que ve al empezar. */

    #[tokio::test]
    async fn sin_desde_responde_al_instante() {
        let v = esperar_cambio(None, Duration::from_secs(5)).await;
        assert_eq!(v, version());
    }

    #[tokio::test]
    async fn version_distinta_responde_al_instante() {
        let actual = version();
        let v = esperar_cambio(Some(actual.wrapping_add(7)), Duration::from_secs(5)).await;
        assert!(v >= actual);
    }

    #[tokio::test]
    async fn sin_cambios_agota_la_espera_y_devuelve_lo_mismo() {
        let antes = version();
        let v = esperar_cambio(Some(antes), Duration::from_millis(30)).await;
        // Otro test puede notificar a la vez: basta con no ir hacia atrás.
        assert!(v >= antes);
    }

    #[tokio::test]
    async fn notificar_despierta_la_espera() {
        let antes = version();
        let espera = tokio::spawn(esperar_cambio(Some(antes), Duration::from_secs(5)));
        tokio::time::sleep(Duration::from_millis(20)).await;
        notificar();
        let v = tokio::time::timeout(Duration::from_secs(2), espera)
            .await
            .expect("la espera debió despertar")
            .expect("la tarea no debe fallar");
        assert_ne!(v, antes);
    }
}
