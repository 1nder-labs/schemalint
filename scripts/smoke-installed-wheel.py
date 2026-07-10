#!/usr/bin/env python3
"""Exercise the public command and bundled sidecar from an installed wheel."""

from importlib.metadata import version as package_version
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
from typing import Dict, Optional

from fixtures.installed_wheel import (
    PROFILE,
    SCENARIO_COUNTS,
    VALID_SCHEMA,
    assert_missing_report,
    assert_partial_report,
    assert_pydantic_report,
    assert_server_responses,
    assert_valid_report,
    server_requests,
    write_consumer_project,
)

INSTALLED_COMMAND = Path(sys.executable).with_name("schemalint")
SCHEMALINT = str(
    INSTALLED_COMMAND
    if INSTALLED_COMMAND.exists()
    else shutil.which("schemalint") or INSTALLED_COMMAND
)


def run(
    *args: str,
    cwd: Optional[Path] = None,
    input_text: Optional[str] = None,
) -> subprocess.CompletedProcess[str]:
    env = os.environ.copy()
    env["PATH"] = os.pathsep.join(
        [str(Path(sys.executable).parent), env.get("PATH", "")]
    )
    return subprocess.run(
        args,
        check=False,
        capture_output=True,
        text=True,
        cwd=cwd,
        env=env,
        input=input_text,
    )


def json_report(
    result: subprocess.CompletedProcess[str],
    *,
    returncode: int,
    status: str,
    success: bool,
    counts: Dict[str, int],
) -> dict:
    context = result.stdout + result.stderr
    assert result.returncode == returncode, context
    try:
        payload = json.loads(result.stdout)
    except json.JSONDecodeError as error:
        raise AssertionError(f"command did not emit JSON 1.1:\n{context}") from error

    assert payload["schema_version"] == "1.1", payload
    report = payload["report"]
    coverage = report["coverage"]
    assert report["success"] is success, report
    assert coverage["status"] == status, coverage
    for name, expected in counts.items():
        assert coverage[name] == expected, coverage
    assert payload["summary"]["schemas_checked"] == coverage["checked"], payload
    return payload


def exercise_server_recovery(root: Path) -> None:
    requests = server_requests()
    server = run(
        SCHEMALINT,
        "server",
        cwd=root,
        input_text="".join(f"{json.dumps(request)}\n" for request in requests),
    )
    assert server.returncode == 0, server.stdout + server.stderr
    responses = [json.loads(line) for line in server.stdout.splitlines() if line]
    assert_server_responses(responses)


def exercise_installed_wheel(root: Path) -> str:
    version = run(SCHEMALINT, "--version", cwd=root)
    assert version.returncode == 0, version.stderr
    assert version.stdout.strip(), version

    valid_schema = root / "valid.json"
    valid_schema.write_text(json.dumps(VALID_SCHEMA), encoding="utf-8")
    valid = run(
        SCHEMALINT,
        "check",
        str(valid_schema),
        "--profile",
        PROFILE,
        "--format",
        "json",
        cwd=root,
    )
    valid_payload = json_report(
        valid,
        returncode=0,
        status="complete",
        success=True,
        counts=SCENARIO_COUNTS["valid"],
    )
    assert_valid_report(valid_payload)

    empty = root / "empty"
    empty.mkdir()
    empty_result = run(
        SCHEMALINT,
        "check",
        str(empty),
        "--profile",
        PROFILE,
        "--format",
        "json",
        cwd=root,
    )
    empty_payload = json_report(
        empty_result,
        returncode=1,
        status="empty",
        success=False,
        counts=SCENARIO_COUNTS["empty"],
    )
    assert empty_payload["report"]["failures"] == [], empty_payload

    write_consumer_project(root)
    pydantic_result = run(
        SCHEMALINT,
        "check-python",
        "--package",
        "consumer_models",
        "--python-path",
        sys.executable,
        "--profile",
        PROFILE,
        "--format",
        "json",
        cwd=root,
    )
    pydantic_payload = json_report(
        pydantic_result,
        returncode=1,
        status="complete",
        success=False,
        counts=SCENARIO_COUNTS["pydantic"],
    )
    assert_pydantic_report(pydantic_payload)

    partial_result = run(
        SCHEMALINT,
        "check-python",
        "--package",
        "partial_models",
        "--python-path",
        sys.executable,
        "--profile",
        PROFILE,
        "--format",
        "json",
        cwd=root,
    )
    partial_payload = json_report(
        partial_result,
        returncode=1,
        status="partial",
        success=False,
        counts=SCENARIO_COUNTS["partial"],
    )
    assert_partial_report(partial_payload)

    missing_result = run(
        SCHEMALINT,
        "check-python",
        "--package",
        "consumer_models_missing",
        "--python-path",
        sys.executable,
        "--profile",
        PROFILE,
        "--format",
        "json",
        cwd=root,
    )
    missing_payload = json_report(
        missing_result,
        returncode=1,
        status="failed",
        success=False,
        counts=SCENARIO_COUNTS["missing"],
    )
    assert_missing_report(missing_payload)
    exercise_server_recovery(root)
    return version.stdout.strip()


def main() -> int:
    with tempfile.TemporaryDirectory(prefix="schemalint-wheel-smoke-") as temp_dir:
        installed_version = exercise_installed_wheel(Path(temp_dir))
    print(
        f"installed wheel passed: {installed_version}; "
        f"Pydantic {package_version('pydantic')}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
