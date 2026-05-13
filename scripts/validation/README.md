# Validation Scripts

These scripts validate schemalint's profile accuracy against the real OpenAI
Responses API and Anthropic Claude API structured-output surfaces.

## Why?

Documentation can drift from reality. These scripts provide **ground truth** by
submitting schemas directly to provider APIs with Structured Outputs and
comparing the API's rejection reasons with schemalint's predicted errors.

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

### `validate_anthropic.py`

Validates one or more schemas against the native Claude API. The script can
exercise JSON outputs (`output_config.format`) or strict tool schemas
(`input_schema` with `strict: true`), which share the same documented JSON
Schema limitations.

```bash
# JSON output schema:
python scripts/validation/validate_anthropic.py schema_01.json schema_02.json

# Strict tool input_schema:
python scripts/validation/validate_anthropic.py --surface tool schema_*.json

# Different model:
python scripts/validation/validate_anthropic.py --model sonnet-4.6 schema_*.json
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

### `check_drift.py`

Detects when a provider changes keyword support between validation runs
(e.g., OpenAI adding or removing support for a keyword).

```bash
# Compare against previous results:
python scripts/validation/check_drift.py \
    --previous scripts/validation/results/openai_bulk_2026-05-03.json \
    --latest scripts/validation/results/openai_bulk_2026-05-10.json
```

Exit code 1 if drift detected. Use `--format json` for CI consumption.

## Maintenance Workflow

This is a **local, manual maintenance task** — not automated in CI.
API calls cost money and should only be made intentionally.

Run this weekly or whenever OpenAI's structured outputs docs are updated.

### 1. Run fresh validation against the API

```bash
DATE=$(date +%Y-%m-%d)
python scripts/validation/validate_openai.py \
    --model gpt-4o \
    --output scripts/validation/results/openai_${DATE}.json \
    crates/schemalint/tests/corpus/schema_*.json
# 50 schemas, ~$0.005 total cost
```

### 2. Check for drift (anything change since last run?)

```bash
python scripts/validation/check_drift.py \
    --previous scripts/validation/results/openai_bulk_2026-05-03.json \
    --latest scripts/validation/results/openai_$(date +%Y-%m-%d).json
# Exit code 1 = drift detected, exit 0 = all stable
```

### 3. If drift detected, get detailed mismatches

```bash
python scripts/validation/compare_with_openai.py --all \
    --api-results scripts/validation/results/openai_$(date +%Y-%m-%d).json
```

### 4. Fix the profile and truth files

- Edit the matching profile under `crates/schemalint-profiles/profiles/`
- Edit the matching truth file under `crates/schemalint-profiles/profiles/truth/`
- Regenerate expected files for affected schemas
- Run `cargo test --workspace --exclude schemalint-python`
- Commit: `fix(profile): OpenAI {added,removed} support for {keyword}`

## Interpreting Results

| Scenario | Meaning |
|----------|---------|
| Both reject | Profile is accurate |
| Both accept | Profile is accurate |
| schemalint rejects, OpenAI accepts | **False positive** — profile too strict |
| schemalint accepts, OpenAI rejects | **False negative** — profile too lenient |
| OpenAI transport/API error | Infrastructure failure — rerun; do not classify as schema acceptance/rejection |
| Anthropic transport/API error | Infrastructure failure — rerun; do not classify as schema acceptance/rejection |

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
ANTHROPIC_API_KEY=sk-ant-...
```

No export needed. Both scripts auto-load from `.env`.

## Costs

Each validation calls the Responses API with minimal input/output.
Approximately $0.0001 per schema. 50 schemas ≈ $0.005.
