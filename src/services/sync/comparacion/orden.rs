//! Reorden de fotos visto solo con huellas [25AA-4].
//!
//! Lo comparten el plan (¿el servidor puede quedar igual al local reordenando?
//! entonces esas filas cuentan como operación) y `aplicar` (que además necesita
//! los ids del servidor). Pura: sin BD ni red.

use std::collections::HashMap;

use super::huella::FotoHuella;

/// Cómo quedaría el servidor al reordenar sus `original` como las del local.
#[derive(Debug)]
pub struct Reordenado {
    /// `orden` actual de la `original` del servidor que va en cada posición nueva.
    pub viejos: Vec<i32>,
    /// `orden` viejo → `orden` nuevo (las `mejorada` siguen a su `original`).
    pub nuevo_orden: HashMap<i32, i32>,
    /// Huellas del servidor tras reordenar.
    pub huellas_despues: Vec<FotoHuella>,
}

/// `Some` si las `original` del servidor son exactamente las del local en otro
/// orden. `None` si no hay nada que reordenar o no se puede asegurar (un sha sin
/// calcular, fotos distintas, o ya en orden).
#[must_use]
pub fn reordenar(local: &[FotoHuella], servidor: &[FotoHuella]) -> Option<Reordenado> {
    let mut originales: Vec<&FotoHuella> = local.iter().filter(|f| f.origen == "original").collect();
    originales.sort_by_key(|f| f.orden);
    // sha256 → órdenes del servidor con esa foto (puede haber duplicadas: se gastan una a una).
    let mut libres: HashMap<&str, Vec<i32>> = HashMap::new();
    let mut del_servidor = 0usize;
    for h in servidor.iter().filter(|f| f.origen == "original") {
        libres.entry(h.sha256.as_deref()?).or_default().push(h.orden);
        del_servidor += 1;
    }
    if del_servidor != originales.len() {
        return None;
    }
    let mut viejos = Vec::with_capacity(originales.len());
    let mut nuevo_orden: HashMap<i32, i32> = HashMap::new();
    for (nuevo, h) in originales.iter().enumerate() {
        let viejo = libres.get_mut(h.sha256.as_deref()?)?.pop()?;
        viejos.push(viejo);
        nuevo_orden.insert(viejo, i32::try_from(nuevo).ok()?);
    }
    if nuevo_orden.iter().all(|(viejo, nuevo)| viejo == nuevo) {
        return None;
    }
    let huellas_despues = servidor
        .iter()
        .map(|h| FotoHuella {
            orden: nuevo_orden.get(&h.orden).copied().unwrap_or(h.orden),
            ..h.clone()
        })
        .collect();
    Some(Reordenado { viejos, nuevo_orden, huellas_despues })
}

/// ¿La foto `(origen, orden)` del local queda igual en el servidor tras reordenar?
#[must_use]
pub fn queda_igual(local: &[FotoHuella], despues: &[FotoHuella], origen: &str, orden: i32) -> bool {
    let sha = |hs: &[FotoHuella]| {
        hs.iter()
            .find(|h| h.origen == origen && h.orden == orden)
            .and_then(|h| h.sha256.clone())
    };
    sha(local).is_some() && sha(local) == sha(despues)
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn h(origen: &str, orden: i32, sha: Option<&str>) -> FotoHuella {
        FotoHuella { origen: origen.into(), orden, sha256: sha.map(str::to_string), bytes: None }
    }

    #[test]
    fn mismas_fotos_en_otro_orden_se_reordenan_y_la_mejorada_sigue() {
        let local = [h("original", 0, Some("c")), h("original", 1, Some("a")), h("original", 2, Some("b"))];
        let srv = [h("original", 0, Some("a")), h("original", 1, Some("b")), h("original", 2, Some("c")), h("mejorada", 0, Some("ma"))];
        let r = reordenar(&local, &srv).expect("mismas fotos, otro orden");
        assert_eq!(r.viejos, vec![2, 0, 1]);
        assert!(queda_igual(&local, &r.huellas_despues, "original", 0));
        assert_eq!(r.huellas_despues[3].orden, 1, "la mejorada de A pasa a 1");
    }

    #[test]
    fn nada_que_reordenar_si_ya_estan_en_orden_o_difieren() {
        let local = [h("original", 0, Some("a")), h("original", 1, Some("b"))];
        assert!(reordenar(&local, &local).is_none(), "ya en orden");
        let otra = [h("original", 0, Some("a")), h("original", 1, Some("z"))];
        assert!(reordenar(&local, &otra).is_none(), "una foto es otra");
        let sin_sha = [h("original", 0, Some("b")), h("original", 1, None)];
        assert!(reordenar(&local, &sin_sha).is_none(), "sha sin calcular");
    }
}
