//! Benchmark tests for Matrixon API performance
//!
//! Measures:
//! - Route handling throughput
//! - Middleware overhead
//! - Error handling performance

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use matrixon_api::{routes::create_api_routes, middleware::add_middleware};
use axum::{Router, body::Body};
use hyper::{Request, Response};
use tower::ServiceExt;

async fn make_request(router: &Router, path: &str) -> Response<Body> {
    router
        .clone()
        .oneshot(
            Request::builder()
                .uri(path)
                .body(Body::empty())
                .unwrap()
        )
        .await
        .unwrap()
}

fn bench_routes(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let router = create_api_routes();
    let router = add_middleware(router);

    c.bench_function("health_check", |b| {
        b.to_async(&rt)
            .iter(|| make_request(&router, "/health"))
    });

    c.bench_function("api_version", |b| {
        b.to_async(&rt)
            .iter(|| make_request(&router, "/api/v1/version"))
    });
}

criterion_group!(benches, bench_routes);
criterion_main!(benches);
