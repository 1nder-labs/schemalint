#!/usr/bin/env python3
"""
Compare schemalint predictions against OpenAI API ground truth.

This script:
1. Runs schemalint on a schema to get predicted errors
2. Submits the schema to OpenAI API to get actual errors
3. Compares the two and reports mismatches

Usage:
    OPENAI_API_KEY=sk-xxx python scripts/validation/compare_with_openai.py \
        crates/schemalint/tests/corpus/schema_03.json
"""

import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path


def run_schemalint(schema_path: str) -> list:
    """Run schemalint and return the predicted diagnostics."""
    profile = Path("crates/schemalint-profiles/profiles/openai.so.2026-04-30.toml")

    result = subprocess.run(
        ["cargo", "run", "--", "check", "--profile", str(profile), "--format", "json", schema_path],
        capture_output=True,
        text=True,
        cwd=Path(__file__).parent.parent.parent
    )

    # schemalint outputs JSON to stdout regardless of exit code
    try:
        output = json.loads(result.stdout)
        return output.get("diagnostics", [])
    except json.JSONDecodeError:
        print(f"Failed to parse schemalint output: {result.stdout}", file=sys.stderr)
        return []


def validate_with_openai(schema_path: str, api_key: str) -> dict:
    """Submit schema to OpenAI and capture the result."""
    try:
        from openai import OpenAI
    except ImportError:
        print("Error: openai package not installed. Run: pip install openai")
        sys.exit(1)

    client = OpenAI(api_key=api_key)

    with open(schema_path) as f:
        schema = json.load(f)

    try:
        client.chat.completions.create(
            model="gpt-4o-2024-08-06",
            messages=[
                {"role": "system", "content": "Return any valid JSON."},
                {"role": "user", "content": "Test"}
            ],
            response_format={
                "type": "json_schema",
                "json_schema": {
                    "name": "test",
                    "strict": True,
                    "schema": schema
                }
            },
            max_tokens=1
        )
        return {"rejected": False, "error": None}
    except Exception as e:
        return {"rejected": True, "error": str(e)}


def main():
    if len(sys.argv) < 2:
        print(f"Usage: {sys.argv[0]} <schema.json>")
        sys.exit(1)

    schema_path = sys.argv[1]
    api_key = os.environ.get("OPENAI_API_KEY")

    if not api_key:
        print("Error: OPENAI_API_KEY not set")
        sys.exit(1)

    print("=== Step 1: Running schemalint ===", file=sys.stderr)
    schemalint_diags = run_schemalint(schema_path)
    print(f"schemalint found {len(schemalint_diags)} issues:", file=sys.stderr)
    for d in schemalint_diags:
        print(f"  [{d['severity']}] {d['code']}: {d['message']}", file=sys.stderr)

    print("\n=== Step 2: Validating with OpenAI API ===", file=sys.stderr)
    openai_result = validate_with_openai(schema_path, api_key)

    if openai_result["rejected"]:
        print(f"OpenAI REJECTED: {openai_result['error']}", file=sys.stderr)
    else:
        print("OpenAI ACCEPTED the schema", file=sys.stderr)

    print("\n=== Step 3: Comparison ===", file=sys.stderr)
    if schemalint_diags and not openai_result["rejected"]:
        print("MISMATCH: schemalint found errors but OpenAI accepted", file=sys.stderr)
        print("  → Potential false positive in profile", file=sys.stderr)
    elif not schemalint_diags and openai_result["rejected"]:
        print("MISMATCH: schemalint passed but OpenAI rejected", file=sys.stderr)
        print("  → Missing rule or incorrect profile", file=sys.stderr)
        print(f"  → OpenAI error: {openai_result['error']}", file=sys.stderr)
    elif schemalint_diags and openai_result["rejected"]:
        print("AGREE: Both found issues (good)", file=sys.stderr)
    else:
        print("AGREE: Both passed (good)", file=sys.stderr)

    # Full structured output
    result = {
        "schema": schema_path,
        "schemalint": {
            "issue_count": len(schemalint_diags),
            "diagnostics": schemalint_diags
        },
        "openai": openai_result
    }
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
