use schemalint::normalize::normalize;
use schemalint::profile::load;
use schemalint::rules::registry::{DiagnosticSeverity, Rule, RuleSet, RULES};

#[path = "rules_tests/class_a.rs"]
mod class_a;
#[path = "rules_tests/class_b.rs"]
mod class_b;
#[path = "rules_tests/metadata.rs"]
mod metadata;
#[path = "rules_tests/semantic.rs"]
mod semantic;

pub(crate) fn load_test_profile(toml: &str) -> schemalint::profile::Profile {
    load(toml.as_bytes()).unwrap()
}

pub(crate) fn normalize_schema(
    value: serde_json::Value,
) -> schemalint::normalize::NormalizedSchema {
    normalize(value).unwrap()
}
