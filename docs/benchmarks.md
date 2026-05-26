# Benchmarks

All benchmarks run on Mac M4 Pro, single run, synthetic JSONL corpus (50 words/doc, 10% exact duplicates, 10% near-duplicates).

## Setup

- **textsift** v0.1.0 — `cargo build --release`, multi-threaded (rayon)
- **rensa** v0.2.9 — `pip install rensa`, RMinHash + RMinHashLSH
- **datasketch** latest — `pip install datasketch`, MinHash + MinHashLSH

All tools: threshold=0.8, num_perm=128, word 5-gram shingles.

## Results

### 100K documents

| Tool | Time | Unique | Dupes | Speedup vs datasketch |
|------|-----:|-------:|------:|----------------------:|
| **textsift** | **0.39s** | 90,000 | 10,000 | **96x** |
| rensa RMinHash | 0.92s | 90,000 | 10,000 | 41x |
| datasketch | 37.75s | 90,000 | 10,000 | 1x |

### 1M documents

| Tool | Time | Unique | Dupes | Speedup vs datasketch |
|------|-----:|-------:|------:|----------------------:|
| **textsift** | **5.13s** | 900,000 | 100,000 | **~79x** |
| rensa RMinHash | 9.75s | 900,000 | 100,000 | ~42x |
| datasketch | ~405s | 900,000 | 100,000 | 1x |

### Head-to-head: textsift vs rensa

| Dataset | textsift | rensa | textsift speedup |
|---------|----------|-------|:----------------:|
| 100K | 0.39s | 0.92s | **2.3x** |
| 1M | 5.13s | 9.75s | **1.9x** |

textsift is faster because it parallelizes MinHash signature computation with rayon (multi-core), while rensa processes documents sequentially in the Python event loop despite having a Rust core.

## Correctness

All three tools produce identical dedup results on the same corpus:

- 100K docs: all detect exactly 10,000 duplicates, keep 90,000 unique
- Verified with `scripts/correctness_test.py`: 0% difference between textsift and datasketch on 100K docs

## Reproduce

```bash
# Generate test data
uv run scripts/generate_testdata.py --size 100000 --output testdata/100k.jsonl
uv run scripts/generate_testdata.py --size 1000000 --output testdata/1m.jsonl

# Build textsift
cargo build --release

# Run benchmarks
uv run --with rensa --with datasketch scripts/benchmark_rensa.py testdata/100k.jsonl --field text
uv run --with rensa --with datasketch scripts/benchmark_rensa.py testdata/1m.jsonl --field text --skip-datasketch

# Run correctness test
uv run --with datasketch scripts/correctness_test.py
```
