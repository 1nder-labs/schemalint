use crate::ir::{Arena, NodeId};
use crate::profile::{Profile, Severity};
use crate::rules::metadata::{RuleCategory, RuleMetadata};
use crate::rules::registry::{Diagnostic, DiagnosticSeverity, Rule};

/// Anthropic's structured-outputs documentation names "Recursive schemas" as
/// unsupported. `tarjan_scc` (`crate::normalize::refs::tarjan_scc`) marks
/// `is_cyclic` on every node that participates in a `$ref` cycle, not only
/// the cycle's named entry point, so a rule that fires on each cyclic node
/// would report one diagnostic per participating node instead of one per
/// cycle. This rule fires only on a node that is itself a top-level
/// `$defs`/`definitions` entry — the thing a schema author can name and
/// fix — which reports the cycle exactly once.
#[derive(Debug, Clone)]
pub(super) struct RecursiveSchemaRule {
    pub(super) profile_name: String,
}

impl Rule for RecursiveSchemaRule {
    fn check(&self, node: NodeId, arena: &Arena, profile: &Profile) -> Vec<Diagnostic> {
        let node_ref = &arena[node];
        if !node_ref.is_cyclic || !is_defs_entry(&node_ref.json_pointer) {
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
            description: "A $defs/definitions entry must not form a $ref cycle".into(),
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

/// A top-level `$defs`/`definitions` entry: the only node whose
/// `json_pointer` has exactly the shape `/$defs/<name>` or
/// `/definitions/<name>` with no further segment. Matches the pointers
/// `normalize::build_defs` mints for these entries.
fn is_defs_entry(pointer: &str) -> bool {
    let rest = pointer
        .strip_prefix("/\u{24}defs/")
        .or_else(|| pointer.strip_prefix("/definitions/"));
    matches!(rest, Some(rest) if !rest.is_empty() && !rest.contains('/'))
}
