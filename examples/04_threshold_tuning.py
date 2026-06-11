"""Tune the similarity threshold to control dedup aggressiveness.

Lower threshold = more aggressive (catches looser matches).
Higher threshold = more conservative (only very similar pairs).

Run this on your data to find the sweet spot.
"""
from textsift import dedup

texts = [
    "The quick brown fox jumps over the lazy dog",
    "The quick brown fox leaps over the lazy dog",           # 1 word diff
    "A quick brown fox jumps over a lazy dog",               # 2 words diff
    "The slow white cat crawls under the energetic dog",     # very different
    "Fast brown fox jumping over lazy dogs in the park",     # moderate diff
    "The quick brown fox jumps over the lazy dog again",     # 1 word added
]

print(f"Testing {len(texts)} docs at different thresholds\n")

for threshold in [0.3, 0.5, 0.7, 0.8, 0.9, 1.0]:
    result = dedup(texts, threshold=threshold)
    n_dupes = result.exact_dupes + result.near_dupes
    print(f"  threshold={threshold:.1f}  →  {result.unique_clusters} unique, {n_dupes} dupes")

print()
print("Lower threshold = more dedup. Pick based on your tolerance for false positives.")
