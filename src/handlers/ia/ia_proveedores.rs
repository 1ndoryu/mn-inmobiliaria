//! [08AA-8] Proveedores de IA (`GloryAPI` + `OpenCode` Go + Groq STT).
//!
//! Extraído de `rutas.rs` (god-object + límite 500): cliente HTTP, `ping`
//! de ambos proveedores, `completar` de ambos, transcripción de audio con
//! rotación de claves y descripción de fotos. `rutas.rs` conserva el boundary
//! HTTP (`routes` y `completar`) y re-exporta lo que usan terceros
//! (`marketplace`, `turno`); el estado y la config viven en `ia_estado.rs` e
//! `ia_admin.rs` ([16AA-9 F4]).

use std::time::Duration;

use crate::errors::AppError;

use super::{glory_base, leer_env, MAX_FOTOS};
/* [09AA-5] Eventos de la tab de Logs (sin PII). */
use crate::handlers::marketplace::logs::{mp_log, LogNivel};

pub(super) fn cliente_http(segs: u64) -> Result<reqwest::Client, AppError> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(segs))
        .build()
        .map_err(|e| AppError::Internal(format!("AI http: {e}")))
}

pub(super) async fn ping_glory(base: &str, key: &str) -> Result<String, String> {
    let cliente = cliente_http(30).map_err(|e| e.to_string())?;
    let resp = cliente
        .post(format!("{base}/v1/chat/completions"))
        .header("Authorization", format!("Bearer {key}"))
        .json(&serde_json::json!({
            "model": crate::config::ia::MODELO_GLORYAPI,
            "stream": false,
            "max_tokens": 16,
            "messages": [
                {"role": "system", "content": "Responde exactamente: OK"},
                {"role": "user", "content": "ping"},
            ],
        }))
        .send()
        .await
        .map_err(|e| format!("GloryAPI red: {e}"))?;
    if resp.status() == 401 || resp.status() == 403 {
        return Err("GloryAPI rechazo la clave (revisa GLORY_API_KEY)".to_string());
    }
    if !resp.status().is_success() {
        return Err(format!("GloryAPI HTTP {}", resp.status()));
    }
    let cuerpo: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("GloryAPI respuesta no JSON: {e}"))?;
    Ok(cuerpo
        .pointer("/model")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("auto")
        .to_string())
}

pub(super) async fn ping_opencode(key: &str) -> Result<String, String> {
    let cliente = cliente_http(30).map_err(|e| e.to_string())?;
    let config = glory_agent::providers::ProviderConfig::opencode_go(key.to_string());
    let cuerpo = glory_agent::providers::build_responses_body(
        &config.model,
        &[serde_json::json!({"role": "user", "content": "Responde exactamente: OK"})],
        None,
        glory_agent::providers::ChatApiOptions::terse(16),
    );
    let resp = cliente
        .post(config.responses_url())
        .header("Authorization", format!("Bearer {key}"))
        .header("x-opencode-session", uuid::Uuid::new_v4().to_string())
        .json(&cuerpo)
        .send()
        .await
        .map_err(|e| format!("OpenCode Go red: {e}"))?;
    if resp.status() == 401 || resp.status() == 403 {
        return Err("OpenCode Go rechazo la clave (revisa OPENCODE_GO_API_KEY)".to_string());
    }
    if !resp.status().is_success() {
        let trozo: String = resp
            .text()
            .await
            .unwrap_or_default()
            .chars()
            .take(120)
            .collect();
        return Err(format!("OpenCode Go HTTP: {trozo}"));
    }
    Ok(config.model.clone())
}

fn texto_glory(cuerpo: &serde_json::Value) -> Option<String> {
    let contenido = cuerpo.pointer("/choices/0/message/content")?;
    let texto = contenido.as_str()?;
    let texto = texto.trim();
    if texto.is_empty() {
        None
    } else {
        Some(texto.to_string())
    }
}

pub(super) async fn completar_glory(
    system: &str,
    texto: &str,
    fotos: &[String],
) -> Result<(String, String), String> {
    let key = leer_env("GLORY_API_KEY");
    if key.is_empty() {
        return Err("Sin GLORY_API_KEY en .env".to_string());
    }
    completar_glory_con(&glory_base(), &key, system, texto, fotos).await
}

