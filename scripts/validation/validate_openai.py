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

# Load .env file from script directory if present
def _load_env():
    env_path = Path(__file__).resolve().parent / ".env"
    if env_path.exists():
        try:
            from dotenv import load_dotenv
            load_dotenv(env_path)
        except ImportError:
            with open(env_path) as f:
                for line in f:
                    line = line.strip()
                    if line and not line.startswith("#") and "=" in line:
                        key, _, val = line.partition("=")
                        os.environ.setdefault(key.strip(), val.strip().strip('"\''))
_load_env()


# Known models with structured outputs support and their keyword differences.
# Source: https://platform.openai.com/docs/guides/structured-outputs
MODELS = {
    "gpt-4o": {
        "id": "gpt-4o-2024-08-06",
        "desc": "GPT-4o (full). Supports all non-composition keywords.",
    },
    "gpt-4o-mini": {
        "id": "gpt-4o-mini",
        "desc": "GPT-4o-mini. Same schema support as gpt-4o.",
    },
    "ft": {
        "id": "gpt-4o-2024-08-06",
        "desc": "Fine-tuned models. Additionally forbids: minLength, maxLength, pattern, format, minimum, maximum, multipleOf, patternProperties, minItems, maxItems",
        "extra_forbidden": ["minLength", "maxLength", "pattern", "format", "minimum", "maximum", "multipleOf", "patternProperties", "minItems", "maxItems"],
    },
}


def resolve_model(model_key: str) -> dict:
    """Resolve a short model key to its config."""
    if model_key in MODELS:
        return MODELS[model_key]
    # Allow raw model IDs
    return {"id": model_key, "desc": model_key}


def validate_schema(schema_path: str, api_key: Optional[str] = None, model: str = "gpt-4o") -> dict:
    """Validate a single schema against OpenAI's API."""
    try:
        from openai import OpenAI
    except ImportError:
        print("Error: openai package not installed. Run: pip install openai")
        sys.exit(1)

    client = OpenAI(api_key=api_key or os.environ.get("OPENAI_API_KEY"))
    model_config = resolve_model(model)

    with open(schema_path) as f:
        schema = json.load(f)

    try:
        client.responses.create(
            model=model_config["id"],
            input=[
                {"role": "system", "content": "Return any valid JSON."},
                {"role": "user", "content": "Test"}
            ],
            text={
                "format": {
                    "type": "json_schema",
                    "name": "test_schema",
                    "strict": True,
                    "schema": schema
                }
            }
        )
        return {
            "schema_path": schema_path,
            "model": model_config["id"],
            "model_key": model,
            "status": "accepted",
            "schema_rejected": False,
            "api_error": None
        }
    except Exception as e:
        error_str = str(e)
        return {
            "schema_path": schema_path,
            "model": model_config["id"],
            "model_key": model,
            "status": "rejected",
            "schema_rejected": True,
            "api_error": error_str
        }


def main():
    import argparse

    parser = argparse.ArgumentParser(
        description="Validate JSON schemas against OpenAI Structured Outputs API",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  python validate_openai.py schema.json
  python validate_openai.py --model gpt-4o-mini schema.json
  python validate_openai.py --model ft schema.json           # fine-tuned restrictions
  python validate_openai.py --output results.json *.json

Models:
  gpt-4o        GPT-4o (full keyword support)
  gpt-4o-mini   GPT-4o-mini (same support as gpt-4o)
  ft            Fine-tuned models (additional forbidden keywords)
"""
    )
    parser.add_argument("schemas", nargs="*", help="JSON schema files to validate")
    parser.add_argument("--model", default="gpt-4o", help="Model to test against (default: gpt-4o)")
    parser.add_argument("--list-models", action="store_true", help="List available models and exit")
    parser.add_argument("--output", "-o", help="Save results to file instead of stdout")
    args = parser.parse_args()

    if args.list_models:
        print("Available models:")
        for key, info in MODELS.items():
            print(f"  {key:15s}  {info['desc']}")
            if info.get("extra_forbidden"):
                print(f"  {' ':15s}  Extra forbidden: {', '.join(info['extra_forbidden'])}")
        return

    if not args.schemas:
        parser.print_help()
        sys.exit(1)

    api_key = os.environ.get("OPENAI_API_KEY")
    if not api_key:
        print("Error: OPENAI_API_KEY environment variable not set")
        sys.exit(1)

    results = []
    for schema_path in args.schemas:
        print(f"Validating {schema_path}...", file=sys.stderr)
        result = validate_schema(schema_path, api_key, args.model)
        results.append(result)

        # Stop immediately on auth/permission errors (401, 403)
        if result.get("api_error"):
            err_lower = result["api_error"].lower()
            if "401" in err_lower or "403" in err_lower or "invalid_api_key" in err_lower or "permission" in err_lower:
                print(f"\nFATAL: Authentication error. Check your API key.", file=sys.stderr)
                print(json.dumps(results, indent=2))
                sys.exit(1)

        # Rate limit: max 5 requests per second for validation
        time.sleep(0.2)

    # Print/save results
    output_json = json.dumps(results, indent=2)
    if args.output:
        with open(args.output, "w") as f:
            f.write(output_json + "\n")
        print(f"\nSaved to {args.output}", file=sys.stderr)
    else:
        print(output_json)

    # Summary
    accepted = sum(1 for r in results if r["status"] == "accepted")
    rejected = sum(1 for r in results if r["status"] == "rejected")
    print(f"\nSummary: {accepted} accepted, {rejected} rejected (model: {args.model})", file=sys.stderr)

    # If tested against ft model, flag which keywords need a separate profile
    if args.model == "ft":
        extra = MODELS["ft"].get("extra_forbidden", [])
        print(f"\nFine-tuned model keywords that SHOULD be forbidden (not in current profile):", file=sys.stderr)
        for kw in extra:
            print(f"  - {kw}", file=sys.stderr)


if __name__ == "__main__":
    main()
