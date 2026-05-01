use crate::ir::{Arena, NodeId};
use crate::profile::Profile;

/// A lint diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: String,
    pub severity: crate::profile::Severity,
    pub message: String,
    pub pointer: String,
    pub source: Option<()>, // placeholder for SourceSpan
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
