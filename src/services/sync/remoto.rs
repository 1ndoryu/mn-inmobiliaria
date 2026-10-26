//! Cliente del servidor de producción para la sincronización [13AA-12, 14AA-2].
//! Lectura (manifiesto, detalle, inmueble, ficha, fotos) para comparar y
//! escritura (alta, receta, ficha, fotos, publicación) para publicar o traer un
//! inmueble completo (F5).
//!
//! Las credenciales viven en `scripts/.env.prod.local` (o en el archivo que
//! apunta `SYNC_REMOTO_ENV`), gitignored, no en el `.env` del backend. Nunca se registran ni se
//! muestran. El token vive solo en memoria y se renueva ante un 401.

use std::path::PathBuf;
use std::sync::Mutex;

use axum::http::StatusCode;
use serde::de::IgnoredAny;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::detalle::DetalleSync;
use super::huella::{Manifiesto, VERSION};
use crate::models::{CopyInmueble, CreateInmuebleRequest, RecetaPublicidad, MAX_FOTO_BYTES};

mod http;

use http::{
    cliente, enviar, error_de_estado, escribir, escribir_sin_cuerpo, leer, red, Envio,
    TOPE_ESCRITURA,
};

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
    /// El servidor responde y rechaza la operación (400, 404, 409, 413...). El motivo es el suyo.
    Rechazado(String),
}

impl std::fmt::Display for ErrorRemoto {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Desactivado => write!(
                f,
                "sincronización desactivada (falta el archivo de credenciales)"
            ),
            Self::SinCredenciales => {
                write!(f, "credenciales del servidor rechazadas o incompletas")
            }
            Self::SinConexion(motivo) => write!(f, "sin conexión con el servidor: {motivo}"),
            Self::SinSoporte(motivo) => {
                write!(f, "el servidor no admite la sincronización: {motivo}")
            }
            Self::Rechazado(motivo) => write!(f, "el servidor rechazó la operación: {motivo}"),
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
        let pares =
            dotenvy::from_path_iter(ruta_credenciales()).map_err(|_| ErrorRemoto::Desactivado)?;
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

