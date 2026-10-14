//! Cliente del servidor de producción para la comparación de sincronización
//! [13AA-12]. Solo lectura: login admin y `GET` de manifiesto y detalle.
//!
//! Las credenciales viven en `scripts/.env.prod.local` (o en el archivo que
//! apunta `SYNC_REMOTO_ENV`), gitignored, no en el `.env` del backend. Nunca se registran ni se
//! muestran. El token vive solo en memoria y se renueva ante un 401.

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use axum::http::StatusCode;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::detalle::DetalleSync;
use super::huella::{Manifiesto, VERSION};

/// Tope por petición: un servidor lento no bloquea la vigilancia.
const TIMEOUT: Duration = Duration::from_secs(10);

/// Por qué no se pudo leer al servidor. Se traduce a `EstadoRemoto` en la
/// vigilancia y a un mensaje en las respuestas HTTP.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorRemoto {
    /// Sin archivo de credenciales: la comparación está apagada.
    Desactivado,
    /// Archivo incompleto o login rechazado.
    SinCredenciales,
    /// Red, tiempo agotado o 5xx.
    SinConexion(String),
    /// El servidor responde, pero no tiene la ruta o usa otra versión de huella.
    SinSoporte(String),
}

impl std::fmt::Display for ErrorRemoto {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Desactivado => write!(f, "sincronización desactivada (falta el archivo de credenciales)"),
            Self::SinCredenciales => {
                write!(f, "credenciales del servidor rechazadas o incompletas")
            }
            Self::SinConexion(motivo) => write!(f, "sin conexión con el servidor: {motivo}"),
            Self::SinSoporte(motivo) => {
                write!(f, "el servidor no admite la sincronización: {motivo}")
            }
        }
    }
}

/// Credenciales de la cuenta admin del servidor. Sin `Debug` a propósito.
pub struct Credenciales {
    base: String,
    email: String,
    password: String,
}

impl Credenciales {
    /// Lee `PROD_BASE`, `PROD_EMAIL` y `PROD_PASSWORD` del archivo de
    /// `ruta_credenciales()`. Se relee en cada consulta: crear, editar o rotar
    /// el archivo no exige reiniciar el backend.
    pub fn cargar() -> Result<Self, ErrorRemoto> {
        // `from_path_iter` no toca el entorno del proceso: las claves no se filtran a otros hijos.
        let pares = dotenvy::from_path_iter(ruta_credenciales()).map_err(|_| ErrorRemoto::Desactivado)?;
        let (mut base, mut email, mut password) = (None, None, None);
        for par in pares {
            let (clave, valor) = par.map_err(|_| ErrorRemoto::SinCredenciales)?;
            match clave.as_str() {
                "PROD_BASE" => base = Some(valor),
                "PROD_EMAIL" => email = Some(valor),
                "PROD_PASSWORD" => password = Some(valor),
                _ => {}
            }
        }
        let (Some(base), Some(email), Some(password)) = (base, email, password) else {
            return Err(ErrorRemoto::SinCredenciales);
        };
        let base = base.trim_end_matches('/').to_string();
        validar_base(&base)?;
        Ok(Self {
            base,
            email,
            password,
        })
    }

    /// Origen del servidor, sin barra final. Solo para armar URL públicas (`/uploads/...`).
    #[must_use]
    pub fn base(&self) -> &str {
        &self.base
    }
}

/// Archivo de credenciales cuando no hay `SYNC_REMOTO_ENV`: el mismo que usan los
/// scripts de producción (gitignored). Así, configurar o cambiar la comparación
/// no exige variables de entorno ni reiniciar el backend.
const CREDENCIALES_POR_DEFECTO: &str = "scripts/.env.prod.local";

/// `SYNC_REMOTO_ENV` si está definida; si no, `CREDENCIALES_POR_DEFECTO` junto al
/// proyecto. Sin archivo en ninguno de los dos, la comparación queda desactivada.
fn ruta_credenciales() -> PathBuf {
    match std::env::var("SYNC_REMOTO_ENV") {
        Ok(ruta) if !ruta.trim().is_empty() => PathBuf::from(ruta),
        _ => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(CREDENCIALES_POR_DEFECTO),
    }
}

/// Las credenciales solo viajan por https. `http` se admite solo hacia
/// localhost, para la segunda instancia de las pruebas (F4).
fn validar_base(base: &str) -> Result<(), ErrorRemoto> {
    let es_local = ["localhost", "127.0.0.1", "[::1]"].iter().any(|host| {
        let resto = base
            .strip_prefix("http://")
            .and_then(|r| r.strip_prefix(host));
        matches!(resto, Some(r) if r.is_empty() || r.starts_with(':') || r.starts_with('/'))
    });
    if base.starts_with("https://") || es_local {
        Ok(())
    } else {
        Err(ErrorRemoto::SinConexion(
            "PROD_BASE debe ser https (http solo en localhost)".into(),
        ))
    }
}

