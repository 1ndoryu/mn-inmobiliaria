//! [18AA-8] Vía del borrador de Marketplace: dos intentos con
//! `reasoning.effort=low` y un presupuesto total.
//!
//! [por que] Va aparte de `ia_proveedores.rs`: ese fichero ya pasaba de las 500
//! líneas efectivas (`limite-lineas`) y este bloque es de otro dominio —el
//! borrador, no el proveedor—. Allí se quedan el centro IA, las fotos y el STT.

use std::time::Duration;

use super::ia_proveedores::{
    cliente_http, diagnostico_respuesta_vacia, entrada_borrador, uso_de, UsoIa, TOPE_IA_ESTANDAR,
};
use super::leer_env;
use crate::handlers::marketplace::logs::{mp_log, LogNivel};

/* [09AA-15] Vía rápida para borradores (pedido de ella: el borrador tardó
 * 75 s en un hilo). Sonda real contra el relay: SÍ acepta
 * `"reasoning": {"effort": "low"}` (la nota de 09AA-3 que decía que no había
 * perilla era inferencia sin probar; lo que da 400 es `reasoningSummary`, no
 * `effort`). Fail-open: ante 400, vacío, `incomplete`, timeout o red se usa la
 * vía estándar y queda `ia.rapido_fallback` en Logs; el llamador no cambia de
 * forma. Solo la usa el borrador de Marketplace; centro IA y fotos siguen
 * estándar.
 * [18AA-8] 4000, no 2500: el "~60 tokens de razonamiento" de 09AA-15 se midió
 * con un prompt mínimo y no se sostiene con el prompt real del borrador (2754
 * caracteres de instrucciones). Medido contra el relay el 2026-10-20 con ese
 * prompt, a esfuerzo bajo el razonamiento va de 1004 a 2447 tokens según la
 * vuelta —o sea que 2500 caía justo en el borde y la vía rápida salía
 * `incomplete` cada dos por tres, gastando ~20 s para nada antes de caer a la
 * estándar. El tope es un techo, no un objetivo: subirlo no cuesta cuando el
 * modelo para antes. */
const TOPE_BORRADOR_RAPIDO: u32 = 4000;
const TIMEOUT_RAPIDO_SEGS: u64 = 45;

/* [18AA-8] Presupuesto total del borrador. El cliente que lo pide es el
 * núcleo del escritorio (`opencode-propio`), que aborta a los 90 s
 * (`NUCLEO_TIMEOUT_MS`) y pinta «El núcleo no respondió a tiempo»; 75 s deja
 * margen para que el handler conteste algo (IA o plantilla) antes de eso.
 * [por que] Sin este tope el peor caso del borrador eran 45 s de vía rápida
 * + 120 s de estándar + 120 s de reintento ciego = 285 s (18AA-8): el núcleo
 * se cansaba mucho antes y el borrador «no llegaba» aunque el backend
 * siguiera trabajando para nadie. */
const PRESUPUESTO_BORRADOR_SEGS: u64 = 75;

/// Cuerpo Responses del borrador: el estándar más `reasoning.effort=low`.
/// Pura (testeable sin red).
/// [18AA-8] El esfuerzo bajo no es solo velocidad: medido contra el relay el
/// 2026-10-20 con el mismo prompt, el razonamiento baja de ~2500-2800 tokens a
/// ~770-1090, así que deja de comerse el tope entero. Ese era justo el caso
/// que dejaba el borrador sin texto (`incomplete` + `tipos=[reasoning]` +
/// `motivo=max_output_tokens`): subir el tope no reduce el razonamiento, solo
/// lo aloja y tarda más. Lo usan las DOS vías del borrador; centro IA y fotos
/// siguen con el razonamiento por defecto.
fn cuerpo_borrador(
    modelo: &str,
    entrada: &[serde_json::Value],
    tope: u32,
    segs: u64,
) -> serde_json::Value {
    let mut cuerpo = glory_agent::providers::build_responses_body(
        modelo,
        entrada,
        None,
        glory_agent::providers::ChatApiOptions {
            max_output_tokens: tope,
            timeout_secs: segs,
        },
    );
    cuerpo["reasoning"] = serde_json::json!({"effort": "low"});
    cuerpo
}

