//! Comparación de tres vías entre el inmueble local, el del servidor y la
//! última base en que coincidieron. Pura (sin BD ni red): se prueba con una
//! tabla de verdad.
//!
//! Cada campo huellado y cada foto es un «elemento» con clave propia
//! (`titulo`, `foto:original:3`...). Para un elemento con local `L`, servidor
//! `R` y base `B` (`None` = ausente):
//! - `L == R` → coincide.
//! - `B == R` → solo cambió el local (va por delante).
//! - `B == L` → solo cambió el servidor.
//! - ni uno ni otro → cambiaron los dos.
//!
//! Sin base (nunca coincidieron) no se puede saber quién cambió: se estima con
//! `updated_at` y se marca `estimado`.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use chrono::Duration;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

use super::huella::{FotoHuella, InmuebleHuella};

/// Margen bajo el cual dos `updated_at` se consideran simultáneos.
const MARGEN_ESTIMACION_SEG: i64 = 1;

/// Estado de un inmueble respecto al servidor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Estado {
    Igual,
    LocalAdelante,
    ServidorAdelante,
    /// Hay elementos cambiados en cada lado (o el mismo en ambos).
    Ambos,
    /// Difieren y no hay base ni diferencia de `updated_at` que oriente.
    Diferente,
    SoloLocal,
    /// Faltan huellas de fotos por calcular: no se puede afirmar «igual».
    Calculando,
}

/// Quién cambió un elemento concreto.
// `Lado` choca con `marketplace::burbujas::Lado` en el esquema OpenAPI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = SyncLado)]
pub enum Lado {
    Local,
    Servidor,
    Ambos,
    /// Sin base: se sabe que difiere pero no quién cambió.
    Indeterminado,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Diferencia {
    /// Nombre del campo o `foto:<origen>:<orden>`.
    pub clave: String,
    pub lado: Lado,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Resultado {
    pub estado: Estado,
    /// `true` si la dirección sale de `updated_at` por no haber base.
    pub estimado: bool,
    pub diferencias: Vec<Diferencia>,
}

/// Último contenido en que local y servidor coincidieron (`sync_base`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Base {
    pub campos: BTreeMap<String, String>,
    pub fotos: Vec<FotoHuella>,
}

impl Base {
    #[must_use]
    pub fn de(h: &InmuebleHuella) -> Self {
        Self {
            campos: h.campos.clone(),
            fotos: h.fotos.clone(),
        }
    }

