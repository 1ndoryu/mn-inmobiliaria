//! Mover las fotos de un plan de resolución [F3 de 14AA-2].
//!
//! Los valores de texto se copian desde el inmueble tipado; las fotos, no: van
//! por bytes en los dos sentidos (D6). Se copia lo que el lado ganador tiene y
//! el otro no, y se borra del lado que pierde lo que el ganador no tiene.
//!
//! El orden importa. Las copias van primero y los borrados después, de mayor a
//! menor `orden` y la `mejorada` antes que su `original`: borrar una `original`
//! renumera las que van por encima y arrastra a su hermana ([08AA-4]), así que
//! lo que queda por hacer no cambia de número. Un borrado que ya no encuentra
//! nada se da por hecho: lo pudo cumplir la cascada de otro.

use std::path::Path;

use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{FotoPublica, MAX_FOTO_BYTES};
use crate::services::inmueble::InmuebleService;
use crate::services::sync::comparacion::resolver::{Accion, Eleccion, FilaPlan};
use crate::services::sync::operacion::paso;
use crate::services::sync::remoto::{self, Credenciales, FotoRemota};
use crate::services::sync::traer::MAX_BYTES_TOTALES;

/// Una fila del plan que es una foto (`foto:<origen>:<orden>`). El valor no se
/// copia como texto: los bytes van de un lado al otro.
#[derive(Debug, Clone)]
pub(super) struct FilaFoto {
    pub(super) clave: String,
    pub(super) origen: String,
    pub(super) orden: i32,
    /// Lado que gana: de ahí salen los bytes cuando hay que copiar.
    pub(super) ganador: Eleccion,
    /// `Escribir` = el otro lado tiene que acabar con esa foto; `Quitar` = sin ella.
    pub(super) accion: Accion,
}

/// Las fotos que el plan manda mover, con las de cada lado para copiarlas por
/// bytes (por URL) y borrarlas por id.
pub(super) struct MoverFotos<'a> {
    pub(super) plan: Vec<FilaFoto>,
    pub(super) local: &'a [FotoPublica],
    pub(super) servidor: &'a [FotoRemota],
}

/// `None` si la clave no es de foto (o no tiene la forma esperada).
pub(super) fn fila_foto(fila: &FilaPlan) -> Option<FilaFoto> {
    let (origen, orden) = fila.clave.strip_prefix("foto:")?.split_once(':')?;
    Some(FilaFoto {
        clave: fila.clave.clone(),
        origen: origen.to_string(),
        orden: orden.parse().ok()?,
        ganador: fila.elegido,
        accion: fila.accion,
    })
}

/// Mueve las fotos en el local: copia las que el plan trae del servidor y borra
/// las que el lado ganador no tiene.
pub(super) async fn al_local(
    pool: &PgPool,
    upload_dir: &Path,
    cred: &Credenciales,
    id: Uuid,
    mover: &MoverFotos<'_>,
    hecho: &mut Vec<String>,
    pendientes: &mut Vec<String>,
) -> Result<(), String> {
    let destino: Vec<(String, i32)> = mover.local.iter().map(|f| (f.origen.clone(), f.orden)).collect();
    let mut total = 0usize;
    for fila in mover.plan.iter().filter(|f| f.ganador == Eleccion::Servidor && f.accion == Accion::Escribir) {
        // Solo se añade lo que falta. Si el local ya tiene esa clave con otro
        // contenido, reemplazarla exigiría borrar y renumerar el álbum entero:
        // se dice y se deja para la mano.
        if hay_local(mover.local, fila).is_some() {
            pendientes.push(format!(
                "{} (el local ya tiene esa clave con otro contenido)",
                fila.clave
            ));
            continue;
        }
        let Some(remota) = hay_remota(mover.servidor, fila) else {
            pendientes.push(format!("{} (el servidor ya no la tiene)", fila.clave));
            continue;
        };
        let bytes = remoto::descargar_foto(cred, &remota.url)
            .await
            .map_err(|e| paso(&fila.clave, e))?;
        total += bytes.len();
        if total > MAX_BYTES_TOTALES {
            return Err(paso(
                &fila.clave,
                format!("las fotos del plan pasan de {MAX_BYTES_TOTALES} bytes"),
            ));
        }
        InmuebleService::subir_foto(
            pool,
            upload_dir,
            id,
            &nombre_de(&remota.url),
            Some(&fila.origen),
            Some(fila.orden),
            &bytes,
        )
        .await
        .map_err(|e| paso(&fila.clave, e))?;
        hecho.push(format!("Foto {} copiada al local", fila.clave));
    }
    for fila in de_mayor_a_menor(&mover.plan, Eleccion::Servidor) {
        let Some(actual) = hay_local(mover.local, fila) else {
            continue; // ya no está: el objetivo ya se cumplía
        };
        if arrastra_mejorada(&destino, &mover.plan, fila, Eleccion::Servidor) {
            pendientes.push(format!(
                "{} (borrarla se llevaría su mejorada del mismo orden, que el plan conserva)",
                fila.clave
            ));
            continue;
        }
        match InmuebleService::delete_foto(pool, upload_dir, actual.id).await {
            Ok(()) => hecho.push(format!("Foto {} borrada del local", fila.clave)),
            // Se la llevó la cascada de otra: el objetivo ya está cumplido.
            Err(AppError::NotFound(_)) => {}
            Err(e) => return Err(paso(&fila.clave, e)),
        }
    }
    Ok(())
}

