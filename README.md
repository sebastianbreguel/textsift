# textsift

Fast text deduplication CLI for ML dataset curation. Written in Rust.

Exact dedup (hash-based) + near-duplicate detection (MinHash LSH) on JSONL corpora. Plug-and-play — one command, no configuration required.

## Benchmarks

Synthetic JSONL corpus (50 words/doc, 10% exact duplicates, 10% near-duplicates). Mac M4 Pro, single-threaded.

| Dataset | textsift | datasketch (Python) | Speedup |
|---------|----------|--------------------:|--------:|
| 100K docs | **1.5s** | 38.6s | **26x** |
| 1M docs | **15.7s** | 405s | **26x** |

Both tools detect the same duplicates at the same threshold. textsift is ~26x faster thanks to Rust + ahash.

## Install

```bash
cargo install textsift
```

Or build from source:

```bash
git clone https://github.com/sebastianbreguel/textsift
cd textsift
cargo build --release
```

## Usage

```bash
# Remove duplicates from a JSONL corpus
textsift corpus.jsonl --field text > clean.jsonl

# See duplicate clusters
textsift corpus.jsonl --field text --clusters > clusters.jsonl

# Only exact duplicates (skip MinHash)
textsift corpus.jsonl --field text --exact-only > clean.jsonl

# Print stats
textsift corpus.jsonl --field text --stats > clean.jsonl

# Read from stdin
cat corpus.jsonl | textsift - --field text > clean.jsonl

# Adjust similarity threshold (default: 0.8)
textsift corpus.jsonl --field text --threshold 0.5 > clean.jsonl
```

## How it works

textsift runs a two-layer dedup pipeline:

1. **Exact dedup** — hash each text with ahash, skip if seen before. O(n).
2. **MinHash LSH** — compute MinHash signatures (word 5-gram shingles), index with LSH banding, cluster candidates with Union-Find. Near-duplicates above `--threshold` are grouped.

### Output modes

**Default** — outputs deduplicated JSONL. Same schema as input, no extra fields:

```bash
$ textsift data.jsonl --field text
{"text": "the quick brown fox", "id": 1}
{"text": "another document", "id": 3}
```

**Clusters** (`--clusters`) — outputs all docs with `cluster_id` and `is_representative`:

```bash
$ textsift data.jsonl --field text --clusters
{"text": "the quick brown fox", "id": 1, "cluster_id": 0, "is_representative": true}
{"text": "the quick brown fox", "id": 2, "cluster_id": 0, "is_representative": false}
{"text": "another document", "id": 3, "cluster_id": 1, "is_representative": true}
```

Filter representatives with jq:

```bash
textsift data.jsonl --field text --clusters | jq 'select(.is_representative)'
```

## CLI Reference

```
textsift [OPTIONS] <INPUT>

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

## Design decisions

- **Zero normalization** — text is hashed as-is. Normalize before piping to textsift if needed.
- **Word 5-grams** — default shingle size, standard for document-level dedup. Use `--shingle-size 3` for short texts.
- **ahash** — fast non-cryptographic hashing, hardware-accelerated on ARM (Apple Silicon).
- **Auto LSH params** — bands and rows auto-calculated from threshold to minimize false negatives.

## License

MIT
