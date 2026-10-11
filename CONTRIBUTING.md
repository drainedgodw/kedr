# Contributing

Core only for now. No GUI, product catalog or claimed feature parity.
Keep modules small, validate behavior against reference models and retain raw
performance evidence. English documentation; focused one-word commit subjects
such as `init`, `text`, `tests` and `bench`.

Before a change: `cargo fmt --check`, `cargo test --locked`,
`cargo clippy --locked --all-targets -- -D warnings`.
Benchmarks may be noisy; never use a single VM run as a universal speed claim.
