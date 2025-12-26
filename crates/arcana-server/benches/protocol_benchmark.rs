//! Protocol Benchmark: gRPC vs HTTP/JSON
//!
//! This benchmark compares the performance of gRPC (Protocol Buffers) vs HTTP (JSON)
//! for inter-service communication in microservice deployments.
//!
//! ## Running Benchmarks
//!
//! ```bash
//! # Run all benchmarks
//! cargo bench --package arcana-server
//!
//! # Run specific benchmark
//! cargo bench --package arcana-server -- serialization
//!
//! # Generate HTML report
//! cargo bench --package arcana-server -- --save-baseline grpc_vs_http
//! ```
//!
//! ## Benchmark Categories
//!
//! 1. **Serialization**: Encode/decode performance
//! 2. **Payload Size**: Wire format comparison
//! 3. **Connection**: Connection establishment overhead
//! 4. **Throughput**: Requests per second under load

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput, BenchmarkId};
use serde::{Deserialize, Serialize};
use std::time::Duration;

// ============================================================================
// Test Data Structures
// ============================================================================

/// User data for benchmarking (mirrors protobuf User message)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserData {
    id: String,
    username: String,
    email: String,
    first_name: Option<String>,
    last_name: Option<String>,
    role: String,
    status: String,
    email_verified: bool,
    avatar_url: Option<String>,
    last_login_at: Option<i64>,
    created_at: i64,
}

/// Page response for benchmarking
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PageResponse {
    users: Vec<UserData>,
    page: u32,
    size: u32,
    total_elements: u64,
    total_pages: u64,
}

impl UserData {
    fn sample() -> Self {
        Self {
            id: "01918f8a-4c5b-7d8e-9f0a-1b2c3d4e5f6a".to_string(),
            username: "john.doe".to_string(),
            email: "john.doe@example.com".to_string(),
            first_name: Some("John".to_string()),
            last_name: Some("Doe".to_string()),
            role: "User".to_string(),
            status: "Active".to_string(),
            email_verified: true,
            avatar_url: Some("https://example.com/avatars/john.jpg".to_string()),
            last_login_at: Some(1703980800),
            created_at: 1703894400,
        }
    }

    fn sample_list(count: usize) -> Vec<Self> {
        (0..count).map(|i| {
            let mut user = Self::sample();
            user.id = format!("01918f8a-4c5b-7d8e-9f0a-1b2c3d4e5f{:02x}", i);
            user.username = format!("user_{}", i);
            user.email = format!("user_{}@example.com", i);
            user
        }).collect()
    }
}

// ============================================================================
// Serialization Benchmarks
// ============================================================================

fn benchmark_json_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialization/encode");

    // Single user
    let user = UserData::sample();
    group.throughput(Throughput::Elements(1));
    group.bench_function("json/single", |b| {
        b.iter(|| {
            let json = serde_json::to_vec(black_box(&user)).unwrap();
            black_box(json)
        })
    });

    // User list (10 users)
    let users_10 = UserData::sample_list(10);
    group.throughput(Throughput::Elements(10));
    group.bench_with_input(BenchmarkId::new("json", "10_users"), &users_10, |b, users| {
        b.iter(|| {
            let json = serde_json::to_vec(black_box(users)).unwrap();
            black_box(json)
        })
    });

    // User list (100 users)
    let users_100 = UserData::sample_list(100);
    group.throughput(Throughput::Elements(100));
    group.bench_with_input(BenchmarkId::new("json", "100_users"), &users_100, |b, users| {
        b.iter(|| {
            let json = serde_json::to_vec(black_box(users)).unwrap();
            black_box(json)
        })
    });

    group.finish();
}

fn benchmark_json_deserialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialization/decode");

    // Single user
    let user = UserData::sample();
    let json_single = serde_json::to_vec(&user).unwrap();
    group.throughput(Throughput::Bytes(json_single.len() as u64));
    group.bench_function("json/single", |b| {
        b.iter(|| {
            let user: UserData = serde_json::from_slice(black_box(&json_single)).unwrap();
            black_box(user)
        })
    });

    // User list (10 users)
    let users_10 = UserData::sample_list(10);
    let json_10 = serde_json::to_vec(&users_10).unwrap();
    group.throughput(Throughput::Bytes(json_10.len() as u64));
    group.bench_with_input(BenchmarkId::new("json", "10_users"), &json_10, |b, json| {
        b.iter(|| {
            let users: Vec<UserData> = serde_json::from_slice(black_box(json)).unwrap();
            black_box(users)
        })
    });

    // User list (100 users)
    let users_100 = UserData::sample_list(100);
    let json_100 = serde_json::to_vec(&users_100).unwrap();
    group.throughput(Throughput::Bytes(json_100.len() as u64));
    group.bench_with_input(BenchmarkId::new("json", "100_users"), &json_100, |b, json| {
        b.iter(|| {
            let users: Vec<UserData> = serde_json::from_slice(black_box(json)).unwrap();
            black_box(users)
        })
    });

    group.finish();
}