/// Token de la sesión admin, solo en memoria.
#[derive(Default)]
pub struct Sesion(Mutex<Option<String>>);

impl Sesion {
    fn actual(&self) -> Option<String> {
        self.0.lock().ok().and_then(|s| s.clone())
    }

    fn guardar(&self, token: String) {
        if let Ok(mut s) = self.0.lock() {
            *s = Some(token);
        }
    }
}

static SESION: Sesion = Sesion(Mutex::new(None));

#[derive(Serialize)]
struct LoginEnvio<'a> {
    email: &'a str,
    password: &'a str,
}

#[derive(Deserialize)]
struct TokenRespuesta {
    token: String,
}

fn cliente() -> Result<reqwest::Client, ErrorRemoto> {
    reqwest::Client::builder()
        .timeout(TIMEOUT)
        // Un redirect podría reenviar el token a otro host.
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| ErrorRemoto::SinConexion(format!("cliente HTTP: {e}")))
}

fn red(e: &reqwest::Error) -> ErrorRemoto {
    ErrorRemoto::SinConexion(if e.is_timeout() {
        "tiempo de espera agotado".into()
    } else {
        "sin respuesta".into()
    })
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

async fn pedir(
    http: &reqwest::Client,
    cred: &Credenciales,
    ruta: &str,
    token: &str,
) -> Result<reqwest::Response, ErrorRemoto> {
    http.get(format!("{}{ruta}", cred.base))
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| red(&e))
}

/// `GET` autenticado. `Ok(None)` = 404: el servidor no tiene esa ruta o ese
/// inmueble; el llamador decide si eso es «sin soporte» o «no existe».
async fn get_con<T: DeserializeOwned>(
    cred: &Credenciales,
    sesion: &Sesion,
    ruta: &str,
) -> Result<Option<T>, ErrorRemoto> {
    let http = cliente()?;
    let token = match sesion.actual() {
        Some(token) => token,
        None => renovar(&http, cred, sesion).await?,
    };
    let mut respuesta = pedir(&http, cred, ruta, &token).await?;
    if respuesta.status() == StatusCode::UNAUTHORIZED {
        // El token caducó o el servidor se reinició: una renovación y un reintento.
        let nuevo = renovar(&http, cred, sesion).await?;
        respuesta = pedir(&http, cred, ruta, &nuevo).await?;
    }
    match respuesta.status() {
        s if s.is_success() => respuesta
            .json::<T>()
            .await
            .map(Some)
            .map_err(|e| ErrorRemoto::SinSoporte(format!("respuesta no válida: {e}"))),
        StatusCode::NOT_FOUND => Ok(None),
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Err(ErrorRemoto::SinCredenciales),
        s => Err(ErrorRemoto::SinConexion(format!("HTTP {}", s.as_u16()))),
    }
}

async fn manifiesto_con(cred: &Credenciales, sesion: &Sesion) -> Result<Manifiesto, ErrorRemoto> {
    let m: Manifiesto = get_con(cred, sesion, "/api/admin/sync/manifiesto")
        .await?
        .ok_or_else(|| {
            ErrorRemoto::SinSoporte("falta /api/admin/sync/manifiesto en el servidor".into())
        })?;
    if m.version != VERSION {
        return Err(ErrorRemoto::SinSoporte(format!(
            "versión de huella {} (local {VERSION})",
            m.version
        )));
    }
    Ok(m)
}

/// Manifiesto del servidor (huellas de todos sus inmuebles).
pub async fn manifiesto(cred: &Credenciales) -> Result<Manifiesto, ErrorRemoto> {
    manifiesto_con(cred, &SESION).await
}

/// Detalle de un inmueble en el servidor. `Ok(None)` = no existe allí.
pub async fn detalle(cred: &Credenciales, slug: &str) -> Result<Option<DetalleSync>, ErrorRemoto> {
    get_con(cred, &SESION, &format!("/api/admin/sync/inmuebles/{slug}")).await
}

/// Foto del servidor. `url` ya es absoluta: los bytes son públicos (`/uploads`),
/// así que el navegador las carga sin token.
#[derive(Debug, Clone)]
pub struct FotoRemota {
    pub origen: String,
    pub orden: i32,
    pub url: String,
}

#[derive(Deserialize)]
struct FotoRemotaRespuesta {
    origen: String,
    orden: i32,
    url: String,
}

#[derive(Deserialize)]
struct InmuebleRemotoRespuesta {
    #[serde(default)]
    fotos: Vec<FotoRemotaRespuesta>,
}

