# Validation Scripts

These scripts validate schemalint's profile accuracy against the real OpenAI Responses API (Structured Outputs).

## Why?

Documentation can drift from reality. These scripts provide **ground truth** by
submitting schemas directly to OpenAI's Responses API with Structured Outputs
and comparing the API's rejection reasons with schemalint's predicted errors.

## Supported Models

| Flag | Model | Notes |
|------|-------|-------|
| `--model gpt-4o` | gpt-4o-2024-08-06 | Full keyword support (default) |
| `--model gpt-4o-mini` | gpt-4o-mini | Same schema support as gpt-4o |
| `--model ft` | gpt-4o-2024-08-06 | Fine-tuned restrictions (extra forbidden keywords) |

Fine-tuned models additionally reject: `minLength`, `maxLength`, `pattern`,
`format`, `minimum`, `maximum`, `multipleOf`, `patternProperties`, `minItems`,
`maxItems`.

## Scripts

### `validate_openai.py`

Validates one or more schemas against the OpenAI Responses API.
Uses `client.responses.create()` with `text.format.type = "json_schema"`.

```bash
# Default (gpt-4o):
python scripts/validation/validate_openai.py schema_01.json schema_02.json

# Different model:
python scripts/validation/validate_openai.py --model gpt-4o-mini schema_*.json

# Save results:
python scripts/validation/validate_openai.py --output results.json schema_*.json
```

### `compare_with_openai.py`

Compares schemalint's predictions with OpenAI's actual behavior.

**Offline mode** (no API calls — uses previously saved results):
```bash
python scripts/validation/compare_with_openai.py --all \
    --api-results scripts/validation/results/openai_bulk_2026-05-03.json
```

**Live mode** (makes API calls):
```bash
python scripts/validation/compare_with_openai.py schema_03.json
```

## Interpreting Results

| Scenario | Meaning |
|----------|---------|
| Both reject | Profile is accurate |
| Both accept | Profile is accurate |
| schemalint rejects, OpenAI accepts | **False positive** — profile too strict |
| schemalint accepts, OpenAI rejects | **False negative** — profile too lenient |

## API Key

Create `scripts/validation/.env`:
```
OPENAI_API_KEY=sk-proj-...
```

No export needed. Both scripts auto-load from `.env`.

## Costs

Each validation calls the Responses API with minimal input/output.
Approximately $0.0001 per schema. 50 schemas ≈ $0.005.
