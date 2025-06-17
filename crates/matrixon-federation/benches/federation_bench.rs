//! Matrixon Federation Benchmarks
//!
//! Author: arkSong <arksong2018@gmail.com>
//! Version: 0.11.0-alpha
//! Date: 2025-06-15
//!
//! Benchmark tests for Matrixon Federation performance characteristics

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use matrixon_federation::{
    FederationConfig, FederationServer, 
    traits::{MockServerDiscovery, MockEventExchange, MockStateResolution, MockRequestHandler},
    FederationRequest
};
use std::sync::Arc;
use tokio::runtime::Runtime;

fn config_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("config");
    group.sample_size(1000);
    group.measurement_time(std::time::Duration::from_secs(10));

    group.bench_function("create_config", |b| {
        b.iter(|| {
            let config = FederationConfig::new(black_box("benchmark.server"));
            black_box(config);
        })
    });

    group.bench_function("validate_config", |b| {
        let config = FederationConfig::new("benchmark.server");
        b.iter(|| {
            black_box(config.validate()).unwrap();
        })
    });
}

fn server_benchmark(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("server");
    group.throughput(Throughput::Elements(1));
    group.sample_size(500);
    group.measurement_time(std::time::Duration::from_secs(15));

    group.bench_function("start_server", |b| {
        b.to_async(&rt).iter(|| async {
            let config = FederationConfig::new("benchmark.server");
            let discovery = Arc::new(MockServerDiscovery::new());
            let event_exchange = Arc::new(MockEventExchange::new());
            let state_resolution = Arc::new(MockStateResolution::new());
            let request_handler = Arc::new(MockRequestHandler::new());
            
            let server = FederationServer::new(
                config,
                discovery,
                event_exchange,
                state_resolution,
                request_handler,
            ).unwrap();
            
            black_box(server.start().await).unwrap();
        })
    });

    group.bench_function("handle_request", |b| {
        b.to_async(&rt).iter(|| async {
            let config = FederationConfig::new("benchmark.server");
            let discovery = Arc::new(MockServerDiscovery::new());
            let event_exchange = Arc::new(MockEventExchange::new());
            let state_resolution = Arc::new(MockStateResolution::new());
            let mut request_handler = MockRequestHandler::new();
            
            request_handler
                .expect_validate_request()
                .returning(|_| Ok(()));
            
            request_handler
                .expect_process_request()
                .returning(|_| Ok(FederationResponse::default()));
            
            let server = FederationServer::new(
                config,
                discovery,
                event_exchange,
                state_resolution,
                Arc::new(request_handler),
            ).unwrap();
            
            let request = FederationRequest::default();
            black_box(server.handle_request(request).await).unwrap();
        })
    });
}

fn concurrency_benchmark(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("concurrency");
    group.throughput(Throughput::Elements(100));
    group.sample_size(100);
    group.measurement_time(std::time::Duration::from_secs(30));

    group.bench_function("concurrent_requests", |b| {
        b.to_async(&rt).iter(|| async {
            let config = FederationConfig::new("benchmark.server");
            let discovery = Arc::new(MockServerDiscovery::new());
            let event_exchange = Arc::new(MockEventExchange::new());
            let state_resolution = Arc::new(MockStateResolution::new());
            let mut request_handler = MockRequestHandler::new();
            
            request_handler
                .expect_validate_request()
                .returning(|_| Ok(()));
            
            request_handler
                .expect_process_request()
                .returning(|_| Ok(FederationResponse::default()));
            
            let server = Arc::new(FederationServer::new(
                config,
                discovery,
                event_exchange,
                state_resolution,
                Arc::new(request_handler),
            ).unwrap());
            
            let mut handles = vec![];
            for _ in 0..100 {
                let server = server.clone();
                handles.push(tokio::spawn(async move {
                    let request = FederationRequest::default();
                    black_box(server.handle_request(request).await).unwrap();
                }));
            }
            
            futures::future::join_all(handles).await;
        })
    });
}

criterion_group!{
    name = benches;
    config = Criterion::default()
        .with_profiler(criterion::profiler::FlamegraphProfiler::new(100))
        .sample_size(1000);
    targets = config_benchmark, server_benchmark, concurrency_benchmark
}

criterion_main!(benches);
