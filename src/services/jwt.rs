//! JWT HS256 mínimo para `auth` y el token `mp`.
//!
//! Sustituye a `jsonwebtoken` 10 con `rust_crypto`, que arrastra el crate `rsa`
//! (RUSTSEC-2023-0071, sin parche). El backend `aws_lc_rs` exige cmake y NASM
//! en Windows. Solo se acepta HS256: el `alg` de la cabecera se compara con una
//! constante y nunca elige cómo verificar, así que no hay `alg: none` ni
//! confusión de tipos de clave.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use hmac::{Hmac, Mac};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Margen de `exp` en segundos; mismo valor por defecto que `jsonwebtoken`.
const MARGEN_EXP_SEG: i64 = 60;

/// Motivo de rechazo. Los llamadores lo traducen a `AppError::Unauthorized`
/// sin exponerlo al cliente; sirve para tests y logs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorJwt {
    Formato,
    Algoritmo,
    Firma,
    Payload,
    Expirado,
    Emisor,
    Audiencia,
}

#[derive(Deserialize)]
struct Cabecera {
    alg: String,
}

/// Firma `claims` con HS256 y devuelve el token compacto (`cabecera.payload.firma`).
/// Los claims deben llevar `exp`, igual que los que se verifican con `verificar`.
pub fn firmar<T: Serialize>(claims: &T, secreto: &[u8]) -> Result<String, ErrorJwt> {
    let cabecera = URL_SAFE_NO_PAD.encode(br#"{"typ":"JWT","alg":"HS256"}"#);
    let payload = serde_json::to_vec(claims).map_err(|_| ErrorJwt::Payload)?;
    let entrada = format!("{cabecera}.{}", URL_SAFE_NO_PAD.encode(payload));
    let firma = firma_hs256(entrada.as_bytes(), secreto)?;
    Ok(format!("{entrada}.{}", URL_SAFE_NO_PAD.encode(firma)))
}

/// Verifica firma, `alg` (solo HS256), `exp` con margen, `iss` y `aud`, y
/// devuelve los claims. `emisor: None` no comprueba `iss`. `audiencia: None`
/// rechaza cualquier token que traiga `aud`, como `Validation::default()`.
pub fn verificar<T: DeserializeOwned>(
    token: &str,
    secreto: &[u8],
    emisor: Option<&str>,
    audiencia: Option<&str>,
) -> Result<T, ErrorJwt> {
    let partes: Vec<&str> = token.split('.').collect();
    let &[cabecera, payload, firma] = partes.as_slice() else {
        return Err(ErrorJwt::Formato);
    };
    let cab_bytes = URL_SAFE_NO_PAD
        .decode(cabecera)
        .map_err(|_| ErrorJwt::Formato)?;
    let cab: Cabecera = serde_json::from_slice(&cab_bytes).map_err(|_| ErrorJwt::Formato)?;
    if cab.alg != "HS256" {
        return Err(ErrorJwt::Algoritmo);
    }

    // La firma se comprueba antes de leer ningún claim: nada no firmado influye.
    let entrada = &token[..cabecera.len() + 1 + payload.len()];
    let recibida = URL_SAFE_NO_PAD.decode(firma).map_err(|_| ErrorJwt::Formato)?;
    let mut mac = mac_con_clave(secreto)?;
    mac.update(entrada.as_bytes());
    // `verify_slice` compara en tiempo constante.
    mac.verify_slice(&recibida).map_err(|_| ErrorJwt::Firma)?;

    let bytes = URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|_| ErrorJwt::Formato)?;
    let valor: Value = serde_json::from_slice(&bytes).map_err(|_| ErrorJwt::Payload)?;
    let exp = valor
        .get("exp")
        .and_then(Value::as_i64)
        .ok_or(ErrorJwt::Payload)?;
    if exp.saturating_add(MARGEN_EXP_SEG) < chrono::Utc::now().timestamp() {
        return Err(ErrorJwt::Expirado);
    }
    if let Some(esperado) = emisor {
        if valor.get("iss").and_then(Value::as_str) != Some(esperado) {
            return Err(ErrorJwt::Emisor);
        }
    }
    if let Some(esperada) = audiencia {
        let coincide = match valor.get("aud") {
            Some(Value::String(a)) => a == esperada,
            Some(Value::Array(lista)) => lista.iter().any(|a| a.as_str() == Some(esperada)),
            _ => false,
        };
        if !coincide {
            return Err(ErrorJwt::Audiencia);
        }
    } else if valor.get("aud").is_some() {
        return Err(ErrorJwt::Audiencia);
    }
    serde_json::from_value(valor).map_err(|_| ErrorJwt::Payload)
}

fn mac_con_clave(secreto: &[u8]) -> Result<HmacSha256, ErrorJwt> {
    HmacSha256::new_from_slice(secreto).map_err(|_| ErrorJwt::Firma)
}

