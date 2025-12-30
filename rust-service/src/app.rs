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
        // Logs HTTP automáticos (latencia, status, método, etc.)
        .layer(
            TraceLayer::new_for_http().make_span_with(|req: &axum::http::Request<_>| {
                // Nota: MatchedPath aún no siempre está en este punto, pero igual es útil.
                tracing::span!(
                    Level::INFO,
                    "http_request",
                    method = %req.method(),
                    uri = %req.uri(),
                )
            }),
        )
        // Métricas (usa MatchedPath para no explotar cardinalidad)
        .layer(middleware::from_fn_with_state(
            metrics_state,
            metrics::metrics_middleware,
        ))
        .with_state(state)
}
