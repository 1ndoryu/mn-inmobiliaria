#![cfg(test)]
//! Formato de párrafos, configuración de tokens y binding CLI.

use super::*;

#[test]
fn formatear_parrafos_une_saltos_sueltos_y_separa_bloques() {
    let entrado = "Hola, Andreina, buenas noches.\nTe escribo por la casa.\nSí, sigue disponible.\nCuéntame qué estás buscando y con gusto te ayudo, cualquier cosa escríbeme al 04249208855 https://wa.me/584249208855";
    let salido = formatear_parrafos(entrado);
    assert_eq!(
        salido,
        "Hola, Andreina, buenas noches. Te escribo por la casa. Sí, sigue disponible. Cuéntame qué estás buscando y con gusto te ayudo, cualquier cosa escríbeme al 04249208855\n\nhttps://wa.me/584249208855"
    );
}

#[test]
fn formatear_parrafos_respeta_lista_y_no_duplica() {
    let entrado = "Tiene:\n1. Piscina\n2. Planta eléctrica\n\nhttps://wa.me/584249208855";
    let salido = formatear_parrafos(entrado);
    assert!(salido.contains("Tiene:\n\n1. Piscina\n\n2. Planta eléctrica"));
    assert_eq!(salido.matches(CONTACTO_WA).count(), 1);
}

#[test]
fn sub_exento_lee_env() {
    std::env::set_var("MP_SIN_LIMITE_SUB", "ella,otro");
    assert!(sub_exento("ella"));
    assert!(!sub_exento("plugin"));
    std::env::remove_var("MP_SIN_LIMITE_SUB");
    assert!(!sub_exento("ella"));
}

/* [08AA-20] Por defecto CLI 8h/panel 15min; `MP_CLI_MINUTOS`
 * manda cuando es entero positivo; lo inválido cae al default. */
#[test]
fn cli_vive_8h_y_panel_15min() {
    std::env::remove_var("MP_CLI_MINUTOS");
    assert_eq!(minutos_para_cli(true), 480);
    assert_eq!(minutos_para_cli(false), 15);
    std::env::set_var("MP_CLI_MINUTOS", "43200");
    assert_eq!(minutos_para_cli(true), 43200);
    assert_eq!(minutos_para_cli(false), 15);
    std::env::set_var("MP_CLI_MINUTOS", "basura");
    assert_eq!(minutos_para_cli(true), 480);
    std::env::remove_var("MP_CLI_MINUTOS");
}

#[test]
fn maquina_solo_hex64() {
    assert!(maquina_valida(&"a".repeat(64)));
    assert!(maquina_valida(&"A1".repeat(32)));
    assert!(!maquina_valida("corto"));
    assert!(!maquina_valida(&"z".repeat(64)));
    assert!(!maquina_valida(""));
}

#[test]
fn binding_solo_cuando_hay_mid() {
    assert!(maquina_autorizada(None, None));
    assert!(maquina_autorizada(None, Some("x")));
    assert!(!maquina_autorizada(Some("a"), None));
    assert!(!maquina_autorizada(Some("a"), Some("b")));
    assert!(maquina_autorizada(Some("a"), Some("a")));
}

/* Tests M4: hashes estables, invalidación honesta y ciclo de la caché.
 * Los vivos usan `pool_si_hay` (sin `DATABASE_URL` se omiten). */

/* Expiración (DoD E3): un token con `exp` pasado no decodifica — la misma
 * `decode`+`Validation` que usa `MpAuth`, así que el rechazo queda
 * probado a nivel JWT (el chequeo DB `expira_en > now()` es redundante). */
#[test]
fn decode_rechaza_expirado() {
    use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
    let pasado = usize::try_from(chrono::Utc::now().timestamp() - 60).unwrap_or(0);
    let claims = MpClaims {
        iss: "mn-backend".to_string(),
        sub: "s".to_string(),
        aud: "mp".to_string(),
        scope: "mp:borrador".to_string(),
        exp: pasado,
        jti: "j".to_string(),
        mid: Some("a".repeat(64)),
    };
    let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(b"x")).unwrap();
    let r = decode::<MpClaims>(
        &token,
        &DecodingKey::from_secret(b"x"),
        &Validation::new(jsonwebtoken::Algorithm::HS256),
    );
    assert!(r.is_err());
}
