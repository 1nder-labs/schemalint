/// Simple glob matcher for exclude patterns.
///
/// Handles `*` (match anything within a single path segment except `/`).
/// `**` is handled by the caller before this function is invoked.
/// `?` is not supported; use `*` instead.
///
/// # v1.0 glob scope
///
/// Exclude globs support `*` (segment-local wildcard) and leading/trailing `**`
/// (which the caller strips via `trim_start_matches("**/")` and
/// `strip_suffix("/**")`/`strip_suffix("/*")` before calling this function).
/// A **middle** `**` — e.g. `src/**/gen` — is **not** passed through the
/// caller's strip logic and reaches this function intact. Here `**` is split on
/// `*` into three parts `["", "", ""]` (for `**`) or treated as consecutive `*`
/// wildcards, each of which is segment-local and does **not** span path
/// separators. The failure mode is **over-inclusion**: a path that a full-glob
/// engine would exclude may pass through and be linted. This is intentional for
/// v1.0 — over-inclusion means a file gets linted (safe), never silently skipped,
/// and never causes a crash or an incorrect lint result. Extending to a full
/// recursive `**` matcher is tracked as a future enhancement.
///
/// # Contract
///
/// The pattern is split on `*` into literal parts.
///
/// - **No `*` (one part):** pure substring match — `path.contains(part)`.
///   This is unanchored so that e.g. `"node_modules"` matches `"pkg/node_modules/foo"`.
///
/// - **With `*`:** the literals must appear in order in `path` such that:
///   - The match is **unanchored at the start** — the first literal may begin at
///     any position (including after a `/`).
///   - A leading `*` (empty first part) lets the prefix before the first literal
///     contain anything, including `/`.
///   - Each `*` **between** two literals matches `[^/]*` — the gap between
///     consecutive literals must contain **no** `/` (segment-local).
///   - A non-empty **last** literal is **end-anchored**: `path` must end with it.
///   - A trailing `*` (empty last part) means no end anchor.
///
/// Because the first literal is unanchored and `*` is segment-local, the matcher
/// **backtracks**: it tries every occurrence of each literal, not just the first,
/// so a valid match at a later position is never missed.  Memoization prevents the
/// exponential blowup from pathological patterns like `a*a*a*…*a` against long
/// repeated strings — worst-case work is O(P × N²) where P is the number of parts
/// and N is the path length.
pub(crate) fn glob_match(pattern: &str, path: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 1 {
        // No wildcard — substring match (unanchored).
        return path.contains(parts[0]);
    }
    // Memoized backtracking matcher.  `slash_free` is false only for the
    // gap before the very first non-empty literal (unanchored start).
    let mut memo = std::collections::HashMap::new();
    match_parts(&parts, path, 0, 0, false, &mut memo)
}

