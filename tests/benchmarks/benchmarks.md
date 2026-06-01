# NFR Benchmarks — T6.5

> Scaffolding for Non-Functional Requirement validation.
> Actual benchmarks require realistic fixtures (5000 files, 2000 nodes) — not available in CI yet.

## Benchmarks to implement

| #   | Benchmark                           | Threshold   | Status          |
| --- | ----------------------------------- | ----------- | --------------- |
| B1  | Architecture detection — 5000 files | <3s         | Pending fixture |
| B2  | Impact analysis — single node       | <5s         | Pending fixture |
| B3  | Graph insights — 2000 nodes         | <2s         | Pending fixture |
| B4  | Export JSON — 5000 nodes            | <5s         | Pending fixture |
| B5  | WAL concurrency — 10 parallel reads | 0 deadlocks | ✅ Ready        |

## How to run (once fixtures exist)

```bash
# Full benchmark suite
cargo bench --manifest-path engine/Cargo.toml

# Individual benchmarks
cargo bench --manifest-path engine/Cargo.toml -- architecture_detection
cargo bench --manifest-path engine/Cargo.toml -- impact_analysis
cargo bench --manifest-path engine/Cargo.toml -- graph_insights
cargo bench --manifest-path engine/Cargo.toml -- export_json
```

## WAL concurrency test

The WAL concurrency test (B5) can be run as a regular test:

```bash
cd engine && cargo test --test wal_concurrency
```

## Thresholds

Thresholds are proposed (T6.5, pending approval in SDD proposal). See:

- `docs/V2_READY_CHECKLIST.md` §4 NFR Targets
- `openspec/changes/v2-advanced-analysis/design.md` §7 NFR validation strategy