/* Un intento contra GloryAPI. El `bool` dice si merece repetirse: un 4xx de
 * verdad (400 mal armado, 401/403 clave) no cambia por insistir y solo quema
 * tiempo; red rota, 408, 429 y 5xx sí son pasajeros. */
async fn intento_glory(
    cliente: &reqwest::Client,
    base: &str,
    key: &str,
    cuerpo: &serde_json::Value,
) -> Result<serde_json::Value, (bool, String)> {
    let resp = cliente
        .post(format!("{base}/v1/chat/completions"))
        .header("Authorization", format!("Bearer {key}"))
        .json(cuerpo)
        .send()
        .await
        .map_err(|e| (true, format!("GloryAPI red: {e}")))?;
    if resp.status() == 401 || resp.status() == 403 {
        return Err((
            false,
            "GloryAPI rechazo la clave (revisa GLORY_API_KEY)".to_string(),
        ));
    }
    if !resp.status().is_success() {
        let estado = resp.status().as_u16();
        let trozo: String = resp
            .text()
            .await
            .unwrap_or_default()
            .chars()
            .take(200)
            .collect();
        let pasajero = estado == 408 || estado == 429 || estado >= 500;
        return Err((pasajero, format!("GloryAPI HTTP: {trozo}")));
    }
    resp.json()
        .await
        .map_err(|e| (false, format!("GloryAPI respuesta no JSON: {e}")))
}

/* [09AA-11] Base y clave inyectables (testeable con un mock local). Un fallo
 * pasajero se repite una sola vez: antes, una caída puntual dejaba al cliente
 * sin respuesta y el hilo se quedaba a medias. La pausa es corta a propósito:
 * el tope de 18AA-8 ya es largo y esto suma como mucho un intento. */
async fn completar_glory_con(
    base: &str,
    key: &str,
    system: &str,
    texto: &str,
    fotos: &[String],
) -> Result<(String, String), String> {
    let mut partes = vec![serde_json::json!({"type": "text", "text": texto})];
    for foto in fotos.iter().take(MAX_FOTOS) {
        partes.push(serde_json::json!({"type": "image_url", "image_url": {"url": foto}}));
    }
    let cuerpo = serde_json::json!({
        "model": crate::config::ia::MODELO_GLORYAPI,
        "stream": false,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": partes},
        ],
    });
    let cliente = cliente_http(120).map_err(|e| e.to_string())?;
    let respuesta = match intento_glory(&cliente, base, key, &cuerpo).await {
        Ok(r) => r,
        Err((true, primero)) => {
            tokio::time::sleep(Duration::from_millis(400)).await;
            match intento_glory(&cliente, base, key, &cuerpo).await {
                Ok(r) => {
                    mp_log(
                        LogNivel::Info,
                        "ia.reintento_glory",
                        "ia",
                        "el reintento a GloryAPI trajo respuesta".to_string(),
                        &[("primer_fallo", serde_json::json!(primero))],
                    );
                    r
                }
                Err((_, segundo)) => return Err(format!("{primero} (tras reintento: {segundo})")),
            }
        }
        Err((false, fallo)) => return Err(fallo),
    };
    let modelo = respuesta
        .pointer("/model")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("auto")
        .to_string();
    texto_glory(&respuesta)
        .map(|t| (t, modelo))
        .ok_or_else(|| "GloryAPI devolvio una respuesta sin texto".to_string())
}

/* [03AA-3 M3] Se expone al handler `marketplace` para generar borradores;
 * sigue sin ruta HTTP propia (solo `probar`/`completar` del centro de IA).
 * [09AA-4] Diagnóstico sin PII de una respuesta Responses sin texto: solo
 * forma (estado, tipos de `output[]`, uso, motivo de `incomplete`), nunca el
 * contenido. El vacío existe de verdad (2026-10-09: 200 sin `message`, sin
 * `AI incompleta` en el log = no es tope de tokens; el relay estaba sano en
 * replay mínimo), así que ante un vacío persistente el WARN dice QUÉ vino. */
