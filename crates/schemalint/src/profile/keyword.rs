use std::borrow::Borrow;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::str::FromStr;

use serde_json::Value;

use crate::ir::Node;

/// Function pointer for extracting a JSON Schema keyword from an IR node.
pub type KeywordAccessor = fn(&Node) -> Option<&Value>;

/// JSON Schema keywords understood by the normalized IR and profile format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keyword {
    Type,
    Properties,
    Required,
    AdditionalProperties,
    Items,
    PrefixItems,
    MinItems,
    MaxItems,
    UniqueItems,
    Contains,
    Minimum,
    Maximum,
    ExclusiveMinimum,
    ExclusiveMaximum,
    MultipleOf,
    MinLength,
    MaxLength,
    Pattern,
    Format,
    Enum,
    Const,
    PatternProperties,
    UnevaluatedProperties,
    PropertyNames,
    MinProperties,
    MaxProperties,
    Description,
    Title,
    Default,
    Discriminator,
    Ref,
    Defs,
    Definitions,
    AnyOf,
    AllOf,
    OneOf,
    Not,
    If,
    Then,
    Else,
    DependentRequired,
    DependentSchemas,
}

impl Keyword {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Type => "type",
            Self::Properties => "properties",
            Self::Required => "required",
            Self::AdditionalProperties => "additionalProperties",
            Self::Items => "items",
            Self::PrefixItems => "prefixItems",
            Self::MinItems => "minItems",
            Self::MaxItems => "maxItems",
            Self::UniqueItems => "uniqueItems",
            Self::Contains => "contains",
            Self::Minimum => "minimum",
            Self::Maximum => "maximum",
            Self::ExclusiveMinimum => "exclusiveMinimum",
            Self::ExclusiveMaximum => "exclusiveMaximum",
            Self::MultipleOf => "multipleOf",
            Self::MinLength => "minLength",
            Self::MaxLength => "maxLength",
            Self::Pattern => "pattern",
            Self::Format => "format",
            Self::Enum => "enum",
            Self::Const => "const",
            Self::PatternProperties => "patternProperties",
            Self::UnevaluatedProperties => "unevaluatedProperties",
            Self::PropertyNames => "propertyNames",
            Self::MinProperties => "minProperties",
            Self::MaxProperties => "maxProperties",
            Self::Description => "description",
            Self::Title => "title",
            Self::Default => "default",
            Self::Discriminator => "discriminator",
            Self::Ref => "$ref",
            Self::Defs => "$defs",
            Self::Definitions => "definitions",
            Self::AnyOf => "anyOf",
            Self::AllOf => "allOf",
            Self::OneOf => "oneOf",
            Self::Not => "not",
            Self::If => "if",
            Self::Then => "then",
            Self::Else => "else",
            Self::DependentRequired => "dependentRequired",
            Self::DependentSchemas => "dependentSchemas",
        }
    }

    /// Return the IR accessor for this keyword.
    pub const fn accessor(self) -> KeywordAccessor {
        match self {
            Self::Type => |n| n.annotations.r#type.as_ref(),
            Self::Properties => |n| n.annotations.properties.as_ref(),
            Self::Required => |n| n.annotations.required.as_ref(),
            Self::AdditionalProperties => |n| n.annotations.additional_properties.as_ref(),
            Self::Items => |n| n.annotations.items.as_ref(),
            Self::PrefixItems => |n| n.annotations.prefix_items.as_ref(),
            Self::MinItems => |n| n.annotations.min_items.as_ref(),
            Self::MaxItems => |n| n.annotations.max_items.as_ref(),
            Self::UniqueItems => |n| n.annotations.unique_items.as_ref(),
            Self::Contains => |n| n.annotations.contains.as_ref(),
            Self::Minimum => |n| n.annotations.minimum.as_ref(),
            Self::Maximum => |n| n.annotations.maximum.as_ref(),
            Self::ExclusiveMinimum => |n| n.annotations.exclusive_minimum.as_ref(),
            Self::ExclusiveMaximum => |n| n.annotations.exclusive_maximum.as_ref(),
            Self::MultipleOf => |n| n.annotations.multiple_of.as_ref(),
            Self::MinLength => |n| n.annotations.min_length.as_ref(),
            Self::MaxLength => |n| n.annotations.max_length.as_ref(),
            Self::Pattern => |n| n.annotations.pattern.as_ref(),
            Self::Format => |n| n.annotations.format.as_ref(),
            Self::Enum => |n| n.annotations.enum_values.as_ref(),
            Self::Const => |n| n.annotations.const_value.as_ref(),
            Self::PatternProperties => |n| n.annotations.pattern_properties.as_ref(),
            Self::UnevaluatedProperties => |n| n.annotations.unevaluated_properties.as_ref(),
            Self::PropertyNames => |n| n.annotations.property_names.as_ref(),
            Self::MinProperties => |n| n.annotations.min_properties.as_ref(),
            Self::MaxProperties => |n| n.annotations.max_properties.as_ref(),
            Self::Description => |n| n.annotations.description.as_ref(),
            Self::Title => |n| n.annotations.title.as_ref(),
            Self::Default => |n| n.annotations.default.as_ref(),
            Self::Discriminator => |n| n.annotations.discriminator.as_ref(),
            Self::Ref => |n| n.annotations.r#ref.as_ref(),
            Self::Defs => |n| n.annotations.defs.as_ref(),
            Self::Definitions => |n| n.annotations.definitions.as_ref(),
            Self::AnyOf => |n| n.annotations.any_of.as_ref(),
            Self::AllOf => |n| n.annotations.all_of.as_ref(),
            Self::OneOf => |n| n.annotations.one_of.as_ref(),
            Self::Not => |n| n.annotations.not.as_ref(),
            Self::If => |n| n.annotations.if_schema.as_ref(),
            Self::Then => |n| n.annotations.then_schema.as_ref(),
            Self::Else => |n| n.annotations.else_schema.as_ref(),
            Self::DependentRequired => |n| n.annotations.dependent_required.as_ref(),
            Self::DependentSchemas => |n| n.annotations.dependent_schemas.as_ref(),
        }
    }
}

