//! Database performance benchmarks for Matrixon
//! 
//! Author: arkSong <arksong2018@gmail.com>
//! Date: 2025-06-15
//! Version: 0.1.0
//! 
//! This module contains benchmarks for critical database operations.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use matrixon_db::pool::DatabasePool;
use tokio::runtime::Runtime;

async fn setup_db() -> DatabasePool {
    // Initialize test database connection pool
    DatabasePool::new_test_pool().await.unwrap()
}

fn bench_insert(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let pool = rt.block_on(setup_db());

    c.bench_function("insert_event", |b| {
        b.to_async(&rt).iter(|| async {
            let event = TestEvent::new();
            black_box(pool.insert_event(&event).await.unwrap());
        })
    });
}

fn bench_query(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let pool = rt.block_on(setup_db());

    c.bench_function("query_events", |b| {
        b.to_async(&rt).iter(|| async {
            black_box(pool.query_events("test_room").await.unwrap());
        })
    });
}

criterion_group!(
    benches,
    bench_insert,
    bench_query
);
criterion_main!(benches);
