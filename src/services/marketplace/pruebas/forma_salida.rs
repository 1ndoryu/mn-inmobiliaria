#![cfg(test)]
//! Poda y canonización de la forma de salida de la IA.

use super::*;

#[test]
fn forma_poda_oferta_de_fotos() {
    let r = imponer_forma_borrador(
        &ia_fabio("¿Te comparto fotos para que la veas por dentro?"),
        None,
    );
    assert!(!r.contains("fotos"), "{r}");
    assert!(!r.contains('?'), "{r}");
    assert!(r.contains("$43.000"), "{r}");
    assert!(r.trim_end().ends_with(CIERRE_BORRADOR), "{r}");
}

#[test]
fn forma_poda_sinonimo_vigente() {
    let r = imponer_forma_borrador(&ia_fabio("Sí, la publicación sigue vigente."), None);
    assert!(!r.contains("vigente"), "{r}");
    assert!(!r.contains("publicación"), "{r}");
    /* [21AA-2] Apertura + cierre. */
    assert_eq!(r.split("\n\n").count(), 2, "{r}");
}

#[test]
fn forma_conserva_dato_banos() {
    let r = imponer_forma_borrador(&ia_fabio("Tiene 2 baños y 3 habitaciones."), None);
    assert!(r.contains("Tiene 2 baños y 3 habitaciones."), "{r}");
    /* [21AA-2] La frase del dato va pegada a la apertura: un solo párrafo. */
    assert_eq!(r.split("\n\n").count(), 2, "{r}");
}

#[test]
fn forma_sanea_pregunta_pegada_en_p1() {
    let ia = "Hola, Fabio, ¿Te comparto fotos? La Casa en Riberas del Caroní está disponible en $43.000 negociable.\n\nCuéntame qué estás buscando y con gusto te ayudo.";
    let r = imponer_forma_borrador(ia, None);
    assert!(!r.contains('?'), "{r}");
    assert!(r.contains("$43.000"), "{r}");
}

#[test]
fn forma_p1_invalido_da_minimo() {
    let r = imponer_forma_borrador("¿Te comparto fotos para que la veas?", None);
    assert!(r.contains(&FALLBACK_BORRADOR[..10]), "{r}");
    assert!(r.trim_end().ends_with(CONTACTO_WA), "{r}");
}

#[test]
fn forma_final_siempre_canonico() {
    let r = imponer_forma_borrador(&ia_fabio("Tiene 2 baños."), None);
    assert!(r.contains(CIERRE_BORRADOR), "{r}");
    assert!(r.trim_end().ends_with(CONTACTO_WA), "{r}");
}

#[test]
fn forma_partir_no_rompe_cifras() {
    let f = partir_frases("Cuesta $43.000 negociable. Tiene 2 baños.");
    assert!(f.iter().any(|x| x.contains("$43.000")), "{f:?}");
}

/// Regresión Kama Merú: la ubicación con «Av.» no debe cortarse en «Av.».
#[test]
fn forma_no_corta_ubicacion_con_abreviatura() {
    let ia = "El apartamento en Residencias Kama Merú sigue disponible en 35.000$ negociable.\n\nEl inmueble se ubica en Av. Principal de Villa Icabarú (Paso Caroní), Puerto Ordaz.\n\nCuéntame qué estás buscando y con gusto te ayudo.";
    let r = imponer_forma_borrador(ia, None);
    assert!(
        r.contains(
            "El inmueble se ubica en Av. Principal de Villa Icabarú (Paso Caroní), Puerto Ordaz."
        ),
        "{r}"
    );
}

/// Un trozo de ubicación sin la forma pedida no sale; la frase completa sí.
#[test]
fn forma_ubicacion_solo_completa() {
    let suelta = "El apartamento sigue disponible en 35.000$ negociable.\n\nSe ubica en Av. Principal de Villa Icabarú.\n\nCuéntame qué estás buscando.";
    let r = imponer_forma_borrador(suelta, None);
    assert!(!r.contains("Principal de Villa"), "{r}");
    let completa = "El apartamento sigue disponible en 35.000$ negociable.\n\nEl inmueble se ubica en Av. Principal de Villa Icabarú.\n\nCuéntame qué estás buscando.";
    let r = imponer_forma_borrador(completa, None);
    assert!(
        r.contains("El inmueble se ubica en Av. Principal de Villa Icabarú."),
        "{r}"
    );
}

#[test]
fn partir_frases_no_parte_abreviaturas() {
    let f = partir_frases("Se ubica en Av. Principal. Tiene 2 baños.");
    assert_eq!(f, vec!["Se ubica en Av. Principal.", "Tiene 2 baños."]);
}

/* [20AA-1] Con ficha, el primer párrafo lleva la cifra publicada aunque la IA
 * escriba el fallback. Testigos reales: el hilo `eduanys|apartamento planta
 * baja con patio en residencias kama merú…` con la ficha en `datos` dio el
 * fallback a las 03:42 del 2026-10-20 y el precio correcto a las 11:02. */
