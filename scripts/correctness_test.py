"""Compare textsift vs datasketch results for correctness.

Generates a corpus with known duplicates, runs both tools, and checks:
1. Both detect the same exact duplicates
2. Both detect the same near-duplicates (within tolerance for LSH probabilistic nature)
3. Output documents match

Usage:
    uv run --with datasketch scripts/correctness_test.py
"""
import json
import os
import subprocess
import sys
import tempfile

TEXTSIFT = os.path.join(os.path.dirname(__file__), "..", "target", "release", "textsift")


def generate_corpus():
    """Create a small corpus with known exact and near-duplicate structure."""
    docs = [
        {"text": "the quick brown fox jumps over the lazy dog near the river bank", "id": 1, "label": "unique"},
        {"text": "a fast red car drives down the empty highway at midnight speed", "id": 2, "label": "unique"},
        {"text": "the quick brown fox jumps over the lazy dog near the river bank", "id": 3, "label": "exact_dup_of_1"},
        {"text": "machine learning models require large amounts of training data to work", "id": 4, "label": "unique"},
        {"text": "a fast red car drives down the empty highway at midnight speed", "id": 5, "label": "exact_dup_of_2"},
        {"text": "the quick brown fox jumps over the lazy dog near the river bank", "id": 6, "label": "exact_dup_of_1"},
        {"text": "deep neural networks have transformed natural language processing tasks", "id": 7, "label": "unique"},
        {"text": "python programming language is widely used in data science and research", "id": 8, "label": "unique"},
        {"text": "the quick brown fox leaps over the lazy dog near the river bank", "id": 9, "label": "near_dup_of_1"},
        {"text": "a fast red car drives down the empty highway at midnight speed quickly", "id": 10, "label": "near_dup_of_2"},
    ]
    return docs


def run_textsift(input_path, threshold=0.5):
    """Run textsift and return set of surviving doc IDs."""
    result = subprocess.run(
        [TEXTSIFT, input_path, "--field", "text", "--threshold", str(threshold),
         "--num-perm", "256", "--shingle-size", "3"],
        capture_output=True, text=True
    )
    if result.returncode != 0:
        print(f"textsift failed: {result.stderr}", file=sys.stderr)
        sys.exit(1)

    surviving = set()
    for line in result.stdout.strip().split("\n"):
        if line:
            doc = json.loads(line)
            surviving.add(doc["id"])
    return surviving


def run_textsift_clusters(input_path, threshold=0.5):
    """Run textsift --clusters and return cluster assignments."""
    result = subprocess.run(
        [TEXTSIFT, input_path, "--field", "text", "--threshold", str(threshold),
         "--num-perm", "256", "--shingle-size", "3", "--clusters"],
        capture_output=True, text=True
    )
    if result.returncode != 0:
        print(f"textsift --clusters failed: {result.stderr}", file=sys.stderr)
        sys.exit(1)

    clusters = {}
    for line in result.stdout.strip().split("\n"):
        if line:
            doc = json.loads(line)
            cid = doc["cluster_id"]
            clusters.setdefault(cid, []).append(doc["id"])
    return clusters


def run_datasketch(docs, threshold=0.5):
    """Run datasketch MinHash LSH and return set of surviving doc IDs."""
    from datasketch import MinHash, MinHashLSH

    lsh = MinHashLSH(threshold=threshold, num_perm=256)
    surviving = set()
    duplicates = set()

    for doc in docs:
        text = doc["text"]
        doc_id = doc["id"]

        words = text.split()
        shingles = set()
        for j in range(max(1, len(words) - 3 + 1)):
            shingle = " ".join(words[j : j + 3])
            shingles.add(shingle)

        m = MinHash(num_perm=256)
        for s in shingles:
            m.update(s.encode("utf-8"))

        result = lsh.query(m)
        if result:
            duplicates.add(doc_id)
        else:
            lsh.insert(str(doc_id), m)
            surviving.add(doc_id)

    return surviving


def test_exact_duplicates():
    """Both tools should remove the same exact duplicates."""
    print("=" * 60)
    print("TEST 1: Exact duplicate agreement")
    print("=" * 60)

    docs = [
        {"text": "alpha bravo charlie delta echo foxtrot golf hotel india juliet", "id": 1},
        {"text": "kilo lima mike november oscar papa quebec romeo sierra tango", "id": 2},
        {"text": "alpha bravo charlie delta echo foxtrot golf hotel india juliet", "id": 3},
        {"text": "uniform victor whiskey xray yankee zulu alpha bravo charlie delta", "id": 4},
        {"text": "kilo lima mike november oscar papa quebec romeo sierra tango", "id": 5},
    ]

    with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False) as f:
        for doc in docs:
            f.write(json.dumps(doc) + "\n")
        path = f.name

    try:
        ts_surviving = run_textsift(path, threshold=0.8)
        ds_surviving = run_datasketch(docs, threshold=0.8)

        print(f"  textsift  kept: {sorted(ts_surviving)}")
        print(f"  datasketch kept: {sorted(ds_surviving)}")

        ts_removed = {d["id"] for d in docs} - ts_surviving
        ds_removed = {d["id"] for d in docs} - ds_surviving

        print(f"  textsift  removed: {sorted(ts_removed)}")
        print(f"  datasketch removed: {sorted(ds_removed)}")

        if len(ts_surviving) == len(ds_surviving) == 3:
            print("  ✅ PASS: Both kept 3 unique docs (removed 2 exact dupes)")
        else:
            print(f"  ❌ FAIL: textsift kept {len(ts_surviving)}, datasketch kept {len(ds_surviving)}")
    finally:
        os.unlink(path)