/// Memoized backtracking helper for [`glob_match`].
///
/// Parameters are expressed as stable integer indices into the original slices
/// so that sub-problem keys are cheap and unambiguous:
///
/// - `parts_idx`   — index of the current literal in the `parts` array.
/// - `path_offset` — byte offset into `path` of the remaining suffix to match.
/// - `slash_free`  — `true` when the gap from `path_offset` to the next literal
///   must contain no `/` (i.e. after at least one real literal has been matched).
///
/// **Why `slash_free` is part of the memo key:**
/// The same `(parts_idx, path_offset)` can be reached with both `slash_free=false`
/// (through an empty/leading `*` that skips real literals) and `slash_free=true`
/// (after anchoring on a real literal).  These two states yield different matching
/// outcomes — `slash_free=false` allows `/` in the gap while `slash_free=true`
/// forbids it — so caching only `(parts_idx, path_offset)` would cause incorrect
/// results.  Including all three components in the key is required for correctness.
fn match_parts(
    parts: &[&str],
    path: &str,
    parts_idx: usize,
    path_offset: usize,
    slash_free: bool,
    memo: &mut std::collections::HashMap<(usize, usize, bool), bool>,
) -> bool {
    let key = (parts_idx, path_offset, slash_free);
    if let Some(&cached) = memo.get(&key) {
        return cached;
    }

    let result = if parts_idx == parts.len() - 1 {
        // Base case: last literal must end-anchor the remaining path suffix.
        let last = parts[parts_idx];
        if last.is_empty() {
            true // trailing '*' — no end anchor required
        } else {
            match path[path_offset..].strip_suffix(last) {
                // The prefix between path_offset and the final literal must not
                // contain a '/' if the previous gap was segment-local.
                Some(pre) => !slash_free || !pre.contains('/'),
                None => false,
            }
        }
    } else {
        let part = parts[parts_idx];
        if part.is_empty() {
            // Leading or consecutive '*' — skip this empty part and continue.
            // The slash_free constraint only activates after a real literal, so
            // keep the current flag when the star is merely vacuous.
            match_parts(parts, path, parts_idx + 1, path_offset, slash_free, memo)
        } else {
            // Try every occurrence of `part` in the remaining path suffix.
            // We must backtrack rather than greedily anchoring on the first hit,
            // because a later occurrence may be the one that satisfies the
            // remaining constraints.  `match_indices` yields offsets relative to
            // the suffix, so we add `path_offset` to get absolute byte offsets
            // (stable as memo keys and correct for slash-gap checks).
            let mut matched = false;
            for (rel_off, _) in path[path_offset..].match_indices(part) {
                let abs_off = path_offset + rel_off;
                if slash_free && path[path_offset..abs_off].contains('/') {
                    // All later occurrences will have even more '/' in the gap,
                    // so we can stop early.
                    break;
                }
                let after = abs_off + part.len();
                // After consuming this literal the next gap is segment-local.
                if match_parts(parts, path, parts_idx + 1, after, true, memo) {
                    matched = true;
                    break;
                }
            }
            matched
        }
    };

    memo.insert(key, result);
    result
}

#[cfg(test)]
mod glob_tests {
    use super::glob_match;

    // -----------------------------------------------------------------------
    // Contract-pinning tests: current-correct behavior BEFORE any logic change.
    // These must remain green after the fix — they document the intended
    // unanchored/substring semantics the caller depends on.
    // -----------------------------------------------------------------------

    /// No-wildcard core: must substring-match anywhere in the path.
    #[test]
    fn no_wildcard_substring_match() {
        assert!(glob_match("node_modules", "a/node_modules/b"));
        assert!(glob_match("node_modules", "node_modules"));
        assert!(glob_match("foo", "x/foo/y"));
        assert!(!glob_match("foo", "bar"));
        // "foo" IS a substring of "foobar" — substring semantics means this is true.
        assert!(glob_match("foo", "foobar"));
    }

    /// `*.ts` with a leading `*`: the star may consume the cross-segment prefix
    /// (because it's the FIRST wildcard — unanchored), but the literal after it
    /// must not be separated by another `/`.
    #[test]
    fn leading_star_ts_single_level() {
        // Leading `*` is unanchored — may span the prefix.
        assert!(glob_match("*.ts", "src/a/types.ts"));
        assert!(glob_match("*.ts", "types.ts"));
        // Multi-segment path still matches when the filename ends in .ts.
        assert!(glob_match("*.ts", "src/a/b/types.ts"));
    }

    /// A no-wildcard pattern is a raw substring match — even if the literal is
    /// embedded inside a longer word the match is true (unanchored semantics).
    #[test]
    fn no_wildcard_negative() {
        // "node_modules" IS a substring of "notnode_modules_here" — true.
        assert!(glob_match("node_modules", "notnode_modules_here/foo"));
        // Only false when the literal is genuinely absent.
        assert!(!glob_match("node_modules", "vendor/lodash/index.js"));
    }

    // -----------------------------------------------------------------------
    // Bug-fix tests: `src/*/types.ts` must not cross `/` between literal parts.
    // -----------------------------------------------------------------------

    /// `src/*/types.ts` should match exactly one segment between `src/` and `/types.ts`.
    #[test]
    fn star_does_not_cross_slash_positive() {
        // One segment between src/ and /types.ts — should match.
        assert!(glob_match("src/*/types.ts", "src/a/types.ts"));
        assert!(glob_match("src/*/types.ts", "src/models/types.ts"));
    }

