#!/usr/bin/env python3
"""
Validate schemalint's OpenAI profile against the real OpenAI API.

This script takes JSON schemas and attempts to create them via OpenAI's
structured outputs API. If the API rejects the schema, we capture the error
and compare it against schemalint's predictions.

Usage:
    OPENAI_API_KEY=sk-xxx python scripts/validation/validate_openai.py \
        crates/schemalint/tests/corpus/schema_03.json

Requirements:
    pip install openai
"""

import json
import os
import sys
import time
from pathlib import Path
from typing import Optional


def validate_schema(schema_path: str, api_key: Optional[str] = None) -> dict:
    """Validate a single schema against OpenAI's API."""
    try:
        from openai import OpenAI
    except ImportError:
        print("Error: openai package not installed. Run: pip install openai")
        sys.exit(1)

    client = OpenAI(api_key=api_key or os.environ.get("OPENAI_API_KEY"))

    with open(schema_path) as f:
        schema = json.load(f)

    # Wrap in the response_format structure that OpenAI expects
    response_format = {
        "type": "json_schema",
        "json_schema": {
            "name": "test_schema",
            "strict": True,
            "schema": schema
        }
    }

    try:
        # We use a minimal prompt and max_tokens to minimize cost
        response = client.chat.completions.create(
            model="gpt-4o-2024-08-06",
            messages=[
                {"role": "system", "content": "Return any valid JSON."},
                {"role": "user", "content": "Test"}
            ],
            response_format=response_format,
            max_tokens=1,
            temperature=0
        )
        return {
            "schema_path": schema_path,
            "status": "accepted",
            "schema_rejected": False,
            "api_error": None
        }
    except Exception as e:
        error_str = str(e)
        return {
            "schema_path": schema_path,
            "status": "rejected",
            "schema_rejected": True,
            "api_error": error_str
        }


def main():
    if len(sys.argv) < 2:
        print(f"Usage: {sys.argv[0]} <schema.json> [schema2.json ...]")
        print("")
        print("Environment:")
        print("  OPENAI_API_KEY - Your OpenAI API key")
        sys.exit(1)

    api_key = os.environ.get("OPENAI_API_KEY")
    if not api_key:
        print("Error: OPENAI_API_KEY environment variable not set")
        sys.exit(1)

    results = []
    for schema_path in sys.argv[1:]:
        print(f"Validating {schema_path}...", file=sys.stderr)
        result = validate_schema(schema_path, api_key)
        results.append(result)

        # Rate limit: max 5 requests per second for validation
        time.sleep(0.2)

    # Print results as JSON
    print(json.dumps(results, indent=2))

    # Summary
    accepted = sum(1 for r in results if r["status"] == "accepted")
    rejected = sum(1 for r in results if r["status"] == "rejected")
    print(f"\nSummary: {accepted} accepted, {rejected} rejected", file=sys.stderr)


if __name__ == "__main__":
    main()
