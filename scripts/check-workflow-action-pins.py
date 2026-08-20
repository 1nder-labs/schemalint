#!/usr/bin/env python3
"""Reject mutable third-party GitHub Action references."""

from pathlib import Path
import re
import sys


ROOT = Path(__file__).resolve().parents[1]
WORKFLOWS = ROOT / ".github" / "workflows"
USES = re.compile(r"^\s*(?:-\s*)?uses:\s*([^\s#]+)")
COMMIT = re.compile(r"^[0-9a-f]{40}$")


def main() -> int:
    failures: list[str] = []
    for workflow in sorted(WORKFLOWS.glob("*.yml")):
        for line_number, line in enumerate(workflow.read_text().splitlines(), 1):
            match = USES.match(line)
            if not match:
                continue
            reference = match.group(1)
            if reference.startswith(("./", "docker://")):
                continue
            _, separator, revision = reference.rpartition("@")
            if not separator or not COMMIT.fullmatch(revision):
                failures.append(
                    f"{workflow.relative_to(ROOT)}:{line_number}: {reference}"
                )

    if failures:
        print("workflow actions must use immutable 40-character commit SHAs:", file=sys.stderr)
        for failure in failures:
            print(f"  {failure}", file=sys.stderr)
        return 1
    print("workflow action pin gate passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