    #[test]
    fn star_does_not_cross_slash_negative() {
        // Two segments between src/ and /types.ts — must NOT match.
        assert!(!glob_match("src/*/types.ts", "src/a/b/types.ts"));
        assert!(!glob_match("src/*/types.ts", "src/a/b/c/types.ts"));
    }

    /// `node_modules` core (no wildcard) still matches mid-path.
    #[test]
    fn node_modules_substring_still_works() {
        assert!(glob_match("node_modules", "pkg/node_modules/foo"));
        assert!(glob_match(
            "node_modules",
            "very/deep/pkg/node_modules/lodash/index.js"
        ));
    }

    /// `*.spec.ts` — leading star, two literal segments after it.
    #[test]
    fn star_spec_ts() {
        assert!(glob_match("*.spec.ts", "foo.spec.ts"));
        assert!(glob_match("*.spec.ts", "src/foo.spec.ts"));
        // The second literal `.ts` follows `.spec` with no `/` — OK.
        assert!(glob_match("*.spec.ts", "deep/a/b/foo.spec.ts"));
        // A slash between .spec and .ts would be weird but let's be explicit:
        assert!(!glob_match("*.spec.ts", "deep/foo.spec/bar.ts"));
    }

    // -----------------------------------------------------------------------
    // Regression tests: backtracking — first occurrence is not always correct.
    // -----------------------------------------------------------------------

    /// `a/*/b.ts` vs `a/x/a/y/b.ts`:
    ///
    /// The greedy (broken) matcher anchors `a/` at offset 0, sees gap `x/` which
    /// contains `/`, and returns false.  The correct backtracking matcher tries the
    /// second `a/` at offset 4, sees gap `y` (no `/`), end-anchors on `b.ts` — true.
    #[test]
    fn backtracking_second_occurrence() {
        assert!(glob_match("a/*/b.ts", "a/x/a/y/b.ts")); // REGRESSION FIX
                                                         // Sanity: two-segment gap still fails (x/y has a slash).
        assert!(!glob_match("a/*/b.ts", "a/x/y/b.ts"));
    }

    // -----------------------------------------------------------------------
    // Explicit pinning of the full spec table from the task description.
    // -----------------------------------------------------------------------

    #[test]
    fn spec_table_no_wildcard() {
        assert!(glob_match("node_modules", "a/node_modules/b"));
        assert!(glob_match("node_modules", "node_modules"));
        assert!(glob_match("node_modules", "notnode_modules_here/foo")); // substring
        assert!(!glob_match("node_modules", "vendor/lodash/index.js"));
    }

    #[test]
    fn spec_table_leading_star_ts() {
        assert!(glob_match("*.ts", "src/a/b/types.ts")); // leading * is unanchored
        assert!(glob_match("*.ts", "types.ts"));
    }

    #[test]
    fn spec_table_src_star_types() {
        assert!(glob_match("src/*/types.ts", "src/a/types.ts")); // single segment gap
        assert!(!glob_match("src/*/types.ts", "src/a/b/types.ts")); // two segments — fixed bug

        // `src/types.ts`: after anchoring the first literal `src/`, the gap before
        // `/types.ts` (note: the slash is part of that literal) is the empty string
        // `""`.  `"".strip_suffix("/types.ts")` is `None`, so the match fails.
        // `*` matching zero characters would require `src/` + `` + `/types.ts` =
        // `src//types.ts` which is NOT equal to `src/types.ts`.  Result: false.
        assert!(!glob_match("src/*/types.ts", "src/types.ts"));
    }

    /// Multibyte (UTF-8) path: must not panic and must return the correct result.
    #[test]
    fn multibyte_path_no_panic() {
        // café and 索引 are multibyte; match_indices/strip_suffix always land on
        // char boundaries, so this must not panic.
        assert!(glob_match("*.ts", "src/café/索引.ts"));
    }
}
