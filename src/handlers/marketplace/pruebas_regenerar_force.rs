#![cfg(test)]
//! [10AA-17] Regenerar (`force`): el hit de IA se descarta y la fila se pisa;
//! una corrección de la dueña nunca se pisa.

use super::*;
use crate::services::marketplace::{buscar_cache, Coste, Generado};

fn pool_si_hay() -> Option<sqlx::PgPool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy(&url)
        .ok()
}

fn clave_azar() -> String {
    format!(
        "{:x}{:x}",
        uuid::Uuid::new_v4().as_simple(),
        uuid::Uuid::new_v4().as_simple()
    )
}

#[test]
fn aceptar_hit_regenerar_descarta_ia_y_respeta_correccion() {
    let ia = || Some(("texto-ia".to_string(), false));
    let corregida = || Some(("texto de la dueña".to_string(), true));
    assert!(
        aceptar_hit(ia(), false).is_some(),
        "sin force, el hit de IA vale"
    );
    assert!(
        aceptar_hit(ia(), true).is_none(),
        "force descarta el hit de IA"
    );
    assert!(aceptar_hit(corregida(), false).is_some());
    assert!(
        aceptar_hit(corregida(), true).is_some(),
        "force nunca descarta una corrección"
    );
    assert!(aceptar_hit(None, true).is_none());
}

#[tokio::test]
async fn guardar_generado_pisar_reemplaza_la_fila_y_sin_pisar_la_conserva() {
    let Some(pool) = pool_si_hay() else { return };
    let (firma, ph, ch) = (clave_azar(), clave_azar(), clave_azar());
    let foto = FotoHilo {
        thread_id: "hilo-regenerar-force",
        excerpt: "x",
        excerpt_crudo: "crudo-x",
    };
    let cl = ClavesBorrador {
        firma_cache: &firma,
        firma_legacy: None,
        precio_hash: &ph,
        catalog_hash: &ch,
    };
    guardar_cache(
        &pool,
        &firma,
        &ph,
        &ch,
        "texto-viejo",
        &foto,
        Coste::default(),
    )
    .await
    .expect("guarda la fila previa");
    let nuevo = Generado {
        texto: "texto-nuevo".to_string(),
        fuente: "ia".to_string(),
        coste: Coste::default(),
    };

    guardar_generado(&pool, &cl, &foto, &nuevo, None, false)
        .await
        .expect("sin pisar");
    assert_eq!(
        buscar_cache(&pool, &firma, &ph, &ch).await.expect("busca"),
        Some(("texto-viejo".to_string(), false)),
        "sin force, la fila previa no se toca (DO NOTHING)"
    );

    guardar_generado(&pool, &cl, &foto, &nuevo, None, true)
        .await
        .expect("pisando");
    assert_eq!(
        buscar_cache(&pool, &firma, &ph, &ch).await.expect("busca"),
        Some(("texto-nuevo".to_string(), false)),
        "con force, la fila se reemplaza por el texto generado"
    );
}
