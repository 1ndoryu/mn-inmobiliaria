#![allow(clippy::cast_possible_truncation, clippy::too_many_lines)]
//! [159A-2] Import real del rescate a la API levantada in-process.
//! Ignorado por defecto: muta la BD de desarrollo. Ejecutar con:
//! ```sh
//! INMOBILIARIA_IMPORT_TEST=1 cargo test --test importar_rescate -- --ignored --nocapture
//! ```
//! Requiere Postgres local con `inmobiliaria_db` (lee `DATABASE_URL`/`JWT_SECRET` del
//! entorno o usa los valores de desarrollo).
//! `allow` acotado: el decoder base64 opera con valores <256 por construccion y el
//! test es un flujo lineal de importacion, no codigo de produccion.

use std::path::PathBuf;

use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

const RESCATE: &str = "../INMOBILIARIA/rescates/inmobiliaria-rescate-20260915-1205.json";

#[derive(Deserialize)]
struct AuthResponse {
    token: String,
}

#[derive(Deserialize)]
struct InmuebleCreado {
    id: Uuid,
    slug: String,
}

#[derive(Deserialize)]
struct PaginaAdmin {
    items: Vec<ItemAdmin>,
    total: i64,
}

#[derive(Deserialize)]
struct ItemAdmin {
    id: Uuid,
    fotos: Vec<Value>,
    copy: Option<Value>,
}

#[derive(Deserialize)]
struct PaginaPublica {
    total: i64,
}

/// Decodificador base64 estándar mínimo (evita una dependencia solo para el test)
fn decodificar_base64(input: &str) -> Vec<u8> {
    const TABLA: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut valores = [255u8; 256];
    for (i, c) in TABLA.iter().enumerate() {
        valores[*c as usize] = i as u8;
    }
    let limpio: Vec<u8> = input.bytes().filter(|b| *b != b'=').collect();
    let mut salida = Vec::with_capacity(limpio.len() * 3 / 4);
    for grupo in limpio.chunks(4) {
        let mut n: u32 = 0;
        for (i, b) in grupo.iter().enumerate() {
            n |= u32::from(valores[*b as usize]) << (18 - 6 * i);
        }
        salida.push((n >> 16) as u8);
        if grupo.len() > 2 {
            salida.push((n >> 8) as u8);
        }
        if grupo.len() > 3 {
            salida.push(n as u8);
        }
    }
    salida
}

fn data_url_a_bytes(data_url: &str) -> (&'static str, Vec<u8>) {
    let cuerpo = data_url
        .split_once(',')
        .expect("dataURL sin coma separadora")
        .1;
    assert!(
        data_url.starts_with("data:"),
        "se esperaba una dataURL, no una URL remota"
    );
    let bytes = decodificar_base64(cuerpo);
    // La extensión se deduce de los bytes reales: alguna mejorada declara en la
    // cabecera un formato distinto al contenido y el servidor exige coherencia.
    let extension = if bytes.starts_with(&[137, 80, 78, 71, 13, 10, 26, 10]) {
        "png"
    } else if bytes.starts_with(&[82, 73, 70, 70]) && bytes.get(8..12) == Some(b"WEBP") {
        "webp"
    } else if bytes.starts_with(&[255, 216, 255]) {
        "jpg"
    } else {
        panic!("magia de imagen desconocida en el rescate");
    };
    (extension, bytes)
}

