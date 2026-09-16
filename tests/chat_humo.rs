//! Humo 169A-4 del chat con atención humana (ignorado por defecto).
//! Ejecutar con la BD de rama migrada:
//! `DATABASE_URL=postgres://postgres@localhost:5432/glory_backend_inmobiliaria
//!  cargo test --test chat_humo -- --ignored --nocapture`
//! Verifica el flujo HTTP completo: visitante → bandeja staff → respuesta
//! humana (toma el hilo) → config → contacto. Limpia sus filas al final.

use glory_backend::config::AppConfig;
use glory_backend::services::Claims;
use uuid::Uuid;

fn base() -> String {
    std::env::var("DATABASE_URL").expect("humo: falta DATABASE_URL")
}

async fn servidor() -> (String, sqlx::PgPool) {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&base())
        .await
        .unwrap();
    let config = AppConfig {
        database_url: base(),
        jwt_secret: "humo-169A4-secreto-local-12345678".to_string(),
        host: "127.0.0.1".to_string(),
        port: 0,
        upload_dir: r"C:\tmp\humo169A4-uploads-test".to_string(),
    };
    let app = glory_backend::handlers::create_router(pool.clone(), config.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (url, pool)
}

fn jwt_staff(secreto: &str) -> String {
    let exp = usize::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 3600,
    )
    .unwrap();
    let claims = Claims {
        sub: Uuid::new_v4(),
        exp,
    };
    jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &claims,
        &jsonwebtoken::EncodingKey::from_secret(secreto.as_bytes()),
    )
    .unwrap()
}

async fn limpiar(pool: &sqlx::PgPool, sid: &str) {
    sqlx::query("DELETE FROM agent_outbox WHERE payload->>'session_id' = $1")
        .bind(sid)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM agent_messages WHERE session_id = $1::UUID")
        .bind(sid)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM agent_response_cycles WHERE session_id = $1::UUID")
        .bind(sid)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM agent_sessions WHERE id = $1::UUID")
        .bind(sid)
        .execute(pool)
        .await
        .unwrap();
}

#[ignore = "humo 169A-4: exige DATABASE_URL de rama y muta sus tablas de chat"]
#[tokio::test]
async fn visitante_escribe_y_staff_toma_el_hilo() {
    let (url, pool) = servidor().await;
    let cliente = reqwest::Client::new();
    let sid = Uuid::new_v4().to_string();
    let staff = jwt_staff("humo-169A4-secreto-local-12345678");

    // 1. Visitante escribe (IA degradada sin API key: persiste, sin reply).
    let m: serde_json::Value = cliente
        .post(format!("{url}/api/agent/messages"))
        .json(&serde_json::json!({"session_id": sid, "body": "Hola, busco apartamento"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(m["sequence_num"], 1);

    // 2. Bandeja staff ve la sesión abierta.
    let sesiones: serde_json::Value = cliente
        .get(format!("{url}/api/admin/agent/sesiones"))
        .bearer_auth(&staff)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(sesiones.as_array().unwrap().iter().any(|s| s["id"] == sid));

    // 3. Staff responde: toma el hilo (ai_enabled=false).
    let r: serde_json::Value = cliente
        .post(format!("{url}/api/admin/agent/sesiones/{sid}/mensajes"))
        .bearer_auth(&staff)
        .json(&serde_json::json!({"body": "Buenas, le atiende un humano"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(r["ok"], true);
    let sesiones2: serde_json::Value = cliente
        .get(format!("{url}/api/admin/agent/sesiones"))
        .bearer_auth(&staff)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let mia = sesiones2
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == sid)
        .unwrap();
    assert_eq!(mia["ai_enabled"], false);
    limpiar(&pool, &sid).await;
}

#[ignore = "humo 169A-4: exige DATABASE_URL de rama y muta sus tablas de chat"]
#[tokio::test]
async fn config_y_contacto_redondos() {
    let (url, pool) = servidor().await;
    let cliente = reqwest::Client::new();
    let sid = Uuid::new_v4().to_string();
    let staff = jwt_staff("humo-169A4-secreto-local-12345678");

    // 4. Config: guarda y lee de vuelta.
    cliente
        .put(format!("{url}/api/admin/agent/config"))
        .bearer_auth(&staff)
        .json(&serde_json::json!({"whatsapp_admin": "+34600000000"}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    let cfg: serde_json::Value = cliente
        .get(format!("{url}/api/admin/agent/config"))
        .bearer_auth(&staff)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(cfg["whatsapp_admin"], "+34600000000");

    // 5. Clave no editable → 400.
    let mala = cliente
        .put(format!("{url}/api/admin/agent/config"))
        .bearer_auth(&staff)
        .json(&serde_json::json!({"jwt_secret": "x"}))
        .send()
        .await
        .unwrap();
    assert_eq!(mala.status(), 400);

    // 6. Visitante deja contacto por endpoint público.
    let c: serde_json::Value = cliente
        .post(format!("{url}/api/agent/sesiones/{sid}/contacto"))
        .json(&serde_json::json!({"nombre": "Humo", "telefono": "+34611111111"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(c["ok"], true);

    // Limpieza: filas del humo + clave de config tocada.
    limpiar(&pool, &sid).await;
    sqlx::query("DELETE FROM agent_config WHERE key = 'whatsapp_admin'")
        .execute(&pool)
        .await
        .unwrap();
}
