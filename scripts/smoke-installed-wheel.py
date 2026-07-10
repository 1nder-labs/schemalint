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


PROFILE = "openai.so.2026-04-30"
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


def write_consumer_project(root: Path) -> None:
    package = root / "consumer_models"
    package.mkdir()
    (package / "__init__.py").write_text("", encoding="utf-8")
    (package / "models.py").write_text(
        """from typing import List, Optional

from pydantic import BaseModel, Field


class Address(BaseModel):
    street: str
    postal_code: str = Field(alias="postalCode")


class UserProfile(BaseModel):
    address: Address
    display_name: Optional[str] = Field(default=None, alias="displayName")
    tags: List[str] = Field(default_factory=list)
""",
        encoding="utf-8",
    )

    partial = root / "partial_models"
    partial.mkdir()
    (partial / "__init__.py").write_text("", encoding="utf-8")
    (partial / "models.py").write_text(
        """from pydantic import BaseModel


class RetainedModel(BaseModel):
    value: str


class BrokenSchema(BaseModel):
    value: str

    @classmethod
    def model_json_schema(cls, *args, **kwargs):
        raise RuntimeError("intentional model_json_schema failure")

    @classmethod
    def schema(cls, *args, **kwargs):
        raise RuntimeError("intentional schema failure")
""",
        encoding="utf-8",
    )
    (partial / "broken_import.py").write_text(
        'raise RuntimeError("intentional submodule import failure")\n',
        encoding="utf-8",
    )


def assert_partial_python_report(payload: dict) -> None:
    assert payload["summary"]["errors"] > 0, payload
    assert any(
        diagnostic["code"] == "OAI-S-additional-properties-false"
        for diagnostic in payload["diagnostics"]
    ), payload
    failures = payload["report"]["failures"]
    assert len(failures) == 2, payload
    failure_text = "\n".join(
        f"{failure['target']}: {failure['message']}" for failure in failures
    )
    assert "BrokenSchema" in failure_text, failure_text
    assert "partial_models.broken_import" in failure_text, failure_text
    assert "intentional" in failure_text, failure_text


def exercise_server_recovery(root: Path, valid_schema: dict) -> None:
    requests = [
        {
            "jsonrpc": "2.0",
            "method": "checkPython",
            "params": {
                "packages": ["partial_models"],
                "profiles": [PROFILE],
                "format": "json",
            },
            "id": 1,
        },
        {
            "jsonrpc": "2.0",
            "method": "check",
            "params": {
                "schema": valid_schema,
                "profiles": [PROFILE],
                "format": "json",
            },
            "id": 2,
        },
        {"jsonrpc": "2.0", "method": "shutdown", "id": 3},
    ]
    server = run(
        SCHEMALINT,
        "server",
        cwd=root,
        input_text="".join(f"{json.dumps(request)}\n" for request in requests),
    )
    assert server.returncode == 0, server.stdout + server.stderr
    responses = [json.loads(line) for line in server.stdout.splitlines() if line]
    assert [response["id"] for response in responses] == [1, 2, 3], responses

    partial = responses[0]["result"]
    assert partial["success"] is False, partial
    assert partial["report"]["coverage"] == {
        "status": "partial",
        "attempted": 3,
        "excluded": 0,
        "discovered": 1,
        "checked": 1,
        "failed": 2,
    }, partial
    partial_output = json.loads(partial["output"])
    assert_partial_python_report(partial_output)

    recovered = responses[1]["result"]
    assert recovered["success"] is True, recovered
    assert recovered["report"]["coverage"]["status"] == "complete", recovered
    assert recovered["report"]["coverage"]["checked"] == 1, recovered
    assert responses[2]["result"] is None, responses[2]


def exercise_installed_wheel(root: Path) -> str:
    version = run(SCHEMALINT, "--version", cwd=root)
    assert version.returncode == 0, version.stderr
    assert version.stdout.strip(), version

    valid_schema_value = {
        "type": "object",
        "properties": {"answer": {"type": "string"}},
        "required": ["answer"],
        "additionalProperties": False,
    }
    valid_schema = root / "valid.json"
    valid_schema.write_text(json.dumps(valid_schema_value), encoding="utf-8")
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
        counts={
            "attempted": 1,
            "excluded": 0,
            "discovered": 1,
            "checked": 1,
            "failed": 0,
        },
    )
    assert valid_payload["report"]["failures"] == [], valid_payload
    assert valid_payload["diagnostics"] == [], valid_payload

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
        counts={
            "attempted": 1,
            "excluded": 0,
            "discovered": 0,
            "checked": 0,
            "failed": 0,
        },
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
        counts={
            "attempted": 2,
            "excluded": 0,
            "discovered": 2,
            "checked": 2,
            "failed": 0,
        },
    )
    assert pydantic_payload["report"]["failures"] == [], pydantic_payload
    assert pydantic_payload["summary"]["errors"] > 0, pydantic_payload
    assert any(
        diagnostic["code"] == "OAI-S-additional-properties-false"
        for diagnostic in pydantic_payload["diagnostics"]
    ), pydantic_payload

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
        counts={
            "attempted": 3,
            "excluded": 0,
            "discovered": 1,
            "checked": 1,
            "failed": 2,
        },
    )
    assert_partial_python_report(partial_payload)

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
        counts={
            "attempted": 1,
            "excluded": 0,
            "discovered": 0,
            "checked": 0,
            "failed": 1,
        },
    )
    failures = missing_payload["report"]["failures"]
    assert len(failures) == 1, missing_payload
    assert "consumer_models_missing" in failures[0]["target"], missing_payload
    assert failures[0]["message"], missing_payload
    exercise_server_recovery(root, valid_schema_value)
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
