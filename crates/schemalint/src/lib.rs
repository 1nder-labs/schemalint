//! schemalint — Static analysis tool for JSON Schema compatibility with LLM structured-output providers.

pub mod cache;
pub mod cli;
pub mod ingest;
pub mod ir;
pub mod node;
pub mod normalize;
pub mod profile;
pub mod python;
pub mod rules;

pub use cache::Cache;
pub use ir::{Arena, Node, NodeId, NodeKind};
pub use normalize::NormalizedSchema;
pub use profile::{Profile, Severity};
pub use rules::{Diagnostic, Rule};
