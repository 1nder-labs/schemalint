#!/usr/bin/env python3
"""Exercise the public command and bundled sidecar from an installed wheel."""

import json
import os
from pathlib import Path
import subprocess


ROOT = Path(__file__).resolve().parents[1]
PROFILE = "openai.so.2026-04-30"


def run(*args: str, env=None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(args, check=False, capture_output=True, text=True, env=env)


def main() -> int:
    version = run("schemalint", "--version")
    assert version.returncode == 0, version.stderr

    raw = run(
        "schemalint",
        "check",
        str(ROOT / "crates/schemalint/tests/corpus/schema_01.json"),
        "--profile",
        PROFILE,
    )
    assert raw.returncode == 0, raw.stdout + raw.stderr

    env = os.environ.copy()
    fixture_root = ROOT / "crates/schemalint/tests/fixtures"
    env["PYTHONPATH"] = os.pathsep.join(
        filter(None, (str(fixture_root), env.get("PYTHONPATH")))
    )
    pydantic = run(
        "schemalint",
        "check-python",
        "--package",
        "pydantic_fixture",
        "--python-path",
        os.environ.get("PYTHON", "python"),
        "--profile",
        PROFILE,
        "--format",
        "json",
        env=env,
    )
    assert pydantic.returncode == 1, pydantic.stdout + pydantic.stderr
    report = json.loads(pydantic.stdout)
    assert report["schema_version"] == "1.1"
    assert report["report"]["coverage"]["status"] == "complete"
    assert report["summary"]["errors"] > 0
    print(f"installed wheel passed: {version.stdout.strip()}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
