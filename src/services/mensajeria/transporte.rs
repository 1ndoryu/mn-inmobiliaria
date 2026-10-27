/* [06AA-3 F3] Capa `Transporte` (plan 03AA-4 §Arquitectura): el evento crudo
 * del gateway normalizado a decisión de encaminamiento, sin BD de negocio
 * (solo lee `agent_config` para los números A/B). Lo que era prefijo de
 * `handlers/whatsapp.rs` (DTO + secreto + números + reparto puro) vive aquí;
 * el webhook solo orquesta. Sin cambios de conducta: movimientos verbatim.
 * [07AA-2 F5] Un solo número atiende: `reparto` solo reconoce A; el B
 * jubilado se detecta con `destino_jubilado` (el flujo calla con motivo
 * `no:canal-jubilado`, sin persistir ni turno) y lo demás sigue 400. */

use serde::Deserialize;

use crate::repositories::ClienteRepository;
use crate::services::modo_por_canal;

#[derive(Debug, Deserialize)]
pub struct EntradaWhatsapp {
    pub(crate) numero_destino: String,
    pub(crate) remitente: String,
    pub(crate) texto: String,
    pub(crate) nombre: Option<String>,
    pub(crate) media_url: Option<String>,
    /* [06AA-1] Campos del transporte real (F-transporte): el simulado no los
     * manda (`None` = dato ausente = el triage atiende, regla de oro). */
    #[serde(default)]
    pub(crate) ts_ms: Option<i64>,
    #[serde(default)]
    pub(crate) from_me: Option<bool>,
    #[serde(default)]
    pub(crate) es_sistema: Option<bool>,
    #[serde(default)]
    pub(crate) client_seq: Option<String>,
}

/// Números MN por defecto (E.164 sin `+`; se normalizan igual que el resto).
/// El B es el vivo del negocio: no usar hasta el final (plan §Estado).
fn numero_a_defecto() -> String {
    std::env::var("WA_NUMERO_A").unwrap_or_else(|_| "584120825234".to_string())
}

fn numero_b_defecto() -> String {
    std::env::var("WA_NUMERO_B").unwrap_or_else(|_| "584249208855".to_string())
}

/// Tercer número (C): solo archiva, nunca responde (`wa_c`/`archivar`). Sin
/// valor no reparte nada: vacío = C sin configurar.
fn numero_c_defecto() -> String {
    std::env::var("WA_NUMERO_C").unwrap_or_default()
}

/// Números configurados: `agent_config` (`wa_numero_a`/`wa_numero_b`/
/// `wa_numero_c`, editables en consola F5) con fallback a env
/// (`WA_NUMERO_A`/`WA_NUMERO_B`/`WA_NUMERO_C`).
pub(crate) async fn numeros_configurados(pool: &sqlx::PgPool) -> (String, String, String) {
    let a = glory_agent::persistence::get_config(pool, "wa_numero_a")
        .await
        .ok()
        .flatten()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(numero_a_defecto);
    let b = glory_agent::persistence::get_config(pool, "wa_numero_b")
        .await
        .ok()
        .flatten()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(numero_b_defecto);
    let c = glory_agent::persistence::get_config(pool, "wa_numero_c")
        .await
        .ok()
        .flatten()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(numero_c_defecto);
    (
        ClienteRepository::normalizar_telefono(&a),
        ClienteRepository::normalizar_telefono(&b),
        ClienteRepository::normalizar_telefono(&c),
    )
}

/// Reparto puro por destino (testeable sin BD): A atiende
/// (`wa_a`/`completo`) y C solo archiva (`wa_c`/`archivar`: guarda el
/// mensaje y sus medios, sin triage ni turno).
/// [07AA-2 F5] El B jubilado y lo desconocido devuelven `None`: el flujo
/// distingue con `destino_jubilado` (calla con motivo) del 400 clásico.
/// El `modo` sale de `modo_por_canal` (fuente única, compartida con el resolutor).
#[must_use]
pub fn reparto(
    numero_a: &str,
    numero_c: &str,
    numero_destino: &str,
) -> Option<(&'static str, &'static str)> {
    let destino = ClienteRepository::normalizar_telefono(numero_destino);
    let canal = if destino == ClienteRepository::normalizar_telefono(numero_a) {
        "wa_a"
    } else {
        /* C vacío no reparte nada: compararlo dejaría que un destino vacío
         * casara con una C sin configurar. */
        let c = ClienteRepository::normalizar_telefono(numero_c);
        if c.is_empty() || destino != c {
            return None;
        }
        "wa_c"
    };
    Some((canal, modo_por_canal(canal)?))
}

/// Destino al número B jubilado (normalizado igual que el reparto): el
/// webhook responde 2xx `no:canal-jubilado` sin persistir ni correr turno.
/// No valida formato (eso lo hace `repartir_y_vincular` antes): solo compara.
#[must_use]
pub fn destino_jubilado(numero_b: &str, numero_destino: &str) -> bool {
    ClienteRepository::normalizar_telefono(numero_destino)
        == ClienteRepository::normalizar_telefono(numero_b)
}

/// Secreto compartido con el gateway Baileys: el webhook exige la cabecera
/// `X-Gateway-Secret` idéntica a `WA_WEBHOOK_SECRETO` (401 si falta o
/// difiere). Sin secreto configurado rechaza todo (falla cerrado): antes
/// acepta cualquier llamada y cualquiera podía inyectar mensajes. Comparación
/// exacta, sin normalizar (un secreto con espacios se respeta tal cual).
#[must_use]
pub fn secreto_valido(esperado: Option<&str>, recibido: Option<&str>) -> bool {
    let Some(sec) = esperado.filter(|s| !s.is_empty()) else {
        return false;
    };
    recibido.is_some_and(|r| r == sec)
}