fn firma_hs256(entrada: &[u8], secreto: &[u8]) -> Result<Vec<u8>, ErrorJwt> {
    let mut mac = mac_con_clave(secreto)?;
    mac.update(entrada);
    Ok(mac.finalize().into_bytes().to_vec())
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use serde_json::json;

    const SECRETO: &[u8] = b"secreto-de-prueba";

    fn ahora() -> i64 {
        chrono::Utc::now().timestamp()
    }

    fn token_con(cabecera: &str, payload: &Value, firma: &str) -> String {
        format!(
            "{}.{}.{firma}",
            URL_SAFE_NO_PAD.encode(cabecera),
            URL_SAFE_NO_PAD.encode(payload.to_string())
        )
    }

    /// RFC 7515 apéndice A.1: entrada y firma de referencia para HS256.
    #[test]
    fn firma_coincide_con_vector_rfc7515() {
        let clave = URL_SAFE_NO_PAD
            .decode("AyM1SysPpbyDfgZld3umj1qzKObwVMkoqQ-EstJQLr_T-1qS0gZH75aKtMN3Yj0iPS4hcgUuTwjAzZr1Z9CAow")
            .unwrap();
        // Entrada literal del RFC (JSON con ",\r\n" dentro): la forma compacta da otra firma.
        let entrada = "eyJ0eXAiOiJKV1QiLA0KICJhbGciOiJIUzI1NiJ9.eyJpc3MiOiJqb2UiLA0KICJleHAiOjEzMDA4MTkzODAsDQogImh0dHA6Ly9leGFtcGxlLmNvbS9pc19yb290Ijp0cnVlfQ";
        let esperada = URL_SAFE_NO_PAD.decode("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk").unwrap();
        assert_eq!(firma_hs256(entrada.as_bytes(), &clave).unwrap(), esperada);
    }

    #[test]
    fn ida_y_vuelta_con_iss_y_aud() {
        let claims = json!({"sub": "u1", "exp": ahora() + 600, "iss": "mn-backend", "aud": "mp"});
        let token = firmar(&claims, SECRETO).unwrap();
        let r: Value = verificar(&token, SECRETO, Some("mn-backend"), Some("mp")).unwrap();
        assert_eq!(r["sub"], "u1");
    }

    #[test]
    fn expiracion_con_margen_de_60_segundos() {
        let dentro = json!({"exp": ahora() - 30});
        let fuera = json!({"exp": ahora() - 120});
        let ok: Result<Value, _> = verificar(&firmar(&dentro, SECRETO).unwrap(), SECRETO, None, None);
        assert!(ok.is_ok());
        let r: Result<Value, _> = verificar(&firmar(&fuera, SECRETO).unwrap(), SECRETO, None, None);
        assert_eq!(r.unwrap_err(), ErrorJwt::Expirado);
    }

    #[test]
    fn rechaza_alg_none_y_otros_algoritmos() {
        let payload = json!({"exp": ahora() + 600});
        let none = token_con(r#"{"alg":"none"}"#, &payload, "");
        let hs512 = token_con(r#"{"alg":"HS512"}"#, &payload, "AAAA");
        let r1: Result<Value, _> = verificar(&none, SECRETO, None, None);
        let r2: Result<Value, _> = verificar(&hs512, SECRETO, None, None);
        assert_eq!(r1.unwrap_err(), ErrorJwt::Algoritmo);
        assert_eq!(r2.unwrap_err(), ErrorJwt::Algoritmo);
    }

    #[test]
    fn rechaza_payload_alterado_y_secreto_distinto() {
        let token = firmar(&json!({"sub": "a", "exp": ahora() + 600}), SECRETO).unwrap();
        let mut partes: Vec<&str> = token.split('.').collect();
        let falso = URL_SAFE_NO_PAD.encode(json!({"sub": "b", "exp": ahora() + 600}).to_string());
        partes[1] = falso.as_str();
        let alterado = partes.join(".");
        let r1: Result<Value, _> = verificar(&alterado, SECRETO, None, None);
        let r2: Result<Value, _> = verificar(&token, b"otro", None, None);
        assert_eq!(r1.unwrap_err(), ErrorJwt::Firma);
        assert_eq!(r2.unwrap_err(), ErrorJwt::Firma);
    }

    #[test]
    fn rechaza_iss_aud_y_formato_invalidos() {
        let exp = ahora() + 600;
        let otro_iss = firmar(&json!({"exp": exp, "iss": "otro", "aud": "mp"}), SECRETO).unwrap();
        let sin_aud = firmar(&json!({"exp": exp, "iss": "mn-backend"}), SECRETO).unwrap();
        let con_aud = firmar(&json!({"exp": exp, "aud": "mp"}), SECRETO).unwrap();
        let r1: Result<Value, _> = verificar(&otro_iss, SECRETO, Some("mn-backend"), Some("mp"));
        let r2: Result<Value, _> = verificar(&sin_aud, SECRETO, Some("mn-backend"), Some("mp"));
        let r3: Result<Value, _> = verificar(&con_aud, SECRETO, None, None);
        let r4: Result<Value, _> = verificar("a.b", SECRETO, None, None);
        assert_eq!(r1.unwrap_err(), ErrorJwt::Emisor);
        assert_eq!(r2.unwrap_err(), ErrorJwt::Audiencia);
        assert_eq!(r3.unwrap_err(), ErrorJwt::Audiencia);
        assert_eq!(r4.unwrap_err(), ErrorJwt::Formato);
    }
}
