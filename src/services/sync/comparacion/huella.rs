//! Huellas deterministas de un inmueble para compararlo entre entornos.
//!
//! Los valores privados (`precio_minimo`, `extras`) viajan solo como huella.
//! Ambos entornos corren este mismo código, así que la canonización es la
//! misma a los dos lados; `VERSION` cambia si cambia el contrato de la huella
//! (un servidor con otra versión se informa como «sin soporte», no como
//! diferencia).

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::models::InmuebleRow;
use crate::repositories::sync::SyncRepository;

pub const VERSION: u32 = 1;

/// Marca persistida en `fotos.sha256` cuando el archivo ya no existe en el
/// volumen: evita reintentar la lectura en cada consulta.
pub const SIN_ARCHIVO: &str = "sin-archivo";

/// Campos huellados, en el orden en que se muestran. `marketplace_id` y
/// `alias_titulos` no están: son datos propios de cada entorno (informativos).
pub const CAMPOS: &[&str] = &[
    "titulo",
    "descripcion",
    "ubicacion",
    "puestos",
    "residencia",
    "precio",
    "tipo",
    "operacion",
    "habitaciones",
    "banos",
    "metros",
    "metros_terreno",
    "estado",
    "publicado",
    "copy_corta",
    "copy_larga",
    "copy_modelo",
    "receta",
    "extras",
    "precio_minimo",
];

#[must_use]
pub fn sha256_hex(datos: &[u8]) -> String {
    format!("{:x}", Sha256::digest(datos))
}

/// Texto canónico: saltos de línea unificados y sin espacios en los extremos.
#[must_use]
pub fn canon_texto(s: &str) -> String {
    s.replace("\r\n", "\n")
        .replace('\r', "\n")
        .trim()
        .to_string()
}

/// Número canónico a 2 decimales (el precio y los metros no deben diferir por
/// ruido de coma flotante). `+ 0.0` convierte `-0.0` en `0.0`.
#[must_use]
pub fn canon_num(n: f64) -> String {
    let redondeado = (n * 100.0).round() / 100.0;
    format!("{:.2}", redondeado + 0.0)
}

/// JSON canónico: claves ordenadas y textos canonizados. No depende de que
/// `serde_json` conserve o no el orden de inserción.
#[must_use]
pub fn canon_json(v: &Value) -> String {
    match v {
        Value::Object(mapa) => {
            let mut claves: Vec<&String> = mapa.keys().collect();
            claves.sort();
            let partes: Vec<String> = claves
                .iter()
                .map(|k| format!("{}:{}", Value::String((*k).clone()), canon_json(&mapa[*k])))
                .collect();
            format!("{{{}}}", partes.join(","))
        }
        Value::Array(items) => {
            let partes: Vec<String> = items.iter().map(canon_json).collect();
            format!("[{}]", partes.join(","))
        }
        Value::String(s) => Value::String(canon_texto(s)).to_string(),
        otro => otro.to_string(),
    }
}

/// Huella de una foto: `(origen, orden)` identifican la foto entre entornos
/// (los `id` y los `storage_key` difieren). `sha256 = None` = aún sin calcular.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct FotoHuella {
    pub origen: String,
    pub orden: i32,
    pub sha256: Option<String>,
    pub bytes: Option<i32>,
}

/// Huella completa de un inmueble.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InmuebleHuella {
    pub slug: String,
    pub id: Uuid,
    pub titulo: String,
    pub updated_at: DateTime<Utc>,
    /// Informativos: propios de cada entorno, nunca cambian el estado.
    pub marketplace_id: Option<String>,
    pub alias_titulos: Vec<String>,
    /// Campo → sha256 de su valor canónico.
    pub campos: BTreeMap<String, String>,
    pub fotos: Vec<FotoHuella>,
}

