# textsift

**96x faster than datasketch. 2x faster than rensa. Same results.**

Fast text deduplication for ML datasets. Exact hash + MinHash LSH in 789 lines of Rust. One command or one function call.

```bash
# CLI
textsift corpus.jsonl --field text > clean.jsonl

# Python
from textsift import dedup
result = dedup(texts, threshold=0.8)
clean = [t for t, rep in zip(texts, result.is_representative) if rep]
```

## Benchmarks

Synthetic JSONL corpus (50 words/doc, 10% exact duplicates, 10% near-duplicates). Mac M4 Pro.

| Tool | 100K docs | 1M docs | Language | Interface |
|------|----------|---------|----------|-----------|
| **textsift** | **0.39s** | **5.1s** | Rust | CLI + Python |
| rensa | 0.92s | 9.8s | Rust+PyO3 | Python only |
| datasketch | 37.8s | 405s | Python | Python only |
| text-dedup | ~3 min | ~5 min | Python | CLI + Python |

All tools detect the same duplicates at the same threshold — [verified with 0% difference](scripts/correctness_test.py) on 100K docs.

### Why it's fast

- Rayon multi-threaded MinHash signature computation (uses all cores)
- ahash for hashing (hardware-accelerated on ARM / Apple Silicon)
- Auto-calculated LSH bands/rows from threshold (no manual tuning)
- Zero-copy JSONL passthrough (preserves all original JSON fields)

## Install

**CLI** (Rust):
```bash
cargo install textsift
```

**Python** (requires Rust toolchain):
```bash
pip install maturin
git clone https://github.com/sebastianbreguel/textsift
cd textsift && maturin develop --release
```

## Usage

### CLI

```bash
# Remove duplicates — output is clean JSONL, same schema as input
textsift data.jsonl --field text > clean.jsonl

# See duplicate clusters before removing anything
textsift data.jsonl --field text --clusters | jq 'select(.is_representative == false)'

# Quick stats: how many duplicates?
textsift data.jsonl --field text --stats > /dev/null

# Exact duplicates only (skip MinHash, fastest mode)
textsift data.jsonl --field text --exact-only > clean.jsonl

# Pipe from stdin
cat data.jsonl | textsift - --field text > clean.jsonl

# Lower threshold to catch more near-duplicates
textsift data.jsonl --field text --threshold 0.5 > clean.jsonl
```

### Python

```python
from textsift import dedup

texts = ["doc one", "doc two", "doc one", "doc three", "doc two"]

result = dedup(texts, threshold=0.8)

# Filter to unique texts
clean = [t for t, rep in zip(texts, result.is_representative) if rep]
# ["doc one", "doc two", "doc three"]

# Inspect clusters
for i, (text, cid, rep) in enumerate(zip(texts, result.cluster_ids, result.is_representative)):
    print(f"  doc {i}: cluster={cid}, representative={rep}")

# Stats
print(result.stats())
# "total: 5, exact_dupes: 2, near_dupes: 0, unique: 3"
```

**Parameters:**
```python
result = dedup(
    texts,
    threshold=0.8,      # Jaccard similarity threshold (0.0-1.0)
    num_perm=128,        # MinHash permutations (more = more accurate, slower)
    shingle_size=5,      # Word n-gram size (lower for short texts)
    exact_only=False,    # Skip MinHash, only exact hash dedup
)
```

### Output Modes

**Default** — clean JSONL, no extra fields:
```jsonl
{"text": "the quick brown fox", "id": 1}
{"text": "another document", "id": 3}
```

**Clusters** (`--clusters`) — every doc labeled with cluster assignment:
```jsonl
{"text": "the quick brown fox", "id": 1, "cluster_id": 0, "is_representative": true}
{"text": "the quick brown fox", "id": 2, "cluster_id": 0, "is_representative": false}
{"text": "another document", "id": 3, "cluster_id": 1, "is_representative": true}
```

## When to use textsift vs alternatives

| Need | Tool |
|------|------|
| Fastest single-machine dedup, CLI or Python | **textsift** |
| Rich sketch algorithms (HyperLogLog, Weighted MinHash) | datasketch |
| SimHash, suffix arrays, Bloom filter | text-dedup |
| Distributed multi-node / GPU dedup | datatrove or NeMo Curator |
| Bloom filter n-gram dedup at 300T+ token scale | bff (Allen AI) |

textsift does one thing: dedup text fast on a single machine. If you need distributed processing, GPU acceleration, or algorithms beyond MinHash LSH, use the tool built for that.

## How it works

1. **Exact dedup** — hash each text with ahash, skip if seen before. O(n).
2. **Shingling** — split text into word 5-grams (configurable with `--shingle-size`).
3. **MinHash** — compute 128 min-hash signatures per document (configurable with `--num-perm`). Parallelized with rayon.
4. **LSH banding** — split signatures into bands, hash each band to buckets. Documents sharing a bucket in any band are candidate pairs. Bands/rows auto-calculated from `--threshold`.
5. **Union-Find clustering** — candidate pairs verified against threshold, then clustered with disjoint-set (path compression + union by rank).
6. **Output** — emit representatives (default) or all docs with cluster labels (`--clusters`).

## CLI Reference

```
textsift [OPTIONS] --field <FIELD> <INPUT>

Arguments:
  <INPUT>    Input file (JSONL). Use - for stdin.

Options:
  -f, --field <FIELD>          JSON field containing text [required]
  -t, --threshold <FLOAT>      Jaccard similarity threshold [default: 0.8]
  -n, --num-perm <INT>         MinHash permutations [default: 128]
      --shingle-size <INT>     Word n-gram size [default: 5]
      --clusters               Output clusters instead of deduplicated file
      --exact-only             Skip MinHash, only exact dedup
      --stats                  Print statistics to stderr
  -o, --output <FILE>          Output file [default: stdout]
  -h, --help                   Print help
  -V, --version                Print version
```

## Reproduce benchmarks

```bash
# Generate test data
uv run scripts/generate_testdata.py --size 100000 --output testdata/100k.jsonl
uv run scripts/generate_testdata.py --size 1000000 --output testdata/1m.jsonl

# Build
cargo build --release

# Run benchmarks (textsift vs rensa vs datasketch)
uv run --with rensa --with datasketch scripts/benchmark_rensa.py testdata/100k.jsonl --field text

# Correctness test
uv run --with datasketch scripts/correctness_test.py
```

## Design decisions

- **Zero normalization** — text is hashed as-is. Normalize before piping if needed.
- **Word 5-grams** — default shingle size for document-level dedup. Use `--shingle-size 3` for short texts.
- **ahash** — fast non-cryptographic hashing, hardware-accelerated on Apple Silicon (ARM).
- **Auto LSH params** — bands and rows auto-calculated from threshold. No manual tuning needed.
- **Callback-based candidate pairs** — Union-Find is idempotent, so pairs are processed inline without materializing a full pair list. Prevents OOM on dense datasets.

## Limitations

- Loads all documents + signatures in memory. Practical limit ~5M docs on 16GB RAM.
- JSONL only — no Parquet, no CSV, no HuggingFace datasets (yet).
- MinHash LSH only — no SimHash, no suffix arrays, no semantic dedup.
- Python bindings require building from source (PyPI wheels coming soon).

## License

MIT
