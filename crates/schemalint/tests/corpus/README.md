# Regression Corpus

50 curated JSON Schemas with expected diagnostic sets.

## Sourcing Methodology

Schemas are sourced from public bug reports, OpenAI Community forum posts,
Pydantic AI issues, and SDK forums.

## Curation Process

1. Run `schemalint check` against each schema with the OpenAI profile.
2. Verify each diagnostic against the OpenAI Structured Outputs documentation.
3. Store expected diagnostics in `.expected` files (JSON format).

## Updating Expected Output

When rules change, expected output must be explicitly updated after human review.
Do not silently update `.expected` files.
