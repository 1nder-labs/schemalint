pub mod dialect;
pub mod desugar;
pub mod refs;
pub mod traverse;

use crate::ir::{Arena, NodeId};

#[derive(Debug, Clone)]
pub struct NormalizedSchema {
    pub arena: Arena,
    pub root_id: NodeId,
}
