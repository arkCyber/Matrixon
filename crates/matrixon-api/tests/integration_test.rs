use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use matrixon_api::routes::create_router;
use tower::ServiceExt; // for `oneshot` method

#[tokio::test]
async fn test_health_check() {
    let router = create_router();

    let response = router
        .oneshot(
            Request::builder()
                .uri("/_matrix/client/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
