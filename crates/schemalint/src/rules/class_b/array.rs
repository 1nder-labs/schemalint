use crate::ir::{Arena, NodeId};
use crate::profile::{Profile, Severity};
use crate::rules::class_b::helpers::schema_is_array;
use crate::rules::metadata::{RuleCategory, RuleMetadata};
use crate::rules::registry::{Diagnostic, DiagnosticSeverity, Rule};

#[derive(Debug, Clone)]
pub(super) struct ArrayItemsRule {
    pub(super) profile_name: String,
}

impl Rule for ArrayItemsRule {
    fn check(&self, node: NodeId, arena: &Arena, profile: &Profile) -> Vec<Diagnostic> {
        let node_ref = &arena[node];
        if !schema_is_array(node_ref) || node_ref.annotations.items.is_some() {
            return Vec::new();
        }
        vec![Diagnostic {
            code: format!("{}-S-array-items", profile.code_prefix),
            severity: DiagnosticSeverity::Error,
            message: "array schema must declare items".to_string(),
            pointer: node_ref.json_pointer.clone(),
            source: None,
            profile: self.profile_name.clone(),
            hint: Some("Add an items schema for provider compatibility".to_string()),
        }]
    }

    fn metadata(&self) -> Option<RuleMetadata> {
        Some(RuleMetadata {
            name: "array-items".into(),
            code: "{prefix}-S-array-items".into(),
            description: "Array schemas must declare an items schema".into(),
            rationale: format!(
                "{} rejects array schemas that omit the items keyword.",
                self.profile_name
            ),
            severity: Severity::Forbid,
            category: RuleCategory::Structural,
            bad_example: r#"{ "type": "array" }"#.into(),
            good_example: r#"{ "type": "array", "items": { "type": "string" } }"#.into(),
            see_also: Vec::new(),
            profile: Some(self.profile_name.clone()),
        })
    }
}
