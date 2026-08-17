use indexmap::IndexMap;
use serde_json::Value;

use super::Keyword;

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

/// Policy for a keyword the engine does not recognize at all — one that
/// carries no accessor in `Keyword` and lands in `Node::unknown`.
///
/// This is deliberately a separate type from `Severity`. `Severity::Unknown`
/// already states a different fact: a keyword the engine DOES recognize, but
/// whose provider status was never verified. Reusing that variant here would
/// merge two statements ("the engine does not know this keyword" vs. "the
/// engine knows this keyword but not the provider's stance on it") that must
/// stay distinguishable in the profile data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UnknownKeywordPolicy {
    Allow,
    #[default]
    Warn,
    Forbid,
}

/// A loaded capability profile.
#[derive(Debug, Clone)]
pub struct Profile {
    pub name: String,
    pub version: String,
    pub code_prefix: String,
    /// Keyword → severity mapping in profile declaration order.
    pub keyword_map: IndexMap<Keyword, Severity>,
    /// Keyword → allowed values mapping for restricted keywords.
    pub restrictions: IndexMap<Keyword, Restriction>,
    pub structural: StructuralLimits,
}

/// Value restriction for a keyword.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Restriction {
    pub allowed_values: Vec<Value>,
}

/// Structural limits from the profile `[structural]` section.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct StructuralLimits {
    pub require_object_root: bool,
    pub require_additional_properties_false: bool,
    pub require_all_properties_in_required: bool,
    pub require_array_items: bool,
    pub forbid_root_any_of: bool,
    pub forbid_root_enum: bool,
    /// Treat an object with `additionalProperties: false` and no usable
    /// `properties` as an error rather than a warning. OpenAI rejects such
    /// schemas ("object schema missing properties"); providers that strip or
    /// tolerate them leave this `false`.
    pub forbid_empty_object: bool,
    pub max_object_depth: u32,
    pub max_total_properties: u32,
    pub max_total_enum_values: u32,
    pub max_string_length_total: u32,
    /// Apply the per-enum string budget only when an enum has more values
    /// than this threshold. Zero disables the conditional budget.
    pub enum_string_length_threshold: u32,
    /// Maximum Unicode-character count for one enum after the threshold is
    /// exceeded. Zero disables the conditional budget.
    pub max_enum_string_length: u32,
    pub max_optional_properties: u32,
    pub max_union_properties: u32,
    pub external_refs: bool,
    /// When `true`, schemas that combine `allOf` with a `$ref` inside its
    /// branches are rejected.  Currently used by the Anthropic profile, which
    /// does not support that pattern in Structured Outputs.
    pub forbid_allof_with_ref: bool,
    /// Policy for a keyword the engine does not recognize at all (see
    /// `UnknownKeywordPolicy`). Absent from the TOML means `Warn`.
    pub unknown_keyword_policy: UnknownKeywordPolicy,
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
    #[error("unknown JSON Schema keyword '{0}' in profile")]
    UnknownKeyword(String),
    #[error("invalid value for keyword '{0}'; expected a severity string or restricted table")]
    InvalidKeywordValue(String),
    #[error("invalid restrictions container; expected an array of tables")]
    InvalidRestrictionsContainer,
    #[error("invalid restriction for keyword '{0}': missing 'allowed' array")]
    InvalidRestriction(String),
    #[error("keyword '{0}' cannot define both a severity and a restriction")]
    ConflictingKeyword(String),
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

    let mut keyword_map = IndexMap::new();
    let mut restrictions = IndexMap::new();

    // Walk top-level entries for keywords and restrictions.
    for (key, val) in table {
        match key.as_str() {
            "name" | "version" | "code_prefix" | "structural" | "restrictions" => continue,
            _ => {}
        }

        let keyword = key
            .parse::<Keyword>()
            .map_err(|()| ProfileError::UnknownKeyword(key.clone()))?;

        match val {
            toml::Value::String(s) => {
                let sev = Severity::parse(s)?;
                keyword_map.insert(keyword, sev);
            }
            toml::Value::Table(t)
                if t.get("kind").and_then(|v| v.as_str()) == Some("restricted") =>
            {
                restrictions.insert(keyword, parse_restriction(t, key)?);
            }
            _ => {
                return Err(ProfileError::InvalidKeywordValue(key.clone()));
            }
        }
    }

    // Also process [[restrictions]] array-of-tables if present.
    if let Some(container) = table.get("restrictions") {
        let arr = container
            .as_array()
            .ok_or(ProfileError::InvalidRestrictionsContainer)?;
        for entry in arr {
            let t = entry
                .as_table()
                .ok_or(ProfileError::MissingField("restrictions entry"))?;
            let keyword = t
                .get("keyword")
                .and_then(|v| v.as_str())
                .ok_or(ProfileError::MissingField("restrictions.keyword"))?;
            let typed_keyword = keyword
                .parse::<Keyword>()
                .map_err(|()| ProfileError::UnknownKeyword(keyword.to_string()))?;
            if keyword_map.contains_key(&typed_keyword) {
                return Err(ProfileError::ConflictingKeyword(keyword.to_string()));
            }
            restrictions.insert(typed_keyword, parse_restriction(t, keyword)?);
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

fn parse_restriction(table: &toml::Table, keyword: &str) -> Result<Restriction, ProfileError> {
    let allowed = table
        .get("allowed")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| ProfileError::InvalidRestriction(keyword.to_string()))?;
    let allowed_values = allowed
        .iter()
        .cloned()
        .map(toml_to_json)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Restriction { allowed_values })
}

fn parse_structural(val: Option<&toml::Value>) -> Result<StructuralLimits, ProfileError> {
    let Some(v @ toml::Value::Table(_)) = val else {
        // Missing [structural] is fatal in Phase 1 per plan U3.
        return Err(ProfileError::MissingField("[structural] section"));
    };
    // toml::de::Error is #[from]-mapped to ProfileError::InvalidToml.
    Ok(v.clone().try_into()?)
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