    /// Host del servidor, con puerto si lo lleva: el diálogo de publicar y traer lo nombra (D16).
    #[must_use]
    pub fn host(&self) -> String {
        match reqwest::Url::parse(&self.base) {
            Ok(url) => {
                let host = url.host_str().unwrap_or_default();
                match url.port() {
                    Some(puerto) => format!("{host}:{puerto}"),
                    None => host.to_string(),
                }
            }
            Err(_) => self.base.clone(),
        }
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

async fn manifiesto_con(cred: &Credenciales, sesion: &Sesion) -> Result<Manifiesto, ErrorRemoto> {
    let m: Manifiesto = leer(
        enviar(
            cred,
            sesion,
            &Envio::lectura("/api/admin/sync/manifiesto".into()),
        )
        .await?,
    )
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
    let ruta = format!("/api/admin/sync/inmuebles/{slug}");
    leer(enviar(cred, &SESION, &Envio::lectura(ruta)).await?).await
}

/// Foto del servidor. `url` es absoluta en `fotos()`: los bytes son públicos
/// (`/uploads`), así que se descargan sin token. `id` solo hace falta para
/// borrarla allí (F3).
#[derive(Debug, Clone, Deserialize)]
pub struct FotoRemota {
    pub id: Uuid,
    pub origen: String,
    pub orden: i32,
    pub url: String,
}

/// Inmueble del servidor tal como lo devuelve la API admin, con lo que hace falta
/// para traerlo. Los campos extra del JSON se ignoran.
#[derive(Debug, Clone, Deserialize)]
pub struct InmuebleRemoto {
    pub id: Uuid,
    pub slug: String,
    pub titulo: String,
    pub descripcion: String,
    pub ubicacion: String,
    pub puestos: i32,
    pub residencia: String,
    /* [18AA-6] Los servidores anteriores al campo no lo mandan: vacío, y quien
     * lo use cae a la sugerencia. */
    #[serde(default)]
    pub nombre_corto: String,
    pub precio: f64,
    pub tipo: String,
    pub operacion: String,
    pub habitaciones: i32,
    pub banos: i32,
    pub metros: f64,
    pub metros_terreno: f64,
    pub estado: String,
    pub publicado: bool,
    #[serde(default)]
    pub marketplace_id: Option<String>,
    #[serde(default)]
    pub alias_titulos: Vec<String>,
    #[serde(default)]
    pub copy: Option<CopyInmueble>,
    #[serde(default)]
    pub receta: Option<RecetaPublicidad>,
    #[serde(default)]
    pub fotos: Vec<FotoRemota>,
}

/// Ficha privada (`/ask`) de un inmueble del servidor.
#[derive(Debug, Clone, Deserialize)]
pub struct FichaRemota {
    pub extras: Value,
    pub precio_minimo: Option<f64>,
}

/// Las fotos llegan con ruta relativa (`/uploads/...`): se ancla al origen configurado.
fn absoluta(cred: &Credenciales, url: &str) -> String {
    if url.starts_with('/') {
        format!("{}{url}", cred.base)
    } else {
        url.to_string()
    }
}

/// Inmueble del servidor por su id. `Ok(None)` = no existe allí. Las URLs de
/// sus fotos salen ya absolutas.
pub async fn inmueble(
    cred: &Credenciales,
    id: Uuid,
) -> Result<Option<InmuebleRemoto>, ErrorRemoto> {
    let ruta = format!("/api/admin/inmuebles/{id}");
    let leido: Option<InmuebleRemoto> =
        leer(enviar(cred, &SESION, &Envio::lectura(ruta)).await?).await?;
    Ok(leido.map(|mut inmueble| {
        for foto in &mut inmueble.fotos {
            foto.url = absoluta(cred, &foto.url);
        }
        inmueble
    }))
}

/// Fotos de un inmueble del servidor, por su id remoto. El detalle de sync no
/// trae URLs; la vista admin sí, con token.
pub async fn fotos(cred: &Credenciales, remote_id: Uuid) -> Result<Vec<FotoRemota>, ErrorRemoto> {
    let inmueble = inmueble(cred, remote_id)
        .await?
        .ok_or_else(|| ErrorRemoto::SinSoporte(format!("el servidor no tiene {remote_id}")))?;
    Ok(inmueble.fotos)
}

/// Ficha privada de un inmueble del servidor. `Ok(None)` = no existe allí.
pub async fn ficha(cred: &Credenciales, id: Uuid) -> Result<Option<FichaRemota>, ErrorRemoto> {
    let ruta = format!("/api/admin/inmuebles/{id}/ficha");
    leer(enviar(cred, &SESION, &Envio::lectura(ruta)).await?).await
}

/// Alta del inmueble en el servidor. El servidor genera el slug a partir del
/// título (D11): el llamador lo comprueba antes de llegar aquí.
pub async fn crear_inmueble(
    cred: &Credenciales,
    alta: &CreateInmuebleRequest,
) -> Result<InmuebleRemoto, ErrorRemoto> {
    let envio = Envio::json(reqwest::Method::POST, "/api/admin/inmuebles".into(), alta)?;
    escribir(enviar(cred, &SESION, &envio).await?).await
}

/// Cambia campos del inmueble en el servidor. `cambios` es un objeto JSON con
/// solo los campos a tocar (el PUT del servidor es parcial: lo que no viaja se
/// queda como está). Nunca lleva `alias_titulos`: es dato propio de cada
/// entorno. [F2 de 14AA-2] El `marketplace_id` sí viaja desde el 2026-10-18
/// (18AA-3): el vínculo tiene que estar en los dos lados, y su PUT es
/// tri-estado, así que un `null` aquí desvincula allí.
pub async fn actualizar_inmueble(
    cred: &Credenciales,
    id: Uuid,
    cambios: &Value,
) -> Result<InmuebleRemoto, ErrorRemoto> {
    let ruta = format!("/api/admin/inmuebles/{id}");
    let envio = Envio::json(reqwest::Method::PUT, ruta, cambios)?;
    escribir(enviar(cred, &SESION, &envio).await?).await
}

#[derive(Serialize)]
struct EstadoEnvio<'a> {
    estado: &'a str,
}

/// Cambia el estado (`vendido`/`alquilado` despublican también, como en local).
pub async fn poner_estado(cred: &Credenciales, id: Uuid, estado: &str) -> Result<(), ErrorRemoto> {
    let ruta = format!("/api/admin/inmuebles/{id}/estado");
    let envio = Envio::json(reqwest::Method::PATCH, ruta, &EstadoEnvio { estado })?;
    escribir::<IgnoredAny>(enviar(cred, &SESION, &envio).await?).await?;
    Ok(())
}

#[derive(Serialize)]
struct RecetaEnvio<'a> {
    receta: &'a RecetaPublicidad,
}

