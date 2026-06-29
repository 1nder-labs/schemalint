"""Schema cases for the generated regression corpus."""

def append_part1(schemas: list) -> None:
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
