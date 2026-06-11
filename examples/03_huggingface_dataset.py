"""Deduplicate a HuggingFace dataset before training.

Common workflow: download dataset from HF Hub, dedup, save clean version.

Usage:
    uv run --with datasets --with textsift examples/03_huggingface_dataset.py
"""
from datasets import load_dataset
from textsift import dedup

ds = load_dataset("stanfordnlp/imdb", split="train")
texts = ds["text"]

print(f"Original: {len(texts):,} docs")

result = dedup(texts, threshold=0.8)

keep_indices = result.unique_indices()
clean_ds = ds.select(keep_indices)

print(f"After dedup: {len(clean_ds):,} docs")
print(f"Removed: {result.exact_dupes} exact + {result.near_dupes} near dupes")

# clean_ds.save_to_disk("imdb_deduped")
