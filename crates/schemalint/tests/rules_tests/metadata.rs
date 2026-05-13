use super::*;

#[derive(Debug)]
struct TestRule;

impl Rule for TestRule {
    fn check(
        &self,
        _node: schemalint::ir::NodeId,
        _arena: &schemalint::ir::Arena,
        _profile: &schemalint::profile::Profile,
    ) -> Vec<schemalint::rules::Diagnostic> {
        Vec::new()
    }
}

#[linkme::distributed_slice(schemalint::rules::RULES)]
static TEST_RULE: &dyn Rule = &TestRule;

#[test]
fn linkme_auto_registration_works() {
    let found = RULES.iter().any(|&r| std::ptr::eq(r, TEST_RULE));
    assert!(found, "TEST_RULE should be auto-registered via linkme");
}

#[test]
fn all_static_rules_have_metadata() {
    let mut count = 0;
    for rule in RULES {
        let Some(metadata) = rule.metadata() else {
            continue;
        };
        count += 1;
        assert!(
            !metadata.name.is_empty(),
            "RuleMetadata::name must be non-empty"
        );
        assert!(
            metadata.code.contains("{prefix}"),
            "RuleMetadata::code must use {{prefix}} placeholder, got: {}",
            metadata.code
        );
        assert!(
            !metadata.description.is_empty(),
            "RuleMetadata::description must be non-empty"
        );
        assert!(
            !metadata.rationale.is_empty(),
            "RuleMetadata::rationale must be non-empty"
        );
        assert!(
            !metadata.bad_example.is_empty(),
            "RuleMetadata::bad_example must be non-empty"
        );
        assert!(
            !metadata.good_example.is_empty(),
            "RuleMetadata::good_example must be non-empty"
        );
        assert!(
            metadata.profile.is_none(),
            "Static rules must have profile=None; got: {:?} for rule '{}'",
            metadata.profile,
            metadata.name
        );
    }
    assert!(count > 0, "At least one static rule must return metadata");
}

#[test]
fn dynamic_rules_have_metadata() {
    let profile_toml = r#"
        name = "test-profile"
        version = "1.0"
        code_prefix = "TST"

        allOf = "forbid"
        format = { kind = "restricted", allowed = ["date-time", "email"] }

        [structural]
        require_object_root = true
        require_additional_properties_false = true
        max_object_depth = 5
    "#;
    let profile = load_test_profile(profile_toml);
    let rule_set = RuleSet::from_profile(&profile);

    for rule in rule_set.dynamic_rules() {
        let metadata = rule
            .metadata()
            .unwrap_or_else(|| panic!("dynamic rule missing metadata"));
        assert!(
            !metadata.name.is_empty(),
            "dynamic RuleMetadata::name must be non-empty"
        );
        assert!(
            !metadata.description.is_empty(),
            "dynamic RuleMetadata::description must be non-empty"
        );
        assert!(
            metadata.profile.is_some(),
            "Dynamic rules must have profile=Some: {}",
            metadata.name
        );
    }
}
