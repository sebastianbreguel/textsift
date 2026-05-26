"""Generate synthetic JSONL corpus with controlled duplicate rates.

Usage:
    uv run scripts/generate_testdata.py --size 100000 --exact-dup-rate 0.1 --near-dup-rate 0.1 --output testdata/100k.jsonl
"""
import argparse
import json
import random
import string
import sys


def random_text(word_count=50):
    words = []
    for _ in range(word_count):
        length = random.randint(3, 10)
        words.append("".join(random.choices(string.ascii_lowercase, k=length)))
    return " ".join(words)


def make_near_duplicate(text, change_rate=0.2):
    words = text.split()
    n_changes = max(1, int(len(words) * change_rate))
    indices = random.sample(range(len(words)), min(n_changes, len(words)))
    for idx in indices:
        length = random.randint(3, 10)
        words[idx] = "".join(random.choices(string.ascii_lowercase, k=length))
    return " ".join(words)


def main():
    parser = argparse.ArgumentParser(description="Generate synthetic dedup benchmark data")
    parser.add_argument("--size", type=int, default=100_000, help="Total docs to generate")
    parser.add_argument("--exact-dup-rate", type=float, default=0.10, help="Fraction of exact duplicates")
    parser.add_argument("--near-dup-rate", type=float, default=0.10, help="Fraction of near-duplicates")
    parser.add_argument("--word-count", type=int, default=50, help="Words per document")
    parser.add_argument("--output", required=True, help="Output JSONL file")
    parser.add_argument("--seed", type=int, default=42, help="Random seed")
    args = parser.parse_args()

    random.seed(args.seed)

    n_exact = int(args.size * args.exact_dup_rate)
    n_near = int(args.size * args.near_dup_rate)
    n_unique = args.size - n_exact - n_near

    print(f"Generating {args.size} docs: {n_unique} unique, {n_exact} exact dupes, {n_near} near dupes",
          file=sys.stderr)

    unique_texts = [random_text(args.word_count) for _ in range(n_unique)]

    docs = []
    for i, text in enumerate(unique_texts):
        docs.append({"text": text, "id": i})

    for i in range(n_exact):
        source = random.choice(unique_texts)
        docs.append({"text": source, "id": n_unique + i})

    for i in range(n_near):
        source = random.choice(unique_texts)
        near = make_near_duplicate(source, change_rate=0.2)
        docs.append({"text": near, "id": n_unique + n_exact + i})

    random.shuffle(docs)

    with open(args.output, "w") as f:
        for doc in docs:
            f.write(json.dumps(doc) + "\n")

    print(f"Written to {args.output}", file=sys.stderr)


if __name__ == "__main__":
    main()
