use crate::{handlers, metrics, state::AppState};
use axum::{middleware, routing::get, Router};
use tower_http::trace::TraceLayer;
use tracing::Level;

pub fn build_router(state: AppState) -> Router {
    let metrics_state = state.metrics.clone();

    Router::new()
        .route("/health", get(handlers::health))
        .route("/api/item/{id}", get(handlers::get_item))
        .route("/metrics", get(handlers::metrics_endpoint))
        // HTTP request logging
        .layer(
            TraceLayer::new_for_http().make_span_with(|req: &axum::http::Request<_>| {
                // Note: MatchedPath is not always available at this point yet, but it's still useful.
                tracing::span!(
                    Level::INFO,
                    "http_request",
                    method = %req.method(),
                    uri = %req.uri(),
                )
            }),
        )
        // Metrics (uses MatchedPath to avoid cardinality explosion)
        .layer(middleware::from_fn_with_state(
            metrics_state,
            metrics::metrics_middleware,
        ))
        .with_state(state)
}