pub(super) fn diagnostico_respuesta_vacia(resp: &serde_json::Value) -> String {
    let estado = resp
        .get("status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("?");
    let mut tipos: Vec<&str> = Vec::new();
    if let Some(items) = resp.get("output").and_then(serde_json::Value::as_array) {
        for item in items {
            if let Some(t) = item.get("type").and_then(serde_json::Value::as_str) {
                tipos.push(t);
            }
        }
    }
    let motivo = resp
        .get("incomplete_details")
        .and_then(|d| d.get("reason"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("-");
    /* [09AA-11] Solo `usage`: la rama de `uso` se escribió por si el relay lo
     * mandaba en castellano y nunca hubo productor (grep en `src/`: ninguno). */
    let uso = resp
        .get("usage")
        .map_or_else(|| "-".to_string(), uso_responses);
    format!(
        "estado={estado} tipos=[{}] motivo={motivo} uso={uso}",
        tipos.join(",")
    )
}

/// `in/out` de `usage` como texto; ausente o no numérico → `-`.
fn uso_responses(uso: &serde_json::Value) -> String {
    let num = |c: &str| uso.get(c).and_then(serde_json::Value::as_u64).unwrap_or(0);
    format!("in={} out={}", num("input_tokens"), num("output_tokens"))
}

async fn llamar_opencode(
    config: &glory_agent::providers::ProviderConfig,
    entrada: &[serde_json::Value],
    cliente: &reqwest::Client,
    tope: u32,
    sesion: &str,
) -> Result<serde_json::Value, String> {
    glory_agent::providers::call_provider(
        config,
        entrada,
        None,
        glory_agent::providers::ChatApiOptions {
            max_output_tokens: tope,
            timeout_secs: 120,
        },
        Some(sesion),
        cliente,
    )
    .await
    .map_err(|e| format!("OpenCode Go: {e}"))
}

/* [09AA-15] Entrada Responses compartida por la vía estándar y la rápida
 * (mismo system + texto + fotos): lo único que cambia es el cuerpo
 * (tope + `reasoning.effort`) y el timeout. */
pub(super) fn entrada_borrador(
    system: &str,
    texto: &str,
    fotos: &[String],
) -> Vec<serde_json::Value> {
    let mut contenido: Vec<serde_json::Value> =
        vec![serde_json::json!({"type": "input_text", "text": texto})];
    for foto in fotos.iter().take(MAX_FOTOS) {
        contenido.push(serde_json::json!({"type": "input_image", "image_url": foto}));
    }
    vec![
        serde_json::json!({"role": "system", "content": system}),
        serde_json::json!({"role": "user", "content": contenido}),
    ]
}

/* `sesion`: id estable por conversación para `x-opencode-session` (afinidad
 * de ruteo + prompt caching del relay; ver docs de Go). Estable = mismo
 * valor para el mismo hilo (un hash, jamás PII en claro); quien no tiene
 * hilo pasa una etiqueta fija (`"centro-ia"`, `"fotos"`). Un uuid fresco por
 * llamada también evita el 400, pero rompe la afinidad y parece abuso. */
pub(crate) async fn completar_opencode(
    system: &str,
    texto: &str,
    fotos: &[String],
    sesion: &str,
) -> Result<(String, String), String> {
    completar_opencode_con_uso(system, texto, fotos, sesion)
        .await
        .map(|(t, modelo, _)| (t, modelo))
}

/* [08AA-19] 4000, no 2500: el modelo razona antes de redactar y un prompt
 * normal ya quema ~1788 tokens de razonamiento (medido 2026-10-08 contra el
 * endpoint real); con 2500 el borrador largo caía en `incomplete` sin
 * `message` y acababa en `reserva`. El texto útil son ~150 tokens.
 * [09AA-4] 8000, no 4000: el vacío del 2026-10-09 no era tope (sin `AI
 * incompleta`), pero el razonamiento varía por hilo y 4000 se midió una sola
 * vez; 8000 da aire como `standard()` (8192) sin cambiar el coste del texto
 * útil. */
pub(super) const TOPE_IA_ESTANDAR: u32 = 8000;

/// [09AA-30] Igual que `completar_opencode` pero devuelve también los tokens
/// del `usage` de la respuesta que trajo el texto (`None` si el relay no lo
/// trae).
pub(crate) async fn completar_opencode_con_uso(
    system: &str,
    texto: &str,
    fotos: &[String],
    sesion: &str,
) -> Result<(String, String, Option<UsoIa>), String> {
    let key = leer_env("OPENCODE_GO_API_KEY");
    if key.is_empty() {
        return Err("Sin OPENCODE_GO_API_KEY en .env".to_string());
    }
    let config = glory_agent::providers::ProviderConfig::opencode_go(key.clone());
    let entrada = entrada_borrador(system, texto, fotos);
    let cliente = cliente_http(120).map_err(|e| e.to_string())?;
    let mut respuesta =
        llamar_opencode(&config, &entrada, &cliente, TOPE_IA_ESTANDAR, sesion).await?;
    let mut texto_ia = glory_agent::providers::extract_first_text(&respuesta);
    /* [09AA-4] Un reintento ante vacío: el relay a veces devuelve 200 sin
     * `message` de forma transitoria (incidente 2026-10-09 con relay sano
     * en replay). Solo en el camino de fallo, como mucho una llamada más. */
    if texto_ia.is_none() {
        respuesta = llamar_opencode(&config, &entrada, &cliente, TOPE_IA_ESTANDAR, sesion).await?;
        texto_ia = glory_agent::providers::extract_first_text(&respuesta);
        /* [09AA-5] Lo que hizo el reintento queda en la tab de Logs (el
         * WARN de abajo sigue yendo además al log de texto del vivo). */
        if texto_ia.is_some() {
            mp_log(
                LogNivel::Info,
                "ia.reintento_ok",
                "ia",
                "el reintento trajo texto; se usa la IA".to_string(),
                &[("modelo", serde_json::json!(config.model))],
            );
        } else {
            mp_log(
                LogNivel::Warn,
                "ia.vacia",
                "fallback",
                format!(
                    "OpenCode Go vacío tras reintento ({}); va fallback",
                    diagnostico_respuesta_vacia(&respuesta)
                ),
                &[("modelo", serde_json::json!(config.model))],
            );
        }
    }
    let uso = uso_de(&respuesta);
    texto_ia
        .map(|t| (t, config.model.clone(), uso))
        .ok_or_else(|| {
            tracing::warn!(
                "OpenCode Go vacío persistente ({}); va fallback",
                diagnostico_respuesta_vacia(&respuesta)
            );
            "OpenCode Go devolvio una respuesta sin texto".to_string()
        })
}

/// [09AA-30] Tokens que la Responses API reporta en `usage`. `None` si el
/// relay no lo trae (el texto vale igual; solo falta el coste).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct UsoIa {
    pub entrada: i64,
    pub salida: i64,
}

pub(super) fn uso_de(json: &serde_json::Value) -> Option<UsoIa> {
    let usage = json.get("usage")?;
    Some(UsoIa {
        entrada: usage.get("input_tokens")?.as_i64()?,
        salida: usage.get("output_tokens")?.as_i64()?,
    })
}

/// [309A-4] Transcribe una nota de voz con Groq Whisper
/// (`whisper-large-v3-turbo`, endpoint OpenAI-compatible). Opencode Go no
/// pasa audio por ninguna vía (verificado: Responses pela `input_audio`,
/// chat pela `audio_url`, sin endpoint `/audio/transcriptions`), así que el
/// STT sale por aquí. Best-effort: cualquier `Err` y el llamador conserva el
/// `[audio]` pelado + el prompt pide que lo escriban.
/// [309A-5] Rotación: prueba `GROQ_API_KEY` + respaldos `_2..=_9` en orden;
/// ante 401/403/429/5xx/red rota a la siguiente (WARN con el # de clave,
/// nunca la clave). Agotadas todas → `Err` y fail-open en el hilo.
pub(crate) async fn transcribir_audio(
    bytes: &[u8],
    nombre: &str,
    mime: &str,
) -> Result<String, String> {
    const GROQ_URL: &str = "https://api.groq.com/openai/v1/audio/transcriptions";
    transcribir_audio_con(bytes, nombre, mime, &claves_groq(), GROQ_URL).await
}

/// [309A-5] Claves STT en orden (primaria + respaldos no vacíos).
fn claves_groq() -> Vec<String> {
    let mut todas = vec![leer_env("GROQ_API_KEY")];
    for n in 2..=9 {
        todas.push(leer_env(&format!("GROQ_API_KEY_{n}")));
    }
    todas.into_iter().filter(|k| !k.is_empty()).collect()
}

/// [309A-5] Núcleo con claves+URL inyectables (testeable con mock local).
/// 400 no rota (petición malformada: reintentar no ayuda); 200 vacío sí rota
/// (puede ser flakiness del modelo). Timeout 120 s por intento: corre en el
/// spawn del webhook, nunca en el camino del 2xx ni del turno.
pub(super) async fn transcribir_audio_con(
    bytes: &[u8],
    nombre: &str,
    mime: &str,
    claves: &[String],
    url: &str,
) -> Result<String, String> {
    const MAX_TEXTO: usize = 2000;
    if claves.is_empty() {
        return Err("Sin GROQ_API_KEY en .env".to_string());
    }
    let mut ultimo = "sin intentos".to_string();
    for (i, key) in claves.iter().enumerate() {
        let parte = reqwest::multipart::Part::bytes(bytes.to_vec())
            .file_name(nombre.to_string())
            .mime_str(mime)
            .map_err(|e| format!("Groq audio invalido: {e}"))?;
        let forma = reqwest::multipart::Form::new()
            .part("file", parte)
            .text("model", crate::config::ia::MODELO_STT)
            .text("language", "es")
            .text("response_format", "text");
        let cliente = cliente_http(120).map_err(|e| e.to_string())?;
        let resp = match cliente
            .post(url)
            .header("Authorization", format!("Bearer {key}"))
            .multipart(forma)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                ultimo = format!("Groq red: {e}");
                tracing::warn!("STT: clave #{} sin red, roto a la siguiente", i + 1);
                continue;
            }
        };
        let estado = resp.status();
        if estado == 401 || estado == 403 || estado == 429 || estado.is_server_error() {
            ultimo = format!("Groq HTTP {estado} con clave #{}", i + 1);
            tracing::warn!("STT: {ultimo}, roto a la siguiente");
            continue;
        }
        if !estado.is_success() {
            let trozo: String = resp
                .text()
                .await
                .unwrap_or_default()
                .chars()
                .take(200)
                .collect();
            return Err(format!("Groq HTTP {estado}: {trozo}"));
        }
        let texto: String = resp
            .text()
            .await
            .map_err(|e| format!("Groq respuesta no texto: {e}"))?
            .trim()
            .chars()
            .take(MAX_TEXTO)
            .collect();
        if texto.is_empty() {
            ultimo = "Groq devolvio transcripcion vacia".to_string();
            tracing::warn!("STT: {ultimo}, roto a la siguiente");
            continue;
        }
        return Ok(texto);
    }
    Err(ultimo)
}

