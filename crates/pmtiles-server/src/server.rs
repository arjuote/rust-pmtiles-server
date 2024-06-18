use crate::config::ServerConfig;
use crate::error::APIError;
use crate::routes::create_router;
use anyhow::Error;
use aws_sdk_s3 as s3;
use axum::response::Response;
use axum::{body::Body, http};
use http::Request;
use pmtiles_core::cache::InMemoryCache;
use pmtiles_core::fetcher::{Fetcher, S3OrLocalFetcher};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tower_http::trace;
use tower_http::{compression::CompressionLayer, cors::CorsLayer, trace::TraceLayer};
use tracing::Level;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};

// TODO:
//       - add endpoints fetching sprites
//       - put lambda specific stuff in server behind a feature flag
//       - put s3 specific stuff in pmtiles behind a feature flag
//       - test as MVTLayer lambda
//       - test as mapbox style lambda
//       - fix bug with real lambda dropping off most of the binary payload if no compression is used
//          - this is likely related to the strangeness of lambda binary data handling. Investigate cargo-lambda docs for clues.

#[derive(Clone)]
pub struct AppState {
    pub fetcher: Arc<S3OrLocalFetcher>,
    pub cache: Arc<InMemoryCache>,
    pub config: Arc<ServerConfig>,
}

pub async fn get_config<T: Fetcher>(client: &T, path: &str) -> Result<ServerConfig, APIError> {
    tracing::info!("reading config from {}", path);
    let (data, _) = client.get_data(path).await?;
    let cfg: ServerConfig = serde_json::from_slice(&data).map_err(|err| {
        tracing::error!("failed to deserialize configuration: {}", err);
        APIError::Internal("invalid configuration".to_string())
    })?;
    Ok(cfg)
}

pub async fn serve(serve: bool, listen_addr: &str, port: u32) -> Result<(), Error> {
    // Trace every request
    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(
            trace::DefaultMakeSpan::new()
                .include_headers(true)
                .level(Level::INFO),
        )
        .on_response(
            |res: &Response<Body>, duration: Duration, span: &tracing::Span| {
                let status = res.status();
                let duration_ms = duration.as_millis();
                tracing::info!(parent: span, status = %status, duration = %duration_ms);
            },
        );

    let lyr = tracing_subscriber::fmt::Layer::default()
        .with_file(true)
        .with_line_number(true);
    Registry::default()
        .with(lyr)
        .with(EnvFilter::from("info"))
        .init();
    tracing::info!("Starting server");

    // Set up CORS
    let cors_layer = CorsLayer::new()
        .allow_headers(tower_http::cors::Any)
        .allow_methods(tower_http::cors::Any)
        .allow_origin(tower_http::cors::Any);

    let config = aws_config::from_env().load().await;

    tracing::info!("Setting up state");

    // Create an S3 client and fetcher
    let client = s3::Client::new(&config);
    let fetcher = S3OrLocalFetcher::new(client);
    let default_path = "./config.json";
    let cfg_path = std::env::var("CONFIG_PATH").unwrap_or_else(|_| default_path.into());
    let config = get_config(&fetcher, &cfg_path).await?;

    let state = AppState {
        fetcher: Arc::new(fetcher),
        cache: Arc::new(InMemoryCache::new()),
        config: Arc::new(config),
    };

    let app = create_router(state)
        .layer(trace_layer)
        .layer(cors_layer)
        .layer(CompressionLayer::new().gzip(true).br(true));

    if serve {
        tracing::info!(
            "Running pmtiles-server as server at {}:{}",
            listen_addr,
            port
        );
        let listen_addr = format!("{}:{}", listen_addr, port);
        let addr: std::net::SocketAddr = listen_addr.parse().expect("invalid listen address");
        let listener = TcpListener::bind(addr).await.unwrap();
        axum::serve(listener, app.into_make_service())
            .await
            .map_err(|err| {
                tracing::error!("{}", err);
                Error::msg(err)
            })
    } else {
        tracing::info!("Running pmtiles-server as a lambda function");
        let app = tower::ServiceBuilder::new()
            .layer(axum_aws_lambda::LambdaLayer::default())
            .service(app);

        lambda_http::run(app).await.map_err(Error::msg)
    }
}
