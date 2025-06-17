//! Matrixon Web3 Benchmarks
//!
//! Author: arkSong (arksong2018@gmail.com)
//! Date: 2025-06-15
//! Version: 0.1.0
//!
//! Benchmark tests for Web3 functionality in Matrixon.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use matrixon_web3::{client::Web3Client, wallet::Wallet};
use web3::transports::Http;

fn wallet_creation_benchmark(c: &mut Criterion) {
    c.bench_function("wallet_creation", |b| {
        b.iter(|| {
            let _ = black_box(Wallet::generate());
        });
    });
}

fn client_initialization_benchmark(c: &mut Criterion) {
    c.bench_function("client_initialization", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| async {
                let transport = Http::new("http://localhost:8545").unwrap();
                let _ = black_box(Web3Client::new(transport));
            });
    });
}

criterion_group!(
    benches,
    wallet_creation_benchmark,
    client_initialization_benchmark
);
criterion_main!(benches);
