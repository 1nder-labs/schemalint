use crate::ir::{Arena, NodeId};
use crate::normalize::NormalizeError;
use indexmap::IndexMap;
use std::collections::HashSet;

/// Resolve internal `$ref` strings to `NodeId` targets.
///
/// Phase 1 only resolves refs pointing directly to `$defs` or `definitions`
/// entries. Any other internal ref pattern is a fatal error.
/// External refs (http://, https://, absolute paths) are left unresolved and
/// will be caught by structural rules (U6).
pub fn resolve_refs(
    arena: &mut Arena,
    defs: &IndexMap<String, NodeId>,
) -> Result<Vec<(NodeId, NodeId)>, NormalizeError> {
    // Collect ref strings first to avoid borrow issues.
    let refs: Vec<(NodeId, String)> = arena
        .iter()
        .filter_map(|(id, node)| {
            node.annotations
                .r#ref
                .as_ref()
                .and_then(|v| v.as_str().map(|s| (id, s.to_string())))
        })
        .collect();

    let mut edges = Vec::new();

    for (node_id, ref_str) in &refs {
        if let Some(target) = resolve_ref_string(ref_str, defs)? {
            arena[*node_id].ref_target = Some(target);
            edges.push((*node_id, target));
        }
    }

    // Check for unresolved internal refs and report fatal errors.
    for (node_id, ref_str) in &refs {
        if arena[*node_id].ref_target.is_none()
            && !ref_str.starts_with("http://")
            && !ref_str.starts_with("https://")
            && !ref_str.starts_with('/')
        {
            return Err(NormalizeError::UnresolvedRef(ref_str.clone()));
        }
    }

    Ok(edges)
}

fn resolve_ref_string(
    ref_str: &str,
    defs: &IndexMap<String, NodeId>,
) -> Result<Option<NodeId>, NormalizeError> {
    // Strip the fragment prefix.
    let pointer = ref_str.strip_prefix('#').unwrap_or(ref_str);
    // Decode percent-encoded segments (e.g. %24 -> $).
    let decoded = percent_encoding::percent_decode_str(pointer)
        .decode_utf8()
        .map_err(|e| {
            NormalizeError::ParseError(format!("invalid percent-encoding in $ref: {e}"))
        })?;
    let decoded = decoded.as_ref();

    // Internal refs to $defs.
    if let Some(name) = decoded.strip_prefix("/$defs/") {
        return Ok(defs.get(name).copied());
    }
    // Internal refs to definitions (Draft 7).
    if let Some(name) = decoded.strip_prefix("/definitions/") {
        return Ok(defs.get(name).copied());
    }
    // External refs — not resolved in Phase 1.
    if ref_str.starts_with("http://") || ref_str.starts_with("https://") || ref_str.starts_with('/')
    {
        return Ok(None);
    }
    // Any other internal ref pattern is treated as unresolved.
    Ok(None)
}

/// Build transitive ref edges for cycle detection.
///
/// For every `$ref` node, emit an edge from the **schema node that owns the
/// reference** to the ref target.
///
/// - If the `$ref` node itself is a `$defs` entry, the edge is from that node.
/// - Otherwise the edge is from the `$ref` node's parent (the containing schema).
///
/// This captures the logical dependency needed for Tarjan SCC.
pub fn transitive_ref_edges(
    arena: &Arena,
    defs: &IndexMap<String, NodeId>,
) -> Vec<(NodeId, NodeId)> {
    let def_ids: HashSet<NodeId> = defs.values().copied().collect();
    let mut edges = Vec::new();
    for (node_id, node) in arena.iter() {
        if let Some(target) = node.ref_target {
            let source = if def_ids.contains(&node_id) {
                node_id
            } else if let Some(parent_id) = node.parent {
                parent_id
            } else {
                node_id
            };
            edges.push((source, target));
        }
    }
    edges
}

// ---------------------------------------------------------------------------
// Tarjan SCC
// ---------------------------------------------------------------------------

/// Run Tarjan's strongly-connected-components algorithm on the `$ref` graph
/// and mark every node that participates in a cycle.
pub fn tarjan_scc(arena: &mut Arena, edges: &[(NodeId, NodeId)]) {
    let n = arena.len();
    if n == 0 {
        return;
    }

    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (u, v) in edges {
        adj[u.0 as usize].push(v.0 as usize);
    }

    let mut index = 0usize;
    let mut stack = Vec::new();
    let mut on_stack = vec![false; n];
    let mut indices = vec![None; n];
    let mut lowlinks = vec![0usize; n];
    let mut sccs: Vec<Vec<usize>> = Vec::new();

    for v in 0..n {
        if indices[v].is_none() {
            strongconnect(
                v,
                &adj,
                &mut index,
                &mut stack,
                &mut on_stack,
                &mut indices,
                &mut lowlinks,
                &mut sccs,
            );
        }
    }

    // Mark nodes in SCCs of size > 1 or with self-loops.
    for component in &sccs {
        let size = component.len();
        let has_self_loop = size == 1 && adj[component[0]].contains(&component[0]);
        if size > 1 || has_self_loop {
            for &node_idx in component {
                arena[NodeId(node_idx as u32)].is_cyclic = true;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn strongconnect(
    v: usize,
    adj: &[Vec<usize>],
    index: &mut usize,
    stack: &mut Vec<usize>,
    on_stack: &mut [bool],
    indices: &mut [Option<usize>],
    lowlinks: &mut [usize],
    sccs: &mut Vec<Vec<usize>>,
) {
    indices[v] = Some(*index);
    lowlinks[v] = *index;
    *index += 1;
    stack.push(v);
    on_stack[v] = true;

    for &w in &adj[v] {
        if indices[w].is_none() {
            strongconnect(w, adj, index, stack, on_stack, indices, lowlinks, sccs);
            lowlinks[v] = lowlinks[v].min(lowlinks[w]);
        } else if on_stack[w] {
            lowlinks[v] = lowlinks[v].min(indices[w].unwrap());
        }
    }

    if lowlinks[v] == indices[v].unwrap() {
        let mut component = Vec::new();
        loop {
            let w = stack.pop().unwrap();
            on_stack[w] = false;
            component.push(w);
            if w == v {
                break;
            }
        }
        sccs.push(component);
    }
}
