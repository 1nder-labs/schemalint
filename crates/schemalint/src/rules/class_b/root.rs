use crate::ir::{Arena, NodeId};
use crate::profile::{Profile, Severity};
use crate::rules::metadata::{RuleCategory, RuleMetadata};
use crate::rules::registry::{Diagnostic, DiagnosticSeverity, Rule};

#[derive(Debug, Clone)]
pub(super) struct RootAnyOfRule {
    pub(super) profile_name: String,
}

impl Rule for RootAnyOfRule {
    fn check(&self, node: NodeId, arena: &Arena, profile: &Profile) -> Vec<Diagnostic> {
        let node_ref = &arena[node];
        if node_ref.parent.is_some() || node_ref.annotations.any_of.is_none() {
            return Vec::new();
        }
        vec![Diagnostic {
            code: format!("{}-S-root-anyof", profile.code_prefix),
            severity: DiagnosticSeverity::Error,
            message: "root schema must not use anyOf".to_string(),
            pointer: node_ref.json_pointer.clone(),
            source: None,
            profile: self.profile_name.clone(),
            hint: Some("Move anyOf under an object property or use a single root object".into()),
        }]
    }

    fn metadata(&self) -> Option<RuleMetadata> {
        Some(RuleMetadata {
            name: "root-anyof".into(),
            code: "{prefix}-S-root-anyof".into(),
            description: "The root schema must not use anyOf".into(),
            rationale: format!(
                "{} requires a plain object root; anyOf is only valid below the root.",
                self.profile_name
            ),
            severity: Severity::Forbid,
            category: RuleCategory::Structural,
            bad_example: r#"{ "type": "object", "anyOf": [{ "type": "object" }] }"#.into(),
            good_example: r#"{ "type": "object", "properties": { "value": { "anyOf": [{ "type": "string" }, { "type": "number" }] } }, "required": ["value"], "additionalProperties": false }"#.into(),
            see_also: Vec::new(),
            profile: Some(self.profile_name.clone()),
        })
    }
}

#[derive(Debug, Clone)]
pub(super) struct RootEnumRule {
    pub(super) profile_name: String,
}

impl Rule for RootEnumRule {
    fn check(&self, node: NodeId, arena: &Arena, profile: &Profile) -> Vec<Diagnostic> {
        let node_ref = &arena[node];
        if node_ref.parent.is_some() || node_ref.annotations.enum_values.is_none() {
            return Vec::new();
        }
        vec![Diagnostic {
            code: format!("{}-S-root-enum", profile.code_prefix),
            severity: DiagnosticSeverity::Error,
            message: "root schema must not use enum".to_string(),
            pointer: node_ref.json_pointer.clone(),
            source: None,
            profile: self.profile_name.clone(),
            hint: Some("Use an object root with an enum-valued property".into()),
        }]
    }

    fn metadata(&self) -> Option<RuleMetadata> {
        Some(RuleMetadata {
            name: "root-enum".into(),
            code: "{prefix}-S-root-enum".into(),
            description: "The root schema must not use enum".into(),
            rationale: format!(
                "{} requires a plain object root and rejects enum at the top level.",
                self.profile_name
            ),
            severity: Severity::Forbid,
            category: RuleCategory::Structural,
            bad_example: r#"{ "type": "string", "enum": ["yes", "no"] }"#.into(),
            good_example: r#"{ "type": "object", "properties": { "answer": { "type": "string", "enum": ["yes", "no"] } }, "required": ["answer"], "additionalProperties": false }"#.into(),
            see_also: Vec::new(),
            profile: Some(self.profile_name.clone()),
        })
    }
}
