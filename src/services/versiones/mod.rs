//! [30AA-1] Generación y edición de versiones de texto (v2 en adelante).
//!
//! La variante NO puede cambiar la realidad: mismos números, misma ubicación,
//! mismos servicios. Solo cambia el orden de las frases y las palabras. Eso
//! se impone en el prompt (`SISTEMA_VARIANTE`) y en la validación de aquí:
//! si la IA devuelve un número que no estaba en la v1, se rechaza la
//! respuesta en vez de guardar un dato inventado.

use std::fmt::Write as _;

use sqlx::PgPool;
use uuid::Uuid;

use crate::config::ia::SISTEMA_VARIANTE;
use crate::errors::AppError;
use crate::models::{
    EditarVersionRequest, GenerarVersionRequest, GenerarVersionSalida, InmuebleVersion,
    VersionesInmueble, ESTADOS_VERSION,
};
use crate::repositories::{EntradaHistorial, InmuebleRepository, VersionRepository};

pub struct VersionService;

impl VersionService {
    /// Todo lo que pinta la pestaña de versiones del modal.
    pub async fn detalle(pool: &PgPool, inmueble_id: Uuid) -> Result<VersionesInmueble, AppError> {
        let row = InmuebleRepository::find_by_id(pool, inmueble_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Inmueble no encontrado".into()))?;
        let configuradas = VersionRepository::configuradas(pool).await?;
        let variantes = VersionRepository::listar(pool, inmueble_id).await?;
        let fotos = VersionRepository::fotos_de(pool, inmueble_id)
            .await?
            .into_iter()
            .map(Into::into)
            .collect();
        let historial = VersionRepository::historial(pool, inmueble_id).await?;
        Ok(VersionesInmueble {
            configuradas,
            titulo_v1: row.titulo,
            descripcion_v1: row.descripcion,
            variantes,
            fotos,
            historial,
        })
    }

    /// Genera la variante con IA y la guarda en estado `borrador`.
    pub async fn generar(
        pool: &PgPool,
        inmueble_id: Uuid,
        req: &GenerarVersionRequest,
    ) -> Result<GenerarVersionSalida, AppError> {
        let row = InmuebleRepository::find_by_id(pool, inmueble_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Inmueble no encontrado".into()))?;
        let configuradas = VersionRepository::configuradas(pool).await?;
        let version = req.version.unwrap_or(2);
        if version < 2 || version > configuradas {
            return Err(AppError::Validation(format!(
                "Versión fuera de rango (2..={configuradas})"
            )));
        }
        if row.titulo.trim().is_empty() && row.descripcion.trim().is_empty() {
            return Err(AppError::Validation(
                "La ficha no tiene título ni descripción que versionar".into(),
            ));
        }

        let prompt = prompt_variante(&row, req.instruccion.trim());
        let salida = crate::handlers::ia::rutas::completar_texto(SISTEMA_VARIANTE, &prompt, &[]).await;
        let (texto, modelo) = match salida {
            Ok(v) => v,
            Err(motivo) => {
                return Ok(GenerarVersionSalida {
                    ok: false,
                    version: None,
                    modelo: None,
                    motivos: vec![format!("opencode-go: {motivo}")],
                });
            }
        };

        let Some((titulo, descripcion)) = parsear(&texto) else {
            return Ok(GenerarVersionSalida {
                ok: false,
                version: None,
                modelo: Some(modelo),
                motivos: vec!["La IA no devolvió el JSON esperado".into()],
            });
        };

        /* Guarda: si la variante introduce un número que la ficha no tiene, es
         * un dato inventado (más habitaciones, otro precio). Se rechaza.
         * `numeros_de_la_ficha` incluye las columnas: el precio no vive en el
         * texto y sin esto se rechazaba un precio verdadero. */
        let permitidos = numeros_de_la_ficha(&row);
        if let Some(inventado) = numero_inventado(&permitidos, &titulo, &descripcion) {
            return Ok(GenerarVersionSalida {
                ok: false,
                version: None,
                modelo: Some(modelo),
                motivos: vec![format!(
                    "La variante introduce «{inventado}», que no está en la ficha; no se guardó."
                )],
            });
        }

        /* Historial antes de pisar lo que hubiera en esa versión. */
        if let Some(previo) = VersionRepository::find(pool, inmueble_id, version).await? {
            for (campo, anterior, nuevo) in [
                ("titulo", previo.titulo.as_str(), titulo.as_str()),
                ("descripcion", previo.descripcion.as_str(), descripcion.as_str()),
            ] {
                VersionRepository::anotar_historial(
                    pool,
                    &EntradaHistorial {
                        inmueble_id,
                        version,
                        campo,
                        anterior,
                        nuevo,
                        origen: "ia",
                        modelo: &modelo,
                    },
                )
                .await?;
            }
        }

        let guardada = VersionRepository::upsert(
            pool,
            inmueble_id,
            version,
            &titulo,
            &descripcion,
            &modelo,
            &prompt,
        )
        .await?;
        Ok(GenerarVersionSalida {
            ok: true,
            version: Some(guardada),
            modelo: Some(modelo),
            motivos: Vec::new(),
        })
    }

    /// Edición manual: lo que cambia entra al historial con origen `manual`.
    pub async fn editar(
        pool: &PgPool,
        inmueble_id: Uuid,
        version: i16,
        req: &EditarVersionRequest,
    ) -> Result<InmuebleVersion, AppError> {
        let previo = VersionRepository::find(pool, inmueble_id, version)
            .await?
            .ok_or_else(|| AppError::NotFound("Versión no encontrada".into()))?;
        let estado = match req.estado.as_deref().map(str::trim) {
            Some(e) if !e.is_empty() => {
                if !ESTADOS_VERSION.contains(&e) {
                    return Err(AppError::Validation(format!(
                        "Valor inválido para estado: {e}"
                    )));
                }
                Some(e)
            }
            _ => None,
        };
        let titulo = req.titulo.as_deref().map(str::trim);
        let descripcion = req.descripcion.as_deref().map(str::trim);

        /* Solo lo que cambia entra al historial: guardar sin tocar el texto no
         * debe dejar una entrada más. */
        for (campo, previo_valor, nuevo) in [
            ("titulo", previo.titulo.as_str(), titulo),
            ("descripcion", previo.descripcion.as_str(), descripcion),
        ] {
            let Some(nuevo) = nuevo.filter(|n| *n != previo_valor) else {
                continue;
            };
            VersionRepository::anotar_historial(
                pool,
                &EntradaHistorial {
                    inmueble_id,
                    version,
                    campo,
                    anterior: previo_valor,
                    nuevo,
                    origen: "manual",
                    modelo: "",
                },
            )
            .await?;
        }

        VersionRepository::actualizar(pool, inmueble_id, version, titulo, descripcion, estado)
            .await?
            .ok_or_else(|| AppError::NotFound("Versión no encontrada".into()))
    }

    pub async fn borrar(pool: &PgPool, inmueble_id: Uuid, version: i16) -> Result<(), AppError> {
        if !VersionRepository::borrar(pool, inmueble_id, version).await? {
            return Err(AppError::NotFound("Versión no encontrada".into()));
        }
        Ok(())
    }
}

/// Texto que recibe la IA: la ficha completa (para no inventar) y el encargo.
fn prompt_variante(row: &crate::models::InmuebleRow, instruccion: &str) -> String {
    // `writeln!` sobre String no falla nunca; el `let _ =` evita el `unwrap`.
    let mut s = String::with_capacity(4000);
    let _ = writeln!(s, "Ficha original:");
    let _ = writeln!(s, "Título: {}", row.titulo);
    let _ = writeln!(s, "Descripción: {}", row.descripcion);
    let _ = writeln!(s, "Ubicación: {}", row.ubicacion);
    let _ = writeln!(s, "Residencia: {}", row.residencia);
    let _ = writeln!(
        s,
        "Tipo: {} · Operación: {} · Habitaciones: {} · Baños: {} · m²: {} · Terreno: {} · Puestos: {}",
        row.tipo,
        row.operacion,
        row.habitaciones,
        row.banos,
        row.metros,
        row.metros_terreno,
        row.puestos
    );
    let _ = writeln!(s, "Precio: {}", row.precio);
    if row.precio_alquiler > 0.0 {
        let _ = writeln!(s, "Canon mensual: {}", row.precio_alquiler);
    }
    if !instruccion.is_empty() {
        s.push_str("\nInstrucción adicional de la dueña: ");
        s.push_str(instruccion);
    }
    s
}

/// JSON de la IA → (título, descripción). Tolerante: quita vallas de
/// markdown y acepta claves con o sin acentos.
fn parsear(texto: &str) -> Option<(String, String)> {
    let limpio = texto
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    let inicio = limpio.find('{')?;
    let fin = limpio.rfind('}')?;
    if fin <= inicio {
        return None;
    }
    let valor: serde_json::Value = serde_json::from_str(&limpio[inicio..=fin]).ok()?;
    let titulo = valor.get("titulo")?.as_str()?.trim().to_string();
    let descripcion = valor.get("descripcion")?.as_str()?.trim().to_string();
    if titulo.is_empty() || descripcion.is_empty() {
        return None;
    }
    Some((titulo, descripcion))
}

/// Todos los dígitos que la ficha permite citar: el texto **y** las columnas.
///
/// El precio, los metros, las habitaciones y los baños viven en columnas, no
/// en `titulo`/`descripcion`, y son datos tan ciertos como el texto. Mirando
/// solo el texto se rechazaba un precio verdadero (San Ignacio, 29.000).
fn numeros_de_la_ficha(row: &crate::models::InmuebleRow) -> String {
    numeros_permitidos(
        &row.titulo,
        &row.descripcion,
        &[
            row.precio,
            row.precio_alquiler,
            row.metros,
            row.metros_terreno,
            f64::from(row.habitaciones),
            f64::from(row.banos),
            f64::from(row.puestos),
        ],
    )
}

/// Los dígitos del texto más los de las cifras que no sean 0.
///
/// `0` significa «no hay dato» (precio a consultar, sin terreno): permitirlo
/// dejaría a la IA escribir «precio 0», que es justo un dato falso.
fn numeros_permitidos(titulo: &str, descripcion: &str, cifras: &[f64]) -> String {
    let mut s = format!("{titulo} {descripcion}");
    for valor in cifras.iter().filter(|v| **v > 0.0) {
        let _ = write!(s, " {valor}");
    }
    digitos(&s)
}

/// Busca en la variante un número que no aparezca en la ficha original.
/// Devuelve el primero que encuentre. Precios con separadores se comparan por
/// dígitos, y también cuentan los de una cifra: cambiar «3 habitaciones» por
/// «4» es justo el error que hay que parar.
fn numero_inventado(permitidos: &str, titulo_v2: &str, desc_v2: &str) -> Option<String> {
    /* Se compara contra TODOS los dijitos de la v1, no contra cada numero
     * suelto: asi `120.000` y `120000` no se toman por un dato inventado, y
     * `3 habitaciones` → `4 habitaciones` (un solo digito, el cambio que mas
     * importa) si se detecta. */
    numeros(&format!("{titulo_v2} {desc_v2}"))
        .into_iter()
        .find(|n| !permitidos.contains(n.as_str()))
}

/// Todos los digitos del texto, pegados: base de la comparacion anterior.
fn digitos(texto: &str) -> String {
    texto.chars().filter(char::is_ascii_digit).collect()
}

/// Numeros de la variante, de cualquier longitud (un «4» suelto tambien es un
/// dato inventado si la ficha decia «3»). Un digito pegado a una palabra
/// —«3dorm»— cuenta igual: la IA no escribe asi, pero si lo hace, se revisa.
fn numeros(texto: &str) -> std::collections::HashSet<String> {
    texto
        .split(|c: char| !c.is_ascii_digit())
        .filter(|t| !t.is_empty())
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod pruebas {
    use super::{digitos, numero_inventado, numeros_permitidos, parsear};

    #[test]
    fn acepta_json_con_vallas_y_sin_acentos_raros() {
        let t = "```json\n{\"titulo\":\"Apart. luminoso\",\"descripcion\":\"Tres habitaciones.\"}\n```";
        let (titulo, desc) = parsear(t).expect("debe parsear");
        assert_eq!(titulo, "Apart. luminoso");
        assert_eq!(desc, "Tres habitaciones.");
    }

    #[test]
    fn rechaza_texto_sin_json() {
        assert!(parsear("No puedo ayudarte con eso.").is_none());
    }

    #[test]
    fn detecta_dato_inventado() {
        let permitidos = digitos("Apart. 3 habitaciones Precio 120000");
        let v = numero_inventado(&permitidos, "Apart. con 4 habitaciones", "Precio 120000");
        assert_eq!(v.as_deref(), Some("4"));
    }

    #[test]
    fn respeta_los_numeros_que_ya_estaban() {
        let permitidos = digitos("Apart. 3 habitaciones 120 m² por 120000");
        let v = numero_inventado(
            &permitidos,
            "Hermoso apartamento: 3 habitaciones y 120 m²",
            "Se ofrece en 120000",
        );
        assert!(v.is_none());
    }

    /// El precio vive en `inmuebles.precio`, no en el texto: citarlo es
    /// correcto y antes se rechazaba (San Ignacio, 2026-11-12).
    #[test]
    fn acepta_el_precio_de_la_columna() {
        let permitidos = numeros_permitidos("Apart. obra gris de 77 m²", "", &[29_000.0]);
        let v = numero_inventado(&permitidos, "77 m² obra gris", "Se ofrece en venta por 29.000");
        assert!(v.is_none(), "el precio de la ficha no es inventado: {v:?}");
    }

    /// Y sigue parando una cifra que no está ni en el texto ni en la columna.
    #[test]
    fn sigue_parando_un_precio_inventado() {
        let permitidos = numeros_permitidos("Apart. obra gris de 77 m²", "", &[29_000.0]);
        let v = numero_inventado(&permitidos, "77 m² obra gris", "Se ofrece por 35.000");
        assert!(v.is_some(), "35.000 no está en la ficha y debe rechazarse");
    }

    /// Una ficha con `precio = 0` («a consultar») no autoriza escribir «0».
    #[test]
    fn precio_cero_no_autoriza_ninguna_cifra() {
        let permitidos = numeros_permitidos("Apart. a consultar", "", &[0.0]);
        let v = numero_inventado(&permitidos, "Apart. a consultar", "Precio 0");
        assert_eq!(v.as_deref(), Some("0"));
    }
}
