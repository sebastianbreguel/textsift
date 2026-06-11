"""Shared helpers for benchmark scripts."""
import os
import subprocess
import time


def count_jsonl_lines(path):
    n = 0
    with open(path) as f:
        for line in f:
            if line.strip():
                n += 1
    return n


def bench_textsift(input_path, field, threshold, num_perm, shingle_size):
    """Run the release textsift binary, return (elapsed, unique, dupes)."""
    textsift = os.path.join(os.path.dirname(__file__), "..", "target", "release", "textsift")

    t0 = time.perf_counter()
    result = subprocess.run(
        [textsift, input_path, "--field", field,
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
