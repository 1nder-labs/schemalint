#!/usr/bin/env bash
# Check that line coverage for crates/schemalint/src/ meets the threshold.
#
# Parses the LCOV format produced by cargo-llvm-cov:
#   SF:<file-path>      — start of a source file record
#   LF:<lines-found>     — total instrumented lines
#   LH:<lines-hit>       — lines covered by tests
#   end_of_record        — end of the record
#
# Only files under crates/schemalint/src/ are counted toward the threshold.
# Node/Python subprocess modules implicitly have lower coverage because they
# require external runtimes (Node, Python, Pydantic) — they are excluded
# from the target.
#
# The 78% floor prevents erosion of the core crate's line coverage.
# The target for core logic (emitters, normalizer, parser, rules) is 90%+.
# The 78% gate on all of crates/schemalint/src/ ensures no coverage
# regression slips through while the team closes the remaining gap.
#
# Input: reads lcov.info in the current directory.
# Exit:  0 if coverage meets the threshold, 1 otherwise.
set -euo pipefail

LCOV_FILE="${1:-lcov.info}"
THRESHOLD="${2:-78.00}"

if [ ! -f "$LCOV_FILE" ]; then
  echo "::error::Coverage file '${LCOV_FILE}' not found"
  exit 1
fi

total_lf=0
total_lh=0
current_sf=""
in_record=false

while IFS= read -r line; do
  if [[ "$line" == SF:* ]]; then
    current_sf="${line#SF:}"
    in_record=true
  elif [[ "$line" == "end_of_record" ]]; then
    in_record=false
  elif [[ "$in_record" == true && "$line" == LF:* ]]; then
    if [[ "$current_sf" == */crates/schemalint/src/* ]]; then
      lf="${line#LF:}"
      total_lf=$((total_lf + lf))
    fi
  elif [[ "$in_record" == true && "$line" == LH:* ]]; then
    if [[ "$current_sf" == */crates/schemalint/src/* ]]; then
      lh="${line#LH:}"
      total_lh=$((total_lh + lh))
    fi
  fi
done < "$LCOV_FILE"

if [ "$total_lf" -eq 0 ]; then
  echo "::error::No coverage data found for crates/schemalint/src/ in ${LCOV_FILE}"
  exit 1
fi

coverage=$(echo "scale=2; $total_lh * 100 / $total_lf" | bc)

echo ""
echo "Line coverage for crates/schemalint/src/: ${coverage}%"
echo "  Lines found:  $total_lf"
echo "  Lines hit:    $total_lh"

if (( $(echo "$coverage < $THRESHOLD" | bc -l) )); then
  echo ""
  echo "::error::Line coverage ${coverage}% is below threshold of ${THRESHOLD}%"
  echo "See the coverage artifact (${LCOV_FILE}) for file-level details."
  exit 1
fi

echo ""
echo "Coverage gate passed (threshold: ${THRESHOLD}%)"
