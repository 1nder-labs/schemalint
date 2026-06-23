mod array;
mod budget;
pub(crate) mod helpers;
mod object;
mod refs;
mod root;

use crate::profile::Profile;
use crate::rules::registry::Rule;

pub(crate) use helpers::schema_is_object;

use array::ArrayItemsRule;
use budget::{BudgetRule, MaxDepthRule};
use object::{AdditionalPropertiesFalseRule, AllPropertiesRequiredRule, ObjectRootRule};
use refs::{AllOfWithRefRule, ExternalRefsRule};
use root::{RootAnyOfRule, RootEnumRule};

/// Generate all Class B structural rules from a loaded profile.
pub fn generate_class_b_rules(profile: &Profile) -> Vec<Box<dyn Rule>> {
    let mut rules: Vec<Box<dyn Rule>> = Vec::new();
    let s = &profile.structural;

    if s.require_object_root {
        rules.push(Box::new(ObjectRootRule {
            profile_name: profile.name.clone(),
        }));
    }
    if s.require_additional_properties_false {
        rules.push(Box::new(AdditionalPropertiesFalseRule {
            profile_name: profile.name.clone(),
        }));
    }
    if s.require_all_properties_in_required {
        rules.push(Box::new(AllPropertiesRequiredRule {
            profile_name: profile.name.clone(),
        }));
    }
    if s.require_array_items {
        rules.push(Box::new(ArrayItemsRule {
            profile_name: profile.name.clone(),
        }));
    }
    if s.forbid_root_any_of {
        rules.push(Box::new(RootAnyOfRule {
            profile_name: profile.name.clone(),
        }));
    }
    if s.forbid_root_enum {
        rules.push(Box::new(RootEnumRule {
            profile_name: profile.name.clone(),
        }));
    }
    if s.max_object_depth > 0 {
        rules.push(Box::new(MaxDepthRule {
            limit: s.max_object_depth,
            profile_name: profile.name.clone(),
        }));
    }
    if s.max_total_properties > 0 {
        rules.push(Box::new(BudgetRule::max_total_properties(
            s.max_total_properties,
            profile.name.clone(),
        )));
    }
    if s.max_total_enum_values > 0 {
        rules.push(Box::new(BudgetRule::max_total_enum_values(
            s.max_total_enum_values,
            profile.name.clone(),
        )));
    }
    if s.max_string_length_total > 0 {
        rules.push(Box::new(BudgetRule::max_string_length(
            s.max_string_length_total,
            profile.name.clone(),
        )));
    }
    if s.max_optional_properties > 0 {
        rules.push(Box::new(BudgetRule::max_optional_properties(
            s.max_optional_properties,
            profile.name.clone(),
        )));
    }
    if s.max_union_properties > 0 {
        rules.push(Box::new(BudgetRule::max_union_properties(
            s.max_union_properties,
            profile.name.clone(),
        )));
    }
    if s.external_refs {
        rules.push(Box::new(ExternalRefsRule {
            profile_name: profile.name.clone(),
        }));
    }
    if profile.code_prefix == "ANT" {
        rules.push(Box::new(AllOfWithRefRule {
            profile_name: profile.name.clone(),
        }));
    }

    rules
}
