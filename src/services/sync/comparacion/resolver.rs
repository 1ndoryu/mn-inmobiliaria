//! Plan de resolución de diferencias de sync [14AA-2]. Pura (sin BD ni red):
//! decide, campo a campo, qué valor queda y en qué lado se escribe. F1 solo
//! calcula el plan y lo identifica con `plan_id`; aplicarlo es F2.
//!
//! Regla por defecto («lo último»): si solo cambió un lado, gana ese lado; si
//! cambiaron los dos o no hay base, gana el inmueble con `updated_at` más
//! reciente (empate → local). «Conservar lo viejo» invierte la elección de ese
//! campo. Un elemento que no existe en el lado elegido se quita del otro.

use std::collections::{BTreeSet, HashMap};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use utoipa::ToSchema;

use super::comparar::{Estado, Lado};
use super::detalle::DiferenciasSync;
use super::huella::InmuebleHuella;

/// De qué lado queda el valor de un elemento.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Eleccion {
    Local,
    Servidor,
}

impl Eleccion {
    fn contraria(self) -> Self {
        match self {
            Self::Local => Self::Servidor,
            Self::Servidor => Self::Local,
        }
    }
}

/// Qué se hace en el lado que no gana.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Accion {
    /// El lado ganador tiene el elemento: se escribe en el otro.
    Escribir,
    /// El lado ganador no lo tiene: se quita del otro.
    Quitar,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct FilaPlan {
    pub clave: String,
    /// Quién cambió, como en la comparación. Con `ambos`/`indeterminado` decide la fecha.
    pub lado: Lado,
    /// Lado cuyo valor queda.
    pub elegido: Eleccion,
    /// Operación en el lado contrario a `elegido`.
    pub accion: Accion,
    /// Valor de cada lado (`None` = no existe allí).
    pub local: Option<String>,
    pub servidor: Option<String>,
    /// Solo fotos: `/uploads/...` en local, URL absoluta en el servidor.
    pub miniatura_local: Option<String>,
    pub miniatura_servidor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct PlanResolucion {
    pub slug: String,
    /// Huella de las operaciones y de la revisión de cada lado. Si cambia algo
    /// en cualquiera de los dos, cambia. `aplicar` (F2) exigirá este mismo id.
    pub plan_id: String,
    pub estado: Estado,
    /// Dirección estimada por `updated_at` (sin base previa).
    pub estimado: bool,
    /// Operaciones que se harían en el local.
    pub en_local: usize,
    /// Operaciones que se harían en el servidor.
    pub en_servidor: usize,
    pub filas: Vec<FilaPlan>,
}

/// Calcula el plan. `conservar_viejo` = claves cuyo lado contrario gana.
/// Las miniaturas van indexadas por clave de foto (`foto:<origen>:<orden>`).
pub fn planificar(
    dif: &DiferenciasSync,
    local: &InmuebleHuella,
    remoto: &InmuebleHuella,
    conservar_viejo: &BTreeSet<String>,
    miniaturas_local: &HashMap<String, String>,
    miniaturas_servidor: &HashMap<String, String>,
) -> Result<PlanResolucion, serde_json::Error> {
    // Empate → local: el mismo criterio en todo el panel.
    let mas_reciente = if local.updated_at >= remoto.updated_at {
        Eleccion::Local
    } else {
        Eleccion::Servidor
    };
    let filas: Vec<FilaPlan> = dif
        .filas
        .iter()
        .map(|f| {
            let por_defecto = match f.lado {
                Lado::Local => Eleccion::Local,
                Lado::Servidor => Eleccion::Servidor,
                Lado::Ambos | Lado::Indeterminado => mas_reciente,
            };
            let elegido = if conservar_viejo.contains(&f.clave) {
                por_defecto.contraria()
            } else {
                por_defecto
            };
            let valor = match elegido {
                Eleccion::Local => &f.local,
                Eleccion::Servidor => &f.servidor,
            };
            FilaPlan {
                clave: f.clave.clone(),
                lado: f.lado,
                elegido,
                accion: if valor.is_some() {
                    Accion::Escribir
                } else {
                    Accion::Quitar
                },
                local: f.local.clone(),
                servidor: f.servidor.clone(),
                miniatura_local: miniaturas_local.get(&f.clave).cloned(),
                miniatura_servidor: miniaturas_servidor.get(&f.clave).cloned(),
            }
        })
        .collect();
    // Cada fila es una operación en el lado contrario a su elección. Las de foto
    // solo cuentan si `aplicar` va a poder hacerlas. Al **local** se le cambia el
    // archivo en su sitio, así que también vale cuando ya tiene otra foto en ese
    // `orden`; al **servidor** no: su API solo sabe borrar (y renumera) o añadir,
    // así que una clave que ya tiene con otro contenido queda pendiente. [F3]
    let aplicable = |f: &&FilaPlan, ganador: Eleccion| -> bool {
        if f.elegido != ganador {
            return false;
        }
        if !f.clave.starts_with("foto:") || f.accion == Accion::Quitar {
            return true;
        }
        // El destino es el lado contrario al ganador. Los dos valores son la
        // descripción de la foto en cada lado, así que «distintos» es justo lo
        // que hay que escribir.
        match ganador {
            Eleccion::Servidor => f.local != f.servidor,
            Eleccion::Local => f.servidor.is_none(),
        }
    };
    let en_local = filas.iter().filter(|f| aplicable(f, Eleccion::Servidor)).count();
    let en_servidor = filas.iter().filter(|f| aplicable(f, Eleccion::Local)).count();
    let plan_id = plan_id(&dif.slug, &filas, local, remoto)?;
    Ok(PlanResolucion {
        slug: dif.slug.clone(),
        plan_id,
        estado: dif.resultado.estado,
        estimado: dif.resultado.estimado,
        en_local,
        en_servidor,
        filas,
    })
}

/// Huella del plan: operaciones + huella completa de cada lado.
fn plan_id(
    slug: &str,
    filas: &[FilaPlan],
    local: &InmuebleHuella,
    remoto: &InmuebleHuella,
) -> Result<String, serde_json::Error> {
    let revision = |h: &InmuebleHuella| -> Result<String, serde_json::Error> {
        Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(h)?)))
    };
    let cuerpo = serde_json::to_vec(&(slug, filas, revision(local)?, revision(remoto)?))?;
    Ok(format!("{:x}", Sha256::digest(cuerpo)))
}