/// [299A-1 E11] La IA del turno de `WhatsApp` es texto puro (el transporte de
/// `glory-agent` arma `input` Responses solo con strings, sin `input_image`):
/// para que "vea" la foto entrante se describe aquí con visión real
/// (`completar_opencode`, probado con `input_image`) y el llamador anexa el
/// texto al mensaje `[foto]` ANTES del turno. Best-effort: cualquier `Err` y
/// el llamador conserva el `[foto]` pelado + el prompt pide que la describan.
pub(crate) async fn describir_foto(data_url: &str, pie: &str) -> Result<String, String> {
    let pie = pie.trim();
    let texto = if pie.is_empty() || pie == "(foto sin pie)" {
        "Describe lo que se ve en esta foto en 2-3 frases: qué tipo de ambiente \
         o lugar es, su estado y los detalles visibles relevantes para alguien \
         que busca inmueble."
            .to_string()
    } else {
        format!(
            "El visitante envió esta foto con el pie: \"{pie}\". Describe lo que \
             se ve en 2-3 frases (ambiente, estado, detalles visibles para \
             alguien que busca inmueble)."
        )
    };
    let (texto, _) = completar_opencode(
        "Eres el ojo del Asistente de IA de MN Inmobiliaria. Respondes solo con \
         la descripción de la foto, texto plano, sin adornos.",
        &texto,
        &[data_url.to_string()],
        "fotos",
    )
    .await?;
    Ok(texto.chars().take(1200).collect())
}

