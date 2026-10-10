#![cfg(test)]
//! Strip, schema y hash de la ficha que entra al prompt.

use super::*;

/* El strip deja pasar exactamente los 7 campos del allowlist: ni el
 * slug, ni el estado interno, ni la receta viajan al prompt.
 * (`serde_json::Map` ordena claves: se compara ordenado.)
 * [08AA-25] `operacion` viaja (v2): sin ella la IA vendía alquileres. */
#[test]
fn strip_solo_allowlist_siete_campos() {
    let s = strip_ficha_para_prompt(&ficha(), STRIP_VERSION).unwrap();
    let v = serde_json::to_value(&s).unwrap();
    let mut claves: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
    claves.sort_unstable();
    assert_eq!(
        claves,
        vec![
            "descripcion_corta",
            "habitaciones",
            "m2",
            "operacion",
            "precio_publico",
            "titulo",
            "zona"
        ]
    );
    assert_eq!(s.precio_publico, "$43.000");
    assert_eq!(s.operacion, "venta");
    assert_eq!(s.zona, "Puerto Ordaz, Riberas del Caroní");
}

#[test]
fn strip_version_desconocida_se_rechaza() {
    assert!(strip_ficha_para_prompt(&ficha(), "v9").is_err());
}

#[test]
fn precio_agrupa_miles_sin_casts() {
    assert_eq!(precio_publico(43000.0), "$43.000");
    assert_eq!(precio_publico(1_250_000.0), "$1.250.000");
    assert_eq!(precio_publico(900.0), "$900");
}

#[test]
fn schema_valido_pasa_y_cada_campo_malo_falla() {
    assert!(validar_borrador(&pedido()).is_empty());
    let mut malo = pedido();
    malo.thread_id.clear();
    assert!(!validar_borrador(&malo).is_empty());
    let mut malo = pedido();
    malo.firma = "xyz".to_string();
    assert!(!validar_borrador(&malo).is_empty());
    let mut malo = pedido();
    malo.firma_version = "otra".to_string();
    assert!(!validar_borrador(&malo).is_empty());
    let mut malo = pedido();
    malo.lang = "esp".to_string();
    assert!(!validar_borrador(&malo).is_empty());
    let mut malo = pedido();
    malo.excerpt.hora = "2026-10-05T18:00:00Z".to_string();
    assert!(!validar_borrador(&malo).is_empty());
    let mut malo = pedido();
    malo.aviso_id = Some(String::new());
    assert!(!validar_borrador(&malo).is_empty());
}

#[test]
fn hash_ficha_estable_y_hex64() {
    let f = fila_prueba(43_000.0);
    let a = hash_ficha(&f);
    let b = hash_ficha(&fila_prueba(43_000.0));
    assert_eq!(a, b);
    assert!(es_hex64(&a));
}

#[test]
fn hash_ficha_invalida_si_cambia_lo_que_cita() {
    let base = hash_ficha(&fila_prueba(43_000.0));
    let mut f = fila_prueba(45_000.0);
    assert_ne!(hash_ficha(&f), base, "precio distinto debe invalidar");
    f = fila_prueba(43_000.0);
    f.titulo = "Otro título".to_string();
    assert_ne!(hash_ficha(&f), base, "título distinto debe invalidar");
    f = fila_prueba(43_000.0);
    f.estado = "vendido".to_string();
    assert_ne!(hash_ficha(&f), base, "estado distinto debe invalidar");
    /* [08AA-25] La operación condiciona el lenguaje del borrador
     * (venta vs canon mensual): cambiarla invalida la caché. */
    f = fila_prueba(43_000.0);
    f.operacion = "alquiler".to_string();
    assert_ne!(hash_ficha(&f), base, "operación distinta debe invalidar");
}

#[test]
fn hash_ficha_ignora_lo_que_no_entra_al_prompt() {
    let base = hash_ficha(&fila_prueba(43_000.0));
    let mut f = fila_prueba(43_000.0);
    f.copy_corta = Some("Copy marketing".to_string());
    f.extras = sqlx::types::Json(serde_json::json!({"piso": "3"}));
    assert_eq!(hash_ficha(&f), base, "copy/extras no cambian la respuesta");
}

#[test]
fn precio_hash_ata_al_precio_citado() {
    let a = precio_hash_seguro(
        &strip_ficha_para_prompt(&fila_prueba(43_000.0), STRIP_VERSION).unwrap(),
    );
    let b = precio_hash_seguro(
        &strip_ficha_para_prompt(&fila_prueba(45_000.0), STRIP_VERSION).unwrap(),
    );
    assert!(es_hex64(&a));
    assert_ne!(a, b);
}
