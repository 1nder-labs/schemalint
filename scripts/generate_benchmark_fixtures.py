#!/usr/bin/env python3
"""Generate benchmark fixtures."""

import json
import os
from pathlib import Path

FIXTURES_DIR = Path("benches/fixtures")
FIXTURES_DIR.mkdir(parents=True, exist_ok=True)

# ---------------------------------------------------------------------------
# Single large schema: 200 properties, nested 3 levels
# ---------------------------------------------------------------------------

single_large = {"type": "object", "properties": {}, "additionalProperties": False}
for i in range(200):
    prop = f"prop_{i:03d}"
    if i % 10 == 0 and i > 0:
        # Every 10th property is a nested object with 5 sub-properties
        single_large["properties"][prop] = {
            "type": "object",
            "properties": {},
            "additionalProperties": False
        }
        for j in range(5):
            single_large["properties"][prop]["properties"][f"sub_{j}"] = {"type": "string"}
    elif i % 7 == 0:
        # Every 7th property is an array
        single_large["properties"][prop] = {
            "type": "array",
            "items": {"type": "string"}
        }
    elif i % 5 == 0:
        # Every 5th property has an enum
        single_large["properties"][prop] = {
            "type": "string",
            "enum": ["a", "b", "c"]
        }
    else:
        single_large["properties"][prop] = {"type": "string"}

with open(FIXTURES_DIR / "single_large_schema.json", "w") as f:
    json.dump(single_large, f)
    f.write("\n")

print(f"Generated single_large_schema.json with {len(single_large['properties'])} properties")

# ---------------------------------------------------------------------------
# 500 schemas for cold-start benchmark
# ---------------------------------------------------------------------------

project_dir = FIXTURES_DIR / "project_500_schemas"
project_dir.mkdir(parents=True, exist_ok=True)

for i in range(500):
    schema = {
        "type": "object",
        "properties": {},
        "required": [],
        "additionalProperties": False
    }
    # Vary complexity: most are simple, some have nested objects or arrays
    num_props = 3 + (i % 7)
    for j in range(num_props):
        prop_name = f"field_{j}"
        if j % 3 == 0:
            schema["properties"][prop_name] = {"type": "string"}
        elif j % 3 == 1:
            schema["properties"][prop_name] = {"type": "integer"}
        else:
            schema["properties"][prop_name] = {
                "type": "object",
                "properties": {"nested": {"type": "string"}},
                "additionalProperties": False
            }
        schema["required"].append(prop_name)

    with open(project_dir / f"schema_{i:03d}.json", "w") as f:
        json.dump(schema, f)
        f.write("\n")

print(f"Generated 500 schemas in {project_dir}")

# ---------------------------------------------------------------------------
# OpenAI profile bytes for benchmarks
# ---------------------------------------------------------------------------

profile_path = Path("crates/schemalint-profiles/profiles/openai.so.2026-04-30.toml")
with open(profile_path, "rb") as f:
    profile_bytes = f.read()

# Write a copy to fixtures for easy access
with open(FIXTURES_DIR / "openai_profile.toml", "wb") as f:
    f.write(profile_bytes)

print(f"Copied OpenAI profile ({len(profile_bytes)} bytes)")