impl FromStr for Keyword {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "type" => Ok(Self::Type),
            "properties" => Ok(Self::Properties),
            "required" => Ok(Self::Required),
            "additionalProperties" => Ok(Self::AdditionalProperties),
            "items" => Ok(Self::Items),
            "prefixItems" => Ok(Self::PrefixItems),
            "minItems" => Ok(Self::MinItems),
            "maxItems" => Ok(Self::MaxItems),
            "uniqueItems" => Ok(Self::UniqueItems),
            "contains" => Ok(Self::Contains),
            "minimum" => Ok(Self::Minimum),
            "maximum" => Ok(Self::Maximum),
            "exclusiveMinimum" => Ok(Self::ExclusiveMinimum),
            "exclusiveMaximum" => Ok(Self::ExclusiveMaximum),
            "multipleOf" => Ok(Self::MultipleOf),
            "minLength" => Ok(Self::MinLength),
            "maxLength" => Ok(Self::MaxLength),
            "pattern" => Ok(Self::Pattern),
            "format" => Ok(Self::Format),
            "enum" => Ok(Self::Enum),
            "const" => Ok(Self::Const),
            "patternProperties" => Ok(Self::PatternProperties),
            "unevaluatedProperties" => Ok(Self::UnevaluatedProperties),
            "propertyNames" => Ok(Self::PropertyNames),
            "minProperties" => Ok(Self::MinProperties),
            "maxProperties" => Ok(Self::MaxProperties),
            "description" => Ok(Self::Description),
            "title" => Ok(Self::Title),
            "default" => Ok(Self::Default),
            "discriminator" => Ok(Self::Discriminator),
            "$ref" => Ok(Self::Ref),
            "$defs" => Ok(Self::Defs),
            "definitions" => Ok(Self::Definitions),
            "anyOf" => Ok(Self::AnyOf),
            "allOf" => Ok(Self::AllOf),
            "oneOf" => Ok(Self::OneOf),
            "not" => Ok(Self::Not),
            "if" => Ok(Self::If),
            "then" => Ok(Self::Then),
            "else" => Ok(Self::Else),
            "dependentRequired" => Ok(Self::DependentRequired),
            "dependentSchemas" => Ok(Self::DependentSchemas),
            _ => Err(()),
        }
    }
}

impl Borrow<str> for Keyword {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl Hash for Keyword {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

impl fmt::Display for Keyword {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
