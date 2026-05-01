# Validation Scripts

These scripts validate schemalint's profile accuracy against the real OpenAI API.

## Why?

Documentation can drift from reality. These scripts provide **ground truth** by
submitting schemas directly to OpenAI's structured outputs API and comparing
the API's rejection reasons with schemalint's predicted errors.

## Prerequisites

```bash
pip install openai
export OPENAI_API_KEY=sk-your-key-here
```

## Scripts

### `validate_openai.py`

Validates one or more schemas against the OpenAI API and reports whether each
was accepted or rejected.

```bash
python scripts/validation/validate_openai.py \
    crates/schemalint/tests/corpus/schema_03.json \
    crates/schemalint/tests/corpus/schema_04.json
```

Output is JSON with `status` ("accepted" or "rejected") and the API error
message if rejected.

### `compare_with_openai.py`

Compares schemalint's predictions with OpenAI's actual behavior for a single
schema, highlighting mismatches (false positives or false negatives).

```bash
python scripts/validation/compare_with_openai.py \
    crates/schemalint/tests/corpus/schema_03.json
```

This will:
1. Run schemalint on the schema
2. Submit it to OpenAI API
3. Report whether they agree or disagree

## Interpreting Results

| Scenario | Meaning |
|----------|---------|
| Both reject | Profile is accurate |
| Both accept | Profile is accurate |
| schemalint rejects, OpenAI accepts | **False positive** — profile too strict |
| schemalint accepts, OpenAI rejects | **False negative** — profile too lenient |

## Costs

Each validation is a single API call with `max_tokens=1`, costing ~$0.0001
per schema. Validating the entire 50-schema corpus costs approximately $0.005.

## Future Work

- Anthropic API validation (Phase 2)
- Automated daily validation in CI (with API key stored as GitHub secret)
- Synthetic conformance mock (Phase 5) — a local server that simulates
  provider validation without real API calls
