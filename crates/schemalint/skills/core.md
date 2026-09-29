---
name: core
description: Core schemalint usage guide. Read this before running any schemalint command. Lints JSON Schema, Zod (TypeScript) and Pydantic (Python) schemas for OpenAI and Anthropic structured-output compatibility, catching what would otherwise fail at runtime with a 400. Covers the check/fix loop, check vs check-node vs check-python (including monorepos), provider profiles, JSON output, exit codes, reading the coverage line, common fixes, and troubleshooting. Use when the user mentions structured outputs, response_format, json_schema, tool schemas, zodResponseFormat, zodOutputFormat, "schema rejected by OpenAI/Anthropic", additionalProperties, oneOf/anyOf/allOf errors, or asks to lint, validate or fix a JSON Schema, Zod schema or Pydantic model used with an LLM provider.
allowed-tools: Bash(schemalint:*), Bash(npx @1nder-labs/schemalint:*)
---

# schemalint core

Static linter for the schemas you send to an LLM provider as structured
output. It reports which keywords, shapes and limits OpenAI or Anthropic
reject, with a rule id, a JSON pointer into the schema, and usually a rewrite
hint. It never calls a provider; it is fast and deterministic.

Install: `npm i -g @1nder-labs/schemalint`, or run `npx @1nder-labs/schemalint <args>`
(also `pip install schemalint`, `cargo install schemalint`). Examples below use
`schemalint`.

## The core loop

```bash
schemalint check-node --profile openai --source packages/shared/contracts/src   # 1. run
# 2. read each diagnostic: code, message, target name + file:line, pointer, hint
# 3. edit the schema at that pointer (see "Common fixes")
schemalint check-node --profile openai --source packages/shared/contracts/src   # 4. re-run until exit 0
```

Fix errors from the top; one fix often clears several. Stop only at exit `0`
with a `complete` coverage line (see "Exit codes").

## Pick the command

| Schemas live in | Command | Needs |
| --- | --- | --- |
| `.json` files | `schemalint check <files-or-dirs>` | nothing |
| Zod (TypeScript) | `schemalint check-node --source <path-or-glob>` | Node 18+, `tsx` (or `npx`), the project's deps installed (`zod` must resolve) |
| Pydantic models | `schemalint check-python --package <pkg>` | Python 3.9+, the package importable |

```bash
# JSON Schema files
schemalint check --profile anthropic schemas/ --exclude 'schemas/legacy/**'

# Zod, monorepo: run from the repo root; a source is a dir, file or glob
schemalint check-node --profile openai --source packages/shared/contracts/src
schemalint check-node --profile openai -S 'packages/*/src/**/*.ts' --exclude '**/generated/**'

# Pydantic
schemalint check-python --profile openai --package my_app.models
```

- `check-node` sources are relative to the current directory. Each file is
  evaluated against its nearest `tsconfig.json`; `export *` barrels, aliases
  and symlinks are followed.
- A glob (`src/**/*.ts`) lints schemas traced to a provider call site
  (`openai/helpers/zod`, `@anthropic-ai/sdk/helpers/zod`, `ai`); if none of the
  files import a provider SDK, every exported `z.object` is linted. A file or
  directory path additionally lints every exported `z.object` in it. To lint a
  schema module directly, pass its path, not a glob.
- Without `--source`/`--package`, `check-node`/`check-python` read
  `[tool.schemalint]` in `pyproject.toml` or `"schemalint"` in `package.json`
  (`{"profiles": [...], "include": [...]}`). Pass `--config <file>` to point at
  a different one. With neither, they fail with "no sources specified".
- `check` with no paths fails ("no schema files or directories provided").
- `--continue-on-discovery-error` keeps going after one source fails; it never
  turns partial coverage into success.

## Profiles

`schemalint profiles` lists them. `--profile` is repeatable.

| Provider | Profile id | Accepted aliases |
| --- | --- | --- |
| OpenAI | `openai.so.2026-04-30` | `openai`, `openai.so.latest` |
| Anthropic | `anthropic.so.2026-04-30` | `anthropic`, `anthropic.so.latest` |

Also accepts a path to a custom TOML profile (`check`/`check-node`/`check-python`).

If `--profile` is omitted: `check-node` infers the provider from the SDK each
schema is passed to, then from package.json dependencies; `check` looks at
package.json near the schema path. If nothing is detected it falls back to
OpenAI and prints an `info:` line on stderr saying so. Mixed providers with no
signal produce `provider is ambiguous ... pass --profile explicitly`. When you
know the target provider, always pass `--profile`; use both to be safe for
either. Profiles differ (Anthropic forbids numeric/length keywords but does
not require every property in `required`), so a schema clean under one can
fail the other.

## Output

Default is `human` on a terminal and `json` when stdout is piped. Agents
should pass `--format json` (or rely on the pipe) and read:

- `diagnostics[]`: `code` (e.g. `OAI-K-oneOf`), `severity`, `message`,
  `pointer` (JSON pointer to the offending node, `""` = root), `source.file`
  (for Zod also the line and schema `name`), `hint` (rewrite advice, may be
  absent), `seeUrl` (rule page), `providerEvidence`.
- `summary`: `errors`, `warnings`, `schemas_checked`.
- `report.coverage`: `status` (`complete` | `partial` | `failed` | `empty`),
  `attempted`, `discovered`, `checked`, `failed`; plus `report.failures[]`,
  `report.warnings[]`, `report.targets[]` (one per schema, with its status).
