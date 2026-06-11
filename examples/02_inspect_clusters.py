"""Inspect duplicate clusters — see which docs are grouped together.

Useful when you want to understand the duplication patterns in your data
before deciding what to remove.
"""
from textsift import dedup

texts = [
    "The quick brown fox jumps over the lazy dog in the sunny meadow",
    "The quick brown fox jumps over the lazy dog in the green meadow",
    "The quick brown fox jumps over the lazy dog in the sunny meadow near the river",
    "Python is a popular programming language used for machine learning and data science",
    "Python is a popular programming language used for machine learning and AI research",
    "Rust is a systems programming language focused on safety and performance",
]

result = dedup(texts, threshold=0.5, shingle_size=3)

# Group by cluster
clusters: dict[int, list[tuple[int, str, bool]]] = {}
for i, (text, cid, rep) in enumerate(zip(texts, result.cluster_ids, result.is_representative)):
    clusters.setdefault(cid, []).append((i, text, rep))

for cid, members in sorted(clusters.items()):
    if len(members) == 1:
        print(f"Cluster {cid}: unique")
        print(f"  [{members[0][0]}] {members[0][1]}")
    else:
        print(f"Cluster {cid}: {len(members)} duplicates")
        for idx, text, is_rep in members:
            tag = "KEEP" if is_rep else "DROP"
            print(f"  [{idx}] ({tag}) {text}")
    print()