/// Fija la receta publicitaria. Es un PUT parcial: el resto del inmueble no cambia.
pub async fn poner_receta(
    cred: &Credenciales,
    id: Uuid,
    receta: &RecetaPublicidad,
) -> Result<(), ErrorRemoto> {
    let ruta = format!("/api/admin/inmuebles/{id}");
    let envio = Envio::json(reqwest::Method::PUT, ruta, &RecetaEnvio { receta })?;
    escribir::<IgnoredAny>(enviar(cred, &SESION, &envio).await?).await?;
    Ok(())
}

#[derive(Serialize)]
struct FichaEnvio<'a> {
    extras: &'a Value,
    precio_minimo: Option<f64>,
}

/// Guarda la ficha privada (`/ask`): respuestas y precio mínimo.
pub async fn poner_ficha(
    cred: &Credenciales,
    id: Uuid,
    extras: &Value,
    precio_minimo: Option<f64>,
) -> Result<(), ErrorRemoto> {
    let ruta = format!("/api/admin/inmuebles/{id}/ficha");
    let envio = Envio::json(
        reqwest::Method::PUT,
        ruta,
        &FichaEnvio {
            extras,
            precio_minimo,
        },
    )?;
    escribir::<IgnoredAny>(enviar(cred, &SESION, &envio).await?).await?;
    Ok(())
}

/// Sube una foto al inmueble. `nombre` debe llevar extensión válida: el servidor
/// valida la extensión y los bytes (magia) y rechaza lo demás.
pub async fn subir_foto(
    cred: &Credenciales,
    inmueble_id: Uuid,
    nombre: &str,
    origen: &str,
    orden: i32,
    bytes: Vec<u8>,
) -> Result<FotoRemota, ErrorRemoto> {
    let envio = Envio {
        metodo: reqwest::Method::POST,
        ruta: "/api/admin/fotos/upload".into(),
        consulta: vec![
            ("inmueble_id", inmueble_id.to_string()),
            ("filename", nombre.to_string()),
            ("origen", origen.to_string()),
            ("orden", orden.to_string()),
        ],
        cuerpo: Some(("application/octet-stream", bytes)),
        tope: TOPE_ESCRITURA,
    };
    let foto: FotoRemota = escribir(enviar(cred, &SESION, &envio).await?).await?;
    Ok(FotoRemota {
        url: absoluta(cred, &foto.url),
        ..foto
    })
}

/// Borra una foto del servidor por su id [F3]. Es destructivo y no se deshace:
/// allí desaparecen la fila y el archivo, y borrar una `original` arrastra su
/// `mejorada` y renumera las siguientes (mismo código que aquí, [08AA-4]).
pub async fn borrar_foto(cred: &Credenciales, foto_id: Uuid) -> Result<(), ErrorRemoto> {
    let ruta = format!("/api/admin/fotos/{foto_id}");
    escribir_sin_cuerpo(enviar(cred, &SESION, &Envio::borrado(ruta)).await?).await
}

#[derive(Serialize)]
struct OrdenEnvio<'a> {
    ids: &'a [Uuid],
}

/// Reordena las fotos del servidor sin borrar ni subir nada [25AA-3]: `ids` son
/// las `original` en el orden nuevo (su `mejorada` viaja con ellas). El servidor
/// exige una permutación exacta de las que tiene y, si no lo es, no toca nada.
pub async fn reordenar_fotos(
    cred: &Credenciales,
    inmueble_id: Uuid,
    ids: &[Uuid],
) -> Result<(), ErrorRemoto> {
    let ruta = format!("/api/admin/inmuebles/{inmueble_id}/fotos/orden");
    let envio = Envio::json(reqwest::Method::PATCH, ruta, &OrdenEnvio { ids })?;
    escribir_sin_cuerpo(enviar(cred, &SESION, &envio).await?).await
}

#[derive(Serialize)]
struct PublicacionEnvio {
    publicado: bool,
}

/// Cambia la visibilidad pública. Es el último paso de una publicación (D13).
pub async fn poner_publicacion(
    cred: &Credenciales,
    id: Uuid,
    publicado: bool,
) -> Result<InmuebleRemoto, ErrorRemoto> {
    let ruta = format!("/api/admin/inmuebles/{id}/publicacion");
    let envio = Envio::json(
        reqwest::Method::PATCH,
        ruta,
        &PublicacionEnvio { publicado },
    )?;
    escribir(enviar(cred, &SESION, &envio).await?).await
}

