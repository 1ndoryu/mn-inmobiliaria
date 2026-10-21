/* [16AA-12] Lectura del perfil de Marketplace a petición. El modal deja la
 * marca y la lab la ve en el latido que ya sondea cada 5 s
 * (`GET .../borradores/version`); entonces lee el perfil con la sesión de
 * Facebook que solo ella tiene y manda los avisos. Proceso-local, igual que
 * `version_borradores_borrados`: un reinicio del backend pierde la petición
 * pendiente y la usuaria la vuelve a pulsar, que es lo que espera. */

use std::sync::atomic::{AtomicI64, Ordering};

use chrono::{DateTime, TimeZone, Utc};

/// Milisegundos de la petición pendiente (0 = ninguna). Es a la vez la marca
/// que compara la lab: cada pulsación trae un valor nuevo, así que una lectura
/// que falló no se reintenta sola en cada latido.
static SOLICITADA_EN_MS: AtomicI64 = AtomicI64::new(0);

/// Pide una lectura y devuelve el instante de la petición (la marca que verá la lab).
pub fn pedir_lectura() -> DateTime<Utc> {
    let ahora = Utc::now();
    SOLICITADA_EN_MS.store(ahora.timestamp_millis(), Ordering::SeqCst);
    ahora
}

/// Petición pendiente, si la hay. `None` mientras nadie la haya pedido o la lab ya la atendió.
pub fn lectura_pedida() -> Option<DateTime<Utc>> {
    let ms = SOLICITADA_EN_MS.load(Ordering::SeqCst);
    (ms > 0)
        .then(|| Utc.timestamp_millis_opt(ms).single())
        .flatten()
}

/// La lab ya envió la lectura: la petición queda cerrada.
pub fn marcar_atendida() {
    SOLICITADA_EN_MS.store(0, Ordering::SeqCst);
}

#[cfg(test)]
mod pruebas {
    use super::*;

    /// El estado es de proceso: las pruebas de este módulo comparten el estático,
    /// así que cada una deja la marca cerrada al terminar.
    #[test]
    fn pedir_deja_marca_y_atender_la_cierra() {
        marcar_atendida();
        assert!(lectura_pedida().is_none());

        let pedida = pedir_lectura();
        let vista = lectura_pedida().expect("la petición queda pendiente");
        assert_eq!(vista.timestamp_millis(), pedida.timestamp_millis());

        marcar_atendida();
        assert!(lectura_pedida().is_none());
    }
}
