//! [13AA-4] Avisos del perfil de Facebook Marketplace. La lab envía la lectura
//! (`registrar`, token mp) y el modal de Inmuebles la lee (`listar`, staff).
//! Solo ordena candidatos: nunca vincula nada por su cuenta.
//! [16AA-12] La lectura ya no es automática: la pide la usuaria (`pedir`) y la
//! lab la ve en su latido. Aquí solo se deja la marca.

use std::collections::HashSet;

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::MpAuth;
use crate::errors::AppError;
use crate::middleware::AuthStaff;
use crate::repositories::marketplace::{
    avisos_perfil_sin_vincular, inmuebles_sin_vincular, reemplazar_avisos_perfil,
    ultima_lectura_perfil, AvisoPerfilNuevo, InmuebleSinVincular,
};
use crate::services::marketplace::{
    lectura_pedida, marcar_atendida, mejor_puntaje_con_alias, pedir_lectura,
};
use crate::services::InmuebleService;
use crate::AppState;

/// Tope de avisos por lectura: el perfil real tiene decenas; el tope evita un
/// cuerpo sin límite.
const MAX_AVISOS: usize = 500;
/// Inmuebles que el modal ofrece por aviso, los más parecidos primero.
const MAX_CANDIDATOS: usize = 10;
const MAX_TITULO: usize = 300;
const MAX_PRECIO: usize = 40;
/// Las URLs firmadas de fbcdn son largas; más allá de esto no es una foto normal.
const MAX_IMAGEN: usize = 1000;

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AvisoLeido {
    /// ID del aviso (`/marketplace/item/<id>` o dígitos 5–32).
    pub aviso_id: String,
    pub titulo: String,
    /// Precio tal como lo muestra Facebook.
    pub precio: Option<String>,
    /// Foto principal del aviso (https de Facebook o de su CDN). Si no es una URL
    /// válida se descarta: la foto es solo ayuda visual, no invalida la lectura.
    #[serde(default)]
    pub imagen_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct LecturaPerfil {
    pub avisos: Vec<AvisoLeido>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CandidatoInmueble {
    pub inmueble_id: Uuid,
    pub titulo: String,
    pub precio: f64,
    /// Un título contiene al otro (coincidencia fuerte).
    pub directo: bool,
    /// Palabras que comparten el título del aviso y el del inmueble.
    pub palabras_comunes: usize,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AvisoPendiente {
    pub aviso_id: String,
    pub titulo: String,
    pub precio: Option<String>,
    pub url: String,
    /// Foto principal del aviso en Facebook (`null` si la lab no la leyó).
    pub imagen_url: Option<String>,
    pub candidatos: Vec<CandidatoInmueble>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct InmuebleSinAviso {
    pub inmueble_id: Uuid,
    pub titulo: String,
    pub precio: f64,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PerfilAvisos {
    /// Hora de la última lectura de la lab (`null` si nunca se ha leído).
    pub ultima_lectura: Option<DateTime<Utc>>,
    /// [16AA-12] Lectura pedida desde el modal y todavía sin atender por la lab
    /// (`null` si no hay ninguna pendiente). El modal espera mientras la haya.
    pub solicitada_en: Option<DateTime<Utc>>,
    /// Avisos presentes en el perfil que ningún inmueble tiene vinculado.
    pub avisos: Vec<AvisoPendiente>,
    /// Inmuebles sin aviso en el perfil (informativo).
    pub inmuebles_sin_aviso: Vec<InmuebleSinAviso>,
}

/// Registra una lectura del perfil. Reemplaza el estado: lo que no viene pasa a
/// `presente = false`. Una lectura vacía o con un dato malo se rechaza entera,
/// para no dejar el perfil a medias.
#[utoipa::path(
    post,
    path = "/api/admin/marketplace/perfil/avisos",
    request_body = LecturaPerfil,
    responses(
        (status = 200, description = "Lectura registrada: recibidos y dejados de aparecer"),
        (status = 422, description = "Lectura vacía o con un aviso inválido", body = crate::errors::ErrorResponse)
    )
)]
pub async fn registrar(
    State(state): State<AppState>,
    _auth: MpAuth,
    r: Result<Json<LecturaPerfil>, axum::extract::rejection::JsonRejection>,
) -> Result<Response, AppError> {
    let r = r.map_err(|e| AppError::Validation(format!("JSON inválido: {e}")))?;
    let avisos = validar_lectura(r.0)?;
    let ausentes = reemplazar_avisos_perfil(&state.pool, &avisos).await?;
    // [16AA-12] La lectura llegó: la petición del modal queda cerrada. Si algo
    // falló antes, sigue pendiente y la lab no la reintenta sola (la marca no
    // cambia); la usuaria vuelve a pulsar y esa pulsación trae marca nueva.
    marcar_atendida();
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "recibidos": avisos.len(),
            "ausentes": ausentes,
        })),
    )
        .into_response())
}

