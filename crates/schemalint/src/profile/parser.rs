use std::collections::HashMap;

/// Severity levels for keyword and structural rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Severity {
    Allow,
    Warn,
    Strip,
    Forbid,
    Unknown,
}

/// A loaded capability profile.
#[derive(Debug, Clone)]
pub struct Profile {
    pub name: String,
    pub version: String,
    pub keyword_map: HashMap<&'static str, Severity>,
    pub restrictions: HashMap<&'static str, Restriction>,
    pub structural: StructuralLimits,
}

/// Value restriction for a keyword.
#[derive(Debug, Clone)]
pub struct Restriction {
    pub allowed_values: Vec<serde_json::Value>,
}

/// Structural limits from the profile `[structural]` section.
#[derive(Debug, Clone, Default)]
pub struct StructuralLimits {
    pub require_object_root: bool,
    pub require_additional_properties_false: bool,
    pub require_all_properties_in_required: bool,
    pub max_object_depth: u32,
    pub max_total_properties: u32,
    pub max_total_enum_values: u32,
    pub max_string_length_total: u32,
}
