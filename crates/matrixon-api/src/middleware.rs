//! Middleware components for Matrixon API
//! 
//! This module provides middleware components for the Matrixon API server.

use axum::{
    body::Body,
    extract::Request,
    http::Response,
    middleware::Next,
    response::IntoResponse,
};
use tower_http::{
    trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer},
    cors::CorsLayer,
    classify::{ServerErrorsAsFailures, SharedClassifier},
};
use tracing::debug;

/// Authentication middleware
pub async fn auth(
    req: Request<Body>,
    next: Next,
) -> Result<Response<Body>, Response<Body>> {
    // TODO: Implement actual authentication
    debug!("🔒 Authenticating request");
    Ok(next.run(req).await)
}

/// Logging middleware
pub async fn logging_middleware(
    request: Request<Body>,
    next: Next,
) -> impl IntoResponse {
    let start = std::time::Instant::now();
    let path = request.uri().path().to_owned();
    let method = request.method().clone();

    let response = next.run(request).await;

    let latency = start.elapsed();
    let status = response.status();

    debug!(
        method = %method,
        path = %path,
        status = %status,
        latency = ?latency,
        "Request completed"
    );

    response
}

/// Rate limiting middleware
pub async fn rate_limit(
    req: Request<Body>,
    next: Next,
) -> Result<Response<Body>, Response<Body>> {
    // TODO: Implement actual rate limiting
    debug!("⏱️ Rate limiting request");
    Ok(next.run(req).await)
}

/// Create common middleware layers
pub fn create_middleware_stack() -> (
    TraceLayer<SharedClassifier<ServerErrorsAsFailures>>,
    CorsLayer,
) {
    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(DefaultMakeSpan::new().level(tracing::Level::DEBUG))
        .on_request(DefaultOnRequest::new().level(tracing::Level::DEBUG))
        .on_response(DefaultOnResponse::new().level(tracing::Level::DEBUG));

    let cors_layer = tower_http::cors::CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_methods(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any);

    (trace_layer, cors_layer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        routing::get,
        Router,
    };
    use hyper::StatusCode;

    use tower::ServiceExt;

    #[tokio::test]
    async fn test_middleware() {
        let app = Router::new()
            .route("/", get(|| async { "Hello, World!" }))
            .layer(tower::ServiceBuilder::new()
                .layer(tower_http::trace::TraceLayer::new_for_http()));
                
        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
            
        assert_eq!(response.status(), StatusCode::OK);
    }
}
