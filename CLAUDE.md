# textsift

Fast text deduplication for ML datasets. Rust CLI + lib + PyO3 Python bindings.
Exact hash dedup + MinHash LSH near-dedup. The value prop is **speed** — treat
performance regressions as bugs.

## Commands

```bash
cargo test --features python      # full suite (62 tests) — what CI runs
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --check
cargo build --release             # CLI binary at target/release/textsift
```

Python wheel (local): `uvx maturin build --release --out dist -i $(uv python find 3.12)`
— **pyo3 0.23 does not support Python 3.14**, always target 3.12 locally.

## Architecture (src/, ~1000 LOC)

- `pipeline.rs` — the core: `deduplicate()` (pure, no I/O) + `run()` (CLI: JSONL in/out). `DedupConfig::validate()` must be called by every user-facing surface; `deduplicate()` panics on invalid configs (documented).
- `shingle.rs` — word n-grams hashed directly to u64 (no String per shingle; 0xFF separator).
- `minhash.rs` — signatures via one hash per shingle + multiply-add permutations (splitmix64-derived). Fixed seeds = deterministic.
- `lsh.rs` — banding index; `optimal_params()` derives bands/rows from threshold.
- `exact.rs` / `cluster.rs` (union-find) / `io.rs` / `cli.rs` (clap) / `python.rs` (PyO3).
- Public API = `pipeline` + `cli`; everything else is `#[doc(hidden)]` (pub only for integration tests).

## Gotchas

- **Feature wiring**: `python = ["pyo3"]` is testable; `extension-module` (adds `pyo3/extension-module` + `abi3-py39`) is enabled ONLY by maturin via pyproject.toml — putting extension-module on `python` breaks `cargo test` linking.
- **Byte-identical passthrough**: default mode emits the raw input line — never re-serialize JSON in that path (re-serializing alphabetizes keys).
- Docs missing `--field` pass through with a warning — never silently drop records.
- Changing hash seeds/families changes near-dup results: verify counts on testdata before/after (`--stats` diff on testdata/100k.jsonl) — corpora regenerate via `scripts/generate_testdata.py`.

## Benchmarks

`scripts/benchmark_rensa.py` (vs rensa/datasketch) and `scripts/benchmark_textdedup.py` (vs text-dedup) share `scripts/bench_common.py`. Run with `uv run --no-project --python 3.12 --with rensa scripts/benchmark_rensa.py testdata/100k.jsonl --field text --skip-datasketch`. Numbers in README/docs/benchmarks.md are from Mac M4 Pro — re-measure on that machine before updating claims.

## Releases

Tag `v*` triggers `.github/workflows/release.yml` (abi3 wheels + sdist → PyPI trusted publishing; setup steps in the workflow header). crates.io is manual (`cargo publish`).
