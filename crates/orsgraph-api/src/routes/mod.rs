pub mod analytics;
pub mod ask;
pub mod graph;
pub mod health;
pub mod home;
pub mod qc;
pub mod search;
pub mod stats;
pub mod statutes;

use crate::state::AppState;
use axum::Router;

pub fn create_routes() -> Router<AppState> {
    Router::new()
        .route("/home", axum::routing::get(home::get_home))
        .route(
            "/featured-statutes",
            axum::routing::get(home::featured_statutes),
        )
        .route(
            "/analytics/home",
            axum::routing::get(analytics::home_insights),
        )
        .route("/health", axum::routing::get(health::health))
        .route("/stats", axum::routing::get(stats::stats))
        .route("/search", axum::routing::get(search::search))
        .route("/search/open", axum::routing::get(search::open))
        .route("/search/suggest", axum::routing::get(search::suggest))
        .route("/statutes", axum::routing::get(statutes::list_statutes))
        .route(
            "/statutes/:citation",
            axum::routing::get(statutes::get_statute),
        )
        .route(
            "/statutes/:citation/provisions",
            axum::routing::get(statutes::get_provisions),
        )
        .route(
            "/statutes/:citation/citations",
            axum::routing::get(statutes::get_citations),
        )
        .route(
            "/statutes/:citation/semantics",
            axum::routing::get(statutes::get_semantics),
        )
        .route(
            "/statutes/:citation/history",
            axum::routing::get(statutes::get_history),
        )
        .route(
            "/provisions/:id",
            axum::routing::get(statutes::get_provision),
        )
        .route(
            "/graph/neighborhood",
            axum::routing::get(graph::get_neighborhood),
        )
        .route("/qc/summary", axum::routing::get(qc::get_qc_summary))
        .route("/ask", axum::routing::post(ask::ask))
}
