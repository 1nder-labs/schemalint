use std::collections::HashMap;

use serde_json::Value;

/// Severity levels for keyword and structural rules in a profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Severity {
    Allow,
    Warn,
    Strip,
    Forbid,
    Unknown,
}

impl Severity {
    /// Parse a severity string from TOML.
    pub fn parse(s: &str) -> Result<Self, ProfileError> {
        match s {
            "allow" => Ok(Severity::Allow),
            "warn" => Ok(Severity::Warn),
            "strip" => Ok(Severity::Strip),
            "forbid" => Ok(Severity::Forbid),
            "unknown" => Ok(Severity::Unknown),
            other => Err(ProfileError::InvalidSeverity(other.to_string())),
        }
    }
}

/// A loaded capability profile.
#[derive(Debug, Clone)]
pub struct Profile {
    pub name: String,
    pub version: String,
    pub code_prefix: String,
    /// Keyword → severity mapping. Keys are leaked `&'static str` for O(1) lookup.
    pub keyword_map: HashMap<&'static str, Severity>,
    /// Keyword → allowed values mapping for restricted keywords.
    pub restrictions: HashMap<&'static str, Restriction>,
    pub structural: StructuralLimits,
}

/// Value restriction for a keyword.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Restriction {
    pub allowed_values: Vec<Value>,
}

/// Structural limits from the profile `[structural]` section.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StructuralLimits {
    pub require_object_root: bool,
    pub require_additional_properties_false: bool,
    pub require_all_properties_in_required: bool,
    pub max_object_depth: u32,
    pub max_total_properties: u32,
    pub max_total_enum_values: u32,
    pub max_string_length_total: u32,
    pub external_refs: bool,
}

/// Errors that can occur when loading a profile.
#[derive(Debug, thiserror::Error)]
pub enum ProfileError {
    #[error("invalid TOML: {0}")]
    InvalidToml(#[from] toml::de::Error),
    #[error("missing required field: {0}")]
    MissingField(&'static str),
    #[error("invalid severity '{0}'; expected one of: allow, warn, strip, forbid, unknown")]
    InvalidSeverity(String),
    #[error("invalid restriction for keyword '{0}': missing 'allowed' array")]
    InvalidRestriction(String),
}

// ---------------------------------------------------------------------------
// Loader
// ---------------------------------------------------------------------------

/// Load a profile from raw TOML bytes.
pub fn load(bytes: &[u8]) -> Result<Profile, ProfileError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| ProfileError::InvalidSeverity("invalid UTF-8 in profile".to_string()))?;
    let doc: toml::Value = text.parse()?;
    let table = doc
        .as_table()
        .ok_or(ProfileError::MissingField("root table"))?;

    let name = table
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or(ProfileError::MissingField("name"))?
        .to_string();

    let version = table
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let code_prefix = table
        .get("code_prefix")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            let first_segment = name.split('.').next().unwrap_or(&name);
            first_segment.to_uppercase()
        });

    let mut keyword_map = HashMap::new();
    let mut restrictions = HashMap::new();

    const KNOWN_KEYWORDS: &[&str] = &[
        "type",
        "properties",
        "required",
        "additionalProperties",
        "items",
        "prefixItems",
        "minItems",
        "maxItems",
        "uniqueItems",
        "contains",
        "minimum",
        "maximum",
        "exclusiveMinimum",
        "exclusiveMaximum",
        "multipleOf",
        "minLength",
        "maxLength",
        "pattern",
        "format",
        "enum",
        "const",
        "patternProperties",
        "unevaluatedProperties",
        "propertyNames",
        "minProperties",
        "maxProperties",
        "description",
        "title",
        "default",
        "discriminator",
        "$ref",
        "$defs",
        "definitions",
        "anyOf",
        "allOf",
        "oneOf",
        "not",
        "if",
        "then",
        "else",
        "dependentRequired",
        "dependentSchemas",
    ];

    // Walk top-level entries for keywords and restrictions.
    for (key, val) in table {
        match key.as_str() {
            "name" | "version" | "code_prefix" | "structural" | "restrictions" => continue,
            _ => {}
        }

        if !KNOWN_KEYWORDS.contains(&key.as_str()) {
            return Err(ProfileError::InvalidSeverity(format!(
                "unknown keyword '{}' in profile; expected a known JSON Schema keyword",
                key
            )));
        }

        match val {
            toml::Value::String(s) => {
                let sev = Severity::parse(s)?;
                keyword_map.insert(leak_str(key), sev);
            }
            toml::Value::Table(t)
                if t.get("kind").and_then(|v| v.as_str()) == Some("restricted") =>
            {
                let allowed = t
                    .get("allowed")
                    .and_then(|v| v.as_array())
                    .ok_or_else(|| ProfileError::InvalidRestriction(key.clone()))?;
                let mut values = Vec::new();
                for v in allowed {
                    values.push(toml_to_json(v.clone())?);
                }
                restrictions.insert(
                    leak_str(key),
                    Restriction {
                        allowed_values: values,
                    },
                );
            }
            _ => {
                return Err(ProfileError::InvalidSeverity(format!(
                    "invalid value for keyword '{}': expected string severity or restricted table",
                    key
                )));
            }
        }
    }

    // Also process [[restrictions]] array-of-tables if present.
    if let Some(toml::Value::Array(arr)) = table.get("restrictions") {
        for entry in arr {
            let t = entry
                .as_table()
                .ok_or(ProfileError::MissingField("restrictions entry"))?;
            let keyword = t
                .get("keyword")
                .and_then(|v| v.as_str())
                .ok_or(ProfileError::MissingField("restrictions.keyword"))?;
            let allowed = t
                .get("allowed")
                .and_then(|v| v.as_array())
                .ok_or_else(|| ProfileError::InvalidRestriction(keyword.to_string()))?;
            let mut values = Vec::new();
            for v in allowed {
                values.push(toml_to_json(v.clone())?);
            }
            restrictions.insert(
                leak_str(keyword),
                Restriction {
                    allowed_values: values,
                },
            );
        }
    }

    let structural = parse_structural(table.get("structural"))?;

    Ok(Profile {
        name,
        version,
        code_prefix,
        keyword_map,
        restrictions,
        structural,
    })
}

