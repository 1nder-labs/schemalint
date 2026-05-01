#!/usr/bin/env python3
"""Generate 50 synthetic schemas for the regression corpus."""

import json
import os
from pathlib import Path

CORPUS_DIR = Path("crates/schemalint/tests/corpus")

def write_schema(idx: int, schema: dict):
    path = CORPUS_DIR / f"schema_{idx:02d}.json"
    with open(path, "w") as f:
        json.dump(schema, f, indent=2)
        f.write("\n")

schemas = []

# 01: Simple valid object schema
schemas.append({
    "type": "object",
    "properties": {"name": {"type": "string"}},
    "required": ["name"],
    "additionalProperties": False
})

# 02: Simple valid array schema
schemas.append({
    "type": "object",
    "properties": {
        "items": {
            "type": "array",
            "items": {"type": "string"}
        }
    },
    "required": ["items"],
    "additionalProperties": False
})

# 03: Schema with forbidden allOf
schemas.append({
    "allOf": [{"type": "string"}]
})

# 04: Schema with forbidden oneOf
schemas.append({
    "oneOf": [{"type": "string"}, {"type": "number"}]
})

# 05: Schema with forbidden not
schemas.append({
    "not": {"type": "string"}
})

# 06: Schema with warned uniqueItems
schemas.append({
    "type": "object",
    "properties": {
        "tags": {"type": "array", "uniqueItems": True}
    },
    "additionalProperties": False
})

# 07: Schema with warned contains
schemas.append({
    "type": "object",
    "properties": {
        "items": {"type": "array", "contains": {"type": "string"}}
    },
    "additionalProperties": False
})

# 08: Schema with restricted format (allowed)
schemas.append({
    "type": "object",
    "properties": {
        "created_at": {"type": "string", "format": "date-time"}
    },
    "required": ["created_at"],
    "additionalProperties": False
})

# 09: Schema with restricted format (disallowed)
schemas.append({
    "type": "object",
    "properties": {
        "card": {"type": "string", "format": "credit-card"}
    },
    "required": ["card"],
    "additionalProperties": False
})

# 10: Schema missing additionalProperties false
schemas.append({
    "type": "object",
    "properties": {"name": {"type": "string"}},
    "required": ["name"]
})

# 11: Schema with additionalProperties true
schemas.append({
    "type": "object",
    "properties": {"name": {"type": "string"}},
    "required": ["name"],
    "additionalProperties": True
})

# 12: Root not object (string)
schemas.append({"type": "string"})

# 13: Root not object (array)
schemas.append({"type": "array", "items": {"type": "string"}})

# 14: Missing required property
schemas.append({
    "type": "object",
    "properties": {"a": {"type": "string"}, "b": {"type": "string"}},
    "required": ["a"],
    "additionalProperties": False
})

# 15: Deeply nested schema (depth > 5)
deep = {"type": "string"}
for _ in range(6):
    deep = {
        "type": "object",
        "properties": {"next": deep},
        "additionalProperties": False
    }
schemas.append(deep)

# 16: External $ref
schemas.append({"$ref": "https://example.com/schema.json"})

# 17: Internal $ref (valid)
schemas.append({
    "$defs": {
        "name": {"type": "string"}
    },
    "type": "object",
    "properties": {
        "firstName": {"$ref": "#/$defs/name"},
        "lastName": {"$ref": "#/$defs/name"}
    },
    "required": ["firstName", "lastName"],
    "additionalProperties": False
})

# 18: Cyclic $ref
schemas.append({
    "$defs": {
        "node": {
            "type": "object",
            "properties": {
                "value": {"type": "string"},
                "child": {"$ref": "#/$defs/node"}
            },
            "additionalProperties": False
        }
    },
    "type": "object",
    "properties": {"root": {"$ref": "#/$defs/node"}},
    "additionalProperties": False
})

# 19: Enum within limit
schemas.append({
    "type": "object",
    "properties": {
        "status": {"type": "string", "enum": ["active", "inactive"]}
    },
    "required": ["status"],
    "additionalProperties": False
})

# 20: Empty object schema
schemas.append({})

# 21: Boolean true schema
schemas.append(True)

# 22: Boolean false schema
schemas.append(False)

# 23: Type array desugaring
schemas.append({
    "type": "object",
    "properties": {
        "value": {"type": ["string", "null"]}
    },
    "required": ["value"],
    "additionalProperties": False
})

# 24: Multiple forbidden keywords
schemas.append({
    "allOf": [{"type": "string"}],
    "oneOf": [{"type": "string"}],
    "not": {"type": "number"}
})

# 25: Pattern properties (forbidden)
schemas.append({
    "type": "object",
    "patternProperties": {
        "^foo": {"type": "string"}
    },
    "additionalProperties": False
})

# 26: Unevaluated properties (forbidden)
schemas.append({
    "type": "object",
    "properties": {"name": {"type": "string"}},
    "required": ["name"],
    "unevaluatedProperties": False,
    "additionalProperties": False
})

# 27: Property names (forbidden)
schemas.append({
    "type": "object",
    "propertyNames": {"pattern": "^[a-z]+$"},
    "additionalProperties": False
})

# 28: Min/max properties (forbidden)
schemas.append({
    "type": "object",
    "properties": {"a": {"type": "string"}},
    "minProperties": 1,
    "maxProperties": 5,
    "additionalProperties": False
})