/// Mueve las fotos en el servidor, con las mismas reglas que en el local. Esto
/// escribe en producción: es destructivo y no se deshace.
pub(super) async fn al_servidor(
    upload_dir: &Path,
    cred: &Credenciales,
    id: Uuid,
    mover: &MoverFotos<'_>,
    hecho: &mut Vec<String>,
    pendientes: &mut Vec<String>,
) -> Result<(), String> {
    let destino: Vec<(String, i32)> = mover.servidor.iter().map(|f| (f.origen.clone(), f.orden)).collect();
    let mut total = 0usize;
    for fila in mover.plan.iter().filter(|f| f.ganador == Eleccion::Local && f.accion == Accion::Escribir) {
        if hay_remota(mover.servidor, fila).is_some() {
            pendientes.push(format!(
                "{} (el servidor ya tiene esa clave con otro contenido)",
                fila.clave
            ));
            continue;
        }
        let Some(actual) = hay_local(mover.local, fila) else {
            pendientes.push(format!("{} (el local ya no la tiene)", fila.clave));
            continue;
        };
        let bytes = leer_foto_local(upload_dir, actual)
            .await
            .map_err(|e| paso(&fila.clave, e))?;
        if bytes.len() > MAX_FOTO_BYTES {
            return Err(paso(
                &fila.clave,
                format!("la foto pasa de {MAX_FOTO_BYTES} bytes"),
            ));
        }
        total += bytes.len();
        if total > MAX_BYTES_TOTALES {
            return Err(paso(
                &fila.clave,
                format!("las fotos del plan pasan de {MAX_BYTES_TOTALES} bytes"),
            ));
        }
        remoto::subir_foto(cred, id, &nombre_de(&actual.url), &fila.origen, fila.orden, bytes)
            .await
            .map_err(|e| paso(&fila.clave, e))?;
        hecho.push(format!("Foto {} copiada al servidor", fila.clave));
    }
    for fila in de_mayor_a_menor(&mover.plan, Eleccion::Local) {
        let Some(remota) = hay_remota(mover.servidor, fila) else {
            continue;
        };
        if arrastra_mejorada(&destino, &mover.plan, fila, Eleccion::Local) {
            pendientes.push(format!(
                "{} (borrarla se llevaría su mejorada del mismo orden, que el plan conserva)",
                fila.clave
            ));
            continue;
        }
        remoto::borrar_foto(cred, remota.id)
            .await
            .map_err(|e| paso(&fila.clave, e))?;
        hecho.push(format!("Foto {} borrada del servidor", fila.clave));
    }
    Ok(())
}

/// Las filas de borrado del lado contrario a `ganador`, de mayor a menor `orden`
/// y la `mejorada` antes que su `original` (así, cuando le toca a la original, la
/// cascada ya no encuentra hermana).
fn de_mayor_a_menor<'a>(plan: &'a [FilaFoto], ganador: Eleccion) -> Vec<&'a FilaFoto> {
    let mut quitar: Vec<&FilaFoto> = plan
        .iter()
        .filter(|f| f.ganador == ganador && f.accion == Accion::Quitar)
        .collect();
    quitar.sort_by(|a, b| b.orden.cmp(&a.orden).then(a.origen.cmp(&b.origen)));
    quitar
}

/// Bytes de una foto del local, desde su `url` (`/uploads/<clave>`).
async fn leer_foto_local(upload_dir: &Path, foto: &FotoPublica) -> Result<Vec<u8>, AppError> {
    let clave = foto.url.strip_prefix("/uploads/").unwrap_or(&foto.url);
    let ruta = InmuebleService::ruta_archivo(upload_dir, clave);
    tokio::fs::read(&ruta)
        .await
        .map_err(|e| AppError::NotFound(format!("no se pudo leer {}: {e}", ruta.display())))
}

fn hay_local<'a>(fotos: &'a [FotoPublica], fila: &FilaFoto) -> Option<&'a FotoPublica> {
    fotos.iter().find(|f| f.origen == fila.origen && f.orden == fila.orden)
}