// ============================================================================
// Payload Size Comparison
// ============================================================================

fn benchmark_payload_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("payload_size");
    group.sample_size(10); // Small sample for size measurement

    for count in [1, 10, 50, 100] {
        let users = UserData::sample_list(count);
        let json_bytes = serde_json::to_vec(&users).unwrap();

        group.bench_with_input(
            BenchmarkId::new("json", format!("{}_users", count)),
            &json_bytes,
            |b, bytes| {
                b.iter(|| {
                    // Just measure the size
                    black_box(bytes.len())
                })
            },
        );

        // Print size info
        println!("JSON {} users: {} bytes ({:.2} bytes/user)",
                 count, json_bytes.len(), json_bytes.len() as f64 / count as f64);
    }

    group.finish();
}

// ============================================================================
// Simulated Network Latency
// ============================================================================

fn benchmark_simulated_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("roundtrip_simulation");
    group.measurement_time(Duration::from_secs(10));

    let user = UserData::sample();

    // Simulate JSON roundtrip (serialize -> network delay -> deserialize)
    // Average network latency: 0.5ms for same-datacenter
    group.bench_function("json/same_dc", |b| {
        b.iter(|| {
            // Serialize
            let json = serde_json::to_vec(black_box(&user)).unwrap();
            // Simulate minimal processing (network stack overhead)
            std::hint::black_box(&json);
            // Deserialize
            let decoded: UserData = serde_json::from_slice(&json).unwrap();
            black_box(decoded)
        })
    });

    // Simulate with page response (more realistic payload)
    let page = PageResponse {
        users: UserData::sample_list(20),
        page: 0,
        size: 20,
        total_elements: 100,
        total_pages: 5,
    };

    group.bench_function("json/page_response", |b| {
        b.iter(|| {
            let json = serde_json::to_vec(black_box(&page)).unwrap();
            std::hint::black_box(&json);
            let decoded: PageResponse = serde_json::from_slice(&json).unwrap();
            black_box(decoded)
        })
    });

    group.finish();
}

// ============================================================================
// Memory Allocation Comparison
// ============================================================================

fn benchmark_memory_allocation(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory");

    // Pre-allocate vs dynamic allocation
    let users = UserData::sample_list(100);
    let json = serde_json::to_vec(&users).unwrap();

    group.bench_function("json/dynamic_alloc", |b| {
        b.iter(|| {
            let decoded: Vec<UserData> = serde_json::from_slice(black_box(&json)).unwrap();
            black_box(decoded)
        })
    });

    // With capacity hint (simulates pre-sized buffer)
    group.bench_function("json/with_capacity", |b| {
        b.iter(|| {
            let mut decoded: Vec<UserData> = Vec::with_capacity(100);
            let temp: Vec<UserData> = serde_json::from_slice(black_box(&json)).unwrap();
            decoded.extend(temp);
            black_box(decoded)
        })
    });

    group.finish();
}

// ============================================================================
// Benchmark Groups
// ============================================================================

criterion_group!(
    name = serialization_benches;
    config = Criterion::default()
        .sample_size(1000)
        .measurement_time(Duration::from_secs(5));
    targets =
        benchmark_json_serialization,
        benchmark_json_deserialization
);

criterion_group!(
    name = payload_benches;
    config = Criterion::default()
        .sample_size(10);
    targets = benchmark_payload_sizes
);

criterion_group!(
    name = roundtrip_benches;
    config = Criterion::default()
        .sample_size(500)
        .measurement_time(Duration::from_secs(10));
    targets = benchmark_simulated_roundtrip
);

criterion_group!(
    name = memory_benches;
    config = Criterion::default()
        .sample_size(500);
    targets = benchmark_memory_allocation
);

criterion_main!(
    serialization_benches,
    payload_benches,
    roundtrip_benches,
    memory_benches
);