/// Respuesta de `GET /api/admin/sync/manifiesto`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Manifiesto {
    pub version: u32,
    pub generado_en: DateTime<Utc>,
    /// `true` si hubo más inmuebles que el tope y se cortó la lista.
    pub truncado: bool,
    /// Fotos originales cuya huella aún no se ha calculado (relleno perezoso).
    pub fotos_pendientes: i64,
    pub inmuebles: Vec<InmuebleHuella>,
}

/// Valor canónico (texto) de cada campo huellado de una fila.
#[must_use]
pub fn valores_canonicos(f: &InmuebleRow) -> BTreeMap<&'static str, String> {
    let texto_opc = |v: &Option<String>| v.as_deref().map(canon_texto).unwrap_or_default();
    let receta = f
        .receta
        .as_ref()
        .map(|r| canon_json(&serde_json::to_value(&r.0).unwrap_or(Value::Null)))
        .unwrap_or_default();
    let mut m = BTreeMap::new();
    m.insert("titulo", canon_texto(&f.titulo));
    m.insert("descripcion", canon_texto(&f.descripcion));
    m.insert("ubicacion", canon_texto(&f.ubicacion));
    m.insert("puestos", f.puestos.to_string());
    m.insert("residencia", canon_texto(&f.residencia));
    m.insert("precio", canon_num(f.precio));
    m.insert("tipo", canon_texto(&f.tipo));
    m.insert("operacion", canon_texto(&f.operacion));
    m.insert("habitaciones", f.habitaciones.to_string());
    m.insert("banos", f.banos.to_string());
    m.insert("metros", canon_num(f.metros));
    m.insert("metros_terreno", canon_num(f.metros_terreno));
    m.insert("estado", canon_texto(&f.estado));
    m.insert("publicado", f.publicado.to_string());
    m.insert("copy_corta", texto_opc(&f.copy_corta));
    m.insert("copy_larga", texto_opc(&f.copy_larga));
    m.insert("copy_modelo", texto_opc(&f.copy_modelo));
    m.insert("receta", receta);
    m.insert("extras", canon_json(&f.extras.0));
    m.insert(
        "precio_minimo",
        f.precio_minimo.map(canon_num).unwrap_or_default(),
    );
    m
}

/// Huella de una fila con sus fotos (ya ordenadas por `(orden, origen)`).
#[must_use]
pub fn huella_de(f: &InmuebleRow, fotos: Vec<FotoHuella>) -> InmuebleHuella {
    let campos = valores_canonicos(f)
        .into_iter()
        .map(|(k, v)| (k.to_string(), sha256_hex(v.as_bytes())))
        .collect();
    InmuebleHuella {
        slug: f.slug.clone(),
        id: f.id,
        titulo: f.titulo.clone(),
        updated_at: f.updated_at,
        marketplace_id: f.marketplace_id.clone(),
        alias_titulos: f.alias_titulos.clone(),
        campos,
        fotos,
    }
}

