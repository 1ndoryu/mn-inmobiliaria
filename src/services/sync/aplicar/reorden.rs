//! Reordenar las fotos del servidor en vez de reemplazarlas [25AA-3].
//!
//! El plan compara por posición: mover la principal cambia todas las posiciones
//! que se desplazan, aunque las fotos sean las mismas. Hacia el servidor no hay
//! forma de reemplazar los bytes de una posición, pero sí de reordenar. Si las
//! `original` de los dos lados son las mismas (mismo sha256) y solo cambia el
//! orden, basta una llamada: no se borra ni se sube nada.

use std::collections::HashSet;

use uuid::Uuid;

use super::fotos::MoverFotos;
use crate::services::sync::comparacion::huella::FotoHuella;
use crate::services::sync::comparacion::orden;
use crate::services::sync::comparacion::resolver::Eleccion;
use crate::services::sync::remoto::{self, Credenciales, FotoRemota};

/// Qué mandar al servidor y cómo quedan allí sus fotos al terminar.
#[derive(Debug)]
pub(super) struct Reorden {
    /// Ids de las `original` en el orden del local.
    pub(super) ids: Vec<Uuid>,
    /// Fotos del servidor con el `orden` que tendrán tras reordenar (las `mejorada`
    /// siguen a su `original`).
    pub(super) despues: Vec<FotoRemota>,
    /// Lo mismo para las huellas: sirve para saber qué filas del plan ya quedan iguales.
    pub(super) huellas_despues: Vec<FotoHuella>,
}

/// Lo que el reorden dejó hecho: las fotos del servidor como quedan y las claves
/// del plan que ya no hay que tocar.
pub(super) struct Resultado {
    pub(super) servidor: Vec<FotoRemota>,
    pub(super) resueltas: HashSet<String>,
}

/// Reordena el servidor si el plan solo quiere el orden del local y las fotos
/// son las mismas. Si el servidor no tiene el endpoint (uno anterior a 15AA-8) o
/// lo rechaza, no hay reorden: las filas siguen su camino y se dirá qué queda.
pub(super) async fn intentar(
    cred: &Credenciales,
    id: Uuid,
    mover: &MoverFotos<'_>,
    hecho: &mut Vec<String>,
) -> Option<Resultado> {
    // Con filas que el servidor gana no es "solo el orden del local": no se mezcla.
    if mover.plan.iter().any(|f| f.ganador != Eleccion::Local) {
        return None;
    }
    let r = calcular(mover.huella_local, mover.huella_servidor, mover.servidor)?;
    remoto::reordenar_fotos(cred, id, &r.ids).await.ok()?;
    hecho.push(format!("Fotos reordenadas en el servidor ({})", r.ids.len()));
    let resueltas = mover
        .plan
        .iter()
        .filter(|f| orden::queda_igual(mover.huella_local, &r.huellas_despues, &f.origen, f.orden))
        .map(|f| f.clave.clone())
        .collect();
    Some(Resultado { servidor: r.despues, resueltas })
}

/// Lo que hay que mandar al servidor para dejar sus `original` como las del
/// local, con los ids de allí. `None` si `orden::reordenar` no ve reorden o falta alguna foto.
pub(super) fn calcular(
    local: &[FotoHuella],
    huella_servidor: &[FotoHuella],
    servidor: &[FotoRemota],
) -> Option<Reorden> {
    let r = orden::reordenar(local, huella_servidor)?;
    let mut ids = Vec::with_capacity(r.viejos.len());
    for viejo in &r.viejos {
        let remota = servidor
            .iter()
            .find(|f| f.origen == "original" && f.orden == *viejo)?;
        ids.push(remota.id);
    }
    let nuevo = |o: i32| r.nuevo_orden.get(&o).copied().unwrap_or(o);
    let despues = servidor
        .iter()
        .map(|f| FotoRemota { orden: nuevo(f.orden), ..f.clone() })
        .collect();
    Some(Reorden { ids, despues, huellas_despues: r.huellas_despues })
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn huella(origen: &str, orden: i32, sha: Option<&str>) -> FotoHuella {
        FotoHuella {
            origen: origen.into(),
            orden,
            sha256: sha.map(str::to_string),
            bytes: None,
        }
    }

    fn remota(origen: &str, orden: i32) -> FotoRemota {
        FotoRemota {
            id: Uuid::new_v4(),
            origen: origen.into(),
            orden,
            url: format!("https://x/{origen}-{orden}.jpg"),
        }
    }

    #[test]
    fn mover_la_principal_es_un_reorden_y_la_mejorada_sigue_a_su_original() {
        // Local: C, A, B. Servidor: A, B, C (con mejorada de A en 0).
        let local = [huella("original", 0, Some("c")), huella("original", 1, Some("a")), huella("original", 2, Some("b"))];
        let hs = [huella("original", 0, Some("a")), huella("original", 1, Some("b")), huella("original", 2, Some("c"))];
        let srv = [remota("original", 0), remota("original", 1), remota("original", 2), remota("mejorada", 0)];
        let r = calcular(&local, &hs, &srv).expect("mismas fotos, otro orden");
        assert_eq!(r.ids, vec![srv[2].id, srv[0].id, srv[1].id]);
        let orden = |i: usize| r.despues[i].orden;
        assert_eq!((orden(0), orden(1), orden(2), orden(3)), (1, 2, 0, 1), "la mejorada de A pasa a 1");
    }

    #[test]
    fn sin_reorden_si_ya_estan_en_orden_o_las_fotos_difieren() {
        let local = [huella("original", 0, Some("a")), huella("original", 1, Some("b"))];
        let srv = [remota("original", 0), remota("original", 1)];
        let igual = [huella("original", 0, Some("a")), huella("original", 1, Some("b"))];
        assert!(calcular(&local, &igual, &srv).is_none(), "ya en orden");
        let otra = [huella("original", 0, Some("a")), huella("original", 1, Some("z"))];
        assert!(calcular(&local, &otra, &srv).is_none(), "una foto es otra");
        let sin_sha = [huella("original", 0, Some("b")), huella("original", 1, None)];
        assert!(calcular(&local, &sin_sha, &srv).is_none(), "sha sin calcular");
    }
}
