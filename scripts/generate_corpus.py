#!/usr/bin/env python3
"""Generate 50 synthetic schemas for the regression corpus."""

import json
from pathlib import Path

CORPUS_DIR = Path("crates/schemalint/tests/corpus")

def write_schema(idx: int, schema: dict):
    path = CORPUS_DIR / f"schema_{idx:02d}.json"
    with open(path, "w") as f:
        json.dump(schema, f, indent=2)
        f.write("\n")

from corpus_cases_part1 import append_part1
from corpus_cases_part2 import append_part2

schemas = []
append_part1(schemas)
append_part2(schemas)

# Write all schemas
CORPUS_DIR.mkdir(parents=True, exist_ok=True)
for i, schema in enumerate(schemas, 1):
    write_schema(i, schema)

print(f"Generated {len(schemas)} schemas in {CORPUS_DIR}")
