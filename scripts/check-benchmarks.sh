#!/usr/bin/env bash
# Check that Criterion benchmarks are within thresholds.
#
# Parses Criterion's stable bencher output format:
#   test group_name/bench ... bench:   <ns> ns/iter (+/- <variance>)
#
# Thresholds are guardrails: a 2x regression from current measured values
# is the real detection signal. The absolute ceiling values below are the
# phases.md thresholds — current measured values are well under them.
#   single_schema    < 1.0 ms   (current: ~0.48 ms)
#   cold_start       < 500.0 ms (current: ~7.2 ms)
#   incremental      < 5.0 ms   (current: ~1.7 ms)
#
# Input: reads bench_output.txt in the current directory.
# Exit:  0 if all benchmarks pass, 1 if any fail or are not found.
set -euo pipefail

BENCH_FILE="${1:-bench_output.txt}"
fail=0

check_bench() {
  local name="$1"
  local threshold_ns="$2"
  local threshold_ms="$3"

  # Match: test <name>/ ... bench:   <digits_or_commas> ns/iter
  local actual
  actual=$(grep "^test ${name}/" "$BENCH_FILE" \
    | sed 's/.*bench:[[:space:]]*\([0-9,]*\) ns\/iter.*/\1/' \
    | tr -d ',')
  if [ -z "$actual" ]; then
    echo "::error::Benchmark '${name}' not found in ${BENCH_FILE}"
    fail=1
    return
  fi

  # Validate that actual is a positive integer (format-change defense).
  if ! [[ "$actual" =~ ^[0-9]+$ ]]; then
    echo "::error::Benchmark '${name}' value '${actual}' is not a positive integer — output format may have changed"
    fail=1
    return
  fi

  if [ "$actual" -gt "$threshold_ns" ]; then
    actual_ms=$(echo "scale=2; $actual / 1000000" | bc)
    echo "::error::bench_${name}: ${actual_ms} ms exceeds threshold ${threshold_ms} ms"
    fail=1
  else
    actual_ms=$(echo "scale=2; $actual / 1000000" | bc)
    echo "✓ bench_${name}: ${actual_ms} ms (threshold: ${threshold_ms} ms)"
  fi
}

check_bench "single_schema" 1000000 "1.0"
check_bench "cold_start" 500000000 "500.0"
check_bench "incremental" 5000000 "5.0"

if [ "$fail" -ne 0 ]; then
  echo ""
  echo "One or more benchmarks exceeded thresholds."
  echo "Review the benchmark output above and investigate before tagging."
  exit 1
fi

echo ""
echo "All benchmarks within thresholds."
