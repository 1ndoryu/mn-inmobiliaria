//! Fontanería HTTP del cliente remoto: cliente, login perezoso, token en memoria
//! y un único reintento tras un 401. Vive aparte para que `remoto` se lea como lo
//! que es: las rutas del servidor y sus tipos.
//!
//! El token nunca se registra. Un redirect podría reenviar el token a otro host,
//! así que no se sigue (política `none`).

use std::time::Duration;

use axum::http::StatusCode;
use reqwest::header::CONTENT_TYPE;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use super::{Credenciales, ErrorRemoto, Sesion};

/// Tope de lectura: un servidor lento no bloquea la vigilancia.
pub(super) const TOPE_LECTURA: Duration = Duration::from_secs(10);

/// Tope de escritura y descarga de fotos (D15): una foto de 10 MiB puede tardar en red lenta.
pub(super) const TOPE_ESCRITURA: Duration = Duration::from_secs(60);

#[derive(Serialize)]
struct LoginEnvio<'a> {
    email: &'a str,
    password: &'a str,
}

#[derive(Deserialize)]
struct TokenRespuesta {
    token: String,
}

/// Cuerpo de error del servidor (`ErrorResponse`): el motivo que se muestra al usuario.
#[derive(Deserialize)]
struct CuerpoError {
    message: Option<String>,
    error: Option<String>,
}

pub(super) fn cliente(tope: Duration) -> Result<reqwest::Client, ErrorRemoto> {
    reqwest::Client::builder()
        .timeout(tope)
        // Un redirect podría reenviar el token a otro host.
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| ErrorRemoto::SinConexion(format!("cliente HTTP: {e}")))
}

pub(super) fn red(e: &reqwest::Error) -> ErrorRemoto {
    ErrorRemoto::SinConexion(if e.is_timeout() {
        "tiempo de espera agotado".into()
    } else {
        "sin respuesta".into()
    })
}

/// Estado HTTP que no es un resultado de negocio: credenciales o red/servidor.
pub(super) fn error_de_estado(estado: StatusCode) -> ErrorRemoto {
    match estado {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => ErrorRemoto::SinCredenciales,
        s => ErrorRemoto::SinConexion(format!("HTTP {}", s.as_u16())),
    }
}

async fn login(http: &reqwest::Client, cred: &Credenciales) -> Result<String, ErrorRemoto> {
    let envio = LoginEnvio {
        email: &cred.email,
        password: &cred.password,
    };
    let respuesta = http
        .post(format!("{}/api/auth/login", cred.base))
        .json(&envio)
        .send()
        .await
        .map_err(|e| red(&e))?;
    match respuesta.status() {
        s if s.is_success() => {
            let token: TokenRespuesta = respuesta
                .json()
                .await
                .map_err(|e| ErrorRemoto::SinConexion(format!("respuesta de login: {e}")))?;
            Ok(token.token)
        }
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Err(ErrorRemoto::SinCredenciales),
        s => Err(ErrorRemoto::SinConexion(format!(
            "login: HTTP {}",
            s.as_u16()
        ))),
    }
}

async fn renovar(
    http: &reqwest::Client,
    cred: &Credenciales,
    sesion: &Sesion,
) -> Result<String, ErrorRemoto> {
    let token = login(http, cred).await?;
    sesion.guardar(token.clone());
    Ok(token)
}