fn parse_structural(val: Option<&toml::Value>) -> Result<StructuralLimits, ProfileError> {
    let mut limits = StructuralLimits::default();
    let Some(toml::Value::Table(t)) = val else {
        // Missing [structural] is fatal in Phase 1 per plan U3.
        return Err(ProfileError::MissingField("[structural] section"));
    };

    if let Some(v) = t.get("require_object_root").and_then(|v| v.as_bool()) {
        limits.require_object_root = v;
    }
    if let Some(v) = t
        .get("require_additional_properties_false")
        .and_then(|v| v.as_bool())
    {
        limits.require_additional_properties_false = v;
    }
    if let Some(v) = t
        .get("require_all_properties_in_required")
        .and_then(|v| v.as_bool())
    {
        limits.require_all_properties_in_required = v;
    }
    if let Some(v) = t.get("max_object_depth").and_then(|v| v.as_integer()) {
        limits.max_object_depth = u32::try_from(v).map_err(|_| {
            ProfileError::InvalidSeverity(format!("max_object_depth out of u32 range: {v}"))
        })?;
    }
    if let Some(v) = t.get("max_total_properties").and_then(|v| v.as_integer()) {
        limits.max_total_properties = u32::try_from(v).map_err(|_| {
            ProfileError::InvalidSeverity(format!("max_total_properties out of u32 range: {v}"))
        })?;
    }
    if let Some(v) = t.get("max_total_enum_values").and_then(|v| v.as_integer()) {
        limits.max_total_enum_values = u32::try_from(v).map_err(|_| {
            ProfileError::InvalidSeverity(format!("max_total_enum_values out of u32 range: {v}"))
        })?;
    }
    if let Some(v) = t
        .get("max_string_length_total")
        .and_then(|v| v.as_integer())
    {
        limits.max_string_length_total = u32::try_from(v).map_err(|_| {
            ProfileError::InvalidSeverity(format!("max_string_length_total out of u32 range: {v}"))
        })?;
    }
    if let Some(v) = t.get("external_refs").and_then(|v| v.as_bool()) {
        limits.external_refs = v;
    }

    Ok(limits)
}

fn leak_str(s: &str) -> &'static str {
    Box::leak(s.to_owned().into_boxed_str())
}

fn toml_to_json(val: toml::Value) -> Result<Value, ProfileError> {
    match val {
        toml::Value::String(s) => Ok(Value::String(s)),
        toml::Value::Integer(i) => Ok(Value::Number(serde_json::Number::from(i))),
        toml::Value::Float(f) => {
            let num = serde_json::Number::from_f64(f).ok_or_else(|| {
                ProfileError::InvalidSeverity(format!("invalid float value: {f}"))
            })?;
            Ok(Value::Number(num))
        }
        toml::Value::Boolean(b) => Ok(Value::Bool(b)),
        toml::Value::Array(arr) => {
            let mut out = Vec::new();
            for v in arr {
                out.push(toml_to_json(v)?);
            }
            Ok(Value::Array(out))
        }
        toml::Value::Table(map) => {
            let mut out = serde_json::Map::new();
            for (k, v) in map {
                out.insert(k, toml_to_json(v)?);
            }
            Ok(Value::Object(out))
        }
        toml::Value::Datetime(dt) => Ok(Value::String(dt.to_string())),
    }
}
