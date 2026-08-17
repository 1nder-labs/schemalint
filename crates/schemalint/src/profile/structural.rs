use super::StructuralLimits;

/// Stable identity shared by profile configuration, production diagnostics,
/// and executable provider truth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StructuralRuleId {
    ObjectRoot,
    AdditionalPropertiesFalse,
    AllPropertiesRequired,
    ArrayItems,
    RootAnyOf,
    RootEnum,
    EmptyObject,
    MaxDepth,
    MaxTotalProperties,
    MaxTotalEnumValues,
    StringLengthBudget,
    EnumStringLengthBudget,
    MaxOptionalProperties,
    MaxUnionProperties,
    ExternalRefs,
    AllOfWithRef,
}

impl StructuralRuleId {
    pub const fn name(self) -> &'static str {
        match self {
            Self::ObjectRoot => "require_object_root",
            Self::AdditionalPropertiesFalse => "require_additional_properties_false",
            Self::AllPropertiesRequired => "require_all_properties_in_required",
            Self::ArrayItems => "require_array_items",
            Self::RootAnyOf => "forbid_root_any_of",
            Self::RootEnum => "forbid_root_enum",
            Self::EmptyObject => "forbid_empty_object",
            Self::MaxDepth => "max_object_depth",
            Self::MaxTotalProperties => "max_total_properties",
            Self::MaxTotalEnumValues => "max_total_enum_values",
            Self::StringLengthBudget => "max_string_length_total",
            Self::EnumStringLengthBudget => "enum_string_length_budget",
            Self::MaxOptionalProperties => "max_optional_properties",
            Self::MaxUnionProperties => "max_union_properties",
            Self::ExternalRefs => "external_refs",
            Self::AllOfWithRef => "forbid_allof_with_ref",
        }
    }

    pub const fn diagnostic_suffix(self) -> &'static str {
        match self {
            Self::ObjectRoot => "-S-object-root",
            Self::AdditionalPropertiesFalse => "-S-additional-properties-false",
            Self::AllPropertiesRequired => "-S-all-properties-required",
            Self::ArrayItems => "-S-array-items",
            Self::RootAnyOf => "-S-root-anyof",
            Self::RootEnum => "-S-root-enum",
            Self::EmptyObject => "-S-empty-object",
            Self::MaxDepth => "-S-max-depth",
            Self::MaxTotalProperties => "-S-max-total-properties",
            Self::MaxTotalEnumValues => "-S-max-enum-values",
            Self::StringLengthBudget => "-S-string-length-budget",
            Self::EnumStringLengthBudget => "-S-enum-string-length-budget",
            Self::MaxOptionalProperties => "-S-max-optional-properties",
            Self::MaxUnionProperties => "-S-max-union-properties",
            Self::ExternalRefs => "-S-external-refs",
            Self::AllOfWithRef => "-S-allof-with-ref",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name.split("__").next().unwrap_or(name) {
            "require_object_root" => Self::ObjectRoot,
            "additional_properties_false" | "require_additional_properties_false" => {
                Self::AdditionalPropertiesFalse
            }
            "require_all_properties_in_required" => Self::AllPropertiesRequired,
            "require_array_items" => Self::ArrayItems,
            "forbid_root_any_of" => Self::RootAnyOf,
            "forbid_root_enum" => Self::RootEnum,
            "forbid_empty_object" => Self::EmptyObject,
            "max_object_depth" => Self::MaxDepth,
            "max_total_properties" => Self::MaxTotalProperties,
            "max_total_enum_values" => Self::MaxTotalEnumValues,
            "max_string_length_total" => Self::StringLengthBudget,
            "enum_string_length_budget" => Self::EnumStringLengthBudget,
            "max_optional_properties" => Self::MaxOptionalProperties,
            "max_union_properties" => Self::MaxUnionProperties,
            "external_refs" => Self::ExternalRefs,
            "allof_with_ref" | "forbid_allof_with_ref" => Self::AllOfWithRef,
            _ => return None,
        })
    }
}

impl StructuralLimits {
    /// Enumerate every production structural rule enabled by this profile.
    /// The exhaustive destructure makes new configuration fields choose their
    /// conformance identity at compile time rather than silently losing truth.
    pub fn enabled_rule_ids(&self) -> Vec<StructuralRuleId> {
        let Self {
            require_object_root,
            require_additional_properties_false,
            require_all_properties_in_required,
            require_array_items,
            forbid_root_any_of,
            forbid_root_enum,
            forbid_empty_object,
            max_object_depth,
            max_total_properties,
            max_total_enum_values,
            max_string_length_total,
            enum_string_length_threshold,
            max_enum_string_length,
            max_optional_properties,
            max_union_properties,
            external_refs,
            forbid_allof_with_ref,
            // Not a `StructuralRuleId`: its diagnostic defaults to `Warn`
            // (KTD5), and this parity apparatus only expresses rules whose
            // "reject" boundary case is an Error — `evaluate_structural_truth`
            // matches at error severity only (KTD10), so a warn-level reject
            // would misreport as accept. U8 rules it out of the boundary
            // machinery on purpose; the field is still named here so this
            // stays an exhaustive, conscious decision.
            unknown_keyword_policy: _,
        } = self;
        [
            (*require_object_root, StructuralRuleId::ObjectRoot),
            (
                *require_additional_properties_false,
                StructuralRuleId::AdditionalPropertiesFalse,
            ),
            (
                *require_all_properties_in_required,
                StructuralRuleId::AllPropertiesRequired,
            ),
            (*require_array_items, StructuralRuleId::ArrayItems),
            (*forbid_root_any_of, StructuralRuleId::RootAnyOf),
            (*forbid_root_enum, StructuralRuleId::RootEnum),
            (*forbid_empty_object, StructuralRuleId::EmptyObject),
            (*max_object_depth > 0, StructuralRuleId::MaxDepth),
            (
                *max_total_properties > 0,
                StructuralRuleId::MaxTotalProperties,
            ),
            (
                *max_total_enum_values > 0,
                StructuralRuleId::MaxTotalEnumValues,
            ),
            (
                *max_string_length_total > 0,
                StructuralRuleId::StringLengthBudget,
            ),
            (
                *enum_string_length_threshold > 0 && *max_enum_string_length > 0,
                StructuralRuleId::EnumStringLengthBudget,
            ),
            (
                *max_optional_properties > 0,
                StructuralRuleId::MaxOptionalProperties,
            ),
            (
                *max_union_properties > 0,
                StructuralRuleId::MaxUnionProperties,
            ),
            (*external_refs, StructuralRuleId::ExternalRefs),
            (*forbid_allof_with_ref, StructuralRuleId::AllOfWithRef),
        ]
        .into_iter()
        .filter_map(|(enabled, rule)| enabled.then_some(rule))
        .collect()
    }
}
