#!/usr/bin/env python3
"""Probe OpenAI Structured Outputs boundary limits against the live API.

Determines empirically (docs are unreliable / SPA-gated):
  1. Maximum object nesting depth the API accepts.
  2. The enum rule: does a single enum with >250 values cap total enum-string
     length at 15,000 chars, and is the cap per-enum?

Run: python scripts/validation/probe_limits.py
Needs OPENAI_API_KEY in scripts/validation/.env. ~20 calls, ~$0.002.
"""

import json
import os
import sys
import time
from pathlib import Path

from openai_errors import is_openai_schema_error
from _env import load_env

load_env()
from openai import OpenAI  # noqa: E402

client = OpenAI(api_key=os.environ["OPENAI_API_KEY"])
MODEL = "gpt-4o-2024-08-06"


def submit(schema):
    try:
        client.responses.create(
            model=MODEL,
            input=[{"role": "system", "content": "x"}, {"role": "user", "content": "x"}],
            text={"format": {"type": "json_schema", "name": "p", "strict": True, "schema": schema}},
        )
        return ("accepted", None)
    except Exception as e:  # noqa: BLE001
        if not is_openai_schema_error(e):
            return ("transport_error", str(e)[:160])
        return ("rejected", str(e).split("- ", 1)[-1][:160])


def nested(m):
    node = {"type": "string"}
    for i in range(m, 0, -1):
        node = {
            "type": "object",
            "properties": {f"l{i}": node},
            "required": [f"l{i}"],
            "additionalProperties": False,
        }
    return node


def enum_schema(n_values, val_len):
    vals = [f"{i:0{val_len}d}"[:val_len] for i in range(n_values)]
    assert len(set(vals)) == n_values, "enum values must be unique"
    return {
        "type": "object",
        "properties": {"e": {"type": "string", "enum": vals}},
        "required": ["e"],
        "additionalProperties": False,
    }


results = {"depth": [], "enum": []}

print("== DEPTH PROBE (M nested object levels; schemalint node.depth == M) ==")
for m in range(1, 14):
    status, err = submit(nested(m))
    results["depth"].append({"levels": m, "status": status, "error": err})
    print(f"  levels={m:2d}  {status}" + (f"  | {err}" if err else ""))
    time.sleep(0.2)

print("\n== ENUM PROBE (values x per-value-len = total enum chars) ==")
enum_cases = [
    ("250 vals x10c = 2500",   250, 10),
    ("251 vals x10c = 2510",   251, 10),
    ("300 vals x40c = 12000",  300, 40),
    ("300 vals x50c = 15000",  300, 50),
    ("300 vals x60c = 18000",  300, 60),
    ("250 vals x80c = 20000",  250, 80),
    ("260 vals x60c = 15600",  260, 60),
]
for label, n, ln in enum_cases:
    status, err = submit(enum_schema(n, ln))
    results["enum"].append({"label": label, "n_values": n, "val_len": ln,
                            "total_chars": n * ln, "status": status, "error": err})
    print(f"  {label:24s}  {status}" + (f"  | {err}" if err else ""))
    time.sleep(0.2)

out = Path(__file__).resolve().parent / "results" / "probe_limits_2026-06-16.json"
out.write_text(json.dumps(results, indent=2) + "\n")
print(f"\nSaved {out}")
