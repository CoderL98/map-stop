mod auth;
mod config;
mod db;
mod error;
mod geo;
mod routes;
mod routing;

use std::sync::Arc;

use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::auth::AppState;
use crate::config::Config;
use crate::routing::RoadGraph;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "map_stop_api=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cfg = Config::from_env();
    let pool = db::init_pool(&cfg.database_url).await.map_err(|e| anyhow::anyhow!(e.message))?;
    db::seed_admin(&pool, &cfg)
        .await
        .map_err(|e| anyhow::anyhow!(e.message))?;
    db::seed_demo_points(&pool)
        .await
        .map_err(|e| anyhow::anyhow!(e.message))?;

    tracing::info!("building demo road graph (Hangzhou)...");
    let graph = Arc::new(RoadGraph::hangzhou_demo());
    let (a, b, c, d) = graph.bounds();
    tracing::info!("demo bounds: ({a},{b}) – ({c},{d})");

    let http = reqwest::Client::builder()
        .user_agent("map-stop/0.1")
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .expect("http client");
    let state = AppState {
        pool,
        jwt_secret: cfg.jwt_secret.clone(),
        graph,
        http,
        geocode_limiter: crate::routes::geocode::GeocodeLimiter::default(),
    };

    let cors = if cfg.cors_origin == "*" {
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any)
    } else {
        CorsLayer::new()
            .allow_origin(
                cfg.cors_origin
                    .parse::<axum::http::HeaderValue>()
                    .unwrap_or_else(|_| axum::http::HeaderValue::from_static("*")),
            )
            .allow_methods(Any)
            .allow_headers(Any)
    };

    let app = Router::new()
        .nest("/api", routes::api_router(state))
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(&cfg.listen_addr).await?;
    tracing::info!("listening on {}", cfg.listen_addr);
    axum::serve(listener, app).await?;
    Ok(())
}
