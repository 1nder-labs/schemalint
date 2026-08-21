use super::*;
use schemalint::profile::{EvidenceStatus, RuleKey};
use schemalint::profiles::{ANTHROPIC_SO_2026_04_30, OPENAI_SO_2026_04_30};
use schemalint::rules::RuleSet;
use schemalint::rules::RULES;
use std::collections::BTreeSet;

#[test]
fn evidence_statuses_parse_and_preserve_source_order() {
    let profile = load(
        br#"
name = "custom"
[structural]
[[evidence]]
key = "K-example"
status = "inferred"
basis = "Adjacent documentation only."
sources = [
  { title = "First", url = "https://developers.openai.com/example#first" },
  { title = "Second", url = "https://developers.openai.com/example#second" }
]
"#,
    )
    .unwrap();
    let evidence = &profile.evidence[&RuleKey::parse("K-example").unwrap()];
    assert_eq!(evidence.status, EvidenceStatus::Inferred);
    assert_eq!(evidence.sources[0].title, "First");
}

#[test]
fn invalid_status_specific_evidence_is_rejected() {
    let error = load(
        br#"
name = "custom"
[structural]
[[evidence]]
key = "K-example"
status = "unknown"
basis = "No source."
sources = [{ title = "Invented", url = "https://developers.openai.com/example#invented" }]
"#,
    )
    .unwrap_err();
    assert!(matches!(error, ProfileError::InvalidEvidence(_, _)));
}

#[test]
fn documented_example_requires_a_source() {
    let error = load(
        br#"
name = "custom"
[structural]
[[evidence]]
key = "K-example"
status = "documented_example"
basis = "An official example demonstrates the behavior."
"#,
    )
    .unwrap_err();
    assert!(matches!(error, ProfileError::InvalidEvidence(_, _)));
}

#[test]
fn built_in_evidence_exactly_covers_active_rules() {
    for bytes in [OPENAI_SO_2026_04_30, ANTHROPIC_SO_2026_04_30] {
        let profile = load(bytes.as_bytes()).unwrap();
        let rules = RuleSet::from_profile(&profile).unwrap();
        let active: BTreeSet<_> = RULES
            .iter()
            .copied()
            .chain(rules.dynamic_rules())
            .filter_map(|rule| rule.metadata())
            .chain(std::iter::once(schemalint::rules::envelope::metadata()))
            .filter_map(|metadata| {
                let code = metadata.code.replace("{prefix}", &profile.code_prefix);
                RuleKey::from_code(&code, &profile.code_prefix)
            })
            .collect();
        let stored: BTreeSet<_> = profile.evidence.keys().cloned().collect();
        assert_eq!(stored, active, "evidence coverage for {}", profile.name);
    }
}

#[test]
fn legacy_custom_profile_needs_no_evidence() {
    let profile = load(b"name = \"legacy\"\nallOf = \"forbid\"\n[structural]\n").unwrap();
    assert!(profile.evidence.is_empty());
    RuleSet::from_profile(&profile).unwrap();
}

#[test]
fn custom_profile_name_never_enables_repository_policy() {
    let profile = load(
        b"name = \"openai.so.2026-04-30\"\ncode_prefix = \"CUSTOM\"\nallOf = \"forbid\"\n[structural]\n",
    )
    .unwrap();
    assert!(profile.evidence.is_empty());
    RuleSet::from_profile(&profile).unwrap();
}