/// Estado del perfil para el modal: avisos sin vincular con sus candidatos y
/// los inmuebles sin aviso. Lo ya vinculado no aparece en ninguna lista.
#[utoipa::path(
    get,
    path = "/api/admin/marketplace/perfil/avisos",
    responses(
        (status = 200, description = "Avisos del perfil y candidatos", body = PerfilAvisos)
    )
)]
pub async fn listar(State(state): State<AppState>, _auth: AuthStaff) -> Result<Response, AppError> {
    let pool = &state.pool;
    let ultima_lectura = ultima_lectura_perfil(pool).await?;
    let inmuebles = inmuebles_sin_vincular(pool).await?;
    let mut avisos = Vec::new();
    for fila in avisos_perfil_sin_vincular(pool).await? {
        avisos.push(AvisoPendiente {
            candidatos: ordenar_candidatos(&fila.titulo, &inmuebles),
            aviso_id: fila.aviso_id,
            titulo: fila.titulo,
            precio: fila.precio,
            url: fila.url,
            imagen_url: fila.imagen_url,
        });
    }
    let inmuebles_sin_aviso = inmuebles
        .iter()
        .map(|i| InmuebleSinAviso {
            inmueble_id: i.id,
            titulo: i.titulo.clone(),
            precio: i.precio,
        })
        .collect();
    Ok((
        StatusCode::OK,
        Json(PerfilAvisos {
            ultima_lectura,
            solicitada_en: lectura_pedida(),
            avisos,
            inmuebles_sin_aviso,
        }),
    )
        .into_response())
}

/// [16AA-12] La usuaria pide la lectura desde el modal. Aquí no se lee nada:
/// la lab es la única que tiene la sesión de Facebook. Se deja la marca, la lab
/// la ve en su latido de 5 s y manda los avisos, que cierran la petición.
#[utoipa::path(
    post,
    path = "/api/admin/marketplace/perfil/leer",
    responses(
        (status = 200, description = "Lectura pedida: la marca que verá la lab"),
        (status = 401, description = "Sin sesión de staff", body = crate::errors::ErrorResponse)
    )
)]
pub async fn pedir(_auth: AuthStaff) -> Result<Response, AppError> {
    let solicitada_en = pedir_lectura();
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "solicitada_en": solicitada_en })),
    )
        .into_response())
}

/// Valida la lectura entera antes de tocar la BD. Los IDs repetidos cuentan
/// una vez (se queda la primera aparición).
fn validar_lectura(lectura: LecturaPerfil) -> Result<Vec<AvisoPerfilNuevo>, AppError> {
    if lectura.avisos.is_empty() {
        return Err(AppError::Validation(
            "lectura vacía: no se cambia el perfil".into(),
        ));
    }
    if lectura.avisos.len() > MAX_AVISOS {
        return Err(AppError::Validation(format!(
            "demasiados avisos en una lectura (máximo {MAX_AVISOS})"
        )));
    }
    let mut vistos = HashSet::new();
    let mut avisos = Vec::with_capacity(lectura.avisos.len());
    for leido in lectura.avisos {
        let Some(aviso_id) = InmuebleService::normalizar_marketplace_id(Some(&leido.aviso_id))?
        else {
            return Err(AppError::Validation("aviso_id vacío".into()));
        };
        let titulo = leido.titulo.trim();
        if titulo.is_empty() {
            return Err(AppError::Validation(format!("aviso {aviso_id} sin título")));
        }
        if !vistos.insert(aviso_id.clone()) {
            continue;
        }
        let precio = leido
            .precio
            .as_deref()
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .map(|p| p.chars().take(MAX_PRECIO).collect());
        avisos.push(AvisoPerfilNuevo {
            url: format!("https://www.facebook.com/marketplace/item/{aviso_id}"),
            titulo: titulo.chars().take(MAX_TITULO).collect(),
            precio,
            imagen_url: leido.imagen_url.as_deref().and_then(imagen_valida),
            aviso_id,
        });
    }
    Ok(avisos)
}