/* [09AA-4] Puras, sin DB: el diagnóstico solo describe la forma. */
#[cfg(test)]
mod pruebas {
    use super::diagnostico_respuesta_vacia;

    #[test]
    fn diagnostico_describe_respuesta_vacia_sin_pii() {
        let vacia = serde_json::json!({
            "status": "completed",
            "output": [{"type": "reasoning"}],
            "usage": {"input_tokens": 12, "output_tokens": 118},
        });
        assert_eq!(
            diagnostico_respuesta_vacia(&vacia),
            "estado=completed tipos=[reasoning] motivo=- uso=in=12 out=118"
        );
        let incompleta = serde_json::json!({
            "status": "incomplete",
            "output": [{"type": "reasoning"}],
            "incomplete_details": {"reason": "max_output_tokens"},
            "usage": {"input_tokens": 2000, "output_tokens": 8000},
        });
        assert_eq!(
            diagnostico_respuesta_vacia(&incompleta),
            "estado=incomplete tipos=[reasoning] motivo=max_output_tokens uso=in=2000 out=8000"
        );
        let rota: serde_json::Value = serde_json::json!({"output": "no-es-arreglo"});
        assert_eq!(
            diagnostico_respuesta_vacia(&rota),
            "estado=? tipos=[] motivo=- uso=-"
        );
    }

