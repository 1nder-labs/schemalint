use crate::ir::Node;

/// Detected JSON Schema dialect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    Draft2020_12,
    Draft2019_09,
    Draft07,
    Unknown,
}

/// Inspect the `$schema` keyword and keyword heuristics to determine dialect.
pub fn detect(node: &Node) -> Dialect {
    // Explicit $schema URI.
    if let Some(schema) = &node.annotations.schema {
        if let Some(uri) = schema.as_str() {
            if uri.contains("2020-12") {
                return Dialect::Draft2020_12;
            }
            if uri.contains("2019-09") {
                return Dialect::Draft2019_09;
            }
            if uri.contains("draft-07") || uri.contains("draft07") || uri.contains("http://json-schema.org/draft-07")
            {
                return Dialect::Draft07;
            }
        }
    }

    // Heuristic: presence of Draft 2020-12 exclusive keywords.
    if node.annotations.prefix_items.is_some() {
        return Dialect::Draft2020_12;
    }

    Dialect::Unknown
}
