"""Benchmark text-dedup vs textsift on the same corpus.

text-dedup is CLI-only (JSON args to python -m text_dedup.minhash).
Output is HF Arrow format, parsed from stdout timing logs.

Usage:
    uv run --with 'text-dedup>=0.4' --with pyarrow scripts/benchmark_textdedup.py testdata/100k.jsonl --field text
"""
import argparse
import json
import os
import re
import subprocess
import sys
import tempfile
import time

from bench_common import bench_textsift, count_jsonl_lines




def jsonl_to_parquet(jsonl_path, parquet_path):
    import pyarrow as pa
    import pyarrow.parquet as pq

    rows = []
    with open(jsonl_path) as f:
        for line in f:
            if line.strip():
                rows.append(json.loads(line))

    table = pa.Table.from_pylist(rows)
    pq.write_table(table, parquet_path)
    return len(rows)


def bench_textdedup(input_path, field, threshold, num_perm, shingle_size):
    with tempfile.TemporaryDirectory() as tmpdir:
        parquet_path = os.path.join(tmpdir, "input.parquet")
        output_dir = os.path.join(tmpdir, "output")

        print("  Converting JSONL to Parquet...", file=sys.stderr)
        total_docs = jsonl_to_parquet(input_path, parquet_path)

        input_json = json.dumps({
            "input_type": "local_files",
            "file_type": "parquet",
            "read_arguments": {"path": "parquet", "data_files": parquet_path, "split": "train"},
        })
        algo_json = json.dumps({
            "algorithm_name": "minhash",
            "text_column": field,
            "num_perm": num_perm,
            "threshold": threshold,
            "ngram_size": shingle_size,
        })
        output_json = json.dumps({"output_dir": output_dir})
        debug_json = json.dumps({"logging_level": 20, "enable_profiling": False})

        print("  Running text-dedup minhash...", file=sys.stderr)
        t0 = time.perf_counter()
        result = subprocess.run(
            [sys.executable, "-m", "text_dedup.minhash",
             "--input", input_json,
             "--algorithm", algo_json,
             "--output", output_json,
             "--debug", debug_json],
            capture_output=True, text=True
        )
        elapsed = time.perf_counter() - t0

        if result.returncode != 0:
            print(f"  text-dedup FAILED (exit {result.returncode}):", file=sys.stderr)
            print(f"  stderr: {result.stderr[:1000]}", file=sys.stderr)
            return None, None, None, None

        # Parse "Before: N" and "After: N" from stdout
        unique = None
        before = None
        for line in result.stdout.split("\n"):
            m = re.search(r"Before:\s+(\d+)", line)
            if m:
                before = int(m.group(1))
            m = re.search(r"After:\s+(\d+)", line)
            if m:
                unique = int(m.group(1))

        # Also parse internal "Total: X.XX s" for text-dedup's own timing (excludes our subprocess overhead)
        internal_time = None
        for line in result.stdout.split("\n"):
            m = re.search(r"Total:\s+([\d.]+)\s*s", line)
            if m:
                internal_time = float(m.group(1))

        if unique is None:
            unique = total_docs
        dupes = (before or total_docs) - unique

        return elapsed, unique, dupes, internal_time


def main():
    parser = argparse.ArgumentParser(description="Benchmark text-dedup vs textsift")
    parser.add_argument("input", help="Input JSONL file")
    parser.add_argument("--field", default="text")
    parser.add_argument("--threshold", type=float, default=0.8)
    parser.add_argument("--num-perm", type=int, default=128)
    parser.add_argument("--shingle-size", type=int, default=5)
    args = parser.parse_args()

    n = count_jsonl_lines(args.input)
    print(f"Corpus: {n:,} docs | threshold={args.threshold} | num_perm={args.num_perm} | shingle_size={args.shingle_size}")
    print("=" * 70)

    ts_time, ts_unique, ts_dupes = bench_textsift(
        args.input, args.field, args.threshold, args.num_perm, args.shingle_size
    )
    print(f"textsift     | {ts_time:7.2f}s | unique: {ts_unique:>8,} | dupes: {ts_dupes:>8,}")

    td_result = bench_textdedup(
        args.input, args.field, args.threshold, args.num_perm, args.shingle_size
    )
    if td_result[0] is not None:
        td_time, td_unique, td_dupes, td_internal = td_result
        internal_note = f" (internal: {td_internal:.2f}s)" if td_internal else ""
        print(f"text-dedup   | {td_time:7.2f}s | unique: {td_unique:>8,} | dupes: {td_dupes:>8,}{internal_note}")
    else:
        print("text-dedup   | FAILED")
        td_time = None

    print("=" * 70)
    if td_time:
        print(f"\ntextsift vs text-dedup:  {td_time/ts_time:.1f}x faster")
        if td_result[3]:
            print(f"  (using text-dedup internal time: {td_result[3]/ts_time:.1f}x faster)")


if __name__ == "__main__":
    main()