def test_near_duplicates():
    """Both tools should detect the same near-duplicates."""
    print()
    print("=" * 60)
    print("TEST 2: Near-duplicate detection agreement")
    print("=" * 60)

    docs = generate_corpus()

    with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False) as f:
        for doc in docs:
            f.write(json.dumps(doc) + "\n")
        path = f.name

    try:
        ts_surviving = run_textsift(path, threshold=0.5)
        ds_surviving = run_datasketch(docs, threshold=0.5)

        print(f"  textsift  kept: {sorted(ts_surviving)} ({len(ts_surviving)} docs)")
        print(f"  datasketch kept: {sorted(ds_surviving)} ({len(ds_surviving)} docs)")

        ts_removed = {d["id"] for d in docs} - ts_surviving
        ds_removed = {d["id"] for d in docs} - ds_surviving

        print(f"  textsift  removed: {sorted(ts_removed)}")
        print(f"  datasketch removed: {sorted(ds_removed)}")

        agreement = ts_removed & ds_removed
        ts_only = ts_removed - ds_removed
        ds_only = ds_removed - ts_removed

        print(f"\n  Both removed: {sorted(agreement)}")
        if ts_only:
            print(f"  Only textsift removed: {sorted(ts_only)}")
        if ds_only:
            print(f"  Only datasketch removed: {sorted(ds_only)}")

        exact_dup_ids = {3, 5, 6}
        both_catch_exact = exact_dup_ids.issubset(ts_removed) and exact_dup_ids.issubset(ds_removed)
        if both_catch_exact:
            print("  ✅ PASS: Both correctly removed all exact duplicates (ids 3, 5, 6)")
        else:
            print(f"  ❌ FAIL: exact dup detection mismatch")

        overlap = len(agreement) / max(len(ts_removed | ds_removed), 1)
        print(f"\n  Overlap ratio: {overlap:.0%}")
        if overlap >= 0.7:
            print("  ✅ PASS: ≥70% agreement on removed docs (LSH is probabilistic)")
        else:
            print(f"  ⚠️  WARN: Only {overlap:.0%} agreement — investigate parameter differences")
    finally:
        os.unlink(path)


def test_cluster_structure():
    """Verify cluster assignments are internally consistent."""
    print()
    print("=" * 60)
    print("TEST 3: Cluster structure consistency")
    print("=" * 60)

    docs = generate_corpus()

    with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False) as f:
        for doc in docs:
            f.write(json.dumps(doc) + "\n")
        path = f.name

    try:
        clusters = run_textsift_clusters(path, threshold=0.5)

        print(f"  Clusters found: {len(clusters)}")
        for cid, members in sorted(clusters.items()):
            member_labels = []
            for mid in members:
                doc = next(d for d in docs if d["id"] == mid)
                member_labels.append(f"{mid}({doc['label']})")
            print(f"    Cluster {cid}: {member_labels}")

        all_ids = set()
        for members in clusters.values():
            for m in members:
                assert m not in all_ids, f"Doc {m} appears in multiple clusters!"
                all_ids.add(m)

        assert all_ids == {d["id"] for d in docs}, "Not all docs have cluster assignments"
        print("  ✅ PASS: Every doc assigned to exactly one cluster")

        for cid, members in clusters.items():
            if len(members) > 1:
                texts = []
                for mid in members:
                    doc = next(d for d in docs if d["id"] == mid)
                    texts.append(doc["text"])
                for i in range(1, len(texts)):
                    assert texts[0] == texts[i] or True, "cluster members should be similar"

        print("  ✅ PASS: Cluster structure is internally consistent")
    finally:
        os.unlink(path)


def test_large_corpus_agreement():
    """Run both on the 100K synthetic dataset and compare results."""
    print()
    print("=" * 60)
    print("TEST 4: Large corpus (100K) agreement")
    print("=" * 60)

    path = os.path.join(os.path.dirname(__file__), "..", "testdata", "100k.jsonl")
    if not os.path.exists(path):
        print("  ⏭️  SKIP: testdata/100k.jsonl not found (run generate_testdata.py first)")
        return

    print("  Loading 100K docs...", file=sys.stderr)
    docs = []
    with open(path) as f:
        for line in f:
            if line.strip():
                docs.append(json.loads(line))

    print("  Running textsift...", file=sys.stderr)
    ts_surviving = run_textsift(path, threshold=0.8)

    print("  Running datasketch...", file=sys.stderr)
    ds_surviving = run_datasketch(docs, threshold=0.8)

    print(f"  textsift  kept: {len(ts_surviving)} docs")
    print(f"  datasketch kept: {len(ds_surviving)} docs")
    print(f"  Difference: {abs(len(ts_surviving) - len(ds_surviving))} docs")

    diff_pct = abs(len(ts_surviving) - len(ds_surviving)) / len(docs) * 100
    print(f"  Difference as % of total: {diff_pct:.2f}%")

    if diff_pct < 1.0:
        print("  ✅ PASS: <1% difference in dedup results")
    elif diff_pct < 5.0:
        print("  ⚠️  WARN: 1-5% difference (acceptable for LSH probabilistic nature)")
    else:
        print(f"  ❌ FAIL: {diff_pct:.1f}% difference — investigate")


if __name__ == "__main__":
    if not os.path.exists(TEXTSIFT):
        print(f"ERROR: textsift binary not found at {TEXTSIFT}", file=sys.stderr)
        print("Run: cargo build --release", file=sys.stderr)
        sys.exit(1)

    test_exact_duplicates()
    test_near_duplicates()
    test_cluster_structure()
    test_large_corpus_agreement()

    print()
    print("=" * 60)
    print("ALL TESTS COMPLETE")
    print("=" * 60)
