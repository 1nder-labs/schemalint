use schemalint::profile::{load, ProfileError, Severity, StructuralLimits};

#[path = "profile_tests/additional_errors.rs"]
mod additional_errors;
#[path = "profile_tests/anthropic.rs"]
mod anthropic;
#[path = "profile_tests/core_errors.rs"]
mod core_errors;
#[path = "profile_tests/duplicate_restrictions.rs"]
mod duplicate_restrictions;
#[path = "profile_tests/happy.rs"]
mod happy;
#[path = "profile_tests/minimal.rs"]
mod minimal;
#[path = "profile_tests/openai.rs"]
mod openai;
#[path = "profile_tests/restriction_edges.rs"]
mod restriction_edges;
#[path = "profile_tests/severity_parse.rs"]
mod severity_parse;
#[path = "profile_tests/severity_values.rs"]
mod severity_values;
#[path = "profile_tests/structural_limits.rs"]
mod structural_limits;
#[path = "profile_tests/utf8.rs"]
mod utf8;
