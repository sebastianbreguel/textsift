"""Dedup instruction-tuning data before fine-tuning an LLM.

Duplicate training examples waste compute and can cause the model to
memorize specific outputs. This script deduplicates on the instruction
field (or any field you choose).
"""
from textsift import dedup

training_data = [
    {"instruction": "Explain what machine learning is", "output": "Machine learning is..."},
    {"instruction": "What is machine learning?", "output": "ML is a branch of AI..."},
    {"instruction": "Explain what machine learning is", "output": "It's a type of AI..."},  # exact dup instruction
    {"instruction": "Write a Python hello world", "output": "print('hello')"},
    {"instruction": "Write hello world in Python", "output": "print('Hello, World!')"},
    {"instruction": "Translate 'hello' to Spanish", "output": "hola"},
]

# Dedup on instruction field
instructions = [item["instruction"] for item in training_data]
result = dedup(instructions, threshold=0.7, shingle_size=3)

clean_data = [item for item, keep in zip(training_data, result.is_representative) if keep]

print(f"Training examples: {len(training_data)} → {len(clean_data)} after dedup")
print()
for item in clean_data:
    print(f"  {item['instruction']}")
