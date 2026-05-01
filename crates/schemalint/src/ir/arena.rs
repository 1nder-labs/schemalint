use std::ops::{Index, IndexMut};

/// Unique identifier for a node in the arena.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub u32);

/// A node in the normalized IR.
#[derive(Debug, Clone)]
pub struct Node {
    pub kind: NodeKind,
    pub annotations: Annotations,
    pub unknown: indexmap::IndexMap<String, serde_json::Value>,
    pub parent: Option<NodeId>,
    pub depth: u32,
    pub json_pointer: String,
    pub ref_target: Option<NodeId>,
    pub is_cyclic: bool,
}

/// Classification of JSON Schema node kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    Object,
    Array,
    String,
    Integer,
    Number,
    Boolean,
    Null,
    Any,
    Ref,
    AnyOf,
    OneOf,
    AllOf,
    Not,
}

/// Standard JSON Schema keywords.
#[derive(Debug, Clone, Default)]
pub struct Annotations {
    pub r#type: Option<serde_json::Value>,
    pub properties: Option<serde_json::Value>,
    pub required: Option<serde_json::Value>,
    pub additional_properties: Option<serde_json::Value>,
    pub items: Option<serde_json::Value>,
    pub prefix_items: Option<serde_json::Value>,
    pub min_items: Option<serde_json::Value>,
    pub max_items: Option<serde_json::Value>,
    pub unique_items: Option<serde_json::Value>,
    pub contains: Option<serde_json::Value>,
    pub minimum: Option<serde_json::Value>,
    pub maximum: Option<serde_json::Value>,
    pub exclusive_minimum: Option<serde_json::Value>,
    pub exclusive_maximum: Option<serde_json::Value>,
    pub multiple_of: Option<serde_json::Value>,
    pub min_length: Option<serde_json::Value>,
    pub max_length: Option<serde_json::Value>,
    pub pattern: Option<serde_json::Value>,
    pub format: Option<serde_json::Value>,
    pub enum_values: Option<serde_json::Value>,
    pub const_value: Option<serde_json::Value>,
    pub pattern_properties: Option<serde_json::Value>,
    pub unevaluated_properties: Option<serde_json::Value>,
    pub property_names: Option<serde_json::Value>,
    pub min_properties: Option<serde_json::Value>,
    pub max_properties: Option<serde_json::Value>,
    pub description: Option<serde_json::Value>,
    pub title: Option<serde_json::Value>,
    pub default: Option<serde_json::Value>,
    pub discriminator: Option<serde_json::Value>,
    pub r#ref: Option<serde_json::Value>,
    pub defs: Option<serde_json::Value>,
    pub definitions: Option<serde_json::Value>,
    pub any_of: Option<serde_json::Value>,
    pub all_of: Option<serde_json::Value>,
    pub one_of: Option<serde_json::Value>,
    pub not: Option<serde_json::Value>,
    pub if_schema: Option<serde_json::Value>,
    pub then_schema: Option<serde_json::Value>,
    pub else_schema: Option<serde_json::Value>,
    pub dependent_required: Option<serde_json::Value>,
    pub dependent_schemas: Option<serde_json::Value>,
}

/// Arena allocator for IR nodes.
#[derive(Debug, Clone, Default)]
pub struct Arena {
    nodes: Vec<Node>,
}

impl Arena {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn alloc(&mut self, node: Node) -> NodeId {
        let id = NodeId(self.nodes.len() as u32);
        self.nodes.push(node);
        id
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

impl Index<NodeId> for Arena {
    type Output = Node;

    fn index(&self, index: NodeId) -> &Self::Output {
        &self.nodes[index.0 as usize]
    }
}

impl IndexMut<NodeId> for Arena {
    fn index_mut(&mut self, index: NodeId) -> &mut Self::Output {
        &mut self.nodes[index.0 as usize]
    }
}
