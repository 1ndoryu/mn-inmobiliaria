#![cfg(test)]
//! Poda y canonización de la forma de salida de la IA.

use super::*;

#[test]
fn forma_poda_oferta_de_fotos() {
    let r =
        imponer_forma_borrador(&ia_fabio("¿Te comparto fotos para que la veas por dentro?"));
    assert!(!r.contains("fotos"), "{r}");
    assert!(!r.contains('?'), "{r}");
    assert!(r.contains("$43.000"), "{r}");
    assert!(r.contains(CTA_FIJO), "{r}");
    assert!(r.trim_end().ends_with(CONTACTO_WA), "{r}");
}

#[test]
fn forma_poda_sinonimo_vigente() {
    let r = imponer_forma_borrador(&ia_fabio("Sí, la publicación sigue vigente."));
    assert!(!r.contains("vigente"), "{r}");
    assert!(!r.contains("publicación"), "{r}");
    /* P1 + CTA + teléfono + wa (el teléfono va en bloque propio). */
    assert_eq!(r.split("\n\n").count(), 4, "{r}");
}

#[test]
fn forma_conserva_dato_banos() {
    let r = imponer_forma_borrador(&ia_fabio("Tiene 2 baños y 3 habitaciones."));
    assert!(r.contains("Tiene 2 baños y 3 habitaciones."), "{r}");
    /* P1 + dato + CTA + teléfono + wa. */
    assert_eq!(r.split("\n\n").count(), 5, "{r}");
}

#[test]
fn forma_sanea_pregunta_pegada_en_p1() {
    let ia = "Hola, Fabio, ¿Te comparto fotos? La Casa en Riberas del Caroní está disponible en $43.000 negociable.\n\nCuéntame qué estás buscando y con gusto te ayudo.";
    let r = imponer_forma_borrador(ia);
    assert!(!r.contains('?'), "{r}");
    assert!(r.contains("$43.000"), "{r}");
}

#[test]
fn forma_p1_invalido_da_minimo() {
    let r = imponer_forma_borrador("¿Te comparto fotos para que la veas?");
    assert!(r.contains(&FALLBACK_BORRADOR[..10]), "{r}");
    assert!(r.trim_end().ends_with(CONTACTO_WA), "{r}");
}

#[test]
fn forma_final_siempre_canonico() {
    let r = imponer_forma_borrador(&ia_fabio("Tiene 2 baños."));
    assert!(r.contains("Cualquier cosa escríbeme al 04249208855"), "{r}");
    assert!(r.trim_end().ends_with(CONTACTO_WA), "{r}");
}

#[test]
fn forma_partir_no_rompe_cifras() {
    let f = partir_frases("Cuesta $43.000 negociable. Tiene 2 baños.");
    assert!(f.iter().any(|x| x.contains("$43.000")), "{f:?}");
}
