use indexmap::IndexMap;
use serde_json::Value;

use crate::ir::{parse_node, Arena, NodeId};
use crate::normalize::dialect::Dialect;

pub mod desugar;
pub mod dialect;
pub mod refs;
pub mod traverse;

#[derive(Debug, Clone)]
pub struct NormalizedSchema {
    pub arena: Arena,
    pub root_id: NodeId,
    pub defs: IndexMap<String, NodeId>,
    pub dialect: Dialect,
}

#[derive(Debug, thiserror::Error)]
pub enum NormalizeError {
    #[error("parse error: {0}")]
    ParseError(String),
    #[error("unresolved internal $ref: {0}")]
    UnresolvedRef(String),
}

/// Normalize a raw JSON Schema value into a canonical IR graph.
///
/// Pipeline (in order):
/// 1. Parse root node
/// 2. Detect dialect
/// 3. Build `$defs` / `definitions` map
/// 4. Expand tree (create all child nodes, set parent/depth/pointer)
/// 5. Resolve `$ref` edges
/// 6. Tarjan SCC for cycle detection
/// 7. Desugar type arrays
pub fn normalize(value: Value) -> Result<NormalizedSchema, NormalizeError> {
    let mut arena = Arena::new();
    let root_node = parse_node(value).map_err(|e| NormalizeError::ParseError(e.to_string()))?;
    let root_id = arena.alloc(root_node);

    let dialect = dialect::detect(&arena[root_id]);

    let mut defs = IndexMap::new();
    build_defs(&mut arena, root_id, &mut defs)?;

    // Expand the full tree so that every $ref node exists before resolution.
    traverse::expand_and_dfs(&mut arena, root_id)?;

    let _ref_edges = refs::resolve_refs(&mut arena, &defs)?;
    let transitive_edges = refs::transitive_ref_edges(&arena);
    refs::tarjan_scc(&mut arena, &transitive_edges);

    // Desugar type arrays for all nodes.
    let all_ids: Vec<NodeId> = arena.iter().map(|(id, _)| id).collect();
    for node_id in all_ids {
        desugar::desugar_type_arrays(&mut arena, node_id);
    }

    Ok(NormalizedSchema {
        arena,
        root_id,
        defs,
        dialect,
    })
}

/// Parse `$defs` and `definitions` into definition nodes and populate `defs` map.
fn build_defs(
    arena: &mut Arena,
    root_id: NodeId,
    defs: &mut IndexMap<String, NodeId>,
) -> Result<(), NormalizeError> {
    // Clone annotations to avoid borrow issues.
    let defs_val = arena[root_id].annotations.defs.clone();
    let definitions_val = arena[root_id].annotations.definitions.clone();

    // `$defs` (Draft 2020-12)
    if let Some(Value::Object(map)) = defs_val {
        for (name, val) in map {
            let child = parse_node(val).map_err(|e| NormalizeError::ParseError(e.to_string()))?;
            let child_id = arena.alloc(child);
            arena[child_id].json_pointer = format!("/\u{24}defs/{}", name);
            arena[child_id].parent = Some(root_id);
            arena[child_id].depth = 1;
            arena[root_id].children.push(child_id);
            defs.insert(name, child_id);
        }
    }

    // `definitions` (Draft 7) — `$defs` takes precedence on name conflict.
    if let Some(Value::Object(map)) = definitions_val {
        for (name, val) in map {
            if defs.contains_key(&name) {
                continue;
            }
            let child = parse_node(val).map_err(|e| NormalizeError::ParseError(e.to_string()))?;
            let child_id = arena.alloc(child);
            arena[child_id].json_pointer = format!("/definitions/{}", name);
            arena[child_id].parent = Some(root_id);
            arena[child_id].depth = 1;
            arena[root_id].children.push(child_id);
            defs.insert(name, child_id);
        }
    }

    Ok(())
}