/// Bytes de una foto del servidor. Solo desde el origen configurado (el token no
/// viaja aquí), con tope de tamaño: la foto del servidor no es de confianza.
pub async fn descargar_foto(cred: &Credenciales, url: &str) -> Result<Vec<u8>, ErrorRemoto> {
    let url = absoluta(cred, url);
    if !url.starts_with(&format!("{}/", cred.base)) {
        return Err(ErrorRemoto::Rechazado(
            "la foto no está en el servidor configurado".into(),
        ));
    }
    let http = cliente(TOPE_ESCRITURA)?;
    let mut respuesta = http.get(&url).send().await.map_err(|e| red(&e))?;
    match respuesta.status() {
        s if s.is_success() => {}
        StatusCode::NOT_FOUND => {
            return Err(ErrorRemoto::Rechazado(
                "la foto ya no está en el servidor".into(),
            ))
        }
        s => return Err(error_de_estado(s)),
    }
    let mut bytes = Vec::new();
    while let Some(trozo) = respuesta.chunk().await.map_err(|e| red(&e))? {
        if bytes.len() + trozo.len() > MAX_FOTO_BYTES {
            return Err(ErrorRemoto::Rechazado(format!(
                "la foto supera {} MiB",
                MAX_FOTO_BYTES / (1024 * 1024)
            )));
        }
        bytes.extend_from_slice(&trozo);
    }
    Ok(bytes)
}

#[cfg(test)]
mod pruebas {
    use std::sync::{Arc, Mutex as StdMutex};

    use axum::extract::State;
    use axum::http::{header::AUTHORIZATION, HeaderMap};
    use axum::routing::{get, post, put};
    use axum::{Json, Router};
    use serde_json::json;

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

    /// Eco: exige el token vigente y devuelve el cuerpo recibido.
    async fn eco_con_token(
        State(s): State<Simulado>,
        cabeceras: HeaderMap,
        Json(cuerpo): Json<Value>,
    ) -> (StatusCode, Json<Value>) {
        let esperado = format!("Bearer {}", s.vigente.lock().unwrap().clone());
        let recibido = cabeceras
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();
        if recibido != esperado {
            return (StatusCode::UNAUTHORIZED, Json(json!({})));
        }
        (StatusCode::OK, Json(cuerpo))
    }

    async fn conflicto() -> (StatusCode, Json<Value>) {
        (
            StatusCode::CONFLICT,
            Json(json!({ "error": "conflict", "message": "ya existe" })),
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

    #[tokio::test]
    async fn escritura_reenvia_el_cuerpo_tras_un_401() {
        let app = Router::new()
            .route("/api/auth/login", post(login_simulado))
            .route("/api/admin/inmuebles/x", put(eco_con_token))
            .with_state(Simulado {
                vigente: Arc::new(StdMutex::new("t1".into())),
            });
        let base = arrancar(app).await;
        let cred = credenciales(&base, "secreto");
        let sesion = Sesion::default();
        // El token guardado caducó: el primer intento da 401 y el reintento lleva el mismo cuerpo.
        sesion.guardar("caducado".into());
        let envio = Envio::json(
            reqwest::Method::PUT,
            "/api/admin/inmuebles/x".into(),
            &json!({ "receta": { "x": 1 } }),
        )
        .unwrap();
        let eco: Value = leer(enviar(&cred, &sesion, &envio).await.unwrap())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(eco, json!({ "receta": { "x": 1 } }));
        assert_eq!(sesion.actual().as_deref(), Some("t1"));
    }

    #[tokio::test]
    async fn rechazo_del_servidor_lleva_su_motivo() {
        let app = Router::new()
            .route("/api/auth/login", post(login_simulado))
            .route("/api/admin/inmuebles", post(conflicto))
            .with_state(Simulado {
                vigente: Arc::new(StdMutex::new("t1".into())),
            });
        let base = arrancar(app).await;
        let cred = credenciales(&base, "secreto");
        let envio = Envio::json(
            reqwest::Method::POST,
            "/api/admin/inmuebles".into(),
            &json!({}),
        )
        .unwrap();
        let res =
            escribir::<IgnoredAny>(enviar(&cred, &Sesion::default(), &envio).await.unwrap()).await;
        assert_eq!(res.err(), Some(ErrorRemoto::Rechazado("ya existe".into())));
    }

    #[tokio::test]
    async fn foto_fuera_del_servidor_configurado_se_rechaza() {
        // La comprobación de origen va antes de cualquier red: no hace falta servidor.
        let cred = credenciales("https://mn-inmobiliaria.com", "x");
        let res = descargar_foto(&cred, "https://otro.example/uploads/a.jpg").await;
        assert!(matches!(res, Err(ErrorRemoto::Rechazado(_))));
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