/// La foto se pinta en una `<img>` del panel: solo se guarda una URL https que
/// apunte a Facebook o a su CDN (`fbcdn.net`), sin credenciales ni puerto. Una
/// URL ajena o torcida no rechaza la lectura: el aviso queda sin foto.
fn imagen_valida(url: &str) -> Option<String> {
    let url = url.trim();
    if url.is_empty() || url.len() > MAX_IMAGEN || url.chars().any(|c| c.is_whitespace()) {
        return None;
    }
    let resto = url.strip_prefix("https://")?;
    let host = resto.split(['/', '?', '#']).next()?.to_ascii_lowercase();
    if host.contains(['@', ':']) {
        return None;
    }
    let es_facebook = |dominio: &str| host == dominio || host.ends_with(&format!(".{dominio}"));
    (es_facebook("fbcdn.net") || es_facebook("facebook.com")).then(|| url.to_string())
}

/// Inmuebles más parecidos al título del aviso, primero. Usa el mismo criterio
/// que el emparejamiento de borradores (`mejor_puntaje_con_alias`), así el
/// modal y el borrador no discrepan sobre qué es «parecido».
fn ordenar_candidatos(
    aviso_titulo: &str,
    inmuebles: &[InmuebleSinVincular],
) -> Vec<CandidatoInmueble> {
    let mut puntuados: Vec<((bool, usize, usize), &InmuebleSinVincular)> = inmuebles
        .iter()
        .map(|i| {
            (
                mejor_puntaje_con_alias(aviso_titulo, &i.titulo, &i.alias),
                i,
            )
        })
        .collect();
    puntuados.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.titulo.cmp(&b.1.titulo)));
    puntuados
        .into_iter()
        .take(MAX_CANDIDATOS)
        .map(|((directo, palabras_comunes, _), i)| CandidatoInmueble {
            inmueble_id: i.id,
            titulo: i.titulo.clone(),
            precio: i.precio,
            directo,
            palabras_comunes,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leido(id: &str, titulo: &str) -> AvisoLeido {
        AvisoLeido {
            aviso_id: id.to_string(),
            titulo: titulo.to_string(),
            precio: None,
            imagen_url: None,
        }
    }

    fn inmueble(titulo: &str) -> InmuebleSinVincular {
        InmuebleSinVincular {
            id: Uuid::new_v4(),
            titulo: titulo.to_string(),
            alias: Vec::new(),
            precio: 0.0,
        }
    }

    #[test]
    fn lectura_vacia_es_422() {
        let vacia = LecturaPerfil { avisos: Vec::new() };
        assert!(matches!(
            validar_lectura(vacia),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn id_invalido_es_422() {
        let lectura = LecturaPerfil {
            avisos: vec![leido("abc", "Casa en Los Samanes")],
        };
        assert!(matches!(
            validar_lectura(lectura),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn acepta_url_de_item_y_construye_la_url_canonica() {
        let lectura = LecturaPerfil {
            avisos: vec![leido(
                "https://www.facebook.com/marketplace/item/123456789012/",
                "Casa en Los Samanes",
            )],
        };
        let avisos = validar_lectura(lectura).unwrap();
        assert_eq!(avisos[0].aviso_id, "123456789012");
        assert_eq!(
            avisos[0].url,
            "https://www.facebook.com/marketplace/item/123456789012"
        );
    }

    #[test]
    fn imagen_solo_de_facebook_o_su_cdn() {
        let foto = "https://scontent-lga3-1.xx.fbcdn.net/v/t45.5328-4/1_n.jpg?stp=c0.43.261.261";
        assert_eq!(imagen_valida(foto).as_deref(), Some(foto));
        assert!(imagen_valida("https://www.facebook.com/a.jpg").is_some());
        for mala in [
            "",
            "http://scontent.xx.fbcdn.net/a.jpg",
            "https://fbcdn.net.evil.test/a.jpg",
            "https://evilfbcdn.net/a.jpg",
            "https://usuario@scontent.xx.fbcdn.net/a.jpg",
            "https://scontent.xx.fbcdn.net:8443/a.jpg",
            "https://scontent.xx.fbcdn.net/a b.jpg",
            "data:image/png;base64,AAAA",
        ] {
            assert!(imagen_valida(mala).is_none(), "{mala}");
        }
        assert!(imagen_valida(&format!(
            "https://scontent.xx.fbcdn.net/{}",
            "a".repeat(MAX_IMAGEN)
        ))
        .is_none());
    }

    #[test]
    fn foto_invalida_no_rechaza_la_lectura() {
        let mut con_foto = leido("123456789012", "Casa en Los Samanes");
        con_foto.imagen_url = Some("https://scontent.xx.fbcdn.net/a.jpg".into());
        let mut ajena = leido("123456789013", "Casa en Naguanagua");
        ajena.imagen_url = Some("https://evil.test/a.jpg".into());
        let avisos = validar_lectura(LecturaPerfil {
            avisos: vec![con_foto, ajena],
        })
        .unwrap();
        assert_eq!(
            avisos[0].imagen_url.as_deref(),
            Some("https://scontent.xx.fbcdn.net/a.jpg")
        );
        assert_eq!(avisos[1].imagen_url, None);
    }

    #[test]
    fn ids_repetidos_cuentan_una_vez() {
        let lectura = LecturaPerfil {
            avisos: vec![
                leido("123456789012", "Casa en Los Samanes"),
                leido("123456789012", "Casa en Los Samanes (copia)"),
            ],
        };
        let avisos = validar_lectura(lectura).unwrap();
        assert_eq!(avisos.len(), 1);
        assert_eq!(avisos[0].titulo, "Casa en Los Samanes");
    }

    #[test]
    fn candidato_que_comparte_mas_palabras_va_primero() {
        let mas_parecido = inmueble("Casa Los Samanes con piscina y jardín");
        let lejano = inmueble("Apartamento Centro");
        let orden = ordenar_candidatos(
            "Casa Los Samanes con piscina",
            &[lejano, mas_parecido.clone()],
        );
        assert_eq!(orden[0].inmueble_id, mas_parecido.id);
        assert!(orden[0].palabras_comunes > orden[1].palabras_comunes);
    }

    fn pool_si_hay() -> Option<sqlx::PgPool> {
        let url = crate::config::url_bd_pruebas()?;
        sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy(&url)
            .ok()
    }

    fn aviso_nuevo(id: &str) -> AvisoPerfilNuevo {
        AvisoPerfilNuevo {
            aviso_id: id.to_string(),
            titulo: format!("Aviso {id}"),
            precio: None,
            url: format!("https://www.facebook.com/marketplace/item/{id}"),
            imagen_url: Some(format!("https://scontent.xx.fbcdn.net/{id}.jpg")),
        }
    }

    /// Contra la BD de rama (si la hay): una lectura reemplaza el estado. Lo que
    /// deja de venir pasa a `presente = false` sin borrarse y no vuelve a listarse.
    #[tokio::test]
    async fn lectura_reemplaza_y_marca_ausentes() {
        let Some(pool) = pool_si_hay() else { return };
        let (a, b) = ("913013000000001", "913013000000002");
        let ids = vec![a.to_string(), b.to_string()];
        let limpiar = "DELETE FROM marketplace_avisos_perfil WHERE aviso_id = ANY($1)";
        sqlx::query(limpiar)
            .bind(&ids)
            .execute(&pool)
            .await
            .unwrap();

        reemplazar_avisos_perfil(&pool, &[aviso_nuevo(a), aviso_nuevo(b)])
            .await
            .unwrap();
        let antes: Vec<String> = avisos_perfil_sin_vincular(&pool)
            .await
            .unwrap()
            .into_iter()
            .map(|f| f.aviso_id)
            .collect();
        assert!(antes.contains(&a.to_string()) && antes.contains(&b.to_string()));

        let ausentes = reemplazar_avisos_perfil(&pool, &[aviso_nuevo(a)])
            .await
            .unwrap();
        assert!(
            ausentes >= 1,
            "b dejó de venir y debe contarse como ausente"
        );
        let filas = avisos_perfil_sin_vincular(&pool).await.unwrap();
        let despues: Vec<&str> = filas.iter().map(|f| f.aviso_id.as_str()).collect();
        assert!(despues.contains(&a));
        assert!(!despues.contains(&b));
        let fila_a = filas.iter().find(|f| f.aviso_id == a).unwrap();
        assert_eq!(
            fila_a.imagen_url.as_deref(),
            Some(format!("https://scontent.xx.fbcdn.net/{a}.jpg").as_str()),
            "la foto del aviso se guarda y se devuelve"
        );

        let presente_b: bool = sqlx::query_scalar(
            "SELECT presente FROM marketplace_avisos_perfil WHERE aviso_id = $1",
        )
        .bind(b)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(
            !presente_b,
            "el aviso ausente se guarda con presente = false"
        );

        sqlx::query(limpiar)
            .bind(&ids)
            .execute(&pool)
            .await
            .unwrap();
    }
}
