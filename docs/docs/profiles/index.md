# Profiles

A profile defines what a specific LLM provider supports, restricts, or forbids in JSON Schema. Schemalint ships with built-in profiles for:

- [OpenAI Structured Outputs](./openai.md)
- [Anthropic Structured Outputs](./anthropic.md)

## How Profiles Work

Each profile is a TOML file that declares:
- **Keyword behavior**: whether a JSON Schema keyword is allowed, forbidden, warned, stripped, or unknown
- **Value restrictions**: allowed values for keywords like `format` and `additionalProperties`
- **Structural limits**: constraints on schema structure (max depth, max properties, etc.)

The linter compares your schema against the profile and emits diagnostics wherever a keyword or pattern would be rejected or altered by the provider's API.

## Creating a Custom Profile

You can create a custom profile file and pass it with `--profile path/to/custom.toml`:

```toml
name = "my-provider.v1"
version = "1.0"
code_prefix = "MYP"

type = "allow"
properties = "allow"
additionalProperties = { kind = "restricted", allowed = [false] }

[structural]
require_object_root = true
require_additional_properties_false = false
```

Custom profiles inherit all Class A and Class B rules from the rule engine — the profile data alone determines which rules fire and at what severity.
