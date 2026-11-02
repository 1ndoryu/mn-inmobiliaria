//! Solicitudes «Publicar mi inmueble» de producción a la base local [28AE-1].
//!
//! Tarea de fondo activada con `SYNC_SOLICITUDES_SEG` (apagada por defecto; solo
//! en la 89). Trae las solicitudes nuevas con sus fotos. Una solicitud entra solo
//! cuando sus fotos ya están en disco, así nunca queda una fila sin foto. Lo que
//! llega sale sin ver (`aviso_visto = FALSE`) y el admin lo ve en la cabecera.

use std::collections::HashSet;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::time::Duration;

use sqlx::PgPool;
use uuid::Uuid;

use super::remoto::{self, Credenciales, ErrorRemoto, FotoSolicitudRemota, SolicitudRemota};
use crate::models::{clave_foto_valida, MAX_FOTO_BYTES};
use crate::repositories::{NuevaSolicitud, SolicitudRepository, SolicitudSincronizada};
use crate::services::inmueble::InmuebleService;

/// Solicitudes por página (el servidor admite hasta 100)
const POR_PAGINA: i64 = 50;
/// Tope de páginas por pasada: acota el trabajo aunque cambie el orden remoto
const MAX_PAGINAS: i64 = 20;
/// Pausa mínima entre pasadas: cada una lee el listado con fotos
const INTERVALO_MINIMO_SEG: u64 = 10;

type ErrorPasada = Box<dyn Error + Send + Sync>;

/// Intervalo desde `SYNC_SOLICITUDES_SEG`. Sin valor o en 0, la sincronización queda apagada.
pub fn intervalo_desde_entorno() -> Option<Duration> {
    intervalo_de(std::env::var("SYNC_SOLICITUDES_SEG").ok().as_deref())
}

fn intervalo_de(valor: Option<&str>) -> Option<Duration> {
    let seg: u64 = valor?.trim().parse().ok()?;
    (seg > 0).then(|| Duration::from_secs(seg.max(INTERVALO_MINIMO_SEG)))
}

/// Tarea de fondo: una pasada al arrancar y luego cada `intervalo`.
pub async fn vigilar(pool: PgPool, upload_dir: PathBuf, intervalo: Duration) {
    loop {
        match sincronizar(&pool, &upload_dir).await {
            Ok(0) => {}
            Ok(n) => tracing::info!("solicitudes: {n} nuevas de producción"),
            Err(e) => tracing::warn!("solicitudes: sincronización fallida: {e}"),
        }
        tokio::time::sleep(intervalo).await;
    }
}

/// Una pasada: recorre las páginas de producción (la más reciente primero) y se
/// para en la primera ya conocida entera. Devuelve cuántas solicitudes entraron.
pub async fn sincronizar(pool: &PgPool, upload_dir: &Path) -> Result<usize, ErrorPasada> {
    let cred = match Credenciales::cargar() {
        Ok(cred) => cred,
        // Sin archivo de credenciales la sincronización está apagada, no es un fallo
        Err(ErrorRemoto::Desactivado) => return Ok(0),
        Err(e) => return Err(e.into()),
    };
    /* Primera carga: lo que llega ya existía en producción y no debe avisar. Se
     * decide por la marca de la primera pasada y no por «hay sincronizadas»: si
     * producción estaba vacía, la siguiente solicitud real volvería a contar como
     * primera carga y nacería vista. La marca se pone al inicio: si la pasada
     * falla a medias, lo que quede avisará (preferible a perder un aviso). */
    let primera_carga = !SolicitudRepository::primera_pasada_hecha(pool).await?;
    SolicitudRepository::marcar_primera_pasada(pool).await?;
    let mut nuevas = 0;
    for page in 1..=MAX_PAGINAS {
        let pagina = remoto::pagina_solicitudes(&cred, page, POR_PAGINA).await?;
        let ids: Vec<Uuid> = pagina.items.iter().map(|s| s.id).collect();
        let conocidas: HashSet<Uuid> = SolicitudRepository::ids_existentes(pool, &ids)
            .await?
            .into_iter()
            .collect();
        let pendientes: Vec<&SolicitudRemota> = pagina
            .items
            .iter()
            .filter(|s| !conocidas.contains(&s.id))
            .collect();
        if pendientes.is_empty() {
            break;
        }
        for solicitud in pendientes {
            // Un fallo no para a las demás: esa solicitud queda sin insertar y se reintenta.
            match importar(pool, &cred, upload_dir, solicitud, primera_carga).await {
                Ok(true) => nuevas += 1,
                Ok(false) => {}
                Err(e) => tracing::warn!("solicitud {}: se reintentará: {e}", solicitud.id),
            }
        }
        // Una página corta indica que ya no hay más
        if (pagina.items.len() as i64) < POR_PAGINA {
            break;
        }
    }
    Ok(nuevas)
}

