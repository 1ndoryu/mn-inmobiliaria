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
    assert!(r.trim_end().ends_with(CIERRE_BORRADOR), "{r}");
}

#[test]
fn forma_final_siempre_canonico() {
    let r = imponer_forma_borrador(&ia_fabio("Tiene 2 baños."), None);
    assert!(r.contains(CIERRE_BORRADOR), "{r}");
    assert!(r.trim_end().ends_with(CIERRE_BORRADOR), "{r}");
    /* [GW-15] El cierre ya no lleva enlace: ningún borrador debe traerlo. */
    assert!(!r.contains(CONTACTO_WA), "{r}");
    assert!(!r.contains("wa.me"), "{r}");
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
        precio_alquiler_publico: None,
        operacion: "venta".to_string(),
        zona: "Puerto Ordaz, Residencias Kama Merú".to_string(),
        m2: 120.0,
        habitaciones: 3,
        descripcion_corta: "Planta baja con patio".to_string(),
        nombre_corto: "Apart. en Resd. Kama Merú".to_string(),
        inmueble_id: uuid::Uuid::nil(),
        estado: "disponible".to_string(),
    }
}

#[test]
fn forma_con_ficha_no_deja_salir_el_fallback() {
    let ia = format!("{FALLBACK_BORRADOR}\n\nCuéntame qué estás buscando y con gusto te ayudo.");
    let r = imponer_forma_borrador(&ia, Some(&seguro_kama_meru()));
    assert!(!r.contains(&FALLBACK_BORRADOR[..10]), "{r}");
    assert!(
        r.contains("El apartamento en Resd. Kama Merú sigue disponible en 86.000$ negociable."),
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
        r.starts_with("El apartamento en Resd. Kama Merú sigue disponible en 86.000$ negociable."),
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
        r.contains("El canon mensual es de 86.000$ para el apartamento en Resd. Kama Merú."),
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
        r.starts_with("El apartamento en Resd. Kama Merú sigue disponible en 86.000$ negociable."),
        "{r}"
    );
    assert!(!r.contains(&FALLBACK_BORRADOR[..10]), "{r}");
    assert!(r.contains(CONTACTO_TEL), "{r}");
    assert!(r.trim_end().ends_with(CIERRE_BORRADOR), "{r}");
}

#[test]
fn reserva_sin_ficha_conserva_el_fallback() {
    let r = borrador_de_reserva(None);
    assert!(r.contains(&FALLBACK_BORRADOR[..10]), "{r}");
    assert!(r.trim_end().ends_with(CIERRE_BORRADOR), "{r}");
}

/// [21AA-2] Forma aprobada por ella el 2026-10-21, literal: apertura con el
/// dato, la ubicación en la misma línea y el cierre como único final, sin la
/// invitación «Cuéntame qué estás buscando». [GW-15] El cierre ya no lleva el
/// enlace de WhatsApp (decisión de ella 2026-11-21).
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
             escribirnos al whatsapp."
        )
    );
}

/// [GW-15] Un cierre con enlace que devuelva la IA (o que venga de una fila de
/// caché vieja) se sustituye por el vigente: el enlace no puede volver a salir.
#[test]
fn forma_recorta_el_enlace_de_un_cierre_viejo() {
    let ia = format!(
        "Casa en Riberas del Caroní sigue disponible en 43.000$ negociable.\n\n\
         {CIERRE_BORRADOR_CON_ENLACE}"
    );
    let r = imponer_forma_borrador(&ia, None);
    assert!(r.trim_end().ends_with(CIERRE_BORRADOR), "{r}");
    assert!(!r.contains(CONTACTO_WA), "{r}");
    assert!(!r.contains(CIERRE_BORRADOR_CON_ENLACE), "{r}");
}

/// [25AA-7] «Ambas» cita la venta y el canon; sin canon, el molde de venta.
#[test]
fn apertura_de_ambas_cita_venta_y_canon() {
    let mut s = seguro_kama_meru();
    s.operacion = "ambas".to_string();
    s.precio_alquiler_publico = Some("2.800".to_string());
    let (cifras, molde) = apertura_de_ficha(&s).expect("hay apertura");
    assert_eq!(cifras, vec!["86.000", "2.800"]);
    assert!(molde.contains("86.000$") && molde.contains("alquiler por 2.800$ al mes"), "{molde}");

    s.precio_alquiler_publico = None;
    let (cifras, molde) = apertura_de_ficha(&s).expect("hay apertura");
    assert_eq!(cifras, vec!["86.000"]);
    assert!(molde.contains("sigue disponible en 86.000$"), "{molde}");
}

