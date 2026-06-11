# Benchmarks

All benchmarks run on Mac M4 Pro, single run, synthetic JSONL corpus (50 words/doc, 10% exact duplicates, 10% near-duplicates).

## Setup

- **textsift** v0.1.0 — `cargo build --release`, multi-threaded (rayon)
- **rensa** v0.2.9 — `pip install rensa`, RMinHash + RMinHashLSH
- **datasketch** latest — `pip install datasketch`, MinHash + MinHashLSH
- **text-dedup** v0.4.1 — `pip install text-dedup`, MinHash via CLI

All tools: threshold=0.8, num_perm=128, ngram_size=5.
textsift, rensa, and datasketch use word 5-grams. text-dedup uses character 5-grams (its default tokenization).

## Results

### 100K documents

| Tool | Time | Unique | Dupes | Speedup vs datasketch |
|------|-----:|-------:|------:|----------------------:|
| **textsift** | **0.39s** | 90,000 | 10,000 | **96x** |
| rensa RMinHash | 0.92s | 90,000 | 10,000 | 41x |
| text-dedup | 8.15s | 80,621 | 19,379 | 4.6x |
| datasketch | 37.75s | 90,000 | 10,000 | 1x |

### 1M documents

| Tool | Time | Unique | Dupes | Speedup vs datasketch |
|------|-----:|-------:|------:|----------------------:|
| **textsift** | **4.59s** | 900,000 | 100,000 | **~88x** |
| rensa RMinHash | 9.75s | 900,000 | 100,000 | ~42x |
| text-dedup | 50.96s | 806,144 | 193,856 | ~8x |
| datasketch | ~405s | 900,000 | 100,000 | 1x |

### Head-to-head: textsift vs rensa

| Dataset | textsift | rensa | textsift speedup |
|---------|----------|-------|:----------------:|
| 100K | 0.39s | 0.92s | **2.3x** |
| 1M | 4.59s | 9.75s | **2.1x** |

textsift is faster because it parallelizes MinHash signature computation with rayon (multi-core), while rensa processes documents sequentially in the Python event loop despite having a Rust core.

### Head-to-head: textsift vs text-dedup

| Dataset | textsift | text-dedup | textsift speedup |
|---------|----------|------------|:----------------:|
| 100K | 0.39s | 8.15s | **21x** |
| 1M | 4.59s | 50.96s | **11x** |

text-dedup detects more duplicates because it uses character n-grams instead of word n-grams. Character 5-grams produce more shingle overlap between dissimilar texts, increasing false positives.

## Correctness

textsift, rensa, and datasketch produce identical dedup results on the same corpus:

- 100K docs: all three detect exactly 10,000 duplicates, keep 90,000 unique
- Verified with `scripts/correctness_test.py`: 0% difference between textsift and datasketch on 100K docs

text-dedup uses a different shingling strategy (character n-grams vs word n-grams), so its duplicate counts differ at the same threshold.

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
uv run --with 'text-dedup>=0.4' --with pyarrow scripts/benchmark_textdedup.py testdata/100k.jsonl --field text
uv run --with 'text-dedup>=0.4' --with pyarrow scripts/benchmark_textdedup.py testdata/1m.jsonl --field text

# Run correctness test
uv run --with datasketch scripts/correctness_test.py
```
