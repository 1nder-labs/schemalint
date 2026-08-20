#!/usr/bin/env python3
"""Run opt-in OpenAI Structured Outputs boundary probes."""

import argparse
import json
import os
import sys
import time
from datetime import date
from pathlib import Path

from _env import load_env
from openai_errors import is_openai_schema_error

DEFAULT_MODEL = "gpt-4o-2024-08-06"
RESULTS_DIR = Path(__file__).resolve().parent / "results"


def nested(levels):
    node = {"type": "string"}
    for level in range(levels, 0, -1):
        name = f"l{level}"
        node = {
            "type": "object",
            "properties": {name: node},
            "required": [name],
            "additionalProperties": False,
        }
    return node


def local_ref_chain(depth):
    """Build a root-to-terminal chain containing exactly ``depth`` local refs."""
    if depth < 1:
        raise ValueError("reference depth must be positive")
    definitions = {}
    for hop in range(1, depth + 1):
        definitions[f"hop{hop}"] = (
            {"type": "string"}
            if hop == depth
            else {"$ref": f"#/$defs/hop{hop + 1}"}
        )
    return {
        "type": "object",
        "properties": {"value": {"$ref": "#/$defs/hop1"}},
        "required": ["value"],
        "additionalProperties": False,
        "$defs": definitions,
    }


def local_ref_cycle(size):
    """Build a local-reference cycle with ``size`` definition nodes."""
    if size < 1:
        raise ValueError("cycle size must be positive")
    definitions = {
        f"node{index}": {"$ref": f"#/$defs/node{index % size + 1}"}
        for index in range(1, size + 1)
    }
    return {
        "type": "object",
        "properties": {"value": {"$ref": "#/$defs/node1"}},
        "required": ["value"],
        "additionalProperties": False,
        "$defs": definitions,
    }


def enum_schema(n_values, val_len):
    values = [f"{index:0{val_len}d}"[:val_len] for index in range(n_values)]
    if len(set(values)) != n_values:
        raise ValueError("enum values must be unique")
    return {
        "type": "object",
        "properties": {"e": {"type": "string", "enum": values}},
        "required": ["e"],
        "additionalProperties": False,
    }


def classify_exception(error):
    text = str(error)
    status_code = getattr(error, "status_code", None)
    if is_openai_schema_error(error):
        return {
            "kind": "provider_verdict",
            "status": "rejected",
            "error": text.split("- ", 1)[-1][:160],
        }
    if status_code in (401, 403):
        category = "authentication"
    elif status_code == 429:
        category = "rate_limit"
    elif status_code is None:
        category = "transport"
    elif status_code >= 500:
        category = "provider_service"
    else:
        category = "api_error"
    return {
        "kind": "infrastructure_failure",
        "category": category,
        "error": text[:160],
    }


def submit(client, schema, model=DEFAULT_MODEL):
    try:
        client.responses.create(
            model=model,
            input=[
                {"role": "system", "content": "x"},
                {"role": "user", "content": "x"},
            ],
            text={
                "format": {
                    "type": "json_schema",
                    "name": "p",
                    "strict": True,
                    "schema": schema,
                }
            },
        )
        return {"kind": "provider_verdict", "status": "accepted", "error": None}
    except Exception as error:  # noqa: BLE001
        return classify_exception(error)


def record_outcome(results, infrastructure, section, case, outcome):
    if outcome["kind"] == "provider_verdict":
        results[section].append(
            {**case, "status": outcome["status"], "error": outcome["error"]}
        )
        return True
    infrastructure.append(
        {
            "section": section,
            **case,
            "category": outcome["category"],
            "error": outcome["error"],
        }
    )
    return False


def artifact_path(requested=None, today=None):
    if requested is not None:
        return Path(requested)
    stamp = (today or date.today()).isoformat()
    return RESULTS_DIR / f"probe_limits_{stamp}.json"


def probe_groups():
    enum_cases = [
        ("250 vals x10c = 2500", 250, 10),
        ("251 vals x10c = 2510", 251, 10),
        ("300 vals x40c = 12000", 300, 40),
        ("300 vals x50c = 15000", 300, 50),
        ("300 vals x60c = 18000", 300, 60),
        ("250 vals x80c = 20000", 250, 80),
        ("260 vals x60c = 15600", 260, 60),
    ]
    return [
        (
            "depth",
            "INLINE DEPTH",
            [({"levels": levels}, nested(levels)) for levels in range(1, 14)],
        ),
        (
            "ref_depth",
            "LOCAL $REF CHAIN DEPTH",
            [({"hops": hops}, local_ref_chain(hops)) for hops in (10, 11)],
        ),
        (
            "ref_cycle",
            "CYCLIC LOCAL $REF",
            [({"cycle_size": size}, local_ref_cycle(size)) for size in (1, 2)],
        ),
        (
            "enum",
            "ENUM STRING BUDGET",
            [
                (
                    {
                        "label": label,
                        "n_values": count,
                        "val_len": length,
                        "total_chars": count * length,
                    },
                    enum_schema(count, length),
                )
                for label, count, length in enum_cases
            ],
        ),
    ]


def run_probe(client, model=DEFAULT_MODEL, delay=0.2, probe_date=None):
    results = {
        "probe_date": (probe_date or date.today()).isoformat(),
        "model": model,
        "depth": [],
        "ref_depth": [],
        "ref_cycle": [],
        "enum": [],
    }
    infrastructure = []
    for section, title, cases in probe_groups():
        print(f"\n== {title} ==")
        for case, schema in cases:
            outcome = submit(client, schema, model)
            label = ", ".join(f"{key}={value}" for key, value in case.items())
            if not record_outcome(results, infrastructure, section, case, outcome):
                print(f"  {label}  infrastructure:{outcome['category']} | {outcome['error']}")
                return results, infrastructure
            print(f"  {label}  {outcome['status']}" + (f" | {outcome['error']}" if outcome["error"] else ""))
            if delay:
                time.sleep(delay)
    return results, infrastructure


def create_client():
    load_env()
    api_key = os.environ.get("OPENAI_API_KEY")
    if not api_key:
        raise RuntimeError("OPENAI_API_KEY is required for --live")
    from openai import OpenAI  # imported only for an explicit live run

    return OpenAI(api_key=api_key)


def parse_args(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--live", action="store_true", help="confirm live API calls")
    parser.add_argument("--model", default=DEFAULT_MODEL)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--delay", type=float, default=0.2)
    return parser.parse_args(argv)


def main(argv=None):
    args = parse_args(argv)
    if not args.live:
        print("refusing to call the API without --live", file=sys.stderr)
        return 2
    try:
        client = create_client()
    except Exception as error:  # noqa: BLE001
        print(f"infrastructure failure: {error}", file=sys.stderr)
        return 2
    run_date = date.today()
    results, infrastructure = run_probe(client, args.model, args.delay, run_date)
    if infrastructure:
        print("probe stopped after infrastructure failure; no artifact written", file=sys.stderr)
        return 1
    output = artifact_path(args.output, run_date)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(results, indent=2) + "\n", encoding="utf-8")
    print(f"\nSaved {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
