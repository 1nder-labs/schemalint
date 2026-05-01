# Regression Corpus

50 curated synthetic JSON Schemas with expected diagnostic sets.

## Sourcing Methodology

Schemas are synthetically generated to exercise the full surface area of the
OpenAI Structured Outputs profile: allowed keywords, forbidden keywords,
restricted values, structural limits, `$ref` patterns, and edge cases.

## Curation Process

1. Run `schemalint check` against each schema with the OpenAI profile.
2. Verify each diagnostic against the OpenAI Structured Outputs documentation.
3. Store expected diagnostics in `.expected` files (JSON format).

## Updating Expected Output

When rules change, expected output must be explicitly updated after human review.
Do not silently update `.expected` files.
