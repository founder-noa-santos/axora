# OPENAKTA Benchmarks

Performance benchmarks for the OPENAKTA API client and related components.

## Overview

This benchmark suite measures:

1. **API Overhead** - Proto serialization, network latency, circuit breaker checks
2. **Proto Conversion** - Internal ↔ Proto type conversions
3. **Regression Tests** - Performance regression detection

## Running Benchmarks

### Prerequisites

```bash
# Install flamegraph for CPU profiling
cargo install flamegraph

# Install cargo-criterion for better benchmark visualization
cargo install cargo-criterion
```

### Basic Benchmarks

```bash
cd aktacode/benchmarks

# Run all benchmarks
cargo bench

# Run specific benchmark suite
cargo bench --bench api-overhead
cargo bench --bench proto-conversion
cargo bench --bench regression-tests
```

### Advanced Options

```bash
# Run with custom sample size
cargo bench --bench api-overhead -- --sample-size 200

# Run with longer measurement time
cargo bench --bench api-overhead -- --measurement-time 60

# Run specific test (filter by name)
cargo bench --bench api-overhead -- --bench "proto_serialization"
```

### Using Criterion Cargo

```bash
# Better visualization with cargo-criterion
cargo criterion --bench api-overhead

# View results in browser
cargo criterion --bench api-overhead --message-format=json | criterion-cmp
```

## Benchmark Suites

### API Overhead (`api-overhead.rs`)

Measures overhead introduced by the API client layer:

- `proto_serialization` - Time to serialize ProviderRequest
- `proto_deserialization` - Time to deserialize ProviderResponse
- `circuit_breaker` - Circuit breaker check overhead
- `client_creation` - API client creation time
- `pool_acquisition` - Connection pool acquisition time
- `api_roundtrip` - Full API call (requires running server)
- `api_streaming` - Streaming API call (requires running server)
- `embedding_api` - Embedding API call (requires running server)

### Proto Conversion (`proto-conversion.rs`)

Measures conversion overhead between internal and proto types:

- `internal_to_proto` - ModelRequest → ProviderRequest
- `proto_to_internal` - ProviderResponse → ModelResponse
- `roundtrip_conversion` - Full conversion cycle
- `serialization_size` - Proto size for different message counts
- `field_preservation` - Verify all fields preserved during conversion

### Regression Tests (`regression-tests.rs`)

Performance regression detection tests:

- `test_proto_serialization_regression` - Proto serialization regression
- `test_proto_deserialization_regression` - Proto deserialization regression
- `test_api_client_overhead_regression` - API client overhead regression
- `test_circuit_breaker_regression` - Circuit breaker regression
- `test_conversion_overhead_regression` - Conversion overhead regression
- `test_memory_allocation_regression` - Memory allocation patterns
- `test_string_operations_regression` - String operations overhead

## Performance Baselines

**Current Baselines (Phase 6):** See [PHASE6_PERFORMANCE_BASELINE.md](../../PHASE6_PERFORMANCE_BASELINE.md)

**Regression Thresholds:**
- p50 latency: Max 10% regression
- p95 latency: Max 15% regression
- p99 latency: Max 20% regression
- Throughput: Max 10% regression

## Profiling

### CPU Profiling

```bash
# Generate flamegraph
cargo flamegraph --bench api-overhead

# Output: flamegraph.svg in target/criterion/<benchmark>/
```

### Async Profiling

```bash
# Enable tokio console
RUST_LOG=tokio=trace cargo bench --bench api-overhead

# View with tokio-console
cargo install tokio-console
tokio-console
```

### Distributed Tracing

```bash
# Run with Jaeger tracing
OTEL_EXPORTER_JAEGER_ENDPOINT=http://localhost:14268/api/traces \
  cargo bench --bench api-overhead

# View traces in Jaeger UI (http://localhost:16686)
```

## Interpreting Results

### Criterion Output

```
api_overhead/proto_serialization
                        time:   [5.234 μs 5.312 μs 5.398 μs]
                        change: [-2.3% -1.8% -1.2%] (p = 0.00 < 0.05)
                        Performance has improved.
```

- **time:** p50, p95, p99 estimates
- **change:** Performance change from previous run
- **p:** Statistical significance (p < 0.05 = significant)

### Regression Test Failure

```
❌ REGRESSION TESTS FAILED:
  - p99 regression: 250ms > 240ms (baseline + 20%)
```

This indicates p99 latency exceeded the 20% regression threshold.

## CI/CD Integration

### GitHub Actions

```yaml
- name: Run benchmarks
  run: cargo bench --bench api-overhead --bench proto-conversion

- name: Check for regressions
  run: cargo bench --bench regression-tests
```

### Regression Threshold

Set environment variable to fail CI on regression:

```bash
export FAIL_ON_REGRESSION=0.10  # 10% threshold
cargo bench --bench regression-tests
```

## Contributing

### Adding New Benchmarks

1. Create new benchmark file in `benches/` directory
2. Add to `Cargo.toml` `[[bench]]` section
3. Follow Criterion.rs conventions
4. Document in this README

### Benchmark Best Practices

- Use `black_box()` to prevent compiler optimizations
- Include warm-up time (5 seconds default)
- Use adequate sample size (100+ samples)
- Measure for sufficient time (30+ seconds)
- Document what each benchmark measures

## Troubleshooting

### "Benchmark requires running server"

Some benchmarks (api_roundtrip, api_streaming, embedding_api) require a running API server:

```bash
# Start API server in background
cargo run --bin openakta-api &

# Run benchmarks
cargo bench --bench api-overhead
```

### "Benchmark results are noisy"

Increase sample size and measurement time:

```bash
cargo bench -- --sample-size 200 --measurement-time 60
```

### "Flamegraph is empty"

Ensure debug symbols are enabled:

```bash
# In Cargo.toml
[profile.release]
debug = true
```

## References

- [Criterion.rs Documentation](https://bheisler.github.io/criterion.rs/book/)
- [cargo-flamegraph](https://github.com/flamegraph-rs/flamegraph)
- [tokio-console](https://github.com/tokio-rs/console)

---

**Last Updated:** 2026-03-24