/// Un intento del borrador, con esfuerzo bajo. `Err(diagnóstico)` si no trajo
/// texto útil (el llamador lo usa tal cual para el WARN) — también si la
/// respuesta vino `incomplete`: su texto está cortado a media frase y para un
/// borrador es peor que la plantilla local.
async fn intento_borrador(
    config: &glory_agent::providers::ProviderConfig,
    entrada: &[serde_json::Value],
    sesion: &str,
    tope: u32,
    segs: u64,
) -> Result<(String, String, Option<UsoIa>), String> {
    let cliente = cliente_http(segs).map_err(|e| e.to_string())?;
    let cuerpo = cuerpo_borrador(&config.model, entrada, tope, segs);
    let resp = cliente
        .post(config.responses_url())
        .header("Authorization", format!("Bearer {}", config.api_key))
        .header("Content-Type", "application/json")
        .header("x-opencode-session", sesion)
        .timeout(Duration::from_secs(segs))
        .json(&cuerpo)
        .send()
        .await
        .map_err(|e| format!("red: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("AI HTTP {}", resp.status()));
    }
    let json: serde_json::Value = resp.json().await.map_err(|e| format!("JSON: {e}"))?;
    if json.get("status").and_then(serde_json::Value::as_str) == Some("incomplete") {
        return Err(diagnostico_respuesta_vacia(&json));
    }
    match glory_agent::providers::extract_first_text(&json) {
        Some(t) => Ok((t, config.model.clone(), uso_de(&json))),
        None => Err(diagnostico_respuesta_vacia(&json)),
    }
}

/// [09AA-30] Devuelve también los tokens del intento que trajo el texto.
/// [18AA-8] Dos intentos, los dos con esfuerzo bajo, dentro de un
/// presupuesto total. Antes la caída a la vía estándar iba por
/// `completar_opencode_con_uso` —razonamiento por defecto, 120 s y un
/// reintento ciego—, y ahí estaba el fallo: el reintento repetía el mismo
/// prompt con el mismo tope, así que ante `incomplete`/`max_output_tokens`
/// volvía a fallar igual y solo gastaba reloj. Ahora el segundo intento sube
/// el tope (2500 → 8000), que es lo único que desatasca ese caso.
pub(crate) async fn completar_opencode_rapido(
    system: &str,
    texto: &str,
    fotos: &[String],
    sesion: &str,
) -> Result<(String, String, Option<UsoIa>), String> {
    let key = leer_env("OPENCODE_GO_API_KEY");
    if key.is_empty() {
        return Err("Sin OPENCODE_GO_API_KEY en .env".to_string());
    }
    let config = glory_agent::providers::ProviderConfig::opencode_go(key);
    let entrada = entrada_borrador(system, texto, fotos);
    let intento = |tope: u32, segs: u64| intento_borrador(&config, &entrada, sesion, tope, segs);
    /* [09AA-5] Cada intento deja su rastro en la tab de Logs (el WARN de
     * `borrador.rs` sigue yendo además al log de texto del vivo). */
    let intentos = async {
        let fallo_rapido = match intento(TOPE_BORRADOR_RAPIDO, TIMEOUT_RAPIDO_SEGS).await {
            Ok(r) => return Ok(r),
            Err(e) => e,
        };
        mp_log(
            LogNivel::Info,
            "ia.rapido_fallback",
            "ia",
            format!("la vía rápida no trajo texto ({fallo_rapido}); va la estándar"),
            &[("modelo", serde_json::json!(config.model))],
        );
        match intento(TOPE_IA_ESTANDAR, PRESUPUESTO_BORRADOR_SEGS).await {
            Ok(r) => {
                mp_log(
                    LogNivel::Info,
                    "ia.reintento_ok",
                    "ia",
                    "el segundo intento trajo texto; se usa la IA".to_string(),
                    &[("modelo", serde_json::json!(config.model))],
                );
                Ok(r)
            }
            Err(e) => {
                mp_log(
                    LogNivel::Warn,
                    "ia.vacia",
                    "fallback",
                    format!("OpenCode Go sin texto en los dos intentos ({e}); va fallback"),
                    &[("modelo", serde_json::json!(config.model))],
                );
                Err(e)
            }
        }
    };
    /* El reloj que manda es el de fuera: cada intento lleva su propio
     * timeout, pero el presupuesto corta antes de que el núcleo se canse. Si
     * el primero se agota pronto, el segundo hereda lo que quede. */
    match tokio::time::timeout(Duration::from_secs(PRESUPUESTO_BORRADOR_SEGS), intentos).await {
        Ok(r) => r,
        Err(_) => {
            mp_log(
                LogNivel::Warn,
                "ia.presupuesto",
                "fallback",
                format!("el borrador agotó su presupuesto de {PRESUPUESTO_BORRADOR_SEGS} s; va fallback"),
                &[("modelo", serde_json::json!(config.model))],
            );
            Err(format!(
                "presupuesto del borrador agotado ({} s)",
                PRESUPUESTO_BORRADOR_SEGS
            ))
        }
    }
}

/* [09AA-4] Puras, sin DB: el cuerpo y el presupuesto no tocan la red. */
#[cfg(test)]
mod pruebas {
    use super::{
        cuerpo_borrador, PRESUPUESTO_BORRADOR_SEGS, TIMEOUT_RAPIDO_SEGS, TOPE_BORRADOR_RAPIDO,
        TOPE_IA_ESTANDAR,
    };

    /* [09AA-15] Las dos vías del borrador piden esfuerzo bajo, sin cambiar
     * modelo ni entrada; lo único que cambia entre ellas es el tope. */
    #[test]
    fn cuerpo_borrador_pide_esfuerzo_bajo_en_las_dos_vias() {
        let entrada = vec![serde_json::json!({"role": "user", "content": "hola"})];
        for tope in [TOPE_BORRADOR_RAPIDO, TOPE_IA_ESTANDAR] {
            let cuerpo = cuerpo_borrador("modelo-x", &entrada, tope, TIMEOUT_RAPIDO_SEGS);
            assert_eq!(cuerpo["model"], serde_json::json!("modelo-x"));
            assert_eq!(cuerpo["reasoning"], serde_json::json!({"effort": "low"}));
            assert_eq!(cuerpo["max_output_tokens"], serde_json::json!(tope));
            assert_eq!(cuerpo["input"], serde_json::json!(entrada));
        }
        assert_eq!(TOPE_BORRADOR_RAPIDO, 4000);
        assert_eq!(TOPE_IA_ESTANDAR, 8000);
    }

    /* [18AA-8] El presupuesto del borrador tiene que caber bajo el corte del
     * núcleo del escritorio (opencode-propio, `NUCLEO_TIMEOUT_MS` = 90 s): si
     * no, el borrador «no llega» aunque el backend siga trabajando. Y el
     * segundo intento necesita más tope que el primero: es lo único que
     * desatasca el `incomplete` por `max_output_tokens`. */
    #[test]
    fn presupuesto_del_borrador_cabe_bajo_el_corte_del_nucleo() {
        const CORTE_NUCLEO_SEGS: u64 = 90;
        assert!(PRESUPUESTO_BORRADOR_SEGS < CORTE_NUCLEO_SEGS);
        assert!(TIMEOUT_RAPIDO_SEGS < PRESUPUESTO_BORRADOR_SEGS);
        assert!(TOPE_IA_ESTANDAR > TOPE_BORRADOR_RAPIDO);
    }
}
