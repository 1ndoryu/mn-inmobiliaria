use std::io::Cursor;

use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use chrono::{NaiveDate, Utc};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

use crate::errors::AppError;
use crate::middleware::AuthUser;
use crate::models::{
    formato_de_proporcion, normalizar_tags, ActualizarPublicacionRequest, CrearPublicacionRequest,
    Destinos, Pieza, PiezaRow, Propuesta, ProponerRequest, Publicacion, PublicacionRow,
    SaltarPropuestaRequest,
};
use crate::repositories::{NuevaPieza, PublicacionRepository};
use crate::services::InmuebleService;
use crate::AppState;

/* [28AA-3] Calendario de publicaciones en redes (admin). La cola elige el
 * siguiente inmueble; «saltar» lo deja registrado sin contarlo como publicado;
 * el checklist de destinos [D1] decide cuándo una publicación está publicada.
 * Requiere JWT en cada ruta. */

/// Máximo de días por consulta del calendario: un mes y medio
const MAX_DIAS_RANGO: i64 = 62;

#[derive(Debug, Deserialize)]
pub struct RangoParams {
    pub desde: NaiveDate,
    pub hasta: NaiveDate,
}

/// Publicaciones del calendario entre dos días (incluidos), con títulos y piezas
#[utoipa::path(
    get,
    path = "/api/admin/publicaciones",
    params(
        ("desde" = String, Query, description = "Primer día (YYYY-MM-DD)"),
        ("hasta" = String, Query, description = "Último día (YYYY-MM-DD)")
    ),
    responses(
        (status = 200, description = "Publicaciones del rango", body = [Publicacion]),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse),
        (status = 422, description = "Rango no válido", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_publicaciones(
    State(state): State<AppState>,
    _auth: AuthUser,
    Query(rango): Query<RangoParams>,
) -> Result<Json<Vec<Publicacion>>, AppError> {
    if rango.hasta < rango.desde || (rango.hasta - rango.desde).num_days() > MAX_DIAS_RANGO {
        return Err(AppError::Validation(
            "Rango no válido: hasta debe ser posterior a desde y no pasar de 62 días".into(),
        ));
    }
    let filas =
        PublicacionRepository::listar_entre(&state.pool, rango.desde, rango.hasta).await?;
    Ok(Json(ensamblar(&state.pool, filas).await?))
}

/// Siguiente inmueble a publicar: el que más tiempo lleva sin salir
#[utoipa::path(
    post,
    path = "/api/admin/publicaciones/siguiente",
    request_body = ProponerRequest,
    responses(
        (status = 200, description = "Inmueble propuesto", body = Propuesta),
        (status = 404, description = "No quedan inmuebles publicables", body = crate::errors::ErrorResponse),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn proponer_siguiente(
    State(state): State<AppState>,
    _auth: AuthUser,
    Json(req): Json<ProponerRequest>,
) -> Result<Json<Propuesta>, AppError> {
    let inmueble_id = PublicacionRepository::siguiente_inmueble(&state.pool, req.excluir)
        .await?
        .ok_or_else(|| AppError::NotFound("No quedan inmuebles publicables".into()))?;
    let titulo = titulo_de(&state.pool, inmueble_id).await?;
    Ok(Json(Propuesta { inmueble_id, titulo }))
}

/// Crear el borrador de un inmueble propuesto (requiere JWT)
#[utoipa::path(
    post,
    path = "/api/admin/publicaciones",
    request_body = CrearPublicacionRequest,
    responses(
        (status = 201, description = "Borrador creado", body = Publicacion),
        (status = 404, description = "Inmueble no encontrado", body = crate::errors::ErrorResponse),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse),
        (status = 422, description = "Error de validación", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn crear_publicacion(
    State(state): State<AppState>,
    _auth: AuthUser,
    Json(req): Json<CrearPublicacionRequest>,
) -> Result<(StatusCode, Json<Publicacion>), AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    // Comprueba que el inmueble existe antes de crear nada
    titulo_de(&state.pool, req.inmueble_id).await?;
    let fila =
        PublicacionRepository::crear_borrador(&state.pool, req.inmueble_id, req.programada_para)
            .await?;
    Ok((StatusCode::CREATED, Json(ensamblar_uno(&state.pool, fila).await?)))
}

/// Saltar la propuesta: queda registrada, pero no cuenta como publicada (requiere JWT)
#[utoipa::path(
    post,
    path = "/api/admin/publicaciones/saltar",
    request_body = SaltarPropuestaRequest,
    responses(
        (status = 204, description = "Propuesta saltada"),
        (status = 404, description = "Inmueble no encontrado", body = crate::errors::ErrorResponse),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn saltar_propuesta(
    State(state): State<AppState>,
    _auth: AuthUser,
    Json(req): Json<SaltarPropuestaRequest>,
) -> Result<StatusCode, AppError> {
    titulo_de(&state.pool, req.inmueble_id).await?;
    PublicacionRepository::saltar(&state.pool, req.inmueble_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Editar texto, tags, día y checklist de destinos; solo cambia lo que llega (requiere JWT)
#[utoipa::path(
    put,
    path = "/api/admin/publicaciones/{id}",
    params(("id" = Uuid, Path, description = "ID de la publicación")),
    request_body = ActualizarPublicacionRequest,
    responses(
        (status = 200, description = "Publicación actualizada", body = Publicacion),
        (status = 404, description = "No encontrada", body = crate::errors::ErrorResponse),
        (status = 409, description = "Propuesta saltada, no editable", body = crate::errors::ErrorResponse),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse),
        (status = 422, description = "Error de validación", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn actualizar_publicacion(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<ActualizarPublicacionRequest>,
) -> Result<Json<Publicacion>, AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let mut fila = buscar(&state.pool, id).await?;
    aplicar(&mut fila, &req)?;
    let guardada = PublicacionRepository::guardar(&state.pool, fila)
        .await?
        .ok_or_else(|| AppError::NotFound("Publicación no encontrada".into()))?;
    Ok(Json(ensamblar_uno(&state.pool, guardada).await?))
}

/// Borrar un borrador o una lista (requiere JWT). Lo ya publicado queda en el historial.
#[utoipa::path(
    delete,
    path = "/api/admin/publicaciones/{id}",
    params(("id" = Uuid, Path, description = "ID de la publicación")),
    responses(
        (status = 204, description = "Borrada"),
        (status = 404, description = "No encontrada", body = crate::errors::ErrorResponse),
        (status = 409, description = "Ya publicada, no se borra", body = crate::errors::ErrorResponse),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn borrar_publicacion(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let fila = buscar(&state.pool, id).await?;
    if fila.estado == "publicada" {
        return Err(AppError::Conflict(
            "Una publicación ya salida no se borra: queda en el historial".into(),
        ));
    }
    if !PublicacionRepository::borrar(&state.pool, id).await? {
        return Err(AppError::NotFound("Publicación no encontrada".into()));
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Historial de un inmueble: cada vez que salió publicado (requiere JWT)
#[utoipa::path(
    get,
    path = "/api/admin/publicaciones/historial/{inmueble_id}",
    params(("inmueble_id" = Uuid, Path, description = "ID del inmueble")),
    responses(
        (status = 200, description = "Publicaciones ya salidas", body = [Publicacion]),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn historial(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(inmueble_id): Path<Uuid>,
) -> Result<Json<Vec<Publicacion>>, AppError> {
    let filas = PublicacionRepository::historial_de(&state.pool, inmueble_id).await?;
    Ok(Json(ensamblar(&state.pool, filas).await?))
}

/* [28AA-3] Piezas de una publicación: imágenes en formato de red (1:1, 3:4, 4:5,
 * 9:16). El formato sale de la proporción real, no de lo que diga el nombre. */

/// Subir una pieza (requiere JWT). Cuerpo: bytes crudos de la imagen
#[utoipa::path(
    post,
    path = "/api/admin/publicaciones/{id}/piezas",
    params(
        ("id" = Uuid, Path, description = "ID de la publicación"),
        ("filename" = String, Query, description = "Nombre original (solo se usa su extensión)"),
    ),
    request_body(content = Vec<u8>, description = "Bytes de la imagen (jpg/png/webp, máx 10 MiB)", content_type = "application/octet-stream"),
    responses(
        (status = 201, description = "Pieza subida", body = Pieza),
        (status = 400, description = "Archivo inválido", body = crate::errors::ErrorResponse),
        (status = 404, description = "Publicación no encontrada", body = crate::errors::ErrorResponse),
        (status = 409, description = "Propuesta saltada o ya salida, no admite piezas", body = crate::errors::ErrorResponse),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse),
        (status = 422, description = "Proporción sin formato de red", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn subir_pieza(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
    Query(params): Query<SubirPiezaParams>,
    bytes: Bytes,
) -> Result<(StatusCode, Json<Pieza>), AppError> {
    let publicacion = buscar(&state.pool, id).await?;
    // Una publicación ya salida es historial: no se le añaden piezas después
    if publicacion.estado == "saltada" {
        return Err(AppError::Conflict(
            "La propuesta saltada no admite piezas".into(),
        ));
    }
    if publicacion.estado == "publicada" {
        return Err(AppError::Conflict(
            "Una publicación ya salida no recibe piezas: queda en el historial".into(),
        ));
    }
    let fila = guardar_pieza(&state, publicacion.id, &params.filename, bytes.as_ref()).await?;
    Ok((StatusCode::CREATED, Json(Pieza::from_row(fila))))
}

/// Borrar una pieza (requiere JWT). Una publicación ya salida no pierde piezas
#[utoipa::path(
    delete,
    path = "/api/admin/publicaciones/piezas/{pieza_id}",
    params(("pieza_id" = Uuid, Path, description = "ID de la pieza")),
    responses(
        (status = 204, description = "Pieza borrada"),
        (status = 404, description = "No encontrada", body = crate::errors::ErrorResponse),
        (status = 409, description = "Publicación ya salida, no se borra", body = crate::errors::ErrorResponse),
        (status = 401, description = "No autorizado", body = crate::errors::ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn borrar_pieza(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(pieza_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let pieza = PublicacionRepository::find_pieza(&state.pool, pieza_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Pieza no encontrada".into()))?;
    if buscar(&state.pool, pieza.publicacion_id).await?.estado == "publicada" {
        return Err(AppError::Conflict(
            "Una publicación ya salida no pierde piezas: queda en el historial".into(),
        ));
    }
    PublicacionRepository::borrar_pieza(&state.pool, pieza_id).await?;
    borrar_archivo_pieza(&state, &pieza.storage_key).await;
    Ok(StatusCode::NO_CONTENT)
}

/// Valida la imagen, la guarda bajo `publicaciones/<id>/` y registra la pieza.
/// Si el registro falla, el archivo se borra para no dejar huérfanos.
async fn guardar_pieza(
    state: &AppState,
    publicacion_id: Uuid,
    filename: &str,
    bytes: &[u8],
) -> Result<PiezaRow, AppError> {
    let (ancho, alto) = dimensiones(bytes)?;
    let formato = formato_de_proporcion(ancho, alto).ok_or_else(|| {
        AppError::Validation(format!(
            "Proporción {ancho}x{alto} sin formato de red: usa 1:1, 3:4, 4:5 o 9:16"
        ))
    })?;
    let carpeta = format!("publicaciones/{publicacion_id}");
    let storage_key =
        InmuebleService::guardar_archivo(&state.upload_dir, &carpeta, filename, bytes).await?;
    let nueva = NuevaPieza {
        publicacion_id,
        formato: formato.into(),
        storage_key: storage_key.clone(),
        ancho: entero(ancho)?,
        alto: entero(alto)?,
        bytes: entero(bytes.len() as u32)?,
    };
    match PublicacionRepository::anadir_pieza(&state.pool, nueva).await {
        Ok(fila) => Ok(fila),
        Err(e) => {
            borrar_archivo_pieza(state, &storage_key).await;
            Err(e.into())
        }
    }
}

/// Ancho y alto leídos de la cabecera: no decodifica, así que es barato
fn dimensiones(bytes: &[u8]) -> Result<(u32, u32), AppError> {
    image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .and_then(|lector| lector.into_dimensions().map_err(std::io::Error::other))
        .map_err(|_| AppError::BadRequest("No se pudo leer la imagen".into()))
}

/// La BD guarda enteros de 32 bits; una imagen que no cabe se rechaza, no se trunca
fn entero(valor: u32) -> Result<i32, AppError> {
    i32::try_from(valor).map_err(|_| AppError::BadRequest("Imagen demasiado grande".into()))
}

/// Quita el archivo de una pieza. Si ya no está, no es un error: el registro manda
async fn borrar_archivo_pieza(state: &AppState, storage_key: &str) {
    let ruta = InmuebleService::ruta_archivo(&state.upload_dir, storage_key);
    if let Err(e) = tokio::fs::remove_file(&ruta).await {
        tracing::warn!("No se pudo borrar la pieza {ruta:?}: {e}");
    }
}

#[derive(Debug, Deserialize)]
pub struct SubirPiezaParams {
    pub filename: String,
}

pub fn admin_routes() -> Router<AppState> {
    Router::new()
        .route("/publicaciones", get(list_publicaciones).post(crear_publicacion))
        .route("/publicaciones/siguiente", post(proponer_siguiente))
        .route("/publicaciones/saltar", post(saltar_propuesta))
        .route("/publicaciones/historial/:inmueble_id", get(historial))
        .route(
            "/publicaciones/:id",
            put(actualizar_publicacion).delete(borrar_publicacion),
        )
        .route(
            "/publicaciones/:id/piezas",
            post(subir_pieza).layer(DefaultBodyLimit::disable()),
        )
        .route("/publicaciones/piezas/:pieza_id", delete(borrar_pieza))
}

/// Aplica solo lo que llega y deja el estado coherente con el checklist [D1]
fn aplicar(fila: &mut PublicacionRow, req: &ActualizarPublicacionRequest) -> Result<(), AppError> {
    if fila.estado == "saltada" {
        return Err(AppError::Conflict(
            "La propuesta saltada no se edita".into(),
        ));
    }
    if let Some(texto) = &req.descripcion {
        fila.descripcion = texto.trim().to_string();
    }
    if let Some(tags) = &req.tags {
        fila.tags = normalizar_tags(tags);
    }
    if let Some(prompt) = &req.prompt_usado {
        fila.prompt_usado = prompt.clone();
    }
    if let Some(modelo) = &req.modelo {
        fila.modelo = modelo.trim().to_string();
    }
    if let Some(dia) = req.programada_para {
        fila.programada_para = Some(dia);
    }
    if let Some(d) = req.destinos {
        fila.ig_post = d.ig_post;
        fila.fanpage = d.fanpage;
        fila.ig_historia = d.ig_historia;
        fila.fb_perfil = d.fb_perfil;
    }
    estado_tras(fila);
    Ok(())
}

/// Con los cuatro destinos marcados sale `publicada` (con su fecha real); si se
/// quita alguno, vuelve a `lista`. Un borrador con texto pasa a `lista`.
fn estado_tras(fila: &mut PublicacionRow) {
    let completos = Destinos::from(&*fila).completos();
    if completos && fila.estado != "publicada" {
        fila.estado = "publicada".into();
        fila.publicada_en = Some(Utc::now());
    } else if !completos && fila.estado == "publicada" {
        fila.estado = "lista".into();
        fila.publicada_en = None;
    } else if fila.estado == "borrador" && !fila.descripcion.is_empty() {
        fila.estado = "lista".into();
    }
}

/// Título del inmueble, o 404 si no existe
async fn titulo_de(pool: &PgPool, inmueble_id: Uuid) -> Result<String, AppError> {
    PublicacionRepository::titulos(pool, vec![inmueble_id])
        .await?
        .into_iter()
        .next()
        .map(|(_, titulo)| titulo)
        .ok_or_else(|| AppError::NotFound("Inmueble no encontrado".into()))
}

async fn buscar(pool: &PgPool, id: Uuid) -> Result<PublicacionRow, AppError> {
    PublicacionRepository::find(pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound("Publicación no encontrada".into()))
}

/// Vista de una sola publicación (título y piezas en dos consultas)
async fn ensamblar_uno(pool: &PgPool, fila: PublicacionRow) -> Result<Publicacion, AppError> {
    let titulo = titulo_de(pool, fila.inmueble_id).await?;
    let piezas = PublicacionRepository::piezas_de(pool, vec![fila.id]).await?;
    Ok(Publicacion::from_row(fila, titulo, piezas))
}

/// Vistas de una lista con tres consultas fijas, sin N+1
async fn ensamblar(pool: &PgPool, filas: Vec<PublicacionRow>) -> Result<Vec<Publicacion>, AppError> {
    let mut inmuebles: Vec<Uuid> = filas.iter().map(|f| f.inmueble_id).collect();
    inmuebles.sort();
    inmuebles.dedup();
    let titulos = PublicacionRepository::titulos(pool, inmuebles).await?;
    let ids: Vec<Uuid> = filas.iter().map(|f| f.id).collect();
    let piezas = PublicacionRepository::piezas_de(pool, ids).await?;
    Ok(filas
        .into_iter()
        .map(|f| {
            let titulo = titulos
                .iter()
                .find(|(id, _)| *id == f.inmueble_id)
                .map(|(_, t)| t.clone())
                .unwrap_or_default();
            let mis_piezas = piezas
                .iter()
                .filter(|p| p.publicacion_id == f.id)
                .cloned()
                .collect();
            Publicacion::from_row(f, titulo, mis_piezas)
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::{dimensiones, formato_de_proporcion};

    /// PNG real de `ancho`x`alto` generado en memoria: prueba la lectura de cabecera de verdad
    fn png(ancho: u32, alto: u32) -> Vec<u8> {
        let imagen = image::RgbImage::new(ancho, alto);
        let mut salida = Vec::new();
        image::DynamicImage::ImageRgb8(imagen)
            .write_to(&mut Cursor::new(&mut salida), image::ImageFormat::Png)
            .expect("PNG de prueba");
        salida
    }

    #[test]
    fn lee_dimensiones_y_asigna_formato() {
        let bytes = png(400, 500);
        let (ancho, alto) = dimensiones(&bytes).expect("dimensiones");
        assert_eq!((ancho, alto), (400, 500));
        assert_eq!(formato_de_proporcion(ancho, alto), Some("4:5"));
    }

    /* [28AA-3] Una imagen de cada formato sale clasificada como tal; una
     * proporción fuera de todos (16:9) no entra en ninguno. */
    #[test]
    fn clasifica_una_imagen_de_cada_formato() {
        for (ancho, alto, esperado) in [
            (1080, 1080, "1:1"),
            (1080, 1440, "3:4"),
            (1080, 1350, "4:5"),
            (1080, 1920, "9:16"),
        ] {
            let (a, b) = dimensiones(&png(ancho, alto)).expect("dimensiones");
            assert_eq!(formato_de_proporcion(a, b), Some(esperado), "{ancho}x{alto}");
        }
        let (a, b) = dimensiones(&png(1920, 1080)).expect("dimensiones");
        assert_eq!(formato_de_proporcion(a, b), None, "16:9 no es un formato de pieza");
    }

    #[test]
    fn rechaza_lo_que_no_es_imagen() {
        assert!(dimensiones(b"no es una imagen").is_err());
    }

    /* [28AA-4] La procedencia de la IA se guarda tal cual llega y el modelo sin
     * espacios sobrantes; un campo ausente no pisa lo que ya había. */
    #[test]
    fn aplicar_guarda_prompt_y_modelo_sin_pisar_lo_ausente() {
        use chrono::Utc;
        use uuid::Uuid;

        use crate::models::{ActualizarPublicacionRequest, PublicacionRow};

        let mut fila = PublicacionRow {
            id: Uuid::new_v4(),
            inmueble_id: Uuid::new_v4(),
            estado: "borrador".into(),
            descripcion: "previo".into(),
            tags: vec!["casa".into()],
            prompt_usado: "prompt viejo".into(),
            modelo: "modelo-viejo".into(),
            ig_post: false,
            fanpage: false,
            ig_historia: false,
            fb_perfil: false,
            programada_para: None,
            publicada_en: None,
            creada_en: Utc::now(),
        };
        let req = ActualizarPublicacionRequest {
            descripcion: None,
            tags: None,
            programada_para: None,
            destinos: None,
            prompt_usado: Some("SISTEMA + ficha".into()),
            modelo: Some("  gemini-2.5 ".into()),
        };
        super::aplicar(&mut fila, &req).expect("aplicar");
        assert_eq!(fila.prompt_usado, "SISTEMA + ficha");
        assert_eq!(fila.modelo, "gemini-2.5");
        assert_eq!(fila.descripcion, "previo", "sin descripción en la petición, no se toca");
        assert_eq!(fila.tags, vec!["casa"], "sin tags en la petición, no se tocan");
    }
}
