"""Consumer projects and expected reports for the installed-wheel smoke test."""

from importlib.metadata import version as package_version
import json
from pathlib import Path


PROFILE = "openai.so.2026-04-30"
VALID_SCHEMA = {
    "type": "object",
    "properties": {"answer": {"type": "string"}},
    "required": ["answer"],
    "additionalProperties": False,
}
SCENARIO_COUNTS = {
    "valid": {
        "attempted": 1,
        "excluded": 0,
        "discovered": 1,
        "checked": 1,
        "failed": 0,
    },
    "empty": {
        "attempted": 1,
        "excluded": 0,
        "discovered": 0,
        "checked": 0,
        "failed": 0,
    },
    "pydantic": {
        "attempted": 2,
        "excluded": 0,
        "discovered": 2,
        "checked": 2,
        "failed": 0,
    },
    "partial": {
        "attempted": 4,
        "excluded": 0,
        "discovered": 1,
        "checked": 1,
        "failed": 3,
    },
    "missing": {
        "attempted": 1,
        "excluded": 0,
        "discovered": 0,
        "checked": 0,
        "failed": 1,
    },
}


def write_consumer_project(root: Path) -> None:
    package = root / "consumer_models"
    package.mkdir()
    (package / "__init__.py").write_text(
        "from .models import Address, UserProfile\n", encoding="utf-8"
    )
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
    (partial / "__init__.py").write_text(
        "from .models import BrokenSchema, RetainedModel\n", encoding="utf-8"
    )
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
    (partial / "opaque.py").write_text(
        "def __dir__():\n"
        '    raise RuntimeError("intentional introspection failure")\n',
        encoding="utf-8",
    )


def server_requests() -> list[dict]:
    return [
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
                "schema": VALID_SCHEMA,
                "profiles": [PROFILE],
                "format": "json",
            },
            "id": 2,
        },
        {"jsonrpc": "2.0", "method": "shutdown", "id": 3},
    ]


def assert_valid_report(payload: dict) -> None:
    assert payload["report"]["failures"] == [], payload
    assert payload["diagnostics"] == [], payload


def assert_pydantic_report(payload: dict) -> None:
    assert payload["report"]["failures"] == [], payload
    assert payload["summary"]["errors"] > 0, payload
    assert any(
        diagnostic["code"] == "OAI-S-additional-properties-false"
        for diagnostic in payload["diagnostics"]
    ), payload


def assert_partial_report(payload: dict) -> None:
    assert payload["summary"]["errors"] > 0, payload
    assert any(
        diagnostic["code"] == "OAI-S-additional-properties-false"
        for diagnostic in payload["diagnostics"]
    ), payload
    failures = payload["report"]["failures"]
    assert len(failures) == 3, payload
    schema_message = (
        "model_json_schema() failed: intentional model_json_schema failure"
        if int(package_version("pydantic").split(".", 1)[0]) >= 2
        else "schema() failed: intentional schema failure"
    )
    assert {failure["target"]: failure["message"] for failure in failures} == {
        "package 'partial_models', target 'BrokenSchema'": schema_message,
        "package 'partial_models', target 'partial_models.broken_import'": (
            "module import failed: intentional submodule import failure"
        ),
        "package 'partial_models', target 'partial_models.opaque'": (
            "module introspection failed: intentional introspection failure"
        ),
    }, failures
    failure_text = "\n".join(
        f"{failure['target']}: {failure['message']}" for failure in failures
    )
    assert "BrokenSchema" in failure_text, failure_text
    assert "partial_models.broken_import" in failure_text, failure_text
    assert "partial_models.opaque" in failure_text, failure_text
    assert "module introspection failed" in failure_text, failure_text
    assert "intentional" in failure_text, failure_text


def assert_missing_report(payload: dict) -> None:
    failures = payload["report"]["failures"]
    assert len(failures) == 1, payload
    assert "consumer_models_missing" in failures[0]["target"], payload
    assert failures[0]["message"], payload


def assert_server_responses(responses: list[dict]) -> None:
    assert [response["id"] for response in responses] == [1, 2, 3], responses
    partial = responses[0]["result"]
    assert partial["success"] is False, partial
    assert partial["report"]["coverage"] == {
        "status": "partial",
        **SCENARIO_COUNTS["partial"],
    }, partial
    assert_partial_report(json.loads(partial["output"]))

    recovered = responses[1]["result"]
    assert recovered["success"] is True, recovered
    assert recovered["report"]["coverage"]["status"] == "complete", recovered
    assert recovered["report"]["coverage"]["checked"] == 1, recovered
    assert responses[2]["result"] is None, responses[2]
