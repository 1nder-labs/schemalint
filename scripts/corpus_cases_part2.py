"""Schema cases for the generated regression corpus."""

def append_part2(schemas: list) -> None:
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
