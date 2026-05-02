#!/usr/bin/env python3
"""Generate .expected files for the regression corpus by running the CLI."""

import json
import subprocess
from pathlib import Path

CORPUS_DIR = Path("crates/schemalint/tests/corpus")
PROFILE = Path("crates/schemalint-profiles/profiles/openai.so.2026-04-30.toml")
BIN = Path("target/debug/schemalint")

schemas = sorted(CORPUS_DIR.glob("schema_*.json"))
print(f"Generating expected outputs for {len(schemas)} schemas...")

for schema_path in schemas:
    expected_path = schema_path.with_suffix(".expected")
    result = subprocess.run(
        [str(BIN), "check", "--profile", str(PROFILE), "--format", "json", str(schema_path)],
        capture_output=True,
        text=True
    )
    # We always expect exit code 0 or 1; any other code is an error
    if result.returncode not in (0, 1):
        print(f"ERROR: {schema_path.name} exited {result.returncode}")
        print(result.stderr)
        continue

    try:
        output = json.loads(result.stdout)
    except json.JSONDecodeError as e:
        print(f"ERROR: {schema_path.name} produced invalid JSON: {e}")
        continue

    # Store only the diagnostics array for easier comparison
    with open(expected_path, "w") as f:
        json.dump(output["diagnostics"], f, indent=2)
        f.write("\n")

    print(f"  {schema_path.name} -> {len(output['diagnostics'])} diagnostics")

print("Done.")