/// [28AA-cache] Ficha sin `nombre_corto` (Emperatriz, Desiree): antes no había
/// apertura y el fallback se servía con precio. Ahora cita «Este inmueble».
#[test]
fn forma_con_ficha_sin_nombre_lleva_el_precio() {
    let mut s = seguro_kama_meru();
    s.nombre_corto = String::new();
    let r = imponer_forma_borrador(&format!("{FALLBACK_BORRADOR}\n\n{CONTACTO_WA}"), Some(&s));
    assert!(!r.contains(&FALLBACK_BORRADOR[..10]), "{r}");
    assert!(r.contains("Este inmueble sigue disponible en 86.000$ negociable."), "{r}");
}

/// [28AA-cache] `falta_precio` marca el fallback y la cifra ajena; no la cifra buena.
#[test]
fn falta_precio_marca_fallback_y_cifra_ajena() {
    let s = seguro_kama_meru();
    assert!(falta_precio(FALLBACK_BORRADOR, &s));
    assert!(falta_precio("Apart. en Resd. Kama Merú sigue disponible en 35.000$.", &s));
    assert!(!falta_precio("Apart. en Resd. Kama Merú sigue disponible en 86.000$ negociable.", &s));
    assert!(!falta_precio(FALLBACK_BORRADOR, &PromptSeguro { precio_publico: "$0".into(), ..seguro_kama_meru() }));
}

/// [28AD-1] Solo reservado, vendido y alquilado cuentan como «no disponible».
#[test]
fn no_disponible_solo_los_tres_estados() {
    for e in ["reservado", "vendido", "alquilado", " vendido "] {
        assert!(no_disponible(e), "{e}");
    }
    for e in ["disponible", "", "Vendido"] {
        assert!(!no_disponible(e), "{e}");
    }
}

fn seguro_similar() -> PromptSeguro {
    PromptSeguro {
        nombre_corto: "Apart. en Torre Alta".to_string(),
        precio_publico: "$82.000".to_string(),
        zona: "Puerto Ordaz, Torre Alta".to_string(),
        ..seguro_kama_meru()
    }
}

/// [28AD-1] Avisa que no está y propone el similar, sin enlace.
/// [GW-16] Sin nombrar el estado y con el sujeto natural («El apartamento en…»).
#[test]
fn no_disponible_propone_similar_y_contacto() {
    let mut f = seguro_kama_meru();
    f.estado = "reservado".to_string();
    let s = seguro_similar();
    let r = borrador_no_disponible(&f, Some(&s));
    assert!(
        r.starts_with("El apartamento en Resd. Kama Merú por el momento no está disponible."),
        "{r}"
    );
    assert!(!r.contains("reservado"), "{r}");
    assert!(!r.contains("sigue disponible"), "{r}");
    assert!(r.contains("opción parecida en Puerto Ordaz, Torre Alta"), "{r}");
    assert!(r.contains("el apartamento en Torre Alta"), "{r}");
    assert!(r.contains("82.000$"), "{r}");
    /* [GW-16] Sin enlace de ningún tipo: ni el de WhatsApp ni el del aviso. */
    assert!(!r.contains("facebook.com/marketplace"), "{r}");
    assert!(!r.contains("http"), "{r}");
    assert!(r.contains(CONTACTO_TEL), "{r}");
    assert!(r.trim_end().ends_with(CIERRE_BORRADOR), "{r}");
    assert!(!r.contains(CONTACTO_WA), "{r}");
}

/// [GW-16] El estado no se nombra: vendido, alquilado y reservado dicen lo mismo.
#[test]
fn no_disponible_no_nombra_el_estado() {
    let mut f = seguro_kama_meru();
    for (estado, prohibido) in [("vendido", "vendid"), ("alquilado", "alquilad"), ("reservado", "reservad")] {
        f.estado = estado.to_string();
        let r = borrador_no_disponible(&f, None);
        assert!(r.contains("por el momento no está disponible"), "{r}");
        assert!(!r.contains(prohibido), "«{estado}» no debe salir: {r}");
    }
}