/// Guarda las fotos y, ya en disco, inserta la solicitud. `false` si ya existía.
async fn importar(
    pool: &PgPool,
    cred: &Credenciales,
    upload_dir: &Path,
    s: &SolicitudRemota,
    primera_carga: bool,
) -> Result<bool, ErrorPasada> {
    let mut claves = Vec::with_capacity(s.fotos.len());
    for foto in &s.fotos {
        descargar_si_falta(cred, upload_dir, foto).await?;
        claves.push(foto.storage_key.clone());
    }
    let nueva = SolicitudSincronizada {
        id: s.id,
        nueva: NuevaSolicitud {
            nombre: &s.nombre,
            telefono: &s.telefono,
            email: &s.email,
            descripcion: &s.descripcion,
            ubicacion: &s.ubicacion,
            puestos: s.puestos,
            residencia: &s.residencia,
            precio_estimado: s.precio_estimado,
            operacion: &s.operacion,
            fotos: &claves,
            ip_origen: s.ip_origen.as_deref(),
            user_agent: s.user_agent.as_deref(),
            origen_contacto: &s.origen_contacto,
        },
        estado: &s.estado,
        created_at: s.created_at,
        updated_at: s.updated_at,
        aviso_visto: primera_carga,
    };
    Ok(SolicitudRepository::insertar_sincronizada(pool, &nueva).await?)
}

/// Guarda la foto en `upload_dir/<clave>`, con la misma clave que en producción.
/// Escribe a un temporal y renombra: una descarga cortada nunca deja una foto a medias.
async fn descargar_si_falta(
    cred: &Credenciales,
    upload_dir: &Path,
    foto: &FotoSolicitudRemota,
) -> Result<(), ErrorPasada> {
    if !clave_foto_valida(&foto.storage_key) {
        return Err(format!("clave de foto no válida: {}", foto.storage_key).into());
    }
    let ruta = upload_dir.join(&foto.storage_key);
    if tokio::fs::try_exists(&ruta).await? {
        return Ok(());
    }
    let bytes = remoto::descargar_foto(cred, &foto.url).await?;
    if bytes.len() > MAX_FOTO_BYTES {
        return Err(format!("foto {} demasiado grande", foto.storage_key).into());
    }
    let extension = InmuebleService::extension_valida(&foto.storage_key)?;
    InmuebleService::magia_valida(&bytes, extension)?;
    let carpeta = ruta.parent().ok_or("ruta de foto sin carpeta")?;
    tokio::fs::create_dir_all(carpeta).await?;
    // Canonicalizar y comprobar el prefijo: una clave con «..» o un enlace no escapa de uploads
    let base = tokio::fs::canonicalize(upload_dir).await?;
    if !tokio::fs::canonicalize(carpeta).await?.starts_with(&base) {
        return Err(format!("foto fuera de uploads: {}", foto.storage_key).into());
    }
    let temporal = ruta.with_extension("part");
    tokio::fs::write(&temporal, &bytes).await?;
    tokio::fs::rename(&temporal, &ruta).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::intervalo_de;

    #[test]
    fn apagado_sin_valor_o_en_cero() {
        assert_eq!(intervalo_de(None), None);
        assert_eq!(intervalo_de(Some("0")), None);
        assert_eq!(intervalo_de(Some("no-numero")), None);
    }

    #[test]
    fn intervalo_con_minimo() {
        assert_eq!(intervalo_de(Some("3")), Some(Duration::from_secs(10)));
        assert_eq!(intervalo_de(Some(" 60 ")), Some(Duration::from_secs(60)));
    }
}

#[cfg(test)]
mod pruebas {
    use std::sync::{Arc, Mutex};

    use axum::extract::State;
    use axum::routing::{get, post};
    use axum::{Json, Router};
    use serde_json::{json, Value};

    use super::*;

    /// Cabecera JPEG mínima: la validación mira la firma, no la imagen entera.
    const JPEG: &[u8] = &[0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, b'J', b'F', b'I', b'F', 0x00];

    /// Servidor de producción simulado: login, listado y fotos.
    #[derive(Clone)]
    struct Produccion {
        base: String,
        solicitudes: Arc<Mutex<Vec<Value>>>,
    }

    async fn login() -> Json<Value> {
        Json(json!({ "token": "t1" }))
    }

