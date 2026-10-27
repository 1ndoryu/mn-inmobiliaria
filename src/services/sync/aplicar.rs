//! Aplicar el plan de resolución [F2 de 14AA-2].
//!
//! El plan (`resolver::planificar`) dice, campo a campo, qué valor queda y en
//! qué lado se escribe. Aquí se ejecuta: se recalcula el plan, se comprueba que
//! sigue siendo el mismo (`plan_id`) y se copia cada campo desde el lado
//! ganador al otro. Los valores se copian **tipados** desde el inmueble de cada
//! entorno, no desde el texto canónico del plan: ese texto es para leerlo.
//!
//! Las fotos se copian por bytes en los dos sentidos (D6, F3): de eso se encarga
//! `fotos`, con las reglas de orden y cascada del borrado. Tampoco se puede
//! borrar la receta ni la copy por el PUT de la API, así que si el lado ganador
//! no las tiene, la fila queda pendiente en vez de mentir en el informe.
//!
//! Sin deshacer: si un paso falla, el informe dice qué quedó escrito. Al final
//! se releen los dos lados y se informa del residuo (D5).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sqlx::types::Json;
use sqlx::PgPool;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{validar_extras, CopyInmueble, Inmueble, UpdateInmuebleRequest};
use crate::repositories::InmuebleRepository;
use crate::services::inmueble::InmuebleService;
use crate::services::sync::comparacion::detalle;
use crate::services::sync::comparacion::huella::FotoHuella;
use crate::services::sync::comparacion::resolver::{Accion, Eleccion, FilaPlan};
use crate::services::sync::operacion::{conflicto, paso, Turno};
use crate::services::sync::plan;
use crate::services::sync::remoto::{self, Credenciales, FichaRemota, InmuebleRemoto};
use crate::services::sync::traer::MAX_FOTOS;

mod fotos;
mod reorden;

use fotos::{fila_foto, MoverFotos};

/// Lo aplicado y lo que queda. `fallo` corta la operación: lo anterior ya está escrito.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InformeAplicar {
    pub slug: String,
    /// Plan que se aplicó (el que el panel mostró).
    pub plan_id: String,
    /// Operaciones escritas en el local, en orden.
    pub en_local: Vec<String>,
    /// Operaciones escritas en el servidor, en orden.
    pub en_servidor: Vec<String>,
    /// Filas que no se aplican: fotos (F3) o sin camino de escritura.
    pub pendientes: Vec<String>,
    /// Paso que falló y motivo. `None` = la operación terminó.
    pub fallo: Option<String>,
    /// Diferencias que siguen ahí después de aplicar, ya releídos los dos lados.
    pub residuo: Vec<String>,
}

/// Aplica el plan `plan_id` sobre `slug`. Un `Err` significa que no se escribió nada.
pub async fn aplicar(
    pool: &PgPool,
    upload_dir: &Path,
    cred: &Credenciales,
    slug: &str,
    plan_id_pedido: &str,
    conservar_viejo: &BTreeSet<String>,
) -> Result<InformeAplicar, AppError> {
    let _turno = Turno::tomar(slug)?;
    let Preparado {
        local,
        servidor,
        servidor_id,
        plan_id,
        en_local,
        en_servidor,
        filas_foto,
        huella_local,
        huella_servidor,
    } = preparar(pool, cred, slug, plan_id_pedido, conservar_viejo).await?;

    let fotos = MoverFotos {
        plan: filas_foto,
        local: &local.fotos,
        servidor: &servidor.fotos,
        huella_local: &huella_local,
        huella_servidor: &huella_servidor,
    };
    comprobar_tope(&fotos)?;

    let mut informe = InformeAplicar {
        slug: slug.to_string(),
        plan_id,
        en_local: Vec::new(),
        en_servidor: Vec::new(),
        pendientes: en_local
            .pendientes
            .iter()
            .chain(&en_servidor.pendientes)
            .cloned()
            .collect(),
        fallo: None,
        residuo: Vec::new(),
    };

    // 4. Escrituras: primero los campos, las fotos y la visibilidad la última
    // (como en publicar).
    let destino_local = Destino {
        upload_dir,
        cred,
        id: local.id,
        fotos: &fotos,
    };
    if let Err(motivo) = escribir_local(
        pool,
        &destino_local,
        en_local,
        &mut informe.en_local,
        &mut informe.pendientes,
    )
    .await
    {
        informe.fallo = Some(motivo);
        return Ok(informe);
    }
    let destino_servidor = Destino {
        upload_dir,
        cred,
        id: servidor_id,
        fotos: &fotos,
    };
    if let Err(motivo) = escribir_servidor(
        &destino_servidor,
        en_servidor,
        &mut informe.en_servidor,
        &mut informe.pendientes,
    )
    .await
    {
        informe.fallo = Some(motivo);
        return Ok(informe);
    }

    // 5. Verificación: se releen los dos lados y se dice qué sigue distinto (D5).
    informe.residuo = residuo(pool, cred, slug).await;
    Ok(informe)
}

