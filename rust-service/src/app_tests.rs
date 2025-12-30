use crate::{app::build_router, state::AppState};
use axum::body::Body;
use axum::http::Request;
use tower::ServiceExt; // for `oneshot`

#[tokio::test]
async fn health_returns_ok() {
    let app = build_router(AppState::for_tests());

    let res = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), axum::http::StatusCode::OK);

    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(&bytes[..], b"ok");
}

#[tokio::test]
async fn item_route_exists() {
    let app = build_router(AppState::for_tests());

    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/item/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(res.status().is_server_error() || res.status().is_success());
}
