#!/usr/bin/env python3
"""
Detect keyword support drift between two API validation runs.

Compares two saved API result files and reports any keywords whose
accept/reject status changed between runs. Used to catch provider-side
changes (e.g., OpenAI silently adding/removing keyword support).

Usage:
    # Compare latest vs previous:
    python scripts/validation/check_drift.py \
        --previous scripts/validation/results/openai_bulk_2026-05-03.json \
        --latest scripts/validation/results/openai_bulk_2026-05-10.json

    # If drift detected, exit code 1 (for CI gating).
"""

import json
import sys
from pathlib import Path
from datetime import datetime


def load_results(path: str) -> dict:
    """Load API results and index by schema filename."""
    with open(path) as f:
        data = json.load(f)
    indexed = {}
    for r in data:
        name = Path(r["schema_path"]).name
        indexed[name] = r
    return indexed


def keyword_from_error(error: str) -> str | None:
    """Extract the keyword name from an OpenAI API error message."""
    if not error:
        return None
    # Patterns: "'keyword' is not permitted", "'keyword' is not supported"
    import re
    m = re.search(r"'([^']+)' is not (permitted|supported)", error)
    if m:
        return m.group(1)
    # Pattern: 'In context=(...), 'allOf' is not permitted
    m = re.search(r",\s*'(\w+)'\s+is not permitted", error)
    if m:
        return m.group(1)
    return None


def main():
    import argparse
    parser = argparse.ArgumentParser(
        description="Detect keyword support drift between API validation runs"
    )
    parser.add_argument("--previous", required=True, help="Previous API results JSON")
    parser.add_argument("--latest", required=True, help="Latest API results JSON")
    parser.add_argument("--format", choices=["json", "text"], default="text")
    args = parser.parse_args()

    prev = load_results(args.previous)
    latest = load_results(args.latest)

    # Find schemas present in both runs
    common = set(prev) & set(latest)
    if len(common) == 0:
        print("ERROR: No matching schemas between runs")
        sys.exit(2)

    # Detect drift: any schema that changed accept/reject status
    added_support = []   # was rejected, now accepted
    removed_support = [] # was accepted, now rejected
    unchanged_accepted = 0
    unchanged_rejected = 0
    detail_changes = []  # schemas with same status but different error

    for name in sorted(common):
        p = prev[name]
        l = latest[name]
        p_ok = p["status"] == "accepted"
        l_ok = l["status"] == "accepted"

        if p_ok and l_ok:
            unchanged_accepted += 1
        elif not p_ok and not l_ok:
            unchanged_rejected += 1
            # Check if error message changed (might indicate different rejection reason)
            if p.get("api_error") != l.get("api_error"):
                detail_changes.append((name, p.get("api_error"), l.get("api_error")))
        elif p_ok and not l_ok:
            kw = keyword_from_error(l.get("api_error", ""))
            removed_support.append((name, kw, l.get("api_error", "")))
        elif not p_ok and l_ok:
            kw = keyword_from_error(p.get("api_error", ""))
            added_support.append((name, kw, p.get("api_error", "")))

    has_drift = len(added_support) > 0 or len(removed_support) > 0

    if args.format == "json":
        print(json.dumps({
            "timestamp": datetime.now().isoformat(),
            "previous": args.previous,
            "latest": args.latest,
            "schemas_compared": len(common),
            "unchanged_accepted": unchanged_accepted,
            "unchanged_rejected": unchanged_rejected,
            "added_support": [{"schema": s, "keyword": k, "prev_error": e} for s, k, e in added_support],
            "removed_support": [{"schema": s, "keyword": k, "new_error": e} for s, k, e in removed_support],
            "error_detail_changes": len(detail_changes),
            "drift_detected": has_drift,
        }, indent=2))
    else:
        print(f"Compared {len(common)} schemas:")
        print(f"  Unchanged accepted:  {unchanged_accepted}")
        print(f"  Unchanged rejected:  {unchanged_rejected}")
        print(f"  Added support:       {len(added_support)} (was rejected, now accepted)")
        print(f"  Removed support:     {len(removed_support)} (was accepted, now rejected)")
        print(f"  Error detail changes:{len(detail_changes)} (same status, different reason)")
        print()

        if added_support:
            print("NEWLY SUPPORTED (provider ADDED keyword support):")
            for s, kw, err in added_support:
                print(f"  {s:20s}  keyword={kw or '?'}")
            print()

        if removed_support:
            print("NEWLY UNSUPPORTED (provider REMOVED keyword support):")
            for s, kw, err in removed_support:
                print(f"  {s:20s}  keyword={kw or '?'}")
            print()

        if detail_changes:
            print("ERROR DETAIL CHANGES (same status, different message):")
            for s, old_err, new_err in detail_changes[:5]:
                print(f"  {s}:")
                print(f"    was: {old_err[:120]}")
                print(f"    now: {new_err[:120]}")
            if len(detail_changes) > 5:
                print(f"  ... and {len(detail_changes) - 5} more")
            print()

        if has_drift:
            print("⚠️  DRIFT DETECTED — profile may need updating")
            print(f"    Run: python scripts/validation/compare_with_openai.py --all")
            print(f"    Then update the TOML profile and truth files accordingly.")
        else:
            print("✅  No drift — all keyword behaviors stable")

    sys.exit(1 if has_drift else 0)


if __name__ == "__main__":
    main()