    /// Reconstruye la base desde las columnas JSON de `sync_base`. `None` si
    /// el contenido ya no encaja con el tipo (la base se trata como ausente).
    #[must_use]
    pub fn desde_json(campos: Value, fotos: Value) -> Option<Self> {
        Some(Self {
            campos: serde_json::from_value(campos).ok()?,
            fotos: serde_json::from_value(fotos).ok()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Item {
    Valor(String),
    /// La foto existe pero su huella aún no se calculó.
    Pendiente,
}

#[must_use]
pub fn clave_foto(origen: &str, orden: i32) -> String {
    format!("foto:{origen}:{orden}")
}

/// Elementos comparables. Las `mejorada` solo cuentan como presentes: el
/// servidor las recodifica (JPG q70) y sus bytes difieren por diseño.
fn items(campos: &BTreeMap<String, String>, fotos: &[FotoHuella]) -> BTreeMap<String, Item> {
    let mut m: BTreeMap<String, Item> = campos
        .iter()
        .map(|(k, v)| (k.clone(), Item::Valor(v.clone())))
        .collect();
    for f in fotos {
        let valor = if f.origen == "mejorada" {
            Item::Valor("presente".into())
        } else {
            f.sha256.clone().map_or(Item::Pendiente, Item::Valor)
        };
        m.insert(clave_foto(&f.origen, f.orden), valor);
    }
    m
}

/// Compara un inmueble presente en los dos entornos.
#[must_use]
pub fn comparar(local: &InmuebleHuella, remoto: &InmuebleHuella, base: Option<&Base>) -> Resultado {
    let li = items(&local.campos, &local.fotos);
    let ri = items(&remoto.campos, &remoto.fotos);
    let bi = base.map(|b| items(&b.campos, &b.fotos));
    let claves: BTreeSet<&String> = li.keys().chain(ri.keys()).collect();

    let mut diferencias = Vec::new();
    let mut pendientes = false;
    for clave in claves {
        let (l, r) = (li.get(clave), ri.get(clave));
        // Con la huella sin calcular en un lado y la foto presente en el otro,
        // la igualdad es desconocida: no se afirma ni se niega.
        if matches!((l, r), (Some(a), Some(b)) if *a == Item::Pendiente || *b == Item::Pendiente) {
            pendientes = true;
            continue;
        }
        if l == r {
            continue;
        }
        let lado = match &bi {
            None => Lado::Indeterminado,
            Some(b) => {
                let bv = b.get(clave);
                if bv == r {
                    Lado::Local
                } else if bv == l {
                    Lado::Servidor
                } else {
                    Lado::Ambos
                }
            }
        };
        diferencias.push(Diferencia {
            clave: clave.clone(),
            lado,
        });
    }

    if diferencias.is_empty() {
        let estado = if pendientes {
            Estado::Calculando
        } else {
            Estado::Igual
        };
        return Resultado {
            estado,
            estimado: false,
            diferencias,
        };
    }

    if base.is_some() {
        let hay = |lado: Lado| diferencias.iter().any(|d| d.lado == lado);
        let estado = if hay(Lado::Ambos) || (hay(Lado::Local) && hay(Lado::Servidor)) {
            Estado::Ambos
        } else if hay(Lado::Local) {
            Estado::LocalAdelante
        } else {
            Estado::ServidorAdelante
        };
        return Resultado {
            estado,
            estimado: false,
            diferencias,
        };
    }

    // Sin base: `updated_at` orienta, pero es solo una estimación.
    let margen = Duration::seconds(MARGEN_ESTIMACION_SEG);
    let estado = if local.updated_at > remoto.updated_at + margen {
        Estado::LocalAdelante
    } else if remoto.updated_at > local.updated_at + margen {
        Estado::ServidorAdelante
    } else {
        Estado::Diferente
    };
    Resultado {
        estado,
        estimado: true,
        diferencias,
    }
}

/// Inmueble que solo existe en el servidor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct SoloServidor {
    pub slug: String,
    pub titulo: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Entrada {
    pub slug: String,
    pub resultado: Resultado,
}

/// Resultado global de comparar todos los inmuebles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Comparacion {
    pub inmuebles: Vec<Entrada>,
    pub solo_servidor: Vec<SoloServidor>,
}

/// Compara todo el catálogo por `slug`. El orden de `inmuebles` sigue al local.
#[must_use]
pub fn comparar_todo<S: std::hash::BuildHasher>(
    locales: &[InmuebleHuella],
    remotos: &[InmuebleHuella],
    bases: &HashMap<String, Base, S>,
) -> Comparacion {
    let por_slug: HashMap<&str, &InmuebleHuella> =
        remotos.iter().map(|r| (r.slug.as_str(), r)).collect();
    let inmuebles = locales
        .iter()
        .map(|l| {
            let resultado = match por_slug.get(l.slug.as_str()) {
                Some(r) => comparar(l, r, bases.get(&l.slug)),
                None => Resultado {
                    estado: Estado::SoloLocal,
                    estimado: false,
                    diferencias: Vec::new(),
                },
            };
            Entrada {
                slug: l.slug.clone(),
                resultado,
            }
        })
        .collect();
    let slugs_locales: BTreeSet<&str> = locales.iter().map(|l| l.slug.as_str()).collect();
    let solo_servidor = remotos
        .iter()
        .filter(|r| !slugs_locales.contains(r.slug.as_str()))
        .map(|r| SoloServidor {
            slug: r.slug.clone(),
            titulo: r.titulo.clone(),
        })
        .collect();
    Comparacion {
        inmuebles,
        solo_servidor,
    }
}

#[cfg(test)]
mod pruebas {
    use chrono::{TimeZone, Utc};
    use uuid::Uuid;

    use super::*;

    fn h(campos: &[(&str, &str)], fotos: Vec<FotoHuella>, seg: i64) -> InmuebleHuella {
        InmuebleHuella {
            slug: "apto".into(),
            id: Uuid::nil(),
            titulo: "Apto".into(),
            updated_at: Utc.timestamp_opt(1_800_000_000 + seg, 0).unwrap(),
            alias_titulos: vec![],
            campos: campos
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect(),
            fotos,
        }
    }

    fn foto(origen: &str, orden: i32, sha: Option<&str>) -> FotoHuella {
        FotoHuella {
            origen: origen.into(),
            orden,
            sha256: sha.map(str::to_string),
            bytes: sha.map(|_| 10),
        }
    }

    fn claves(r: &Resultado) -> Vec<(&str, Lado)> {
        r.diferencias
            .iter()
            .map(|d| (d.clave.as_str(), d.lado))
            .collect()
    }

    #[test]
    fn iguales_sin_base() {
        let a = h(&[("titulo", "x")], vec![foto("original", 0, Some("a"))], 0);
        let r = comparar(&a, &a.clone(), None);
        assert_eq!(r.estado, Estado::Igual);
        assert!(!r.estimado && r.diferencias.is_empty());
    }

    #[test]
    fn con_base_cambia_solo_el_local() {
        let base = Base::de(&h(&[("titulo", "x"), ("precio", "1")], vec![], 0));
        let local = h(&[("titulo", "y"), ("precio", "1")], vec![], 50);
        let remoto = h(&[("titulo", "x"), ("precio", "1")], vec![], 0);
        let r = comparar(&local, &remoto, Some(&base));
        assert_eq!(r.estado, Estado::LocalAdelante);
        assert!(!r.estimado);
        assert_eq!(claves(&r), vec![("titulo", Lado::Local)]);
    }

    #[test]
    fn con_base_cambia_solo_el_servidor_aunque_el_local_tenga_fecha_mayor() {
        let base = Base::de(&h(&[("titulo", "x")], vec![], 0));
        let local = h(&[("titulo", "x")], vec![], 900);
        let remoto = h(&[("titulo", "z")], vec![], 10);
        let r = comparar(&local, &remoto, Some(&base));
        assert_eq!(r.estado, Estado::ServidorAdelante);
        assert_eq!(claves(&r), vec![("titulo", Lado::Servidor)]);
    }

    #[test]
    fn con_base_campos_distintos_en_cada_lado_son_ambos() {
        let base = Base::de(&h(&[("titulo", "x"), ("precio", "1")], vec![], 0));
        let local = h(&[("titulo", "y"), ("precio", "1")], vec![], 5);
        let remoto = h(&[("titulo", "x"), ("precio", "2")], vec![], 6);
        let r = comparar(&local, &remoto, Some(&base));
        assert_eq!(r.estado, Estado::Ambos);
        assert_eq!(
            claves(&r),
            vec![("precio", Lado::Servidor), ("titulo", Lado::Local)]
        );
    }

    #[test]
    fn con_base_el_mismo_campo_cambiado_distinto_en_ambos_es_ambos() {
        let base = Base::de(&h(&[("titulo", "x")], vec![], 0));
        let r = comparar(
            &h(&[("titulo", "y")], vec![], 1),
            &h(&[("titulo", "z")], vec![], 2),
            Some(&base),
        );
        assert_eq!(r.estado, Estado::Ambos);
        assert_eq!(claves(&r), vec![("titulo", Lado::Ambos)]);
    }

    #[test]
    fn sin_base_orienta_updated_at_y_se_marca_estimado() {
        let a = h(&[("titulo", "x")], vec![], 100);
        let b = h(&[("titulo", "y")], vec![], 0);
        let r = comparar(&a, &b, None);
        assert_eq!((r.estado, r.estimado), (Estado::LocalAdelante, true));
        assert_eq!(claves(&r), vec![("titulo", Lado::Indeterminado)]);
        let r = comparar(&b, &a, None);
        assert_eq!((r.estado, r.estimado), (Estado::ServidorAdelante, true));
    }

    #[test]
    fn sin_base_y_fechas_simultaneas_es_diferente() {
        let a = h(&[("titulo", "x")], vec![], 0);
        let b = h(&[("titulo", "y")], vec![], 1);
        let r = comparar(&a, &b, None);
        assert_eq!((r.estado, r.estimado), (Estado::Diferente, true));
    }

    #[test]
    fn foto_nueva_en_local_con_base_va_adelante_el_local() {
        let base = Base::de(&h(&[], vec![foto("original", 0, Some("a"))], 0));
        let local = h(
            &[],
            vec![
                foto("original", 0, Some("a")),
                foto("original", 1, Some("b")),
            ],
            5,
        );
        let remoto = h(&[], vec![foto("original", 0, Some("a"))], 0);
        let r = comparar(&local, &remoto, Some(&base));
        assert_eq!(r.estado, Estado::LocalAdelante);
        assert_eq!(claves(&r), vec![("foto:original:1", Lado::Local)]);
    }

    #[test]
    fn foto_borrada_en_el_servidor_con_base_va_adelante_el_servidor() {
        let fotos = vec![
            foto("original", 0, Some("a")),
            foto("original", 1, Some("b")),
        ];
        let base = Base::de(&h(&[], fotos.clone(), 0));
        let local = h(&[], fotos, 0);
        let remoto = h(&[], vec![foto("original", 0, Some("a"))], 5);
        let r = comparar(&local, &remoto, Some(&base));
        assert_eq!(r.estado, Estado::ServidorAdelante);
        assert_eq!(claves(&r), vec![("foto:original:1", Lado::Servidor)]);
    }

    #[test]
    fn bytes_distintos_de_una_original_difieren_pero_los_de_mejorada_no() {
        let a = h(&[], vec![foto("original", 0, Some("a"))], 0);
        let b = h(&[], vec![foto("original", 0, Some("b"))], 0);
        assert_ne!(comparar(&a, &b, None).estado, Estado::Igual);
        let m1 = h(&[], vec![foto("mejorada", 0, Some("a"))], 0);
        let m2 = h(&[], vec![foto("mejorada", 0, Some("b"))], 0);
        assert_eq!(comparar(&m1, &m2, None).estado, Estado::Igual);
    }

    #[test]
    fn mejorada_que_falta_en_un_lado_si_cuenta() {
        let a = h(&[], vec![foto("mejorada", 0, Some("a"))], 0);
        let b = h(&[], vec![], 0);
        assert_ne!(comparar(&a, &b, None).estado, Estado::Igual);
    }

    #[test]
    fn huella_pendiente_no_afirma_igualdad() {
        let a = h(&[("titulo", "x")], vec![foto("original", 0, None)], 0);
        let b = h(&[("titulo", "x")], vec![foto("original", 0, Some("a"))], 0);
        assert_eq!(comparar(&a, &b, None).estado, Estado::Calculando);
        assert_eq!(comparar(&b, &a, None).estado, Estado::Calculando);
    }

    #[test]
    fn huella_pendiente_no_oculta_una_diferencia_ya_cierta() {
        let a = h(&[("titulo", "x")], vec![foto("original", 0, None)], 0);
        let b = h(&[("titulo", "y")], vec![foto("original", 0, Some("a"))], 9);
        let r = comparar(&a, &b, None);
        assert_eq!(r.estado, Estado::ServidorAdelante);
        assert_eq!(claves(&r), vec![("titulo", Lado::Indeterminado)]);
    }

    #[test]
    fn datos_informativos_no_cambian_el_estado() {
        let mut a = h(&[("titulo", "x")], vec![], 0);
        let mut b = a.clone();
        a.alias_titulos = vec!["alias".into()];
        b.id = Uuid::new_v4();
        b.titulo = "otro titulo para mostrar".into();
        assert_eq!(comparar(&a, &b, None).estado, Estado::Igual);
    }

    /// [18AA-3] El vínculo dejó de ser informativo: si un lado lo tiene y el otro
    /// no, el inmueble no está igual y la diferencia se puede resolver desde el
    /// panel como cualquier campo.
    #[test]
    fn el_vinculo_si_cambia_el_estado() {
        let mut a = h(&[("titulo", "x"), ("marketplace_id", "123")], vec![], 0);
        let mut b = h(&[("titulo", "x"), ("marketplace_id", "")], vec![], 0);
        b.updated_at = a.updated_at;
        let r = comparar(&a, &b, None);
        assert_eq!(r.estado, Estado::Diferente);
        assert_eq!(claves(&r), vec![("marketplace_id", Lado::Indeterminado)]);
        // Y sin vínculo en ninguno de los dos, coinciden.
        b.campos
            .insert("marketplace_id".into(), a.campos["marketplace_id"].clone());
        assert_eq!(comparar(&a, &b, None).estado, Estado::Igual);
    }

    #[test]
    fn comparar_todo_empareja_por_slug_y_detecta_los_solitarios() {
        let mut solo_local = h(&[("titulo", "x")], vec![], 0);
        solo_local.slug = "solo-local".into();
        let mut solo_srv = h(&[("titulo", "x")], vec![], 0);
        solo_srv.slug = "solo-servidor".into();
        solo_srv.titulo = "Solo en el servidor".into();
        let comun_l = h(&[("titulo", "x")], vec![], 0);
        let comun_r = h(&[("titulo", "x")], vec![], 0);
        let c = comparar_todo(
            &[comun_l, solo_local],
            &[comun_r, solo_srv],
            &HashMap::new(),
        );
        let estados: Vec<_> = c
            .inmuebles
            .iter()
            .map(|e| (e.slug.as_str(), e.resultado.estado))
            .collect();
        assert_eq!(
            estados,
            vec![("apto", Estado::Igual), ("solo-local", Estado::SoloLocal)]
        );
        assert_eq!(
            c.solo_servidor,
            vec![SoloServidor {
                slug: "solo-servidor".into(),
                titulo: "Solo en el servidor".into()
            }]
        );
    }
}