#[cfg(test)]
mod pruebas {
    use std::collections::BTreeMap;

    use chrono::{TimeZone, Utc};
    use uuid::Uuid;

    use super::*;
    use crate::services::sync::comparar::{Diferencia, Resultado};
    use crate::services::sync::detalle::FilaDiferencia;

    fn huella(seg: i64) -> InmuebleHuella {
        InmuebleHuella {
            slug: "apto".into(),
            id: Uuid::nil(),
            titulo: "Apto".into(),
            updated_at: Utc.timestamp_opt(1_800_000_000 + seg, 0).unwrap(),
            alias_titulos: vec![],
            campos: BTreeMap::new(),
            fotos: vec![],
        }
    }

    fn fila(clave: &str, lado: Lado, local: Option<&str>, servidor: Option<&str>) -> FilaDiferencia {
        FilaDiferencia {
            clave: clave.into(),
            lado,
            local: local.map(str::to_string),
            servidor: servidor.map(str::to_string),
        }
    }

    fn dif(filas: Vec<FilaDiferencia>) -> DiferenciasSync {
        DiferenciasSync {
            slug: "apto".into(),
            resultado: Resultado {
                estado: Estado::Ambos,
                estimado: false,
                diferencias: filas
                    .iter()
                    .map(|f| Diferencia {
                        clave: f.clave.clone(),
                        lado: f.lado,
                    })
                    .collect(),
            },
            filas,
        }
    }

    fn plan(d: &DiferenciasSync, l: &InmuebleHuella, r: &InmuebleHuella, conservar: &[&str]) -> PlanResolucion {
        let conservar: BTreeSet<String> = conservar.iter().map(|c| (*c).to_string()).collect();
        planificar(d, l, r, &conservar, &HashMap::new(), &HashMap::new()).unwrap()
    }