fn hay_remota<'a>(fotos: &'a [FotoRemota], fila: &FilaFoto) -> Option<&'a FotoRemota> {
    fotos.iter().find(|f| f.origen == fila.origen && f.orden == fila.orden)
}

/// ¿Borrar esta `original` se llevaría por delante una `mejorada` que el plan
/// conserva? La cascada ([08AA-4]) borra las dos, así que entonces no se toca
/// nada: se informa. `destino` son las fotos del lado que se está escribiendo.
fn arrastra_mejorada(
    destino: &[(String, i32)],
    plan: &[FilaFoto],
    fila: &FilaFoto,
    ganador: Eleccion,
) -> bool {
    if fila.origen != "original" {
        return false;
    }
    let hermana = destino
        .iter()
        .any(|(origen, orden)| origen == "mejorada" && *orden == fila.orden);
    let se_va_tambien = plan.iter().any(|f| {
        f.origen == "mejorada"
            && f.orden == fila.orden
            && f.ganador == ganador
            && f.accion == Accion::Quitar
    });
    hermana && !se_va_tambien
}

/// Nombre de archivo de una `url`, sin parámetros. `subir_foto` valida la
/// extensión, así que si no la lleva, la subida lo dirá.
fn nombre_de(url: &str) -> String {
    url.split('?')
        .next()
        .unwrap_or(url)
        .rsplit('/')
        .next()
        .unwrap_or("foto.jpg")
        .to_string()
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::services::sync::comparacion::comparar::Lado;

    fn fila(clave: &str, elegido: Eleccion, accion: Accion) -> FilaPlan {
        FilaPlan {
            clave: clave.into(),
            lado: Lado::Ambos,
            elegido,
            accion,
            local: Some("x".into()),
            servidor: Some("y".into()),
            miniatura_local: None,
            miniatura_servidor: None,
        }
    }

    #[test]
    fn las_claves_de_foto_se_leen_origen_y_orden() {
        let f = fila_foto(&fila("foto:mejorada:12", Eleccion::Local, Accion::Quitar)).expect("es foto");
        assert_eq!((f.origen.as_str(), f.orden, f.ganador, f.accion), ("mejorada", 12, Eleccion::Local, Accion::Quitar));
        assert!(fila_foto(&fila("titulo", Eleccion::Local, Accion::Escribir)).is_none());
        assert!(fila_foto(&fila("foto:original", Eleccion::Local, Accion::Escribir)).is_none());
        assert!(fila_foto(&fila("foto:original:x", Eleccion::Local, Accion::Escribir)).is_none());
    }

    #[test]
    fn los_borrados_van_de_mayor_a_menor_orden_y_la_mejorada_primero() {
        let plan: Vec<FilaFoto> = [
            "foto:original:1",
            "foto:mejorada:2",
            "foto:original:2",
            "foto:original:3",
        ]
        .iter()
        .filter_map(|c| fila_foto(&fila(c, Eleccion::Servidor, Accion::Quitar)))
        .collect();
        let orden: Vec<&str> = de_mayor_a_menor(&plan, Eleccion::Servidor)
            .iter()
            .map(|f| f.clave.as_str())
            .collect();
        assert_eq!(
            orden,
            vec!["foto:original:3", "foto:mejorada:2", "foto:original:2", "foto:original:1"],
            "de mayor a menor orden y la mejorada antes que su original"
        );
    }

    #[test]
    fn borrar_una_original_con_mejorada_viva_queda_pendiente() {
        let destino = [("original".to_string(), 2), ("mejorada".to_string(), 2)];
        let solo_original = vec![fila_foto(&fila("foto:original:2", Eleccion::Servidor, Accion::Quitar)).unwrap()];
        let borrar = solo_original[0].clone();
        assert!(
            arrastra_mejorada(&destino, &solo_original, &borrar, Eleccion::Servidor),
            "la cascada se llevaría la mejorada"
        );
        // Si el plan también quita la mejorada, ya no hay nada que proteger.
        let ambas: Vec<FilaFoto> = ["foto:original:2", "foto:mejorada:2"]
            .iter()
            .map(|c| fila_foto(&fila(c, Eleccion::Servidor, Accion::Quitar)).unwrap())
            .collect();
        assert!(!arrastra_mejorada(&destino, &ambas, &borrar, Eleccion::Servidor));
        // Y una mejorada nunca arrastra nada.
        let mejorada = fila_foto(&fila("foto:mejorada:2", Eleccion::Servidor, Accion::Quitar)).unwrap();
        assert!(!arrastra_mejorada(&destino, &ambas, &mejorada, Eleccion::Servidor));
    }
}
