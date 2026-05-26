"""Benchmark rensa vs textsift vs datasketch on the same corpus.

Usage:
    uv run --with rensa --with datasketch scripts/benchmark_rensa.py <input.jsonl> --field text
"""
import argparse
import json
import os
import subprocess
import sys
import time


def bench_datasketch(docs, threshold, num_perm, shingle_size):
    from datasketch import MinHash, MinHashLSH

    t0 = time.perf_counter()
    lsh = MinHashLSH(threshold=threshold, num_perm=num_perm)
    duplicates = set()

    for i, text in enumerate(docs):
        words = text.split()
        shingles = set()
        for j in range(max(1, len(words) - shingle_size + 1)):
            shingles.add(" ".join(words[j:j + shingle_size]))

        m = MinHash(num_perm=num_perm)
        for s in shingles:
            m.update(s.encode("utf-8"))

        if lsh.query(m):
            duplicates.add(i)
        else:
            lsh.insert(i, m)

    elapsed = time.perf_counter() - t0
    unique = len(docs) - len(duplicates)
    return elapsed, unique, len(duplicates)


def compute_lsh_params(threshold, num_perm):
    best = None
    best_dist = float("inf")
    for b in range(1, num_perm + 1):
        if num_perm % b != 0:
            continue
        r = num_perm // b
        inflection = (1.0 / b) ** (1.0 / r)
        dist = abs(inflection - threshold)
        if dist < best_dist:
            best_dist = dist
            best = (b, r)
    return best


def bench_rensa(docs, threshold, num_perm, shingle_size):
    from rensa import RMinHash, RMinHashLSH

    bands, _rows = compute_lsh_params(threshold, num_perm)
    t0 = time.perf_counter()
    lsh = RMinHashLSH(threshold=threshold, num_perm=num_perm, num_bands=bands)
    duplicates = set()

    for i, text in enumerate(docs):
        words = text.split()
        shingles = []
        for j in range(max(1, len(words) - shingle_size + 1)):
            shingles.append(" ".join(words[j:j + shingle_size]))

        m = RMinHash(num_perm=num_perm, seed=42)
        m.update(shingles)

        if lsh.query(m):
            duplicates.add(i)
        else:
            lsh.insert(i, m)

    elapsed = time.perf_counter() - t0
    unique = len(docs) - len(duplicates)
    return elapsed, unique, len(duplicates)


def bench_rensa_cminhash(docs, threshold, num_perm, shingle_size):
    from rensa import CMinHash, RMinHashLSH

    bands, _rows = compute_lsh_params(threshold, num_perm)
    t0 = time.perf_counter()
    lsh = RMinHashLSH(threshold=threshold, num_perm=num_perm, num_bands=bands)
    duplicates = set()

    for i, text in enumerate(docs):
        words = text.split()
        shingles = []
        for j in range(max(1, len(words) - shingle_size + 1)):
            shingles.append(" ".join(words[j:j + shingle_size]))

        m = CMinHash(num_perm=num_perm, seed=42)
        m.update(shingles)

        if lsh.query(m):
            duplicates.add(i)
        else:
            lsh.insert(i, m)

    elapsed = time.perf_counter() - t0
    unique = len(docs) - len(duplicates)
    return elapsed, unique, len(duplicates)


def bench_textsift(input_path, threshold, num_perm, shingle_size):
    textsift = os.path.join(os.path.dirname(__file__), "..", "target", "release", "textsift")

    t0 = time.perf_counter()
    result = subprocess.run(
        [textsift, input_path, "--field", "text",
         "--threshold", str(threshold),
         "--num-perm", str(num_perm),
         "--shingle-size", str(shingle_size),
         "--stats"],
        capture_output=True, text=True
    )
    elapsed = time.perf_counter() - t0

    unique = 0
    dupes = 0
    for line in result.stderr.strip().split("\n"):
        if "unique docs emitted" in line:
            unique = int(line.split(":")[1].strip())
        elif "exact duplicates" in line:
            dupes += int(line.split(":")[1].strip())
        elif "near duplicates" in line:
            dupes += int(line.split(":")[1].strip())

    return elapsed, unique, dupes


def main():
    parser = argparse.ArgumentParser(description="Benchmark rensa vs textsift vs datasketch")
    parser.add_argument("input", help="Input JSONL file")
    parser.add_argument("--field", default="text")
    parser.add_argument("--threshold", type=float, default=0.8)
    parser.add_argument("--num-perm", type=int, default=128)
    parser.add_argument("--shingle-size", type=int, default=5)
    parser.add_argument("--skip-datasketch", action="store_true", help="Skip slow datasketch benchmark")
    args = parser.parse_args()

    docs = []
    with open(args.input) as f:
        for line in f:
            if line.strip():
                obj = json.loads(line)
                docs.append(obj.get(args.field, ""))

    n = len(docs)
    print(f"Corpus: {n:,} docs | threshold={args.threshold} | num_perm={args.num_perm} | shingle_size={args.shingle_size}")
    print("=" * 70)

    # textsift
    ts_time, ts_unique, ts_dupes = bench_textsift(
        args.input, args.threshold, args.num_perm, args.shingle_size
    )
    print(f"textsift     | {ts_time:7.2f}s | unique: {ts_unique:>8,} | dupes: {ts_dupes:>8,}")

    # rensa RMinHash
    r_time, r_unique, r_dupes = bench_rensa(
        docs, args.threshold, args.num_perm, args.shingle_size
    )
    print(f"rensa RMin   | {r_time:7.2f}s | unique: {r_unique:>8,} | dupes: {r_dupes:>8,}")

    # rensa CMinHash — CMinHash not compatible with RMinHashLSH, benchmark hash-only
    try:
        from rensa import CMinHash
        t0 = time.perf_counter()
        for text in docs:
            words = text.split()
            shingles = []
            for j in range(max(1, len(words) - args.shingle_size + 1)):
                shingles.append(" ".join(words[j:j + args.shingle_size]))
            m = CMinHash(num_perm=args.num_perm, seed=42)
            m.update(shingles)
        c_time = time.perf_counter() - t0
        c_unique, c_dupes = 0, 0
        print(f"rensa CMin   | {c_time:7.2f}s | (hash-only, no LSH dedup)")
    except Exception as e:
        c_time = None
        print(f"rensa CMin   | ERROR: {e}")

    # datasketch
    if not args.skip_datasketch:
        ds_time, ds_unique, ds_dupes = bench_datasketch(
            docs, args.threshold, args.num_perm, args.shingle_size
        )
        print(f"datasketch   | {ds_time:7.2f}s | unique: {ds_unique:>8,} | dupes: {ds_dupes:>8,}")
    else:
        ds_time = None
        print(f"datasketch   | SKIPPED")

    print("=" * 70)
    print(f"\ntextsift vs rensa RMinHash:  {r_time/ts_time:.1f}x faster")
    if ds_time:
        print(f"textsift vs datasketch:      {ds_time/ts_time:.1f}x faster")
        print(f"rensa RMin vs datasketch:    {ds_time/r_time:.1f}x faster")


if __name__ == "__main__":
    main()