    #[test]
    fn por_defecto_gana_el_lado_que_cambio() {
        let d = dif(vec![
            fila("titulo", Lado::Local, Some("nuevo"), Some("viejo")),
            fila("precio", Lado::Servidor, Some("1"), Some("2")),
        ]);
        let p = plan(&d, &huella(0), &huella(0), &[]);
        assert_eq!(p.filas[0].elegido, Eleccion::Local);
        assert_eq!(p.filas[1].elegido, Eleccion::Servidor);
        assert!(p.filas.iter().all(|f| f.accion == Accion::Escribir));
        assert_eq!((p.en_local, p.en_servidor), (1, 1));
    }

    #[test]
    fn ambos_o_indeterminado_gana_el_mas_reciente() {
        let d = dif(vec![
            fila("titulo", Lado::Ambos, Some("a"), Some("b")),
            fila("precio", Lado::Indeterminado, Some("1"), Some("2")),
        ]);
        let todo = |p: &PlanResolucion, e: Eleccion| p.filas.iter().all(|f| f.elegido == e);
        assert!(todo(&plan(&d, &huella(100), &huella(0), &[]), Eleccion::Local));
        assert!(todo(&plan(&d, &huella(0), &huella(100), &[]), Eleccion::Servidor));
        // Empate: gana el local.
        assert!(todo(&plan(&d, &huella(5), &huella(5), &[]), Eleccion::Local));
    }

    #[test]
    fn conservar_viejo_invierte_solo_ese_campo() {
        let d = dif(vec![
            fila("titulo", Lado::Local, Some("nuevo"), Some("viejo")),
            fila("precio", Lado::Servidor, Some("1"), Some("2")),
        ]);
        let p = plan(&d, &huella(0), &huella(0), &["titulo"]);
        assert_eq!(p.filas[0].elegido, Eleccion::Servidor);
        assert_eq!(p.filas[1].elegido, Eleccion::Servidor);
    }

    #[test]
    fn foto_solo_en_un_lado_se_añade_o_se_quita_segun_lo_elegido() {
        let d = dif(vec![fila("foto:original:1", Lado::Local, Some("presente (aaaaaaaa)"), None)]);
        // Lo último es el local: la foto se sube al servidor.
        let nueva = plan(&d, &huella(0), &huella(0), &[]);
        assert_eq!((nueva.filas[0].elegido, nueva.filas[0].accion), (Eleccion::Local, Accion::Escribir));
        assert_eq!((nueva.en_local, nueva.en_servidor), (0, 1));
        // Conservar lo viejo (sin la foto): se quita del local.
        let vieja = plan(&d, &huella(0), &huella(0), &["foto:original:1"]);
        assert_eq!((vieja.filas[0].elegido, vieja.filas[0].accion), (Eleccion::Servidor, Accion::Quitar));
        assert_eq!((vieja.en_local, vieja.en_servidor), (1, 0));
    }

    #[test]
    fn plan_id_es_estable_y_cambia_con_la_eleccion_o_una_revision() {
        let d = dif(vec![fila("titulo", Lado::Local, Some("a"), Some("b"))]);
        let a = plan(&d, &huella(0), &huella(0), &[]);
        assert_eq!(a.plan_id, plan(&d, &huella(0), &huella(0), &[]).plan_id);
        assert_ne!(a.plan_id, plan(&d, &huella(0), &huella(0), &["titulo"]).plan_id);
        assert_ne!(a.plan_id, plan(&d, &huella(1), &huella(0), &[]).plan_id);
    }

    #[test]
    fn miniaturas_se_asignan_por_clave() {
        let d = dif(vec![fila("foto:original:0", Lado::Local, Some("presente (aa)"), None)]);
        let mini_local = HashMap::from([("foto:original:0".to_string(), "/uploads/a.jpg".to_string())]);
        let p = planificar(&d, &huella(0), &huella(0), &BTreeSet::new(), &mini_local, &HashMap::new()).unwrap();
        assert_eq!(p.filas[0].miniatura_local.as_deref(), Some("/uploads/a.jpg"));
        assert_eq!(p.filas[0].miniatura_servidor, None);
    }
}
