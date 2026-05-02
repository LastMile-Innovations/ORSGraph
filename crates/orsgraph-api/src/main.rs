use axum::{Router, http::Method, routing::get};
use orsgraph_api::{config::ApiConfig, middleware, routes, state::AppState};
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = ApiConfig::from_env()?;

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| config.log_level.clone().into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting orsgraph-api v0.1.0");
    tracing::info!("Connecting to Neo4j at {}", config.neo4j_uri);

    let state = AppState::new(config.clone()).await?;

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers(Any);

    let app = Router::new()
        .route("/", get(|| async { "ORSGraph API v0.1.0" }))
        .nest("/api/v1", routes::create_routes())
        .layer(cors)
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::optional_api_key_middleware,
        ))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(config.socket_addr()).await?;
    tracing::info!("API server listening on {}", config.socket_addr());

    axum::serve(listener, app).await?;

    Ok(())
}