/// Todo lo que hace falta antes de escribir una sola fila: los dos lados
/// releídos, el plan recalculado (tiene que ser el que el panel mostró, D3) y
/// las operaciones ya separadas por lado. Si algo falla aquí, no se escribió nada.
struct Preparado {
    local: Inmueble,
    servidor: InmuebleRemoto,
    servidor_id: Uuid,
    plan_id: String,
    en_local: Cambios,
    en_servidor: Cambios,
    filas_foto: Vec<fotos::FilaFoto>,
    huella_local: Vec<FotoHuella>,
    huella_servidor: Vec<FotoHuella>,
}

async fn preparar(
    pool: &PgPool,
    cred: &Credenciales,
    slug: &str,
    plan_id_pedido: &str,
    conservar_viejo: &BTreeSet<String>,
) -> Result<Preparado, AppError> {
    // 1. Estado de los dos lados, tal como está ahora.
    let local_det = detalle::detalle_local(pool, slug)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("inmueble {slug}")))?;
    let local = InmuebleService::get_admin(pool, local_det.huella.id).await?;
    let ficha_local = InmuebleRepository::get_ficha(pool, local.id).await?;
    let remoto_det = remoto::detalle(cred, slug)
        .await
        .map_err(conflicto)?
        .ok_or_else(|| AppError::Conflict(format!("{slug} no existe en el servidor")))?;
    let servidor_id = remoto_det.huella.id;
    let servidor = remoto::inmueble(cred, servidor_id)
        .await
        .map_err(conflicto)?
        .ok_or_else(|| AppError::Conflict(format!("{slug} no existe en el servidor")))?;
    let ficha_servidor = remoto::ficha(cred, servidor_id).await.map_err(conflicto)?;

    // 2. El plan de ahora tiene que ser el que el panel mostró (D3).
    let plan = plan::plan_con(pool, cred, &local_det, &remoto_det, conservar_viejo).await?;
    if plan.plan_id != plan_id_pedido {
        return Err(AppError::Conflict(
            "el plan cambió desde que se mostró: vuelve a abrir el diálogo y decide otra vez"
                .into(),
        ));
    }

    // 3. Cada fila es una operación en el lado contrario a su elección. Las de
    // foto van por su propio camino (bytes), así que se sacan del plan antes.
    let fuente_local = campos_local(&local, ficha_local.as_ref());
    let fuente_servidor = campos_servidor(&servidor, ficha_servidor.as_ref());
    Ok(Preparado {
        en_local: reunir(
            &plan.filas,
            Eleccion::Servidor,
            &fuente_servidor,
            local.copy.as_ref(),
        ),
        en_servidor: reunir(
            &plan.filas,
            Eleccion::Local,
            &fuente_local,
            servidor.copy.as_ref(),
        ),
        filas_foto: plan.filas.iter().filter_map(fila_foto).collect(),
        huella_local: local_det.huella.fotos,
        huella_servidor: remoto_det.huella.fotos,
        plan_id: plan.plan_id,
        servidor_id,
        local,
        servidor,
    })
}

/// Antes de escribir nada: un plan con más fotos que el tope es un error de
/// planteamiento, no algo que se descubra a mitad.
fn comprobar_tope(fotos: &MoverFotos<'_>) -> Result<(), AppError> {
    let copias = fotos
        .plan
        .iter()
        .filter(|f| f.accion == Accion::Escribir)
        .count();
    if copias > MAX_FOTOS {
        return Err(AppError::Validation(format!(
            "el plan mueve {copias} fotos y el tope es {MAX_FOTOS}: hazlo en varios pasos"
        )));
    }
    Ok(())
}

