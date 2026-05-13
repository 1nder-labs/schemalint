#!/usr/bin/env python3
"""
Validate schemalint's Anthropic profile against the real Claude API.

The Claude API exposes the same JSON Schema subset for JSON outputs
(`output_config.format`) and strict tool use (`strict: true`). This script can
exercise either surface and records transport/API failures separately from
schema rejections.

Usage:
    ANTHROPIC_API_KEY=sk-ant-xxx python scripts/validation/validate_anthropic.py \
        crates/schemalint/tests/corpus/schema_03.json
"""

import json
import os
import sys
import time
from pathlib import Path


def _load_env() -> None:
    env_path = Path(__file__).resolve().parent / ".env"
    if not env_path.exists():
        return
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

MODELS = {
    "opus-4.7": {
        "id": "claude-opus-4-7",
        "desc": "Claude Opus 4.7 with native structured outputs.",
    },
    "sonnet-4.6": {
        "id": "claude-sonnet-4-6",
        "desc": "Claude Sonnet 4.6 with native structured outputs.",
    },
    "sonnet-4.5": {
        "id": "claude-sonnet-4-5",
        "desc": "Claude Sonnet 4.5 with native structured outputs.",
    },
    "haiku-4.5": {
        "id": "claude-haiku-4-5",
        "desc": "Claude Haiku 4.5 with native structured outputs.",
    },
}


def resolve_model(model_key: str) -> dict:
    return MODELS.get(model_key, {"id": model_key, "desc": model_key})


def is_schema_error(error: Exception) -> bool:
    status_code = getattr(error, "status_code", None)
    error_text = str(error).lower()
    schema_terms = (
        "schema",
        "output_config",
        "output format",
        "input_schema",
        "strict",
        "too complex",
    )
    return status_code == 400 and any(term in error_text for term in schema_terms)


def validate_schema(
    schema_path: str,
    client,
    model_config: dict,
    model_key: str,
    surface: str = "output",
) -> dict:
    with open(schema_path) as f:
        schema = json.load(f)

    try:
        if surface == "tool":
            client.messages.create(
                model=model_config["id"],
                max_tokens=64,
                messages=[{"role": "user", "content": "Call the test tool."}],
                tools=[
                    {
                        "name": "test_schema",
                        "description": "Validate the supplied schema.",
                        "strict": True,
                        "input_schema": schema,
                    }
                ],
                tool_choice={"type": "tool", "name": "test_schema"},
            )
        else:
            client.messages.create(
                model=model_config["id"],
                max_tokens=64,
                messages=[{"role": "user", "content": "Return any valid JSON."}],
                output_config={
                    "format": {
                        "type": "json_schema",
                        "schema": schema,
                    }
                },
            )
        return {
            "schema_path": schema_path,
            "model": model_config["id"],
            "model_key": model_key,
            "surface": surface,
            "status": "accepted",
            "schema_rejected": False,
            "api_error": None,
        }
    except Exception as error:
        if not is_schema_error(error):
            return {
                "schema_path": schema_path,
                "model": model_config["id"],
                "model_key": model_key,
                "surface": surface,
                "status": "error",
                "schema_rejected": False,
                "api_error": str(error),
            }
        return {
            "schema_path": schema_path,
            "model": model_config["id"],
            "model_key": model_key,
            "surface": surface,
            "status": "rejected",
            "schema_rejected": True,
            "api_error": str(error),
        }


def main() -> None:
    import argparse

    parser = argparse.ArgumentParser(
        description="Validate JSON schemas against Anthropic Structured Outputs",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  python validate_anthropic.py schema.json
  python validate_anthropic.py --surface tool schema.json
  python validate_anthropic.py --model sonnet-4.6 --output results.json *.json
""",
    )
    parser.add_argument("schemas", nargs="*", help="JSON schema files to validate")
    parser.add_argument("--model", default="opus-4.7", help="Model key or raw model ID")
    parser.add_argument(
        "--surface",
        choices=["output", "tool"],
        default="output",
        help="Validate output_config.format or strict tool input_schema",
    )
    parser.add_argument("--list-models", action="store_true", help="List model keys")
    parser.add_argument("--output", "-o", help="Save results to file instead of stdout")
    args = parser.parse_args()

    if args.list_models:
        print("Available models:")
        for key, info in MODELS.items():
            print(f"  {key:15s}  {info['desc']}")
        return

    if not args.schemas:
        parser.print_help()
        sys.exit(1)

    api_key = os.environ.get("ANTHROPIC_API_KEY")
    if not api_key:
        print("Error: ANTHROPIC_API_KEY environment variable not set")
        sys.exit(1)

    try:
        from anthropic import Anthropic
    except ImportError:
        print("Error: anthropic package not installed. Run: pip install anthropic")
        sys.exit(1)

    client = Anthropic(api_key=api_key)
    model_config = resolve_model(args.model)

    results = []
    for schema_path in args.schemas:
        print(f"Validating {schema_path}...", file=sys.stderr)
        result = validate_schema(schema_path, client, model_config, args.model, args.surface)
        results.append(result)

        if result.get("api_error"):
            err_lower = result["api_error"].lower()
            if "401" in err_lower or "403" in err_lower or "authentication" in err_lower:
                print("\nFATAL: Authentication error. Check your API key.", file=sys.stderr)
                print(json.dumps(results, indent=2))
                sys.exit(1)

        time.sleep(0.2)

    output_json = json.dumps(results, indent=2)
    if args.output:
        with open(args.output, "w") as f:
            f.write(output_json + "\n")
        print(f"\nSaved to {args.output}", file=sys.stderr)
    else:
        print(output_json)

    accepted = sum(1 for r in results if r["status"] == "accepted")
    rejected = sum(1 for r in results if r["status"] == "rejected")
    errors = sum(1 for r in results if r["status"] == "error")
    print(
        f"\nSummary: {accepted} accepted, {rejected} rejected, {errors} transport/API errors "
        f"(model: {args.model}, surface: {args.surface})",
        file=sys.stderr,
    )
    if errors:
        sys.exit(2)


if __name__ == "__main__":
    main()
