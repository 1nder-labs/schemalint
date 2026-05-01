use crate::ir::{Arena, NodeId};
use crate::normalize::NormalizeError;
use indexmap::IndexMap;

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
        if let Some(target) = resolve_ref_string(ref_str, defs) {
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

fn resolve_ref_string(ref_str: &str, defs: &IndexMap<String, NodeId>) -> Option<NodeId> {
    // Strip the fragment prefix.
    let pointer = ref_str.strip_prefix('#').unwrap_or(ref_str);
    // Decode percent-encoded segments (e.g. %24 -> $).
    let decoded = percent_encoding::percent_decode_str(pointer).decode_utf8_lossy();
    let decoded = decoded.as_ref();

    // Internal refs to $defs.
    if let Some(name) = decoded.strip_prefix("/$defs/") {
        return defs.get(name).copied();
    }
    // Internal refs to definitions (Draft 7).
    if let Some(name) = decoded.strip_prefix("/definitions/") {
        return defs.get(name).copied();
    }
    // External refs — not resolved in Phase 1.
    if ref_str.starts_with("http://") || ref_str.starts_with("https://") || ref_str.starts_with('/')
    {
        return None;
    }
    // Any other internal ref pattern is treated as unresolved.
    None
}

/// Build transitive ref edges for cycle detection.
///
/// For every `$ref` node, emit an edge from its **parent** (the schema node
/// that contains the `$ref`) to the ref target. This captures the logical
/// dependency needed for Tarjan SCC.
pub fn transitive_ref_edges(arena: &Arena) -> Vec<(NodeId, NodeId)> {
    let mut edges = Vec::new();
    for (node_id, node) in arena.iter() {
        if let Some(target) = node.ref_target {
            if let Some(parent_id) = node.parent {
                edges.push((parent_id, target));
            } else {
                edges.push((node_id, target));
            }
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
            );
        }
    }

    // Mark nodes in SCCs of size > 1 or with self-loops.
    let mut visited = vec![false; n];
    for v in 0..n {
        if !visited[v] {
            let mut comp = Vec::new();
            collect_component(v, &adj, &lowlinks, &indices, &mut visited, &mut comp);
            let size = comp.len();
            let has_self_loop = size == 1 && adj[v].contains(&v);
            if size > 1 || has_self_loop {
                for &node_idx in &comp {
                    arena[NodeId(node_idx as u32)].is_cyclic = true;
                }
            }
        }
    }
}

fn strongconnect(
    v: usize,
    adj: &[Vec<usize>],
    index: &mut usize,
    stack: &mut Vec<usize>,
    on_stack: &mut [bool],
    indices: &mut [Option<usize>],
    lowlinks: &mut [usize],
) {
    indices[v] = Some(*index);
    lowlinks[v] = *index;
    *index += 1;
    stack.push(v);
    on_stack[v] = true;

    for &w in &adj[v] {
        if indices[w].is_none() {
            strongconnect(w, adj, index, stack, on_stack, indices, lowlinks);
            lowlinks[v] = lowlinks[v].min(lowlinks[w]);
        } else if on_stack[w] {
            lowlinks[v] = lowlinks[v].min(indices[w].unwrap());
        }
    }

    if lowlinks[v] == indices[v].unwrap() {
        loop {
            let w = stack.pop().unwrap();
            on_stack[w] = false;
            if w == v {
                break;
            }
        }
    }
}

fn collect_component(
    v: usize,
    adj: &[Vec<usize>],
    lowlinks: &[usize],
    indices: &[Option<usize>],
    visited: &mut [bool],
    comp: &mut Vec<usize>,
) {
    visited[v] = true;
    comp.push(v);
    for &w in &adj[v] {
        if !visited[w] && lowlinks[w] == lowlinks[v] && indices[w].is_some() && indices[v].is_some()
        {
            collect_component(w, adj, lowlinks, indices, visited, comp);
        }
    }
}