/// Diferencias que quedan entre los dos lados, ya releídos. Un fallo al releer
/// no invalida lo escrito: se informa como residuo sin detalle.
async fn residuo(pool: &PgPool, cred: &Credenciales, slug: &str) -> Vec<String> {
    let local = detalle::detalle_local(pool, slug).await;
    let Ok(Some(local)) = local else {
        return vec!["no se pudo releer el local".into()];
    };
    let remoto = remoto::detalle(cred, slug).await;
    let Ok(remoto) = remoto else {
        return vec!["no se pudo releer el servidor".into()];
    };
    let Ok(dif) = detalle::diferencias(pool, &local, remoto.as_ref()).await else {
        return vec!["no se pudieron comparar los dos lados".into()];
    };
    dif.filas.iter().map(|f| f.clave.clone()).collect()
}

/// Lo que se va a escribir en un lado, ya separado por el camino de escritura
/// que le corresponde: el PUT de campos, la ficha, el estado y la visibilidad.
#[derive(Default)]
struct Cambios {
    /// Cuerpo del PUT del inmueble (incluye `copy` y `receta`).
    cuerpo: Map<String, Value>,
    estado: Option<String>,
    publicado: Option<bool>,
    /// Ficha privada: `Some` si el plan manda tocar `extras`.
    extras: Option<Value>,
    /// `Some(None)` = dejar el precio mínimo vacío.
    precio_minimo: Option<Option<f64>>,
    /// Claves que no se pueden escribir aquí.
    pendientes: Vec<String>,
}

/// Filas del plan que se escriben en el lado contrario a `ganador`, ya reunidas.
/// `fuente` son los valores tipados de `ganador`; `copy_actual`, la copy del lado
/// que se va a escribir (lo que no se toque se queda como está).
fn reunir(
    filas: &[FilaPlan],
    ganador: Eleccion,
    fuente: &BTreeMap<String, Value>,
    copy_actual: Option<&CopyInmueble>,
) -> Cambios {
    let mut cambios = Cambios::default();
    let mut copy_claves: BTreeSet<String> = BTreeSet::new();
    for fila in filas.iter().filter(|f| f.elegido == ganador) {
        // Las fotos no se escriben aquí: van por `MoverFotos`, con bytes.
        if fila.clave.starts_with("foto:") {
            continue;
        }
        let valor = fuente.get(&fila.clave).cloned().unwrap_or(Value::Null);
        match fila.clave.as_str() {
            // El estado va por su propio camino: `vendido`/`alquilado` despublican.
            "estado" => match valor.as_str() {
                Some(estado) => cambios.estado = Some(estado.to_string()),
                None => cambios
                    .pendientes
                    .push("estado (el lado ganador no lo tiene)".into()),
            },
            // La visibilidad va la última, para que nada quede público a medias.
            "publicado" => match valor.as_bool() {
                Some(publicado) => cambios.publicado = Some(publicado),
                None => cambios.pendientes.push("publicado (valor ilegible)".into()),
            },
            "extras" => cambios.extras = Some(valor),
            "precio_minimo" => cambios.precio_minimo = Some(valor.as_f64()),
            "receta" => {
                if valor.is_null() {
                    cambios.pendientes.push(
                        "receta (el lado ganador no tiene receta y la API no la borra)".into(),
                    );
                } else {
                    cambios.cuerpo.insert("receta".into(), valor);
                }
            }
            clave @ ("copy_corta" | "copy_larga" | "copy_modelo") => {
                copy_claves.insert(clave.to_string());
            }
            clave => {
                cambios.cuerpo.insert(clave.to_string(), valor);
            }
        }
    }
    if !copy_claves.is_empty() {
        match armar_copy(copy_actual, fuente, &copy_claves) {
            Some(copy) => {
                cambios.cuerpo.insert(
                    "copy".into(),
                    serde_json::to_value(copy).unwrap_or(Value::Null),
                );
            }
            None => cambios
                .pendientes
                .push("copy (ninguno de los dos lados tiene copy)".into()),
        }
    }
    cambios
}