fn seguro_kama_meru() -> PromptSeguro {
    PromptSeguro {
        titulo: "Planta baja con patio en Residencias Kama Merú".to_string(),
        precio_publico: "$86.000".to_string(),
        operacion: "venta".to_string(),
        zona: "Puerto Ordaz, Residencias Kama Merú".to_string(),
        m2: 120.0,
        habitaciones: 3,
        descripcion_corta: "Planta baja con patio".to_string(),
        nombre_corto: "Apart. en Resd. Kama Merú".to_string(),
    }
}

#[test]
fn forma_con_ficha_no_deja_salir_el_fallback() {
    let ia = format!("{FALLBACK_BORRADOR}\n\nCuéntame qué estás buscando y con gusto te ayudo.");
    let r = imponer_forma_borrador(&ia, Some(&seguro_kama_meru()));
    assert!(!r.contains(&FALLBACK_BORRADOR[..10]), "{r}");
    assert!(
        r.contains("Apart. en Resd. Kama Merú sigue disponible en 86.000$ negociable."),
        "{r}"
    );
}

/// El molde lo escribe Rust: ni siquiera la cifra del modelo manda si no es la de la ficha.
#[test]
fn forma_con_ficha_corrige_cifra_ajena() {
    let ia = "Apart. en Resd. Kama Merú sigue disponible en 35.000$ negociable.\n\nCuéntame qué estás buscando y con gusto te ayudo.";
    let r = imponer_forma_borrador(ia, Some(&seguro_kama_meru()));
    assert!(r.contains("86.000$"), "{r}");
    assert!(!r.contains("35.000"), "{r}");
}

/// La cifra buena del modelo se respeta: no se reescribe un párrafo ya correcto.
#[test]
fn forma_con_ficha_respeta_la_cifra_buena() {
    let ia = "Apart. en Resd. Kama Merú sigue disponible en 86.000$ negociable.\n\nCuéntame qué estás buscando y con gusto te ayudo.";
    let r = imponer_forma_borrador(ia, Some(&seguro_kama_meru()));
    assert!(
        r.starts_with("Apart. en Resd. Kama Merú sigue disponible en 86.000$ negociable."),
        "{r}"
    );
}

/// Alquiler: el molde es el del canon, no el de venta (08AA-25).
#[test]
fn forma_con_ficha_de_alquiler_usa_el_canon() {
    let mut s = seguro_kama_meru();
    s.operacion = "alquiler".to_string();
    let r = imponer_forma_borrador(FALLBACK_BORRADOR, Some(&s));
    assert!(
        r.contains("El canon mensual es de 86.000$ para Apart. en Resd. Kama Merú."),
        "{r}"
    );
}

/// Sin precio en la ficha (`$0`) no hay nada que decir: el fallback sigue valiendo.
#[test]
fn forma_sin_precio_en_la_ficha_conserva_el_fallback() {
    let mut s = seguro_kama_meru();
    s.precio_publico = "$0".to_string();
    let r = imponer_forma_borrador(FALLBACK_BORRADOR, Some(&s));
    assert!(r.contains(&FALLBACK_BORRADOR[..10]), "{r}");
}

/* [20AA-1] La reserva (IA caída) es la otra vía por la que se perdía el precio:
 * el 2026-10-20 el hilo de Eduanys, con la ficha emparejada, salió por reserva
 * con el fallback pelado. Testigo: `mp_respuestas_cache` de la 89 + el warn
 * «borrador mp: IA caída (OpenCode Go devolvio una respuesta sin texto)». */

#[test]
fn reserva_con_ficha_lleva_el_precio() {
    let r = borrador_de_reserva(Some(&seguro_kama_meru()));
    assert!(
        r.starts_with("Apart. en Resd. Kama Merú sigue disponible en 86.000$ negociable."),
        "{r}"
    );
    assert!(!r.contains(&FALLBACK_BORRADOR[..10]), "{r}");
    assert!(r.contains(CONTACTO_TEL), "{r}");
    assert!(r.trim_end().ends_with(CONTACTO_WA), "{r}");
}

#[test]
fn reserva_sin_ficha_conserva_el_fallback() {
    let r = borrador_de_reserva(None);
    assert!(r.contains(&FALLBACK_BORRADOR[..10]), "{r}");
    assert!(r.trim_end().ends_with(CONTACTO_WA), "{r}");
}

/// [21AA-2] Forma aprobada por ella el 2026-10-21, literal: apertura con el
/// dato, la ubicación en la misma línea y el cierre (teléfono + enlace) como
/// único final, sin la invitación «Cuéntame qué estás buscando».
#[test]
fn forma_aprobada_2026_10_21() {
    let ia = "Casa en Riberas del Caroní sigue disponible en 43.000$ negociable.\n\nEl inmueble se ubica en Av. Atlántico, Riberas del Caroní.\n\nCuéntame qué estás buscando y con gusto te ayudo.";
    let r = imponer_forma_borrador(ia, None);
    assert_eq!(
        r,
        format!(
            "Casa en Riberas del Caroní sigue disponible en 43.000$ negociable. \
             El inmueble se ubica en Av. Atlántico, Riberas del Caroní.\n\n\
             Puede llamarnos al {CONTACTO_TEL} para cualquier consulta, o \
             escribirnos al whatsapp: {CONTACTO_WA}"
        )
    );
}