    /// Listado sin paginar de verdad: todo cabe en una página, así la pasada se corta sola.
    async fn listado(State(p): State<Produccion>) -> Json<Value> {
        let mut items = p.solicitudes.lock().unwrap().clone();
        for item in items.iter_mut() {
            for foto in item["fotos"].as_array_mut().into_iter().flatten() {
                let clave = foto["storage_key"].as_str().unwrap_or_default().to_string();
                foto["url"] = json!(format!("{}/uploads/{clave}", p.base));
            }
        }
        let total = items.len();
        Json(json!({ "items": items, "total": total, "page": 1, "per_page": 50 }))
    }

    async fn foto() -> Vec<u8> {
        JPEG.to_vec()
    }

    fn solicitud_remota(id: Uuid, clave: &str) -> Value {
        json!({
            "id": id,
            "nombre": "Prueba 28AE-1",
            "telefono": "0400000000",
            "email": "",
            "descripcion": "Solicitud de prueba de sincronización",
            "ubicacion": "Caracas",
            "puestos": 1,
            "residencia": "",
            "precio_estimado": null,
            "operacion": "venta",
            "estado": "pendiente",
            "fotos": [{ "storage_key": clave }],
            "ip_origen": null,
            "user_agent": null,
            "origen_contacto": "web",
            "created_at": "2026-11-02T10:00:00Z",
            "updated_at": "2026-11-02T10:00:00Z",
        })
    }

    fn pool_si_hay() -> Option<PgPool> {
        let url = crate::config::url_bd_pruebas()?;
        sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy(&url)
            .ok()
    }

    /// Las solicitudes entran con su foto; solo lo nuevo avisa, y repetir la pasada no duplica.
    #[tokio::test]
    async fn sincroniza_fotos_y_avisa_solo_de_lo_nuevo() {
        let Some(pool) = pool_si_hay() else { return };
        let dir = std::env::temp_dir().join(format!("sync-sol-{}", Uuid::new_v4()));
        let uploads = dir.join("uploads");
        tokio::fs::create_dir_all(&uploads).await.unwrap();

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let solicitudes = Arc::new(Mutex::new(Vec::new()));
        let app = Router::new()
            .route("/api/auth/login", post(login))
            .route("/api/admin/solicitudes", get(listado))
            .route("/uploads/*ruta", get(foto))
            .with_state(Produccion {
                base: base.clone(),
                solicitudes: solicitudes.clone(),
            });
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        let credenciales = dir.join("prod.env");
        let contenido = format!("PROD_BASE={base}\nPROD_EMAIL=admin@x\nPROD_PASSWORD=secreto\n");
        tokio::fs::write(&credenciales, contenido).await.unwrap();
        std::env::set_var("SYNC_REMOTO_ENV", &credenciales);

        // Limpia restos de una ejecución anterior para contar solo lo de esta prueba.
        SolicitudRepository::marcar_vistas(&pool).await.unwrap();

        let (id1, id2) = (Uuid::new_v4(), Uuid::new_v4());
        let clave1 = format!("solicitudes/{id1}/{}.jpg", Uuid::new_v4());
        solicitudes.lock().unwrap().push(solicitud_remota(id1, &clave1));

        // Pasada vacía (producción sin solicitudes): fija la primera carga.
        solicitudes.lock().unwrap().clear();
        assert_eq!(sincronizar(&pool, &uploads).await.unwrap(), 0);

        // Llega una tras la pasada vacía: debe avisar (antes nacía vista).
        solicitudes.lock().unwrap().push(solicitud_remota(id1, &clave1));
        assert_eq!(sincronizar(&pool, &uploads).await.unwrap(), 1);
        assert!(uploads.join(&clave1).exists());
        assert_eq!(SolicitudRepository::contar_no_vistas(&pool).await.unwrap(), 1);
        SolicitudRepository::marcar_vistas(&pool).await.unwrap();

        // Llega una nueva en producción: entra sin ver y avisa.
        let clave2 = format!("solicitudes/{id2}/{}.jpg", Uuid::new_v4());
        solicitudes.lock().unwrap().push(solicitud_remota(id2, &clave2));
        assert_eq!(sincronizar(&pool, &uploads).await.unwrap(), 1);
        assert_eq!(SolicitudRepository::contar_no_vistas(&pool).await.unwrap(), 1);

        // Repetir la pasada no duplica ni vuelve a contar como nueva.
        assert_eq!(sincronizar(&pool, &uploads).await.unwrap(), 0);
        assert_eq!(SolicitudRepository::contar_no_vistas(&pool).await.unwrap(), 1);

        SolicitudRepository::marcar_vistas(&pool).await.unwrap();
        assert_eq!(SolicitudRepository::contar_no_vistas(&pool).await.unwrap(), 0);

        sqlx::query("DELETE FROM solicitudes WHERE id = ANY($1)")
            .bind(vec![id1, id2])
            .execute(&pool)
            .await
            .unwrap();
        let _ = tokio::fs::remove_dir_all(&dir).await;
    }
}
