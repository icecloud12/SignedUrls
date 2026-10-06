use dotenv::dotenv;
use signed_urls::{AppState, Config};

#[tokio::main]
async fn main() {
    dotenv().ok();
    let subscriber = tracing_subscriber::fmt()
        .compact()
        .with_file(true)
        .with_line_number(true)
        .with_target(true)
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    let config = Config::from_env().unwrap_or_else(|e| {
        tracing::error!("Invalid configuration:\n{e}");
        std::process::exit(1);
    });
    let db = signed_urls::connect(&config.database_uri, &config.database_name)
        .await
        .unwrap_or_else(|e| {
            tracing::error!("Unable to set up database client: {e}");
            std::process::exit(1);
        });
    let tls = signed_urls::rustls_config(&config).await.unwrap_or_else(|e| {
        tracing::error!("{e}");
        std::process::exit(1);
    });

    let addr = config.address;
    let app = signed_urls::app(AppState::new(db, config));
    tracing::info!("listening on https://{addr}");
    axum_server::bind_rustls(addr, tls)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
