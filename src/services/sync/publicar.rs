//! Publicar un inmueble local en el servidor [F5b].
//!
//! Antes de escribir nada se comprueba lo que se puede sin el servidor: slug,
//! validación, ficha y archivos de foto en disco. Después, en orden: alta,
//! receta, ficha, fotos y visibilidad (la última, para que nada quede público a medias).
//! Sin deshacer: si un paso falla, el informe dice qué quedó escrito.

use std::path::Path;

use serde_json::Value;
use sqlx::types::Json;
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

use crate::errors::AppError;
use crate::models::{validar_extras, CreateInmuebleRequest, Inmueble, MAX_FOTO_BYTES};
use crate::repositories::sync::{FotoClave, SyncRepository};
use crate::repositories::InmuebleRepository;
use crate::services::inmueble::slug::slugify;
use crate::services::inmueble::InmuebleService;
use crate::services::sync::operacion::{conflicto, paso, InformeSync, Turno};
use crate::services::sync::remoto::{self, Credenciales};

/// Publica el inmueble `slug` del local en el servidor. Un `Err` significa que no se
/// escribió nada; un fallo posterior al alta va en `InformeSync::fallo`.
pub async fn publicar(
    pool: &PgPool,
    upload_dir: &Path,
    cred: &Credenciales,
    slug: &str,
) -> Result<InformeSync, AppError> {
    let _turno = Turno::tomar(slug)?;
    let fila = SyncRepository::find_admin_by_slug(pool, slug)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("inmueble {slug}")))?;
    let local = InmuebleService::get_admin(pool, fila.id).await?;

    // El servidor asigna el slug desde el título: si no coincide, el alta crearía otro slug.
    let generado = slugify(&local.titulo);
    if generado != slug {
        return Err(AppError::Conflict(format!(
            "el servidor asignaría «{generado}» desde el título, no «{slug}»: no se publica para no crear un slug distinto"
        )));
    }
    if remoto::detalle(cred, slug).await.map_err(conflicto)?.is_some() {
        return Err(AppError::Conflict(format!("{slug} ya existe en el servidor")));
    }
    let alta = alta_desde(&local);
    alta.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let ficha = InmuebleRepository::get_ficha(pool, local.id).await?;
    if let Some((extras, _)) = &ficha {
        validar_extras(&extras.0).map_err(AppError::Validation)?;
    }
    let mut fotos = SyncRepository::claves_de_fotos(pool, local.id).await?;
    fotos.sort_by_key(|f| f.orden);
    for foto in &fotos {
        let ruta = InmuebleService::ruta_archivo(upload_dir, &foto.storage_key);
        let tamano = tokio::fs::metadata(&ruta)
            .await
            .map_err(|_| {
                AppError::Validation(format!("falta el archivo de la foto {}", foto.storage_key))
            })?
            .len();
        if tamano > MAX_FOTO_BYTES as u64 {
            return Err(AppError::Validation(format!(
                "la foto {} supera {} MiB",
                foto.storage_key,
                MAX_FOTO_BYTES / (1024 * 1024)
            )));
        }
    }

    // Desde aquí hay escrituras remotas: cualquier fallo va al informe.
    let creado = remoto::crear_inmueble(cred, &alta)
        .await
        .map_err(conflicto)?;
    let mut informe = InformeSync {
        slug: slug.to_string(),
        hecho: vec!["Alta con campos y copy".into()],
        fallo: None,
    };
    if creado.slug != slug {
        informe.fallo = Some(format!(
            "el servidor creó «{}» en lugar de «{slug}»; la publicación no continúa",
            creado.slug
        ));
        return Ok(informe);
    }
    if let Err(motivo) = resto(
        cred,
        upload_dir,
        &local,
        creado.id,
        ficha.as_ref(),
        &fotos,
        &mut informe.hecho,
    )
    .await
    {
        informe.fallo = Some(motivo);
    }
    Ok(informe)
}

/// Receta, ficha, fotos y visibilidad sobre el inmueble ya creado en el servidor.
async fn resto(
    cred: &Credenciales,
    upload_dir: &Path,
    local: &Inmueble,
    remoto_id: Uuid,
    ficha: Option<&(Json<Value>, Option<f64>)>,
    fotos: &[FotoClave],
    hecho: &mut Vec<String>,
) -> Result<(), String> {
    if let Some(receta) = &local.receta {
        remoto::poner_receta(cred, remoto_id, receta)
            .await
            .map_err(|e| paso("receta", e))?;
        hecho.push("Receta publicitaria".into());
    }
    if let Some((extras, precio_minimo)) = ficha {
        remoto::poner_ficha(cred, remoto_id, &extras.0, *precio_minimo)
            .await
            .map_err(|e| paso("ficha", e))?;
        hecho.push("Ficha privada (/ask)".into());
    }
    for foto in fotos {
        let ruta = InmuebleService::ruta_archivo(upload_dir, &foto.storage_key);
        let bytes = tokio::fs::read(&ruta)
            .await
            .map_err(|e| paso(&format!("leer la foto {}", foto.storage_key), e))?;
        let nombre = foto.storage_key.rsplit('/').next().unwrap_or_default();
        remoto::subir_foto(cred, remoto_id, nombre, &foto.origen, foto.orden, bytes)
            .await
            .map_err(|e| paso(&format!("foto {}", foto.orden), e))?;
    }
    if !fotos.is_empty() {
        hecho.push(format!("{} fotos", fotos.len()));
    }
    remoto::poner_publicacion(cred, remoto_id, local.publicado)
        .await
        .map_err(|e| paso("visibilidad", e))?;
    hecho.push(if local.publicado {
        "Publicado en el servidor".into()
    } else {
        "Sin publicar (visibilidad igual que local)".into()
    });
    Ok(())
}

fn alta_desde(local: &Inmueble) -> CreateInmuebleRequest {
    CreateInmuebleRequest {
        titulo: local.titulo.clone(),
        descripcion: local.descripcion.clone(),
        ubicacion: local.ubicacion.clone(),
        puestos: local.puestos,
        residencia: local.residencia.clone(),
        precio: local.precio,
        tipo: local.tipo.clone(),
        operacion: local.operacion.clone(),
        habitaciones: local.habitaciones,
        banos: local.banos,
        metros: local.metros,
        metros_terreno: local.metros_terreno,
        estado: local.estado.clone(),
        marketplace_id: local.marketplace_id.clone(),
        alias_titulos: local.alias_titulos.clone(),
        copy: local.copy.clone(),
    }
}