#[tokio::test]
#[ignore = "destructivo: exige INMOBILIARIA_IMPORT_TEST=1 y muta la BD de desarrollo"]
async fn importar_rescate() {
    assert_eq!(
        std::env::var("INMOBILIARIA_IMPORT_TEST").as_deref(),
        Ok("1"),
        "test destructivo: exige INMOBILIARIA_IMPORT_TEST=1"
    );
    if std::env::var("DATABASE_URL").is_err() {
        // Credenciales de desarrollo (BD local, gitignored en .env real)
        std::env::set_var(
            "DATABASE_URL",
            "postgres://postgres:root@localhost:5432/inmobiliaria_db",
        );
    }
    if std::env::var("JWT_SECRET").is_err() {
        std::env::set_var("JWT_SECRET", "secreto-solo-para-import-test-12345678");
    }
    let dir_subidas = PathBuf::from(r"C:\tmp\inmobiliaria-uploads-test");
    if dir_subidas.exists() {
        tokio::fs::remove_dir_all(&dir_subidas).await.unwrap();
    }
    std::env::set_var("UPLOAD_DIR", dir_subidas.to_str().unwrap());
    std::env::set_var("HOST", "127.0.0.1");
    std::env::remove_var("PORT");

    let config = glory_backend::config::AppConfig::from_env().unwrap();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await
        .unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    // Parte de cero: usuarios e inmuebles de pruebas anteriores fuera
    sqlx::query("DELETE FROM fotos")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM inmuebles")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM users")
        .execute(&pool)
        .await
        .unwrap();

    let app = glory_backend::handlers::create_router(pool.clone(), config);
    // Puerto efimero: evita choques con servidores ajenos olvidados en local
    let oyente = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let puerto = oyente.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(oyente, app).await.unwrap();
    });

    let base = format!("http://127.0.0.1:{puerto}");
    let cliente = reqwest::Client::new();
    let rescate: Value =
        serde_json::from_str(&std::fs::read_to_string(RESCATE).unwrap()).expect("rescate");
    let valor = |v: &Value| {
        if let Some(s) = v.as_str() {
            serde_json::from_str(s).unwrap()
        } else {
            v.clone()
        }
    };
    let inmuebles = valor(&rescate["localStorage"]["inmobiliaria:inmuebles:v1"]);
    let fotos_por_inmueble: std::collections::HashMap<String, Vec<String>> = rescate["indexeddb"]
        ["fotos-inmueble"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            (
                r["inmuebleId"].as_str().unwrap().to_string(),
                r["fotos"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|f| f.as_str().unwrap().to_string())
                    .collect(),
            )
        })
        .collect();

    let auth: AuthResponse = cliente
        .post(format!("{base}/api/auth/register"))
        .json(&serde_json::json!({"email": "import@example.com", "password": "import-secreto-123"}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    let bearer = format!("Bearer {}", auth.token);

    let mut total_originales = 0;
    let mut total_mejoradas = 0;
    let mut con_copy = 0;
    let mut primer_id = None;
    let mut primera_url = None;
    for viejo in inmuebles.as_array().unwrap() {
        let mut payload = serde_json::json!({
            "titulo": viejo["titulo"], "descripcion": viejo["descripcion"],
            "ubicacion": viejo["ubicacion"], "precio": viejo["precio"],
            "tipo": viejo["tipo"], "operacion": viejo["operacion"],
            "habitaciones": viejo["habitaciones"], "banos": viejo["banos"],
            "metros": viejo["metros"], "metros_terreno": viejo["metrosTerreno"],
            "estado": viejo["estado"],
        });
        if viejo["copy"]["corta"].is_string() {
            payload["copy"] = serde_json::json!({
                "corta": viejo["copy"]["corta"], "larga": viejo["copy"]["larga"],
                "modelo": viejo["copy"]["modelo"], "actualizada_en": viejo["copy"]["actualizadaEn"],
            });
            con_copy += 1;
        }
        let creado: InmuebleCreado = cliente
            .post(format!("{base}/api/admin/inmuebles"))
            .header("Authorization", &bearer)
            .json(&payload)
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .json()
            .await
            .unwrap();
        assert!(!creado.slug.is_empty());
        if primer_id.is_none() {
            primer_id = Some(creado.id);
        }

        let originales = &fotos_por_inmueble[viejo["id"].as_str().unwrap()];
        for (i, data_url) in originales.iter().enumerate() {
            let (ext, bytes) = data_url_a_bytes(data_url);
            let foto: Value = cliente
                .post(format!(
                    "{base}/api/admin/fotos/upload?inmueble_id={}&filename=foto-{i}.{ext}&origen=original&orden={i}",
                    creado.id
                ))
                .header("Authorization", &bearer)
                .header("Content-Type", "application/octet-stream")
                .body(bytes)
                .send()
                .await
                .unwrap()
                .error_for_status()
                .unwrap()
                .json()
                .await
                .unwrap();
            total_originales += 1;
            if primera_url.is_none() {
                primera_url = Some(foto["url"].as_str().unwrap().to_string());
            }
        }
        for mejora in rescate["indexeddb"]["fotos-mejora"].as_array().unwrap() {
            if mejora["inmuebleId"] != viejo["id"]
                || mejora["estado"] != "lista"
                || !mejora["mejorada"].is_string()
            {
                continue;
            }
            let orden = originales
                .iter()
                .position(|o| o == mejora["original"].as_str().unwrap())
                .expect("mejorada sin original coincidente");
            let (ext, bytes) = data_url_a_bytes(mejora["mejorada"].as_str().unwrap());
            cliente
                .post(format!(
                    "{base}/api/admin/fotos/upload?inmueble_id={}&filename=mejorada-{orden}.{ext}&origen=mejorada&orden={orden}",
                    creado.id
                ))
                .header("Authorization", &bearer)
                .header("Content-Type", "application/octet-stream")
                .body(bytes)
                .send()
                .await
                .unwrap()
                .error_for_status()
                .unwrap();
            total_mejoradas += 1;
        }
    }

    assert_eq!(total_originales, 48, "originales importadas");
    assert_eq!(total_mejoradas, 34, "mejoradas importadas");
    assert_eq!(con_copy, 2, "inmuebles con copy");

    let admin: PaginaAdmin = cliente
        .get(format!("{base}/api/admin/inmuebles?page=1&per_page=100"))
        .header("Authorization", &bearer)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(admin.total, 5);
    assert_eq!(admin.items.iter().map(|i| i.fotos.len()).sum::<usize>(), 82);

    let publica: PaginaPublica = cliente
        .get(format!("{base}/api/public/inmuebles?page=1&per_page=100"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(publica.total, 0, "importados como borradores");

    // Publicar uno: visible en pública y su archivo se sirve
    cliente
        .patch(format!(
            "{base}/api/admin/inmuebles/{}/publicacion",
            primer_id.unwrap()
        ))
        .header("Authorization", &bearer)
        .json(&serde_json::json!({"publicado": true}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    let publica: PaginaPublica = cliente
        .get(format!("{base}/api/public/inmuebles?page=1&per_page=100"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(publica.total, 1);

    let url = primera_url.unwrap();
    let respuesta = cliente.get(format!("{base}{url}")).send().await.unwrap();
    let estado = respuesta.status();
    let cuerpo = respuesta.bytes().await.unwrap();
    println!("GET {url} -> {estado} ({} bytes)", cuerpo.len());
    assert_eq!(estado, 200);
    let archivo = cliente.get(format!("{base}{url}")).send().await.unwrap();
    assert_eq!(
        archivo.headers()["content-type"],
        "image/jpeg",
        "sirve la foto"
    );
    assert!(!archivo.bytes().await.unwrap().is_empty());

    // 404 ante clave ajena al formato
    let inexistente = cliente
        .get(format!("{base}/uploads/no-es-uuid/archivo.jpg"))
        .send()
        .await
        .unwrap();
    assert_eq!(inexistente.status(), 404);

    let _ = admin
        .items
        .iter()
        .find(|i| i.id == primer_id.unwrap())
        .unwrap();
    let copias = admin.items.iter().filter(|i| i.copy.is_some()).count();
    assert_eq!(copias, 2);
}
