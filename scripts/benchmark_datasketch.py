"""Benchmark datasketch MinHash LSH dedup on a JSONL corpus.

Usage:
    uv run scripts/benchmark_datasketch.py <input.jsonl> --field text [--threshold 0.8] [--num-perm 128]
"""
import argparse
import json
import sys
import time

def main():
    parser = argparse.ArgumentParser(description="Benchmark datasketch dedup")
    parser.add_argument("input", help="Input JSONL file")
    parser.add_argument("--field", required=True, help="JSON field with text")
    parser.add_argument("--threshold", type=float, default=0.8)
    parser.add_argument("--num-perm", type=int, default=128)
    parser.add_argument("--shingle-size", type=int, default=5)
    args = parser.parse_args()

    from datasketch import MinHash, MinHashLSH

    docs = []
    with open(args.input) as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            obj = json.loads(line)
            text = obj.get(args.field, "")
            docs.append(text)

    print(f"Loaded {len(docs)} docs", file=sys.stderr)

    t0 = time.perf_counter()

    lsh = MinHashLSH(threshold=args.threshold, num_perm=args.num_perm)
    minhashes = []
    duplicates = set()

    for i, text in enumerate(docs):
        words = text.split()
        shingles = set()
        for j in range(max(1, len(words) - args.shingle_size + 1)):
            shingle = " ".join(words[j:j + args.shingle_size])
            shingles.add(shingle)

        m = MinHash(num_perm=args.num_perm)
        for s in shingles:
            m.update(s.encode("utf-8"))
        minhashes.append(m)

        result = lsh.query(m)
        if result:
            duplicates.add(i)
        else:
            lsh.insert(str(i), m)

    t1 = time.perf_counter()
    elapsed = t1 - t0

    unique = len(docs) - len(duplicates)
    print(f"Total docs: {len(docs)}", file=sys.stderr)
    print(f"Duplicates: {len(duplicates)}", file=sys.stderr)
    print(f"Unique: {unique}", file=sys.stderr)
    print(f"Time: {elapsed:.3f}s", file=sys.stderr)
    print(f"\n=== RESULT: {elapsed:.3f}s for {len(docs)} docs ===", file=sys.stderr)

if __name__ == "__main__":
    main()
