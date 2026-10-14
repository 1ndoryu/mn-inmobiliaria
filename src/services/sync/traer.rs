//! Traer un inmueble del servidor al local [F5b]. Espejo de `publicar`: todas las
//! descargas (fotos incluidas) se hacen antes de la primera escritura local.
//! Orden de escritura: alta, receta, ficha, fotos y visibilidad. Sin deshacer.

use std::path::Path;

use sqlx::types::Json;
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

use crate::errors::AppError;
use crate::models::{validar_extras, CreateInmuebleRequest, UpdateInmuebleRequest, MAX_FOTO_BYTES};
use crate::repositories::sync::SyncRepository;
use crate::repositories::InmuebleRepository;
use crate::services::inmueble::InmuebleService;
use crate::services::sync::operacion::{conflicto, paso, InformeSync, Turno};
use crate::services::sync::remoto::{self, Credenciales, FichaRemota, InmuebleRemoto};

/// Tope de fotos y de bytes descargados antes de escribir nada en el local.
const MAX_FOTOS: usize = 40;
const MAX_BYTES_TOTALES: usize = 100 * 1024 * 1024;

/// Trae el inmueble `slug` del servidor al local. Un `Err` significa que no se
/// escribió nada; un fallo posterior al alta va en `InformeSync::fallo`.
pub async fn traer(
    pool: &PgPool,
    upload_dir: &Path,
    cred: &Credenciales,
    slug: &str,
) -> Result<InformeSync, AppError> {
    let _turno = Turno::tomar(slug)?;
    if SyncRepository::find_admin_by_slug(pool, slug)
        .await?
        .is_some()
    {
        return Err(AppError::Conflict(format!(
            "{slug} ya existe en este entorno: se resuelve con el panel de diferencias"
        )));
    }
    let en_servidor = remoto::detalle(cred, slug)
        .await
        .map_err(conflicto)?
        .ok_or_else(|| AppError::NotFound(format!("{slug} no existe en el servidor")))?;
    let servidor = remoto::inmueble(cred, en_servidor.huella.id)
        .await
        .map_err(conflicto)?
        .ok_or_else(|| AppError::NotFound(format!("{slug} no existe en el servidor")))?;
    let ficha = remoto::ficha(cred, servidor.id).await.map_err(conflicto)?;

    let alta = alta_desde(&servidor);
    alta.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    if let Some(ficha) = &ficha {
        validar_extras(&ficha.extras).map_err(AppError::Validation)?;
    }
    if servidor.fotos.len() > MAX_FOTOS {
        return Err(AppError::Validation(format!(
            "{slug} tiene {} fotos; el tope para traer es {MAX_FOTOS}",
            servidor.fotos.len()
        )));
    }
    let mut bytes_fotos = Vec::with_capacity(servidor.fotos.len());
    let mut total = 0usize;
    for foto in &servidor.fotos {
        let bytes = remoto::descargar_foto(cred, &foto.url)
            .await
            .map_err(conflicto)?;
        total += bytes.len();
        if bytes.len() > MAX_FOTO_BYTES || total > MAX_BYTES_TOTALES {
            return Err(AppError::Validation(format!(
                "la foto {} o el conjunto de fotos supera el tope",
                foto.orden
            )));
        }
        bytes_fotos.push(bytes);
    }

    // Desde aquí hay escrituras locales: cualquier fallo posterior va al informe.
    let local = InmuebleService::crear_con_slug(pool, alta, slug.to_string()).await?;
    let mut informe = InformeSync {
        slug: slug.to_string(),
        hecho: vec!["Alta local con campos y copy".into()],
        fallo: None,
    };
    if let Err(motivo) = resto(
        pool,
        upload_dir,
        &servidor,
        local.id,
        ficha.as_ref(),
        &bytes_fotos,
        &mut informe.hecho,
    )
    .await
    {
        informe.fallo = Some(motivo);
    }
    Ok(informe)
}

/// Receta, ficha, fotos y visibilidad sobre el inmueble ya creado en el local.
async fn resto(
    pool: &PgPool,
    upload_dir: &Path,
    servidor: &InmuebleRemoto,
    local_id: Uuid,
    ficha: Option<&FichaRemota>,
    fotos: &[Vec<u8>],
    hecho: &mut Vec<String>,
) -> Result<(), String> {
    if let Some(receta) = &servidor.receta {
        // Solo la receta: los demás campos ya llegaron con el alta.
        let cambios = UpdateInmuebleRequest {
            receta: Some(receta.clone()),
            ..Default::default()
        };
        InmuebleService::update(pool, local_id, cambios)
            .await
            .map_err(|e| paso("receta", e))?;
        hecho.push("Receta publicitaria".into());
    }
    if let Some(ficha) = ficha {
        InmuebleRepository::set_ficha(
            pool,
            local_id,
            Json(ficha.extras.clone()),
            ficha.precio_minimo,
        )
        .await
        .map_err(|e| paso("ficha", e))?;
        hecho.push("Ficha privada (/ask)".into());
    }
    for (foto, bytes) in servidor.fotos.iter().zip(fotos) {
        let nombre = foto.url.rsplit('/').next().unwrap_or_default();
        InmuebleService::subir_foto(
            pool,
            upload_dir,
            local_id,
            nombre,
            Some(&foto.origen),
            Some(foto.orden),
            bytes,
        )
        .await
        .map_err(|e| paso(&format!("foto {}", foto.orden), e))?;
    }
    if !fotos.is_empty() {
        hecho.push(format!("{} fotos", fotos.len()));
    }
    InmuebleService::set_publicado(pool, local_id, servidor.publicado)
        .await
        .map_err(|e| paso("visibilidad", e))?;
    hecho.push(if servidor.publicado {
        "Publicado en local".into()
    } else {
        "Sin publicar (visibilidad igual que el servidor)".into()
    });
    Ok(())
}

fn alta_desde(servidor: &InmuebleRemoto) -> CreateInmuebleRequest {
    CreateInmuebleRequest {
        titulo: servidor.titulo.clone(),
        descripcion: servidor.descripcion.clone(),
        ubicacion: servidor.ubicacion.clone(),
        puestos: servidor.puestos,
        residencia: servidor.residencia.clone(),
        precio: servidor.precio,
        tipo: servidor.tipo.clone(),
        operacion: servidor.operacion.clone(),
        habitaciones: servidor.habitaciones,
        banos: servidor.banos,
        metros: servidor.metros,
        metros_terreno: servidor.metros_terreno,
        estado: servidor.estado.clone(),
        marketplace_id: servidor.marketplace_id.clone(),
        alias_titulos: servidor.alias_titulos.clone(),
        copy: servidor.copy.clone(),
    }
}