- `report.success`: true only if coverage is `complete` and there are no errors.

Other formats: `sarif` (code scanning), `gha` (GitHub Actions annotations),
`junit`, `human`. `-o <file>` writes output to a file instead of stdout.

Human output prints, per diagnostic: `error[CODE]: message`, the
`--> file:line Name`, `schema path` (the pointer), `hint`, and `see` URL. It
ends with a summary and a coverage line:

```text
1 issue found (1 error, 0 warnings) across 1 schema in 1143ms
coverage complete (1 attempted, 0 excluded, 1 discovered, 1 checked, 0 failed)
```

Read the coverage line before trusting a clean result. `1 discovered, 1
checked` is real; `0 discovered` is not a pass.

## Exit codes

| Code | Meaning | What to do |
| --- | --- | --- |
| `0` | Complete coverage, no errors | Done |
| `1` | Error diagnostics, or coverage `partial`/`failed` (import failure, missing path, conversion failure) | Fix diagnostics; for `failed`/`partial` read `report.failures` |
| `2` | Could not write the `--output` file | Fix the path or permissions |
| `3` | Empty coverage: nothing was discovered, so nothing was checked | Never treat as a pass. Fix the source/glob/`--package` |

Warnings alone exit `0`. Note that exit `1` with `0 issues` and coverage
`failed` means the tool could not even load the schemas.

## Common fixes

Apply the `hint` first; it is generated for the profile. Typical fixes (rule
docs: `https://1nder-labs.github.io/schemalint/rules/<category>/<name>`):

| Rule (OpenAI `OAI-`, Anthropic `ANT-`) | Fix |
| --- | --- |
| `-K-oneOf` | Replace `oneOf` with `anyOf` (keep branches mutually exclusive in practice). `anyOf` is not allowed at the root; wrap the union in an object property |
| `-K-allOf` (OpenAI) | Merge branches into one object: combine `properties` and `required`, drop `allOf`. Zod: `.and()` / `z.intersection` emit `allOf`; use `.extend()` or `.merge()` |
| `-K-<keyword>` for numeric, string-length, array-size keywords (`minimum`, `maxLength`, `minItems`, `uniqueItems`, ... ) | Remove the keyword; put the constraint in `description`; validate the response in your own code (Zod: drop `.min()`/`.max()` from the sent schema). Which are forbidden differs per profile: Anthropic forbids `minimum`; OpenAI allows `minimum` but warns on `minLength` |
| `-K-<kw>-restricted` | Use one of the values listed in the hint (e.g. a supported `format`), or remove the keyword |
| `-S-additional-properties-false` | Add `"additionalProperties": false` to every object (Zod: `z.object` already does; avoid `.passthrough()`, `z.record`, `.catchall()`) |
| `-S-all-properties-required` (OpenAI) | List every property in `required`. For an optional field make it nullable instead: `{"anyOf":[{"type":"string"},{"type":"null"}]}` or `"type":["string","null"]` (Zod: `.optional()` -> `.nullable()`) |
| `-S-root-anyof`, `-S-object-root`, `-S-root-enum` | Root must be a plain object: wrap the union/enum in an object property |
| `-S-anyof-objects` (warning) | Merge object-only `anyOf` branches into one object when possible |
| `-S-array-items` | Add an `items` schema to every array |
| `-S-recursive-schema`, `-S-external-refs` | Inline or flatten; remove `$ref` cycles and non-local refs |
| `-S-max-depth`, `-S-max-total-properties`, `-S-max-enum-values`, `-S-string-length-budget`, `-S-enum-string-length-budget` | Split the schema or shrink it; these are provider size limits |
| `-S-unknown-keyword` | Remove vendor/unknown keywords (e.g. `x-*`) from the sent schema |
| `-S-envelope-name` | The tool/response-format name must satisfy the provider's name rules; rename it |

If a rule is not listed, open the `seeUrl` from the diagnostic or run
`schemalint check --format json` and read `hint` and `providerEvidence`.
After changing a Zod or Pydantic source, re-run the same command; there is no
cache to clear.

## Troubleshooting

- `tsx or npx not found`: `npm install -g tsx` (or install Node with npx). Use `--node-path` to point at a specific runner.
- `Cannot find module 'zod'` (or another import) with coverage `failed`: install the project's dependencies first (`npm i`/`pnpm i`) so the schema's imports resolve.
- Exit `3` / `0 schemas`: the source matched nothing or no schema was traced. Check that the path is relative to the current directory (run from the repo root), that the extension is `.ts/.tsx/.mts/.cts`, that the file is not excluded by `tsconfig.json` `exclude`/`outDir` or by `--exclude`, and that the schema is exported. If files import a provider SDK but nothing was traced, pass the schema module's path (not a glob) to lint its exports directly.
- `provider is ambiguous ... pass --profile explicitly`: add `--profile openai` or `--profile anthropic`.
- `no sources specified`: pass `--source`/`--package` or configure `package.json` / `pyproject.toml`.
- Python import errors: use `--python-path <venv>/bin/python` so the package's dependencies resolve; `--package` takes a dotted module name.
- Unknown extra output on stderr (`info:`/`warning:` lines) is diagnostic chatter; the result is in stdout and the exit code.

## More

- `schemalint <command> --help` for every flag; `schemalint skills get core --full` appends the full command reference.
- Docs: https://1nder-labs.github.io/schemalint/