    /* [309A-5] Sin claves no hay red: falla antes de mirar la URL. */
    #[test]
    fn transcribir_sin_claves_no_toca_red() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let r = rt.block_on(super::transcribir_audio_con(
            &[1, 2, 3],
            "a.mp3",
            "audio/mpeg",
            &[],
            "http://127.0.0.1:9/",
        ));
        assert!(r.is_err());
    }

    /* [309A-5] Come una petición HTTP/1.1 completa del mock (cabeceras +
     * cuerpo multipart) para que el cliente no vea un RST. */
    async fn comer_peticion(zocalo: &mut tokio::net::TcpStream) {
        use tokio::io::AsyncReadExt;
        let mut todo = Vec::new();
        let mut buf = [0u8; 8192];
        let (cab, largo) = loop {
            let n = zocalo.read(&mut buf).await.expect("mock lee");
            assert!(n > 0, "el cliente cerro sin pedir");
            todo.extend_from_slice(&buf[..n]);
            if let Some(pos) = todo.windows(4).position(|w| w == b"\r\n\r\n") {
                let cabecera = String::from_utf8_lossy(&todo[..pos]).to_string();
                let largo = cabecera
                    .lines()
                    .find_map(|l| {
                        l.to_ascii_lowercase()
                            .strip_prefix("content-length: ")
                            .map(|v: &str| v.trim().parse().unwrap_or(0))
                    })
                    .unwrap_or(0);
                break (pos + 4, largo);
            }
        };
        while todo.len() < cab + largo {
            let n = zocalo.read(&mut buf).await.expect("mock lee cuerpo");
            assert!(n > 0, "cuerpo cortado");
            todo.extend_from_slice(&buf[..n]);
        }
    }

    /* [309A-5] Rotación contra mock local: 401 con la 1ª → la 2ª transcribe.
     * Certeza sin depender de Groq real ni de VPN. */
    #[tokio::test]
    async fn transcribir_rota_clave_tras_401() {
        use tokio::io::AsyncWriteExt;
        let oyente = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!(
            "http://127.0.0.1:{}/audio/transcriptions",
            oyente.local_addr().unwrap().port()
        );
        let servo = tokio::spawn(async move {
            let mut usadas = 0;
            for (estado, cuerpo) in [(401, "no"), (200, "hola rotacion")] {
                let (mut zocalo, _) = oyente.accept().await.unwrap();
                comer_peticion(&mut zocalo).await;
                let resp = format!(
                    "HTTP/1.1 {estado} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{cuerpo}",
                    cuerpo.len()
                );
                zocalo.write_all(resp.as_bytes()).await.unwrap();
                usadas += 1;
            }
            usadas
        });
        let claves = vec!["mala".to_string(), "buena".to_string()];
        let texto = super::transcribir_audio_con(&[1, 2, 3], "a.mp3", "audio/mpeg", &claves, &url)
            .await
            .unwrap();
        assert_eq!(texto, "hola rotacion");
        assert_eq!(servo.await.unwrap(), 2, "debió probar ambas claves");
    }

    /* [309A-5] Groq caído (401 en todas) → Err: el llamador deja el `[audio]`
     * pelado y el prompt pide el texto (fail-open garantizado). */
    #[tokio::test]
    async fn transcribir_caido_en_todas_falla_cerrado_util() {
        use tokio::io::AsyncWriteExt;
        let oyente = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!(
            "http://127.0.0.1:{}/audio/transcriptions",
            oyente.local_addr().unwrap().port()
        );
        let servo = tokio::spawn(async move {
            for _ in 0..2 {
                let (mut zocalo, _) = oyente.accept().await.unwrap();
                comer_peticion(&mut zocalo).await;
                zocalo
                    .write_all(
                        b"HTTP/1.1 401 X\r\nContent-Length: 2\r\nConnection: close\r\n\r\nno",
                    )
                    .await
                    .unwrap();
            }
        });
        let claves = vec!["una".to_string(), "otra".to_string()];
        let r =
            super::transcribir_audio_con(&[1, 2, 3], "a.mp3", "audio/mpeg", &claves, &url).await;
        assert!(r.is_err());
        servo.await.unwrap();
    }

    /* [09AA-11] Un 5xx pasajero se repite una vez y se usa la respuesta
     * buena: antes el hilo se quedaba sin texto por una caída puntual. */
    #[tokio::test]
    async fn glory_reintenta_tras_5xx() {
        use tokio::io::AsyncWriteExt;
        let oyente = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://127.0.0.1:{}", oyente.local_addr().unwrap().port());
        let bueno = r#"{"model":"glory-1","choices":[{"message":{"content":"hola"}}]}"#;
        let servo = tokio::spawn(async move {
            let mut usadas = 0;
            for (estado, cuerpo) in [(502, "mal"), (200, bueno)] {
                let (mut zocalo, _) = oyente.accept().await.unwrap();
                comer_peticion(&mut zocalo).await;
                let resp = format!(
                    "HTTP/1.1 {estado} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{cuerpo}",
                    cuerpo.len()
                );
                zocalo.write_all(resp.as_bytes()).await.unwrap();
                usadas += 1;
            }
            usadas
        });
        let (texto, modelo) = super::completar_glory_con(&base, "k", "sys", "hola", &[])
            .await
            .unwrap();
        assert_eq!(texto, "hola");
        assert_eq!(modelo, "glory-1");
        assert_eq!(servo.await.unwrap(), 2, "debio repetir tras el 5xx");
    }

    /* [09AA-11] Un 400 no se repite: la petición está mal y el reintento solo
     * alargaría la espera del cliente. */
    #[tokio::test]
    async fn glory_no_reintenta_tras_400() {
        use std::time::Duration;
        use tokio::io::AsyncWriteExt;
        let oyente = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://127.0.0.1:{}", oyente.local_addr().unwrap().port());
        let servo = tokio::spawn(async move {
            let mut usadas = 0;
            while usadas < 2 {
                let aceptado = tokio::time::timeout(Duration::from_millis(600), oyente.accept());
                let Ok(Ok((mut zocalo, _))) = aceptado.await else {
                    break;
                };
                comer_peticion(&mut zocalo).await;
                zocalo
                    .write_all(
                        b"HTTP/1.1 400 X\r\nContent-Length: 3\r\nConnection: close\r\n\r\nmal",
                    )
                    .await
                    .unwrap();
                usadas += 1;
            }
            usadas
        });
        let r = super::completar_glory_con(&base, "k", "sys", "hola", &[]).await;
        assert!(r.is_err());
        assert_eq!(servo.await.unwrap(), 1, "un 400 no se repite");
    }
}