/// Fotos de un inmueble del servidor, por su id remoto. El detalle de sync no
/// trae URLs; la vista admin sí, con token.
pub async fn fotos(cred: &Credenciales, remote_id: Uuid) -> Result<Vec<FotoRemota>, ErrorRemoto> {
    let ruta = format!("/api/admin/inmuebles/{remote_id}");
    let inmueble: InmuebleRemotoRespuesta = get_con(cred, &SESION, &ruta)
        .await?
        .ok_or_else(|| ErrorRemoto::SinSoporte(format!("el servidor no tiene {remote_id}")))?;
    Ok(inmueble
        .fotos
        .into_iter()
        .map(|f| FotoRemota {
            url: if f.url.starts_with('/') {
                format!("{}{}", cred.base(), f.url)
            } else {
                f.url
            },
            origen: f.origen,
            orden: f.orden,
        })
        .collect())
}

#[cfg(test)]
mod pruebas {
    use std::sync::{Arc, Mutex as StdMutex};

    use axum::extract::State;
    use axum::http::{header::AUTHORIZATION, HeaderMap};
    use axum::routing::{get, post};
    use axum::{Json, Router};
    use serde_json::{json, Value};

    use super::*;

    /// Servidor simulado: el token válido se puede rotar para forzar un 401.
    #[derive(Clone)]
    struct Simulado {
        vigente: Arc<StdMutex<String>>,
    }

    async fn login_simulado(
        State(s): State<Simulado>,
        Json(cuerpo): Json<Value>,
    ) -> (StatusCode, Json<Value>) {
        if cuerpo["email"] != "admin@x" || cuerpo["password"] != "secreto" {
            return (StatusCode::UNAUTHORIZED, Json(json!({})));
        }
        let token = s.vigente.lock().unwrap().clone();
        (StatusCode::OK, Json(json!({ "token": token })))
    }

    async fn manifiesto_simulado(
        State(s): State<Simulado>,
        cabeceras: HeaderMap,
    ) -> (StatusCode, Json<Value>) {
        let esperado = format!("Bearer {}", s.vigente.lock().unwrap().clone());
        let recibido = cabeceras
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();
        if recibido != esperado {
            return (StatusCode::UNAUTHORIZED, Json(json!({})));
        }
        (
            StatusCode::OK,
            Json(json!({
                "version": VERSION,
                "generado_en": "2026-10-13T00:00:00Z",
                "truncado": false,
                "fotos_pendientes": 0,
                "inmuebles": [],
            })),
        )
    }

    async fn arrancar(app: Router) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        format!("http://{addr}")
    }

    fn credenciales(base: &str, password: &str) -> Credenciales {
        Credenciales {
            base: base.to_string(),
            email: "admin@x".into(),
            password: password.into(),
        }
    }

    #[tokio::test]
    async fn renueva_el_token_cuando_el_servidor_lo_rota() {
        let vigente = Arc::new(StdMutex::new("t1".to_string()));
        let app = Router::new()
            .route("/api/auth/login", post(login_simulado))
            .route("/api/admin/sync/manifiesto", get(manifiesto_simulado))
            .with_state(Simulado {
                vigente: vigente.clone(),
            });
        let base = arrancar(app).await;
        let cred = credenciales(&base, "secreto");
        let sesion = Sesion::default();

        assert!(manifiesto_con(&cred, &sesion).await.is_ok());
        // El servidor reinicia y cambia el token: la sesión guardada ya no vale.
        *vigente.lock().unwrap() = "t2".into();
        assert!(manifiesto_con(&cred, &sesion).await.is_ok());
        assert_eq!(sesion.actual().as_deref(), Some("t2"));
    }

    #[tokio::test]
    async fn contrasena_rechazada_es_sin_credenciales() {
        let app = Router::new()
            .route("/api/auth/login", post(login_simulado))
            .with_state(Simulado {
                vigente: Arc::new(StdMutex::new("t1".into())),
            });
        let base = arrancar(app).await;
        let cred = credenciales(&base, "otra");
        let res = manifiesto_con(&cred, &Sesion::default()).await;
        assert_eq!(res.err(), Some(ErrorRemoto::SinCredenciales));
    }

    #[tokio::test]
    async fn servidor_sin_la_ruta_es_sin_soporte() {
        // Un servidor anterior a F2: login correcto, sin manifiesto.
        let app = Router::new()
            .route("/api/auth/login", post(login_simulado))
            .with_state(Simulado {
                vigente: Arc::new(StdMutex::new("t1".into())),
            });
        let base = arrancar(app).await;
        let cred = credenciales(&base, "secreto");
        let res = manifiesto_con(&cred, &Sesion::default()).await;
        assert!(matches!(res, Err(ErrorRemoto::SinSoporte(_))));
    }

    #[test]
    fn solo_https_fuera_de_localhost() {
        assert!(validar_base("https://mn-inmobiliaria.com").is_ok());
        assert!(validar_base("http://localhost:8787").is_ok());
        assert!(validar_base("http://127.0.0.1:9000").is_ok());
        assert!(validar_base("http://localhost.evil.com").is_err());
        assert!(validar_base("http://mn-inmobiliaria.com").is_err());
    }
}
