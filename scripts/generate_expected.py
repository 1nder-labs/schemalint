#!/usr/bin/env python3
"""Generate .expected files for the regression corpus by running the CLI.

Covers both corpora: `schema_*` files are linted with the OpenAI profile,
`ant_schema_*` files with the Anthropic profile.
"""

import json
import subprocess
from pathlib import Path

CORPUS_DIR = Path("crates/schemalint/tests/corpus")
BIN = Path("target/debug/schemalint")
PROFILES_DIR = Path("crates/schemalint-profiles/profiles")

# (filename prefix, profile path) pairs covering every corpus schema.
CORPORA = [
    ("schema_", PROFILES_DIR / "openai.so.2026-04-30.toml"),
    ("ant_schema_", PROFILES_DIR / "anthropic.so.2026-04-30.toml"),
]

total = 0
for prefix, profile in CORPORA:
    schemas = sorted(CORPUS_DIR.glob(f"{prefix}*.json"))
    print(f"Generating expected outputs for {len(schemas)} {prefix}* schemas ({profile.name})...")

    for schema_path in schemas:
        expected_path = schema_path.with_suffix(".expected")
        result = subprocess.run(
            [str(BIN), "check", "--profile", str(profile), "--format", "json", str(schema_path)],
            capture_output=True,
            text=True,
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

        total += 1
        print(f"  {schema_path.name} -> {len(output['diagnostics'])} diagnostics")

print(f"Done. {total} expected files written.")
