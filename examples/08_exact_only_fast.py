"""Exact-only mode — fastest dedup when you only care about identical texts.

Skips MinHash entirely. Useful for:
- Removing copy-paste duplicates from scraped data
- Quick first pass before running full near-dedup
- Datasets where near-duplicates aren't a concern
"""
from textsift import dedup

texts = [
    "First unique document",
    "Second unique document",
    "First unique document",   # exact copy
    "Third unique document",
    "Second unique document",  # exact copy
    "Second unique document",  # exact copy
    "Fourth unique document",
]

result = dedup(texts, exact_only=True)

print(f"Exact-only dedup: {result.total} → {result.unique_clusters} docs")
print(f"Removed {result.exact_dupes} exact duplicates")
print()

# Two-pass approach: exact first, then near-dedup on the survivors
clean_texts = [t for t, keep in zip(texts, result.is_representative) if keep]
result2 = dedup(clean_texts, threshold=0.8)
print(f"Second pass (near-dedup): {result2.total} → {result2.unique_clusters} docs")
print(f"Removed {result2.near_dupes} near duplicates")
