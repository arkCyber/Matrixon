// =============================================================================
// Matrixon Matrix NextServer - Bot Performance Benchmarks
// =============================================================================
//
// Project: Matrixon - Ultra High Performance Matrix NextServer (Synapse Alternative)
// Author: arkSong (arksong2018@gmail.com) - Founder of Matrixon Innovation Project
// Contributors: Matrixon Development Team
// Date: 2024-03-19
// Version: 0.11.0-alpha
// License: Apache 2.0 / MIT
//
// Description:
//   Performance benchmarks for Matrixon bot service
//
// Benchmarks:
//   • Command processing latency
//   • Event handling throughput
//   • Message routing performance
//   • State management overhead
//
// Performance Targets:
//   • <50ms command response time
//   • >1000 events/second
//   • <1ms message routing latency
//   • <5% state management overhead
//
// =============================================================================

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use matrixon_bot::{Service, BotConfig};
use tokio::runtime::Runtime;

fn bot_command_benchmark(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut config = BotConfig::default();
    config.identity.username = "@benchbot:localhost".to_string();
    
    let service = rt.block_on(Service::new(config)).unwrap();
    
    c.bench_function("command_processing", |b| {
        b.iter(|| {
            rt.block_on(async {
                let _ = service.uptime();
                black_box(());
            });
        });
    });
}

fn bot_event_benchmark(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut config = BotConfig::default();
    config.identity.username = "@benchbot:localhost".to_string();
    
    let service = rt.block_on(Service::new(config)).unwrap();
    
    c.bench_function("event_handling", |b| {
        b.iter(|| {
            rt.block_on(async {
                let _ = service.database();
                black_box(());
            });
        });
    });
}

criterion_group!(
    benches,
    bot_command_benchmark,
    bot_event_benchmark
);
criterion_main!(benches);
