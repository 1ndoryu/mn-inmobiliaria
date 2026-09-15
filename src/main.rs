use glory_backend::config::AppConfig;
use glory_backend::handlers;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                tracing_subscriber::EnvFilter::new("glory_backend=debug,tower_http=debug")
            }),
        )
        .init();

    let config = AppConfig::from_env()?;

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(10)
        .min_connections(2)
        .connect(&config.database_url)
        .await?;

    sqlx::migrate!().run(&pool).await?;

    tokio::fs::create_dir_all(&config.upload_dir).await?;
    tracing::info!("Uploads en {}", config.upload_dir);

    let addr = format!("{}:{}", config.host, config.port);
    tracing::info!("Servidor iniciando en {addr}");
    tracing::info!("Swagger UI disponible en http://{addr}/swagger-ui/");

    let app = handlers::create_router(pool, config);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
