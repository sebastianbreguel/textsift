"""Basic deduplication — remove exact and near-duplicate texts.

This is the simplest use case: you have a list of texts and want the unique ones.
"""
from textsift import dedup

texts = [
    "Machine learning is a subset of artificial intelligence",
    "Machine learning is a subset of artificial intelligence",  # exact dup
    "Deep learning uses neural networks with many layers",
    "Machine learning is a branch of artificial intelligence",  # near dup
    "Natural language processing handles text data",
    "Deep learning uses neural networks with many layers",  # exact dup
]

result = dedup(texts, threshold=0.8)

# Filter to unique texts
clean = [t for t, keep in zip(texts, result.is_representative) if keep]

print(f"Input:  {result.total} docs")
print(f"Output: {result.unique_clusters} unique")
print(f"Removed: {result.exact_dupes} exact + {result.near_dupes} near dupes")
print()
for text in clean:
    print(f"  {text}")
