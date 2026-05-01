use crate::ir::{Arena, NodeId};
use crate::profile::Profile;

/// Severity of a diagnostic emitted by the rule engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
}

/// A lint diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: String,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub pointer: String,
    pub source: Option<()>, // placeholder for SourceSpan (Phase 3+)
    pub profile: String,
    pub hint: Option<String>,
}

/// Trait implemented by all lint rules.
pub trait Rule: Sync {
    fn check(&self, node: NodeId, arena: &Arena, profile: &Profile) -> Vec<Diagnostic>;
}

/// Stable identifier for a rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RuleId(pub u32);
