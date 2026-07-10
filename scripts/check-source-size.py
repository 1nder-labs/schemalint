#!/usr/bin/env python3
"""Fail when a production source file exceeds the repository's 400-line limit."""

from pathlib import Path
import sys


ROOT = Path(__file__).resolve().parents[1]
MAX_LINES = 400
SOURCE_ROOTS = (
    ("crates/schemalint/src", {".rs"}),
    ("crates/schemalint-docgen/src", {".rs"}),
    ("crates/schemalint-conformance/src", {".rs"}),
    ("crates/schemalint-python/src", {".rs"}),
    ("crates/schemalint-python/python/schemalint", {".py"}),
    ("crates/schemalint-python/python/schemalint_pydantic", {".py"}),
    ("npm/schemalint/src", {".ts"}),
    ("npm/schemalint/bin", {".js"}),
    ("npm/schemalint/launcher", {".cjs"}),
    ("npm/schemalint/scripts", {".cjs"}),
)
SOURCE_FILES = ("npm/schemalint/index.cjs",)


def production_sources():
    for relative_path in SOURCE_FILES:
        yield ROOT / relative_path
    for relative_root, suffixes in SOURCE_ROOTS:
        for path in sorted((ROOT / relative_root).rglob("*")):
            if path.is_file() and path.suffix in suffixes and "__tests__" not in path.parts:
                yield path


def main() -> int:
    violations = []
    for path in production_sources():
        lines = sum(1 for _ in path.open(encoding="utf-8"))
        if lines > MAX_LINES:
            violations.append((path.relative_to(ROOT), lines))

    if violations:
        print(f"production sources must not exceed {MAX_LINES} lines:", file=sys.stderr)
        for path, lines in violations:
            print(f"  {path}: {lines}", file=sys.stderr)
        return 1

    print(f"source-size gate passed: every production source is <= {MAX_LINES} lines")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