# 29: If/then/else (forbidden)
schemas.append({
    "type": "object",
    "if": {"properties": {"foo": {"type": "string"}}},
    "then": {"properties": {"bar": {"type": "string"}}},
    "else": {"properties": {"baz": {"type": "string"}}},
    "additionalProperties": False
})

# 30: Dependent required (forbidden)
schemas.append({
    "type": "object",
    "properties": {"name": {"type": "string"}, "credit_card": {"type": "string"}},
    "dependentRequired": {"credit_card": ["name"]},
    "additionalProperties": False
})

# 31: Dependent schemas (forbidden)
schemas.append({
    "type": "object",
    "properties": {"foo": {"type": "string"}},
    "dependentSchemas": {
        "foo": {"properties": {"bar": {"type": "string"}}, "required": ["bar"]}
    },
    "additionalProperties": False
})

# 32: Prefix items (forbidden)
schemas.append({
    "type": "object",
    "properties": {
        "tuple": {
            "type": "array",
            "prefixItems": [{"type": "string"}, {"type": "number"}]
        }
    },
    "additionalProperties": False
})

# 33: Min/max items (forbidden)
schemas.append({
    "type": "object",
    "properties": {
        "list": {"type": "array", "items": {"type": "string"}, "minItems": 1, "maxItems": 10}
    },
    "additionalProperties": False
})

# 34: Multiple of (forbidden)
schemas.append({
    "type": "object",
    "properties": {
        "count": {"type": "integer", "multipleOf": 5}
    },
    "additionalProperties": False
})

# 35: Exclusive min/max (forbidden)
schemas.append({
    "type": "object",
    "properties": {
        "score": {"type": "number", "exclusiveMinimum": 0, "exclusiveMaximum": 100}
    },
    "additionalProperties": False
})

# 36: Pattern (forbidden)
schemas.append({
    "type": "object",
    "properties": {
        "email": {"type": "string", "pattern": "^.*@.*$"}
    },
    "additionalProperties": False
})

# 37: Const (forbidden)
schemas.append({
    "type": "object",
    "properties": {
        "version": {"const": "1.0.0"}
    },
    "additionalProperties": False
})

# 38: Discriminator (forbidden)
schemas.append({
    "type": "object",
    "properties": {"kind": {"type": "string"}},
    "required": ["kind"],
    "discriminator": {"propertyName": "kind"},
    "additionalProperties": False
})

# 39: Schema with description/title/default (allowed)
schemas.append({
    "type": "object",
    "title": "User",
    "description": "A user object",
    "default": {"name": "anon"},
    "properties": {"name": {"type": "string"}},
    "required": ["name"],
    "additionalProperties": False
})

# 40: Max total properties exceeded (>100 in OpenAI profile)
props = {}
for i in range(101):
    props[f"prop_{i}"] = {"type": "string"}
schemas.append({
    "type": "object",
    "properties": props,
    "additionalProperties": False
})

# 41: Max enum values exceeded (>500 in OpenAI profile)
schemas.append({
    "type": "object",
    "properties": {
        "status": {"type": "string", "enum": [f"v{i}" for i in range(501)]}
    },
    "additionalProperties": False
})

# 42: String length budget exceeded
long_name = "a" * 60001
schemas.append({
    "type": "object",
    "properties": {
        long_name: {"type": "string"}
    },
    "additionalProperties": False
})

# 43: AnyOf (forbidden)
schemas.append({
    "anyOf": [{"type": "string"}, {"type": "number"}]
})

# 44: Nested forbidden keywords
schemas.append({
    "type": "object",
    "properties": {
        "config": {
            "allOf": [{"type": "object", "properties": {"debug": {"type": "boolean"}}}]
        }
    },
    "additionalProperties": False
})

# 45: Both structural and keyword violations
schemas.append({
    "allOf": [{"type": "string"}],
    "additionalProperties": True
})

# 46: Valid nested object with refs
schemas.append({
    "$defs": {
        "address": {
            "type": "object",
            "properties": {
                "street": {"type": "string"},
                "city": {"type": "string"}
            },
            "required": ["street", "city"],
            "additionalProperties": False
        }
    },
    "type": "object",
    "properties": {
        "name": {"type": "string"},
        "address": {"$ref": "#/$defs/address"}
    },
    "required": ["name", "address"],
    "additionalProperties": False
})

# 47: Self-referential $ref
schemas.append({
    "$defs": {
        "self": {"$ref": "#/$defs/self"}
    },
    "type": "object",
    "properties": {"me": {"$ref": "#/$defs/self"}},
    "additionalProperties": False
})

# 48: Multiple external refs
schemas.append({
    "type": "object",
    "properties": {
        "a": {"$ref": "https://example.com/a.json"},
        "b": {"$ref": "https://example.com/b.json"}
    },
    "additionalProperties": False
})

# 49: Schema with items (allowed)
schemas.append({
    "type": "object",
    "properties": {
        "tags": {"type": "array", "items": {"type": "string"}}
    },
    "required": ["tags"],
    "additionalProperties": False
})

# 50: Schema with number constraints (minimum, maximum allowed)
schemas.append({
    "type": "object",
    "properties": {
        "age": {"type": "integer", "minimum": 0, "maximum": 150}
    },
    "required": ["age"],
    "additionalProperties": False
})

# Write all schemas
CORPUS_DIR.mkdir(parents=True, exist_ok=True)
for i, schema in enumerate(schemas, 1):
    write_schema(i, schema)

print(f"Generated {len(schemas)} schemas in {CORPUS_DIR}")
