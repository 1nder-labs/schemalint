#!/usr/bin/env python3
"""
Compare schemalint predictions against OpenAI API ground truth.

This script can run in two modes:
1. **Live mode**: submits schema to OpenAI API directly.
2. **Offline mode**: reads previously saved API results via --api-results and
   compares without making any API calls.

Usage:
    # Live mode (makes API call):
    python scripts/validation/compare_with_openai.py crates/schemalint/tests/corpus/schema_03.json

    # Offline mode (reads saved API results, NO API call):
    python scripts/validation/compare_with_openai.py \
        crates/schemalint/tests/corpus/schema_03.json \
        --api-results scripts/validation/results/openai_bulk_2026-05-03.json
"""

import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path
from datetime import datetime

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


def run_schemalint(schema_path: str) -> list:
    """Run schemalint and return the predicted diagnostics."""
    profile = Path("crates/schemalint-profiles/profiles/openai.so.2026-04-30.toml")

    result = subprocess.run(
        ["cargo", "run", "-p", "schemalint", "--", "check", "--profile", str(profile), "--format", "json", schema_path],
        capture_output=True,
        text=True,
        cwd=Path(__file__).parent.parent.parent
    )

    try:
        output = json.loads(result.stdout)
        return output.get("diagnostics", [])
    except json.JSONDecodeError:
        print(f"Failed to parse schemalint output: {result.stdout}", file=sys.stderr)
        return []


def validate_with_openai(schema_path: str, api_key: str) -> dict:
    """Submit schema to OpenAI and capture the result. Makes an API call."""
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
        err_str = str(e)
        if "401" in err_str or "invalid_api_key" in err_str.lower():
            print(f"FATAL: Authentication error: {err_str[:200]}", file=sys.stderr)
            sys.exit(1)
        return {"rejected": True, "error": err_str}


def compare_one(schema_path: str, api_result: dict = None, api_key: str = None):
    """Compare schemalint vs API for a single schema."""
    name = Path(schema_path).name
    schemalint_diags = run_schemalint(schema_path)

    if api_result:
        api_rejected = api_result.get("status") == "rejected"
        api_error = api_result.get("api_error")
    elif api_key:
        result = validate_with_openai(schema_path, api_key)
        api_rejected = result["rejected"]
        api_error = result["error"]
    else:
        raise ValueError("Need either --api-results or OPENAI_API_KEY")

    schemalint_has_errors = len(schemalint_diags) > 0

    if schemalint_has_errors and not api_rejected:
        verdict = "FALSE_POSITIVE"
    elif not schemalint_has_errors and api_rejected:
        verdict = "FALSE_NEGATIVE"
    elif schemalint_has_errors and api_rejected:
        verdict = "AGREE_REJECT"
    else:
        verdict = "AGREE_ACCEPT"

    return {
        "schema": schema_path,
        "name": name,
        "verdict": verdict,
        "schemalint": {
            "issue_count": len(schemalint_diags),
            "diagnostics": schemalint_diags
        },
        "openai": {
            "rejected": api_rejected,
            "error": api_error
        }
    }


def main():
    import argparse
    parser = argparse.ArgumentParser(description="Compare schemalint vs OpenAI API")
    parser.add_argument("schemas", nargs="*", help="Schema files to compare")
    parser.add_argument("--api-results", help="Previously saved API results JSON (offline mode)")
    parser.add_argument("--output", help="Output file for results (default: auto-generated in results/)")
    parser.add_argument("--all", action="store_true", help="Compare all schemas in the corpus")
    args = parser.parse_args()

    if args.all:
        corpus_dir = Path("crates/schemalint/tests/corpus")
        schemas = sorted(corpus_dir.glob("schema_*.json"), key=lambda p: int(p.stem.split("_")[1]))
        args.schemas = [str(s) for s in schemas]

    if not args.schemas:
        parser.print_help()
        sys.exit(1)

    # Load API results if in offline mode
    api_results_map = {}
    if args.api_results:
        with open(args.api_results) as f:
            api_results_list = json.load(f)
        for r in api_results_list:
            name = Path(r["schema_path"]).name
            api_results_map[name] = r
        print(f"Loaded {len(api_results_map)} cached API results (offline mode)", file=sys.stderr)
    else:
        api_key = os.environ.get("OPENAI_API_KEY")
        if not api_key:
            print("Error: OPENAI_API_KEY not set. Use --api-results for offline mode.", file=sys.stderr)
            sys.exit(1)
        print("Live mode: will make API calls", file=sys.stderr)

    # Compare each schema
    results = []
    for schema_path in args.schemas:
        name = Path(schema_path).name
        api_result = api_results_map.get(name) if args.api_results else None
        api_key = None if args.api_results else os.environ.get("OPENAI_API_KEY")

        print(f"Comparing {name}...", file=sys.stderr)
        result = compare_one(schema_path, api_result=api_result, api_key=api_key)
        results.append(result)

        # Show mismatch immediately
        if result["verdict"] in ("FALSE_POSITIVE", "FALSE_NEGATIVE"):
            icon = "FP" if result["verdict"] == "FALSE_POSITIVE" else "FN"
            err = (result["openai"].get("error") or "")[:100]
            print(f"  {icon}: {result['verdict']} {err}", file=sys.stderr)

    # Determine output path
    if args.output:
        outpath = Path(args.output)
    else:
        date = datetime.now().strftime("%Y-%m-%d")
        outdir = Path(__file__).resolve().parent / "results"
        outdir.mkdir(exist_ok=True)
        outpath = outdir / f"compare_openai_{date}.json"

    with open(outpath, "w") as f:
        json.dump(results, f, indent=2)

    # Summary
    fps = sum(1 for r in results if r["verdict"] == "FALSE_POSITIVE")
    fns = sum(1 for r in results if r["verdict"] == "FALSE_NEGATIVE")
    agree = sum(1 for r in results if r["verdict"].startswith("AGREE"))
    total = len(results)
    print(f"\nSaved to {outpath}", file=sys.stderr)
    print(f"Total: {total}  |  Agree: {agree}  |  False positives: {fps}  |  False negatives: {fns}", file=sys.stderr)

    if fps > 0 or fns > 0:
        print(f"\nMISMATCHES:", file=sys.stderr)
        for r in results:
            if r["verdict"] in ("FALSE_POSITIVE", "FALSE_NEGATIVE"):
                diags = r["schemalint"]["diagnostics"]
                api_err = (r["openai"].get("error") or "API accepted")[:120]
                print(f"  {r['verdict']:16s}  {r['name']:20s}", file=sys.stderr)
                if diags:
                    codes = [d.get('code','?') for d in diags[:3]]
                    print(f"    schemalint: {codes}", file=sys.stderr)
                print(f"    openai: {api_err}", file=sys.stderr)


if __name__ == "__main__":
    main()