/// Huella de una foto recién subida (los bytes ya están en memoria). Si falla,
/// el relleno perezoso del manifiesto la calcula en la siguiente pasada.
pub async fn registrar_huella_subida(pool: &PgPool, storage_key: &str, bytes: &[u8]) {
    if let Err(e) = SyncRepository::guardar_huella_por_clave(
        pool,
        storage_key,
        &sha256_hex(bytes),
        i32::try_from(bytes.len()).ok(),
    )
    .await
    {
        tracing::warn!("No se pudo guardar la huella de {storage_key}: {e}");
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use serde_json::json;

    fn fila() -> InmuebleRow {
        InmuebleRow {
            id: Uuid::nil(),
            titulo: "Apto Centro".into(),
            descripcion: "Linea 1\nLinea 2".into(),
            ubicacion: "Centro".into(),
            puestos: 1,
            residencia: String::new(),
            precio: 45000.0,
            tipo: "apartamento".into(),
            operacion: "venta".into(),
            habitaciones: 2,
            banos: 1,
            metros: 60.0,
            metros_terreno: 0.0,
            estado: "disponible".into(),
            publicado: true,
            slug: "apto-centro".into(),
            copy_corta: None,
            copy_larga: None,
            copy_modelo: None,
            copy_actualizada_en: None,
            receta: None,
            extras: sqlx::types::Json(json!({})),
            precio_minimo: None,
            marketplace_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            alias_titulos: vec![],
        }
    }

    #[test]
    fn canon_texto_unifica_saltos_y_recorta() {
        assert_eq!(canon_texto("  a\r\nb\rc \n"), "a\nb\nc");
    }

    #[test]
    fn canon_num_redondea_y_no_distingue_cero_negativo() {
        assert_eq!(canon_num(45000.0), "45000.00");
        assert_eq!(canon_num(45000.004), "45000.00");
        assert_eq!(canon_num(-0.0), "0.00");
        assert_eq!(canon_num(0.1 + 0.2), "0.30");
    }

    #[test]
    fn canon_json_ignora_el_orden_de_claves() {
        let a = json!({"b": 1, "a": {"y": "x ", "x": [1, 2]}});
        let b = json!({"a": {"x": [1, 2], "y": "x"}, "b": 1});
        assert_eq!(canon_json(&a), canon_json(&b));
        assert_ne!(canon_json(&a), canon_json(&json!({"b": 2, "a": {}})));
    }

    #[test]
    fn huella_estable_y_sensible_a_cada_campo() {
        let a = huella_de(&fila(), vec![]);
        let b = huella_de(&fila(), vec![]);
        assert_eq!(a.campos, b.campos);
        assert_eq!(a.campos.len(), CAMPOS.len());
        for campo in CAMPOS {
            assert!(a.campos.contains_key(*campo), "falta {campo}");
        }
        let mut otra = fila();
        otra.descripcion = "Linea 1\nLinea 3".into();
        let c = huella_de(&otra, vec![]);
        let distintos: Vec<_> = a
            .campos
            .iter()
            .filter(|(k, v)| c.campos[*k] != **v)
            .map(|(k, _)| k.as_str())
            .collect();
        assert_eq!(distintos, vec!["descripcion"]);
    }

    #[test]
    fn null_y_vacio_son_lo_mismo_y_los_espacios_no_cuentan() {
        let mut con_vacio = fila();
        con_vacio.copy_corta = Some("  ".into());
        let mut con_crlf = fila();
        con_crlf.descripcion = "Linea 1\r\nLinea 2 ".into();
        let base = huella_de(&fila(), vec![]);
        assert_eq!(huella_de(&con_vacio, vec![]).campos, base.campos);
        assert_eq!(huella_de(&con_crlf, vec![]).campos, base.campos);
    }

    #[test]
    fn id_fechas_slug_y_datos_propios_no_cambian_la_huella() {
        let mut otra = fila();
        otra.id = Uuid::new_v4();
        otra.created_at = Utc::now();
        otra.updated_at = Utc::now();
        otra.marketplace_id = Some("123".into());
        otra.alias_titulos = vec!["Otro nombre".into()];
        assert_eq!(
            huella_de(&otra, vec![]).campos,
            huella_de(&fila(), vec![]).campos
        );
    }

    #[test]
    fn precio_minimo_y_extras_cuentan_pero_solo_como_huella() {
        let mut con_minimo = fila();
        con_minimo.precio_minimo = Some(40000.0);
        let mut con_extras = fila();
        con_extras.extras = sqlx::types::Json(json!({"piso": "3"}));
        let base = huella_de(&fila(), vec![]);
        let hm = huella_de(&con_minimo, vec![]);
        assert_ne!(hm.campos["precio_minimo"], base.campos["precio_minimo"]);
        assert_ne!(
            huella_de(&con_extras, vec![]).campos["extras"],
            base.campos["extras"]
        );
        // Solo viaja el sha256 (64 hex), nunca el valor privado.
        assert_eq!(hm.campos["precio_minimo"].len(), 64);
        assert_ne!(hm.campos["precio_minimo"], "40000.00");
    }
}