/// Petición ya armada. El cuerpo va en bytes para poder reenviarlo tras un 401.
pub(super) struct Envio {
    pub(super) metodo: reqwest::Method,
    pub(super) ruta: String,
    pub(super) consulta: Vec<(&'static str, String)>,
    /// (tipo de contenido, bytes).
    pub(super) cuerpo: Option<(&'static str, Vec<u8>)>,
    pub(super) tope: Duration,
}

impl Envio {
    pub(super) fn lectura(ruta: String) -> Self {
        Self {
            metodo: reqwest::Method::GET,
            ruta,
            consulta: Vec::new(),
            cuerpo: None,
            tope: TOPE_LECTURA,
        }
    }

    pub(super) fn json<B: Serialize + ?Sized>(
        metodo: reqwest::Method,
        ruta: String,
        cuerpo: &B,
    ) -> Result<Self, ErrorRemoto> {
        let bytes = serde_json::to_vec(cuerpo)
            .map_err(|e| ErrorRemoto::Rechazado(format!("cuerpo no serializable: {e}")))?;
        Ok(Self {
            metodo,
            ruta,
            consulta: Vec::new(),
            cuerpo: Some(("application/json", bytes)),
            tope: TOPE_ESCRITURA,
        })
    }

    /// Borrado: sin cuerpo en la petición y sin cuerpo en la respuesta (204).
    pub(super) fn borrado(ruta: String) -> Self {
        Self {
            metodo: reqwest::Method::DELETE,
            ruta,
            consulta: Vec::new(),
            cuerpo: None,
            tope: TOPE_ESCRITURA,
        }
    }
}

async fn intentar(
    http: &reqwest::Client,
    cred: &Credenciales,
    envio: &Envio,
    token: &str,
) -> Result<reqwest::Response, ErrorRemoto> {
    let mut peticion = http
        .request(envio.metodo.clone(), format!("{}{}", cred.base, envio.ruta))
        .bearer_auth(token);
    if !envio.consulta.is_empty() {
        peticion = peticion.query(&envio.consulta);
    }
    if let Some((tipo, bytes)) = &envio.cuerpo {
        peticion = peticion.header(CONTENT_TYPE, *tipo).body(bytes.clone());
    }
    peticion.send().await.map_err(|e| red(&e))
}

/// Envía con token: login perezoso y, ante un 401, una renovación y un único reintento.
pub(super) async fn enviar(
    cred: &Credenciales,
    sesion: &Sesion,
    envio: &Envio,
) -> Result<reqwest::Response, ErrorRemoto> {
    let http = cliente(envio.tope)?;
    let token = match sesion.actual() {
        Some(token) => token,
        None => renovar(&http, cred, sesion).await?,
    };
    let respuesta = intentar(&http, cred, envio, &token).await?;
    if respuesta.status() != StatusCode::UNAUTHORIZED {
        return Ok(respuesta);
    }
    // El token caducó o el servidor se reinició: una renovación y un reintento.
    let nuevo = renovar(&http, cred, sesion).await?;
    intentar(&http, cred, envio, &nuevo).await
}

/// Lectura. `Ok(None)` = 404: el servidor no tiene esa ruta o ese inmueble; el
/// llamador decide si eso es «sin soporte» o «no existe».
pub(super) async fn leer<T: DeserializeOwned>(respuesta: reqwest::Response) -> Result<Option<T>, ErrorRemoto> {
    match respuesta.status() {
        s if s.is_success() => respuesta
            .json::<T>()
            .await
            .map(Some)
            .map_err(|e| ErrorRemoto::SinSoporte(format!("respuesta no válida: {e}"))),
        StatusCode::NOT_FOUND => Ok(None),
        s => Err(error_de_estado(s)),
    }
}

/// Escritura: 2xx devuelve el cuerpo; otros 4xx (salvo credenciales) son `Rechazado`
/// con el motivo del servidor; 5xx y red son `SinConexion`.
pub(super) async fn escribir<T: DeserializeOwned>(respuesta: reqwest::Response) -> Result<T, ErrorRemoto> {
    let estado = respuesta.status();
    if estado.is_success() {
        return respuesta
            .json::<T>()
            .await
            .map_err(|e| ErrorRemoto::SinSoporte(format!("respuesta no válida: {e}")));
    }
    if estado.is_client_error() && !matches!(estado, StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN) {
        let motivo = respuesta
            .json::<CuerpoError>()
            .await
            .ok()
            .and_then(|c| c.message.or(c.error));
        return Err(ErrorRemoto::Rechazado(
            motivo.unwrap_or_else(|| format!("HTTP {}", estado.as_u16())),
        ));
    }
    Err(error_de_estado(estado))
}

/// Escritura que no devuelve cuerpo (204). Mismo criterio de error que `escribir`.
pub(super) async fn escribir_sin_cuerpo(respuesta: reqwest::Response) -> Result<(), ErrorRemoto> {
    let estado = respuesta.status();
    if estado.is_success() {
        return Ok(());
    }
    if estado.is_client_error() && !matches!(estado, StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN) {
        let motivo = respuesta
            .json::<CuerpoError>()
            .await
            .ok()
            .and_then(|c| c.message.or(c.error));
        return Err(ErrorRemoto::Rechazado(
            motivo.unwrap_or_else(|| format!("HTTP {}", estado.as_u16())),
        ));
    }
    Err(error_de_estado(estado))
}
