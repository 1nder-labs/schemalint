use crate::ir::{Arena, NodeId};
use crate::profile::{Profile, Severity};
use crate::rules::metadata::{RuleCategory, RuleMetadata};
use crate::rules::registry::{Diagnostic, DiagnosticSeverity, Rule};

/// Anthropic's structured-outputs documentation names "Recursive schemas" as
/// unsupported.
///
/// `tarjan_scc` marks `is_cyclic` on every node in a `$ref` cycle and
/// `is_cycle_root` on exactly one of them, so this rule fires on the root and
/// reports each cycle once. Gating on the pointer shape instead would miss a
/// cycle that never passes through a `$defs` entry, which `$ref` resolution to
/// an arbitrary pointer makes reachable.
#[derive(Debug, Clone)]
pub(super) struct RecursiveSchemaRule {
    pub(super) profile_name: String,
}

impl Rule for RecursiveSchemaRule {
    fn check(&self, node: NodeId, arena: &Arena, profile: &Profile) -> Vec<Diagnostic> {
        let node_ref = &arena[node];
        if !node_ref.is_cycle_root {
            return Vec::new();
        }
        vec![Diagnostic {
            code: format!("{}-S-recursive-schema", profile.code_prefix),
            severity: DiagnosticSeverity::Error,
            message: "recursive schemas are not supported".to_string(),
            pointer: node_ref.json_pointer.clone(),
            source: None,
            profile: self.profile_name.clone(),
            hint: Some("Remove the $ref cycle; flatten or inline the recursive definition".into()),
        }]
    }

    fn metadata(&self) -> Option<RuleMetadata> {
        Some(RuleMetadata {
            name: "recursive-schema".into(),
            code: "{prefix}-S-recursive-schema".into(),
            description: "A schema must not contain a $ref cycle".into(),
            rationale:
                "Anthropic's structured-outputs documentation lists \"Recursive schemas\" among the unsupported schema features."
                    .into(),
            severity: Severity::Forbid,
            category: RuleCategory::Structural,
            bad_example: r##"{ "type": "object", "properties": { "root": { "$ref": "#/$defs/Node" } }, "$defs": { "Node": { "type": "object", "properties": { "next": { "$ref": "#/$defs/Node" } } } } }"##.into(),
            good_example: r##"{ "type": "object", "properties": { "root": { "$ref": "#/$defs/Node" } }, "$defs": { "Node": { "type": "object", "properties": { "value": { "type": "string" } } } } }"##.into(),
            see_also: Vec::new(),
            profile: Some(self.profile_name.clone()),
        })
    }
}
