use std::ops::{Index, IndexMut};

use indexmap::IndexMap;
use serde_json::Value;

/// Unique identifier for a node in the arena.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub u32);

/// A node in the normalized IR.
#[derive(Debug, Clone)]
pub struct Node {
    pub kind: NodeKind,
    pub annotations: Annotations,
    pub unknown: IndexMap<String, Value>,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
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
    /// TODO: `Ref` is defined but never assigned by the parser or normalizer.
    /// Either implement `$ref` kind assignment or remove this variant.
    Ref,
    AnyOf,
    OneOf,
    AllOf,
    Not,
}

/// Standard JSON Schema keywords.
///
/// Every field is `Option<Value>` so the parser can store the raw JSON value
/// without loss. The normalizer (U4) operates on these raw values.
#[derive(Debug, Clone, Default)]
pub struct Annotations {
    pub r#type: Option<Value>,
    pub properties: Option<Value>,
    pub required: Option<Value>,
    pub additional_properties: Option<Value>,
    pub items: Option<Value>,
    pub prefix_items: Option<Value>,
    pub min_items: Option<Value>,
    pub max_items: Option<Value>,
    pub unique_items: Option<Value>,
    pub contains: Option<Value>,
    pub minimum: Option<Value>,
    pub maximum: Option<Value>,
    pub exclusive_minimum: Option<Value>,
    pub exclusive_maximum: Option<Value>,
    pub multiple_of: Option<Value>,
    pub min_length: Option<Value>,
    pub max_length: Option<Value>,
    pub pattern: Option<Value>,
    pub format: Option<Value>,
    pub enum_values: Option<Value>,
    pub const_value: Option<Value>,
    pub pattern_properties: Option<Value>,
    pub unevaluated_properties: Option<Value>,
    pub property_names: Option<Value>,
    pub min_properties: Option<Value>,
    pub max_properties: Option<Value>,
    pub description: Option<Value>,
    pub title: Option<Value>,
    pub default: Option<Value>,
    pub discriminator: Option<Value>,
    pub r#ref: Option<Value>,
    pub defs: Option<Value>,
    pub definitions: Option<Value>,
    pub any_of: Option<Value>,
    pub all_of: Option<Value>,
    pub one_of: Option<Value>,
    pub not: Option<Value>,
    pub if_schema: Option<Value>,
    pub then_schema: Option<Value>,
    pub else_schema: Option<Value>,
    pub dependent_required: Option<Value>,
    pub dependent_schemas: Option<Value>,
    pub schema: Option<Value>,
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

    pub fn iter(&self) -> impl Iterator<Item = (NodeId, &Node)> {
        self.nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (NodeId(i as u32), n))
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

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

/// Errors that can occur when parsing a JSON Schema into the IR.
#[derive(Debug, Clone, thiserror::Error, PartialEq)]
pub enum ParseError {
    #[error("invalid JSON Schema root: expected object or boolean, got {0}")]
    InvalidRootType(String),
}

/// Parse a JSON Schema value into the IR arena.
///
/// Returns the arena and the root node id. The parse is shallow: one node per
/// schema value. Nested schemas remain as raw `serde_json::Value` inside the
/// node's `annotations`; the normalizer (U4) builds the graph structure.
pub fn parse(value: Value) -> Result<(Arena, NodeId), ParseError> {
    let mut arena = Arena::new();
    let node = parse_node(value)?;
    let id = arena.alloc(node);
    Ok((arena, id))
}

pub fn parse_node(value: Value) -> Result<Node, ParseError> {
    match value {
        Value::Bool(true) => Ok(Node {
            kind: NodeKind::Any,
            annotations: Annotations::default(),
            unknown: IndexMap::new(),
            parent: None,
            children: Vec::new(),
            depth: 0,
            json_pointer: String::new(),
            ref_target: None,
            is_cyclic: false,
        }),
        Value::Bool(false) => Ok(Node {
            kind: NodeKind::Not,
            annotations: Annotations::default(),
            unknown: IndexMap::new(),
            parent: None,
            children: Vec::new(),
            depth: 0,
            json_pointer: String::new(),
            ref_target: None,
            is_cyclic: false,
        }),
        Value::Object(map) => {
            let mut annotations = Annotations::default();
            let mut unknown = IndexMap::new();

            for (key, val) in map {
                match key.as_str() {
                    "type" => annotations.r#type = Some(val),
                    "properties" => annotations.properties = Some(val),
                    "required" => annotations.required = Some(val),
                    "additionalProperties" => annotations.additional_properties = Some(val),
                    "items" => annotations.items = Some(val),
                    "prefixItems" => annotations.prefix_items = Some(val),
                    "minItems" => annotations.min_items = Some(val),
                    "maxItems" => annotations.max_items = Some(val),
                    "uniqueItems" => annotations.unique_items = Some(val),
                    "contains" => annotations.contains = Some(val),
                    "minimum" => annotations.minimum = Some(val),
                    "maximum" => annotations.maximum = Some(val),
                    "exclusiveMinimum" => annotations.exclusive_minimum = Some(val),
                    "exclusiveMaximum" => annotations.exclusive_maximum = Some(val),
                    "multipleOf" => annotations.multiple_of = Some(val),
                    "minLength" => annotations.min_length = Some(val),
                    "maxLength" => annotations.max_length = Some(val),
                    "pattern" => annotations.pattern = Some(val),
                    "format" => annotations.format = Some(val),
                    "enum" => annotations.enum_values = Some(val),
                    "const" => annotations.const_value = Some(val),
                    "patternProperties" => annotations.pattern_properties = Some(val),
                    "unevaluatedProperties" => annotations.unevaluated_properties = Some(val),
                    "propertyNames" => annotations.property_names = Some(val),
                    "minProperties" => annotations.min_properties = Some(val),
                    "maxProperties" => annotations.max_properties = Some(val),
                    "description" => annotations.description = Some(val),
                    "title" => annotations.title = Some(val),
                    "default" => annotations.default = Some(val),
                    "discriminator" => annotations.discriminator = Some(val),
                    "$schema" => annotations.schema = Some(val),
                    "$ref" => annotations.r#ref = Some(val),
                    "$defs" => annotations.defs = Some(val),
                    "definitions" => annotations.definitions = Some(val),
                    "anyOf" => annotations.any_of = Some(val),
                    "allOf" => annotations.all_of = Some(val),
                    "oneOf" => annotations.one_of = Some(val),
                    "not" => annotations.not = Some(val),
                    "if" => annotations.if_schema = Some(val),
                    "then" => annotations.then_schema = Some(val),
                    "else" => annotations.else_schema = Some(val),
                    "dependentRequired" => annotations.dependent_required = Some(val),
                    "dependentSchemas" => annotations.dependent_schemas = Some(val),
                    _ => {
                        unknown.insert(key, val);
                    }
                }
            }

            Ok(Node {
                kind: NodeKind::Object,
                annotations,
                unknown,
                parent: None,
                children: Vec::new(),
                depth: 0,
                json_pointer: String::new(),
                ref_target: None,
                is_cyclic: false,
            })
        }
        other => Err(ParseError::InvalidRootType(json_type_name(&other))),
    }
}

fn json_type_name(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Array(_) => "array".to_string(),
        Value::String(_) => "string".to_string(),
        Value::Number(_) => "number".to_string(),
        Value::Bool(_) => "boolean".to_string(),
        Value::Object(_) => "object".to_string(),
    }
}