/// El similar en alquiler cita el canon mensual.
#[test]
fn no_disponible_similar_en_alquiler_cita_canon() {
    let mut f = seguro_kama_meru();
    f.estado = "alquilado".to_string();
    let mut s = seguro_similar();
    s.operacion = "alquiler".to_string();
    let r = borrador_no_disponible(&f, Some(&s));
    assert!(r.contains("82.000$ al mes"), "{r}");
}

/// [28AD-1] Sin similar: ofrece ayuda para buscar y mantiene el contacto; sin nombre, «Este inmueble».
#[test]
fn no_disponible_sin_similar_ofrece_buscar() {
    let mut f = seguro_kama_meru();
    f.estado = "vendido".to_string();
    f.nombre_corto = String::new();
    let r = borrador_no_disponible(&f, None);
    assert!(r.starts_with("Este inmueble por el momento no está disponible."), "{r}");
    assert!(r.contains("buscar otra opción"), "{r}");
    assert!(!r.contains("facebook.com/marketplace"), "{r}");
    assert!(r.trim_end().ends_with(CIERRE_BORRADOR), "{r}");
    assert!(!r.contains(CONTACTO_WA), "{r}");
}

/// [GW-16] El sujeto de la apertura lleva artículo y el tipo completo. El
/// `nombre_corto` guardado no cambia; solo la frase que se lee.
#[test]
fn sujeto_con_articulo_y_tipo_completo() {
    assert_eq!(sujeto_inicial("Casa en Las Orquídeas"), "La casa en Las Orquídeas");
    assert_eq!(sujeto_inicial("Apart. en Resd. Kama Merú"), "El apartamento en Resd. Kama Merú");
    assert_eq!(sujeto_inicial("Townhouse en Villa Tocoma"), "El townhouse en Villa Tocoma");
    assert_eq!(sujeto_inicial("Local en Av. Manuel Piar"), "El local en Av. Manuel Piar");
    assert_eq!(sujeto_inicial("   "), "Este inmueble");
    /* Un tipo que no conocemos se cita tal cual: nunca un artículo inventado. */
    assert_eq!(sujeto_inicial("Galpón en Zona Industrial"), "Galpón en Zona Industrial");
    assert_eq!(sujeto_en_frase("Apart. en Resd. Kama Merú"), "el apartamento en Resd. Kama Merú");
}

/// [GW-16] La apertura sale del molde con el sujeto natural, no con el
/// `nombre_corto` pelado.
#[test]
fn apertura_usa_el_sujeto_natural() {
    let mut s = seguro_kama_meru();
    s.nombre_corto = "Casa en Las Orquídeas".to_string();
    let r = imponer_forma_borrador(FALLBACK_BORRADOR, Some(&s));
    assert!(
        r.contains("La casa en Las Orquídeas sigue disponible en 86.000$ negociable."),
        "{r}"
    );
    s.operacion = "alquiler".to_string();
    let r = imponer_forma_borrador(FALLBACK_BORRADOR, Some(&s));
    assert!(
        r.contains("El canon mensual es de 86.000$ para la casa en Las Orquídeas."),
        "{r}"
    );
}

/// [GW-16] El sujeto que escribe la IA (o una fila vieja de caché) también se
/// reescribe, y aplicarlo dos veces no lo estropea.
#[test]
fn el_sujeto_de_la_ia_se_reescribe() {
    let ia = "Casa en Las Orquídeas sigue disponible en 43.000$ negociable.";
    let r = imponer_forma_borrador(ia, None);
    assert!(r.starts_with("Casa en Las Orquídeas sigue disponible"), "{r}");
    let reescrito = normalizar_sujeto(&r, "Casa en Las Orquídeas");
    assert!(
        reescrito.starts_with("La casa en Las Orquídeas sigue disponible"),
        "{reescrito}"
    );
    assert_eq!(normalizar_sujeto(&reescrito, "Casa en Las Orquídeas"), reescrito);
}

/// [28AA-cache] La reserva de la ficha lleva el precio; sin ficha, el fallback.
#[test]
fn reserva_sin_nombre_lleva_el_precio() {
    let mut s = seguro_kama_meru();
    s.nombre_corto = String::new();
    let r = borrador_de_reserva(Some(&s));
    assert!(r.contains("86.000$"), "{r}");
    assert_eq!(borrador_de_reserva(None), borrador_minimo(None));
}
