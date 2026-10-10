#![cfg(test)]
//! Puntaje y normalización del título y aviso conocido.

use super::*;

#[test]
fn titulo_normaliza_tildes_caja_y_ruido() {
    assert_eq!(
        normalizar_titulo("VEF0 Casa en venta en Riberas del Caroní, Puerto Ordaz"),
        "vef0 casa en venta en riberas del caroni puerto ordaz"
    );
    assert_eq!(normalizar_titulo("  "), "");
}

#[test]
fn puntaje_titulo_directo_con_prefijo_de_precio() {
    /* Caso andreina 08AA-10: el título trae `VEF0` (precio 0 en
     * Facebook) y aun así empareja con la ficha del catálogo. */
    let (directo, _, _) = puntaje_titulo(
        "VEF0 casa en venta en riberas del caroní, puerto ordaz",
        "Casa en venta en Riberas del Caroní",
    );
    assert!(directo);
}

#[test]
fn puntaje_titulo_no_confunde_avisos_genericos() {
    /* Mismo negocio, distinta zona: sin palabra distintiva no hay
     * emparejamiento (un precio ajeno es peor que el dodge). */
    let (directo, solape, distintivo) = puntaje_titulo(
        "casa en venta en arivana",
        "Casa en venta en Riberas del Caroní",
    );
    assert!(!directo);
    assert!(solape < 3 || distintivo < 1);
    /* Título suelto de 1 palabra jamás es directo. */
    assert!(!puntaje_titulo("apto precioso apTO", "apto").0);
}

/* [09AA-21] `aviso_conocido` del panel: ID exacto vinculado, título que
 * empareja, huérfano que no empareja, y empate entre dos fichas que no
 * reclama a ninguna (mismo criterio que `ficha_por_titulo`).
 * [09AA-23] Los vínculos ahora son mapa ID→título y se verifica además
 * que el título devuelto es el de la ficha emparejada.
 * [09AA-24] Candidatos y vínculos viajan con alias: el hilo puede nombrar
 * cualquiera de los nombres, pero el vinculado es siempre el canónico.
 * Testigo Río Aro: el hilo salazar nombra el alias y empareja Caroní.
 * [09AA-28] El vínculo trae también el id (resuelve la portada): ambas
 * ramas (ID exacto y título) deben devolver el id de la ficha, no solo
 * el título. */
#[test]
fn aviso_conocido_id_titulo_huerfano_y_empate() {
    use std::collections::HashMap;
    type Candidatos = Vec<(Uuid, String, Vec<String>)>;
    let conocido = |hilo: &str, candidatos: &Candidatos, vinculos: &VinculosAviso| {
        titulo_vinculado_del_hilo(hilo, candidatos, vinculos).is_some()
    };
    let id = Uuid::new_v4();
    let titulo_riberas = "Casa en venta en Riberas del Caroní".to_string();
    let candidatos: Candidatos = vec![(id, titulo_riberas.clone(), Vec::new())];
    let vinculos: VinculosAviso = [(
        "123456789012345".to_string(),
        (id, titulo_riberas.clone(), Vec::new()),
    )]
    .into_iter()
    .collect();
    assert_eq!(
        titulo_vinculado_del_hilo("tina|123456789012345", &candidatos, &vinculos),
        Some((id, titulo_riberas.clone()))
    );
    assert!(conocido("tina|123456789012345", &candidatos, &vinculos));
    assert!(!conocido("tina|999999999999999", &candidatos, &vinculos));
    assert_eq!(
        titulo_vinculado_del_hilo(
            "tina|VEF0 casa en venta en riberas del caroní, puerto ordaz",
            &candidatos,
            &vinculos
        ),
        Some((id, titulo_riberas.clone()))
    );
    assert!(!conocido(
        "tina|casa en venta en arivana",
        &candidatos,
        &vinculos
    ));
    assert!(!conocido("sin-separador", &candidatos, &vinculos));
    let empatados: Candidatos = vec![
        (
            Uuid::new_v4(),
            "Casa en venta en Riberas del Caroní Norte".to_string(),
            Vec::new(),
        ),
        (
            Uuid::new_v4(),
            "Casa en venta en Riberas del Caroní Sur".to_string(),
            Vec::new(),
        ),
    ];
    assert!(!conocido(
        "tina|casa en venta en riberas del caroní",
        &empatados,
        &HashMap::new()
    ));
    /* Testigo Río Aro [09AA-24]: la ficha Caroní Plaza declara el alias y
     * el hilo salazar —que nombra el alias— empareja con el canónico. */
    let titulo_caroni = "Apartamento en Caroní Plaza".to_string();
    let id_caroni = Uuid::new_v4();
    let candidatos_alias: Candidatos = vec![(
        id_caroni,
        titulo_caroni.clone(),
        vec!["Apartamento en Río Aro Plaza".to_string()],
    )];
    assert_eq!(
        titulo_vinculado_del_hilo(
            "salazar|VEF0 apartamento residencias rio aro plaza puerto ordaz",
            &candidatos_alias,
            &HashMap::new()
        ),
        Some((id_caroni, titulo_caroni.clone()))
    );
    /* Sin el alias declarado, el mismo hilo sigue huérfano (calibrado). */
    let candidatos_sin_alias: Candidatos = vec![(Uuid::new_v4(), titulo_caroni, Vec::new())];
    assert_eq!(
        titulo_vinculado_del_hilo(
            "salazar|VEF0 apartamento residencias rio aro plaza puerto ordaz",
            &candidatos_sin_alias,
            &vinculos
        ),
        None
    );
}

/* [09AA-24] El alias puntúa igual que el canónico: el mejor de los
 * nombres gana. El alias Río Aro no es substring del hilo (el hilo trae
 * "residencias" donde el alias trae "en"), así que empareja por solape
 * con distintivas — el mismo camino que ya usa `titulo_vinculado`. */
#[test]
fn puntaje_alias_igual_que_canonico_y_mejor_gana() {
    let alias = vec!["Apartamento en Río Aro Plaza".to_string()];
    let (directo, solape, distintivo) = mejor_puntaje_con_alias(
        "VEF0 apartamento residencias rio aro plaza puerto ordaz",
        "Apartamento en Caroní Plaza",
        &alias,
    );
    assert!(!directo, "el alias no es substring del hilo");
    assert!(
        solape >= 3 && distintivo >= 1,
        "el alias empareja por solape con distintivas aunque el canónico no"
    );
    let (directo_canonico, _, _) = mejor_puntaje_con_alias(
        "apartamento en caroní plaza",
        "Apartamento en Caroní Plaza",
        &alias,
    );
    assert!(directo_canonico);
    let (directo_ninguno, solape, distintivo) = mejor_puntaje_con_alias(
        "casa en venta en arivana",
        "Apartamento en Caroní Plaza",
        &alias,
    );
    assert!(!directo_ninguno);
    assert!(solape < 3 || distintivo < 1);
}