/// La copy del lado que se escribe: cada subclave que el plan manda aplicar toma
/// el valor del lado ganador; las demás quedan como estaban. `None` = no hay nada
/// que escribir (ningún lado tiene copy).
fn armar_copy(
    actual: Option<&CopyInmueble>,
    fuente: &BTreeMap<String, Value>,
    claves: &BTreeSet<String>,
) -> Option<CopyInmueble> {
    let toma = |sub: &str, de_actual: Option<&str>| -> String {
        if claves.contains(&format!("copy_{sub}")) {
            fuente
                .get(&format!("copy_{sub}"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string()
        } else {
            de_actual.unwrap_or_default().to_string()
        }
    };
    let copy = CopyInmueble {
        corta: toma("corta", actual.map(|c| c.corta.as_str())),
        larga: toma("larga", actual.map(|c| c.larga.as_str())),
        modelo: toma("modelo", actual.map(|c| c.modelo.as_str())),
        actualizada_en: Utc::now(),
    };
    let vacia = copy.corta.is_empty() && copy.larga.is_empty() && copy.modelo.is_empty();
    if actual.is_none() && vacia {
        return None;
    }
    Some(copy)
}

/// A qué inmueble se escribe y con qué. Los dos lados comparten estas cuatro
/// piezas, así que van juntas en vez de repetidas en cada firma.
struct Destino<'a> {
    upload_dir: &'a Path,
    cred: &'a Credenciales,
    id: Uuid,
    fotos: &'a MoverFotos<'a>,
}

/// Escribe en el local lo que el plan manda. `hecho` recoge lo que sí se escribió.
async fn escribir_local(
    pool: &PgPool,
    destino: &Destino<'_>,
    cambios: Cambios,
    hecho: &mut Vec<String>,
    pendientes: &mut Vec<String>,
) -> Result<(), String> {
    if !cambios.cuerpo.is_empty() {
        let claves = claves_de(&cambios.cuerpo);
        let req: UpdateInmuebleRequest =
            serde_json::from_value(Value::Object(cambios.cuerpo)).map_err(|e| paso("campos", e))?;
        InmuebleService::update(pool, destino.id, req)
            .await
            .map_err(|e| paso("campos", e))?;
        hecho.push(format!("Campos en el local: {claves}"));
    }
    if let Some(extras) = cambios.extras {
        validar_extras(&extras).map_err(|e| paso("ficha", e))?;
        InmuebleRepository::set_ficha(
            pool,
            destino.id,
            Json(extras),
            cambios.precio_minimo.flatten(),
        )
        .await
        .map_err(|e| paso("ficha", e))?;
        hecho.push("Ficha privada (/ask) en el local".into());
    }
    fotos::al_local(
        pool,
        destino.upload_dir,
        destino.cred,
        destino.id,
        destino.fotos,
        hecho,
        pendientes,
    )
    .await?;
    if let Some(estado) = cambios.estado {
        InmuebleService::set_estado(pool, destino.id, &estado)
            .await
            .map_err(|e| paso("estado", e))?;
        hecho.push(format!("Estado en el local: {estado}"));
    }
    if let Some(publicado) = cambios.publicado {
        InmuebleService::set_publicado(pool, destino.id, publicado)
            .await
            .map_err(|e| paso("visibilidad", e))?;
        hecho.push(if publicado {
            "Publicado en el local".into()
        } else {
            "Sin publicar en el local".into()
        });
    }
    Ok(())
}

/// Escribe en el servidor lo que el plan manda, en el mismo orden.
async fn escribir_servidor(
    destino: &Destino<'_>,
    cambios: Cambios,
    hecho: &mut Vec<String>,
    pendientes: &mut Vec<String>,
) -> Result<(), String> {
    if !cambios.cuerpo.is_empty() {
        let claves = claves_de(&cambios.cuerpo);
        // [25AA-6] Un servidor sin la migración de «ambas» rechaza la fila por el
        // CHECK de `operacion`; el aviso dice la causa y solo afecta a esa fila.
        let ambas = cambios.cuerpo.get("operacion").and_then(Value::as_str) == Some("ambas");
        remoto::actualizar_inmueble(destino.cred, destino.id, &Value::Object(cambios.cuerpo))
            .await
            .map_err(|e| paso("campos", if ambas { format!("{e} (sin migración «ambas»: falta 20261027000045)") } else { e.to_string() }))?;
        hecho.push(format!("Campos en el servidor: {claves}"));
    }
    if let Some(extras) = cambios.extras {
        validar_extras(&extras).map_err(|e| paso("ficha", e))?;
        remoto::poner_ficha(
            destino.cred,
            destino.id,
            &extras,
            cambios.precio_minimo.flatten(),
        )
        .await
        .map_err(|e| paso("ficha", e))?;
        hecho.push("Ficha privada (/ask) en el servidor".into());
    }
    fotos::al_servidor(
        destino.upload_dir,
        destino.cred,
        destino.id,
        destino.fotos,
        hecho,
        pendientes,
    )
    .await?;
    if let Some(estado) = cambios.estado {
        remoto::poner_estado(destino.cred, destino.id, &estado)
            .await
            .map_err(|e| paso("estado", e))?;
        hecho.push(format!("Estado en el servidor: {estado}"));
    }
    if let Some(publicado) = cambios.publicado {
        remoto::poner_publicacion(destino.cred, destino.id, publicado)
            .await
            .map_err(|e| paso("visibilidad", e))?;
        hecho.push(if publicado {
            "Publicado en el servidor".into()
        } else {
            "Sin publicar en el servidor".into()
        });
    }
    Ok(())
}

fn claves_de(cuerpo: &Map<String, Value>) -> String {
    cuerpo.keys().cloned().collect::<Vec<_>>().join(", ")
}

/// Valores tipados del local, por clave del plan. Es de donde se copia hacia el
/// servidor: el plan trae el texto canónico solo para mostrarlo.
fn campos_local(
    inmueble: &Inmueble,
    ficha: Option<&(Json<Value>, Option<f64>)>,
) -> BTreeMap<String, Value> {
    /* [18AA-3] El vínculo con Marketplace es un campo más desde que la app vive
     * conectada a producción: va en el mismo `cuerpo` del PUT, y `null` (sin
     * vínculo aquí) desvincula allí, que es lo que se espera al sincronizar. */
    let mut m = BTreeMap::new();
    m.insert("marketplace_id".into(), json!(inmueble.marketplace_id));
    m.insert("titulo".into(), json!(inmueble.titulo));
    m.insert("descripcion".into(), json!(inmueble.descripcion));
    m.insert("ubicacion".into(), json!(inmueble.ubicacion));
    m.insert("puestos".into(), json!(inmueble.puestos));
    m.insert("residencia".into(), json!(inmueble.residencia));
    /* [18AA-6] Contenido de la ficha, como el título: viaja tal cual. */
    m.insert("nombre_corto".into(), json!(inmueble.nombre_corto));
    m.insert("precio".into(), json!(inmueble.precio));
    m.insert("precio_alquiler".into(), json!(inmueble.precio_alquiler));
    m.insert("tipo".into(), json!(inmueble.tipo));
    m.insert("operacion".into(), json!(inmueble.operacion));
    m.insert("habitaciones".into(), json!(inmueble.habitaciones));
    m.insert("banos".into(), json!(inmueble.banos));
    m.insert("metros".into(), json!(inmueble.metros));
    m.insert("metros_terreno".into(), json!(inmueble.metros_terreno));
    m.insert("estado".into(), json!(inmueble.estado));
    m.insert("publicado".into(), json!(inmueble.publicado));
    for (clave, valor) in [
        ("copy_corta", inmueble.copy.as_ref().map(|c| &c.corta)),
        ("copy_larga", inmueble.copy.as_ref().map(|c| &c.larga)),
        ("copy_modelo", inmueble.copy.as_ref().map(|c| &c.modelo)),
    ] {
        m.insert(clave.into(), json!(valor));
    }
    m.insert(
        "receta".into(),
        serde_json::to_value(&inmueble.receta).unwrap_or(Value::Null),
    );
    let (extras, minimo) = match ficha {
        Some((extras, minimo)) => (&extras.0, *minimo),
        None => (&Value::Object(Map::new()), None),
    };
    m.insert("extras".into(), extras.clone());
    m.insert("precio_minimo".into(), json!(minimo));
    m
}

/// Valores tipados del servidor, por clave del plan.
fn campos_servidor(
    inmueble: &InmuebleRemoto,
    ficha: Option<&FichaRemota>,
) -> BTreeMap<String, Value> {
    /* [18AA-3] Igual que en el local: el vínculo viaja en el `cuerpo` del PUT y
     * `null` desvincula (tri-estado). */
    let mut m = BTreeMap::new();
    m.insert("marketplace_id".into(), json!(inmueble.marketplace_id));
    m.insert("titulo".into(), json!(inmueble.titulo));
    m.insert("descripcion".into(), json!(inmueble.descripcion));
    m.insert("ubicacion".into(), json!(inmueble.ubicacion));
    m.insert("puestos".into(), json!(inmueble.puestos));
    m.insert("residencia".into(), json!(inmueble.residencia));
    /* [18AA-6] Se lee del servidor para poder mostrarlo y compararlo; el PUT de
     * vuelta nunca lo vacía (`NULLIF` en el repositorio). */
    m.insert("nombre_corto".into(), json!(inmueble.nombre_corto));
    m.insert("precio".into(), json!(inmueble.precio));
    m.insert("precio_alquiler".into(), json!(inmueble.precio_alquiler));
    m.insert("tipo".into(), json!(inmueble.tipo));
    m.insert("operacion".into(), json!(inmueble.operacion));
    m.insert("habitaciones".into(), json!(inmueble.habitaciones));
    m.insert("banos".into(), json!(inmueble.banos));
    m.insert("metros".into(), json!(inmueble.metros));
    m.insert("metros_terreno".into(), json!(inmueble.metros_terreno));
    m.insert("estado".into(), json!(inmueble.estado));
    m.insert("publicado".into(), json!(inmueble.publicado));
    for (clave, valor) in [
        ("copy_corta", inmueble.copy.as_ref().map(|c| &c.corta)),
        ("copy_larga", inmueble.copy.as_ref().map(|c| &c.larga)),
        ("copy_modelo", inmueble.copy.as_ref().map(|c| &c.modelo)),
    ] {
        m.insert(clave.into(), json!(valor));
    }
    m.insert(
        "receta".into(),
        serde_json::to_value(&inmueble.receta).unwrap_or(Value::Null),
    );
    let (extras, minimo) = match ficha {
        Some(ficha) => (&ficha.extras, ficha.precio_minimo),
        None => (&Value::Object(Map::new()), None),
    };
    m.insert("extras".into(), extras.clone());
    m.insert("precio_minimo".into(), json!(minimo));
    m
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::services::sync::comparacion::comparar::Lado;
    use crate::services::sync::comparacion::resolver::Accion;
    use chrono::TimeZone;

    fn fila(clave: &str, elegido: Eleccion, accion: Accion) -> FilaPlan {
        FilaPlan {
            clave: clave.into(),
            lado: Lado::Ambos,
            elegido,
            accion,
            local: Some("x".into()),
            servidor: Some("y".into()),
            miniatura_local: None,
            miniatura_servidor: None,
        }
    }

    fn fuente(pares: &[(&str, Value)]) -> BTreeMap<String, Value> {
        pares
            .iter()
            .map(|(k, v)| ((*k).to_string(), v.clone()))
            .collect()
    }

    fn copy() -> CopyInmueble {
        CopyInmueble {
            corta: "corta vieja".into(),
            larga: "larga vieja".into(),
            modelo: "modelo viejo".into(),
            actualizada_en: Utc.timestamp_opt(1_800_000_000, 0).unwrap(),
        }
    }

    #[test]
    fn cada_camino_de_escritura_recibe_sus_claves() {
        let filas = vec![
            fila("titulo", Eleccion::Local, Accion::Escribir),
            fila("precio", Eleccion::Local, Accion::Escribir),
            fila("publicado", Eleccion::Local, Accion::Escribir),
            fila("estado", Eleccion::Local, Accion::Escribir),
            fila("extras", Eleccion::Local, Accion::Escribir),
            fila("precio_minimo", Eleccion::Local, Accion::Escribir),
            fila("foto:original:0", Eleccion::Local, Accion::Escribir),
        ];
        let fuente = fuente(&[
            ("titulo", json!("Apto")),
            ("precio", json!(45000.0)),
            ("publicado", json!(true)),
            ("estado", json!("reservado")),
            ("extras", json!({"piso": "4"})),
            ("precio_minimo", json!(40000.0)),
        ]);
        let c = reunir(&filas, Eleccion::Local, &fuente, None);
        assert_eq!(c.cuerpo.get("titulo"), Some(&json!("Apto")));
        assert_eq!(c.cuerpo.get("precio"), Some(&json!(45000.0)));
        assert_eq!(c.estado.as_deref(), Some("reservado"));
        assert_eq!(c.publicado, Some(true));
        assert_eq!(c.extras, Some(json!({"piso": "4"})));
        assert_eq!(c.precio_minimo, Some(Some(40000.0)));
        assert!(
            !c.cuerpo.contains_key("publicado"),
            "la visibilidad no va en el PUT"
        );
        assert!(
            !c.cuerpo.contains_key("estado"),
            "el estado tiene su propio camino"
        );
        assert!(
            c.pendientes.is_empty(),
            "las fotos no pasan por aquí: {:?}",
            c.pendientes
        );
    }

    #[test]
    fn solo_se_reunen_las_filas_del_lado_ganador() {
        let filas = vec![
            fila("titulo", Eleccion::Local, Accion::Escribir),
            fila("precio", Eleccion::Servidor, Accion::Escribir),
        ];
        let f = fuente(&[("titulo", json!("A")), ("precio", json!(1.0))]);
        let en_local = reunir(&filas, Eleccion::Servidor, &f, None);
        assert_eq!(claves_de(&en_local.cuerpo), "precio");
        let en_servidor = reunir(&filas, Eleccion::Local, &f, None);
        assert_eq!(claves_de(&en_servidor.cuerpo), "titulo");
    }

    #[test]
    fn la_copy_se_arma_subclave_a_subclave_y_conserva_lo_no_tocado() {
        let filas = vec![fila("copy_corta", Eleccion::Local, Accion::Escribir)];
        let f = fuente(&[("copy_corta", json!("corta nueva"))]);
        let c = reunir(&filas, Eleccion::Local, &f, Some(&copy()));
        let copy = c.cuerpo.get("copy").expect("copy en el cuerpo");
        assert_eq!(copy["corta"], json!("corta nueva"));
        assert_eq!(
            copy["larga"],
            json!("larga vieja"),
            "lo no tocado se conserva"
        );
        assert_eq!(copy["modelo"], json!("modelo viejo"));
    }

    #[test]
    fn sin_copy_en_ningun_lado_no_se_escribe_una_copy_vacia() {
        let filas = vec![fila("copy_corta", Eleccion::Local, Accion::Quitar)];
        let f = fuente(&[("copy_corta", Value::Null)]);
        let c = reunir(&filas, Eleccion::Local, &f, None);
        assert!(!c.cuerpo.contains_key("copy"));
        assert_eq!(
            c.pendientes,
            vec!["copy (ninguno de los dos lados tiene copy)"]
        );
    }

    #[test]
    fn la_receta_sin_valor_queda_pendiente_en_vez_de_mentir() {
        let filas = vec![fila("receta", Eleccion::Servidor, Accion::Quitar)];
        let f = fuente(&[("receta", Value::Null)]);
        let c = reunir(&filas, Eleccion::Servidor, &f, None);
        assert!(!c.cuerpo.contains_key("receta"));
        assert_eq!(c.pendientes.len(), 1);
        assert!(c.pendientes[0].starts_with("receta"));
    }

    #[test]
    fn una_fila_quitar_en_un_campo_de_texto_escribe_el_vacio() {
        let filas = vec![fila("ubicacion", Eleccion::Servidor, Accion::Quitar)];
        let f = fuente(&[("ubicacion", json!(""))]);
        let c = reunir(&filas, Eleccion::Servidor, &f, None);
        assert_eq!(c.cuerpo.get("ubicacion"), Some(&json!("")));
        assert!(c.pendientes.is_empty());
    }
}
