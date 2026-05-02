pub mod admin;
pub mod analytics;
pub mod ask;
pub mod casebuilder;
pub mod graph;
pub mod health;
pub mod home;
pub mod qc;
pub mod rules;
pub mod search;
pub mod sidebar;
pub mod sources;
pub mod stats;
pub mod statutes;

use crate::state::AppState;
use axum::Router;

pub fn create_routes() -> Router<AppState> {
    Router::new()
        .merge(admin::routes())
        .merge(sidebar::routes())
        .merge(casebuilder::routes())
        .merge(rules::routes())
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
        .route("/sources", axum::routing::get(sources::list_sources))
        .route(
            "/sources/:source_id",
            axum::routing::get(sources::get_source),
        )
        .route("/statutes", axum::routing::get(statutes::list_statutes))
        .route(
            "/statutes/:citation",
            axum::routing::get(statutes::get_statute),
        )
        .route(
            "/statutes/:citation/page",
            axum::routing::get(statutes::get_statute_page),
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
            "/statutes/:citation/chunks",
            axum::routing::get(statutes::get_chunks),
        )
        .route(
            "/provisions/:id",
            axum::routing::get(statutes::get_provision),
        )
        .route(
            "/graph/neighborhood",
            axum::routing::get(graph::get_neighborhood),
        )
        .route("/graph/full", axum::routing::get(graph::get_full))
        .route("/graph/path", axum::routing::get(graph::get_path))
        .route("/qc/summary", axum::routing::get(qc::get_qc_summary))
        .route("/qc/runs", axum::routing::post(qc::run_qc))
        .route(
            "/qc/reports/latest",
            axum::routing::get(qc::get_latest_report),
        )
        .route("/ask", axum::routing::post(ask::ask))
}
