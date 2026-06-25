use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use crate::cli::args::{CheckNodeArgs, OutputFormat};
use crate::cli::node_config;
use crate::cli::pipeline::{aggregate_results, attach_source_spans, emit_output, process_schemas};
use crate::rules::registry::RuleSet;

use super::{load_profiles_from_ids, ANTHROPIC_PROFILE_ID, OPENAI_PROFILE_ID};

pub(super) fn run_check_node(args: CheckNodeArgs) -> i32 {
    let start = std::time::Instant::now();

    // -------------------------------------------------------------------
    // 1. Load package.json configuration
    // -------------------------------------------------------------------
    let config_path = args
        .config
        .as_deref()
        .unwrap_or_else(|| Path::new("package.json"));
    let node_config = match node_config::load_node_config(config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {}", e);
            return 1;
        }
    };

    // -------------------------------------------------------------------
    // 2. Merge CLI flags on top of config
    // -------------------------------------------------------------------
    let sources = if args.sources.is_empty() {
        node_config
            .as_ref()
            .map(|c| c.include.clone())
            .unwrap_or_default()
    } else {
        args.sources.clone()
    };

    let mut profile_args: Vec<String> = if args.profiles.is_empty() {
        node_config
            .as_ref()
            .map(|c| c.profiles.clone())
            .unwrap_or_default()
    } else {
        args.profiles
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect()
    };

    let exclude_globs: Vec<String> = node_config
        .as_ref()
        .map(|c| c.exclude.clone())
        .unwrap_or_default();

    if sources.is_empty() {
        eprintln!(
            "error: no sources specified. Use --source or configure \"schemalint\" in package.json"
        );
        return 1;
    }

    let explicit_profiles = if profile_args.is_empty() {
        None
    } else {
        match load_profiles_from_ids(&profile_args) {
            Ok(profiles) => Some(profiles),
            Err(e) => {
                eprintln!("error: {}", e);
                return 1;
            }
        }
    };

    // -------------------------------------------------------------------
    // 3. Determine output format
    // -------------------------------------------------------------------
    let format = args.format.unwrap_or_else(|| {
        if std::io::stdout().is_terminal() {
            OutputFormat::Human
        } else {
            OutputFormat::Json
        }
    });

    // -------------------------------------------------------------------
    // 4. Spawn Node helper and discover schemas
    // -------------------------------------------------------------------
    let mut helper = match crate::node::NodeHelper::spawn(args.node_path.as_deref()) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("error: {}", e);
            return 1;
        }
    };

    let mut discovered_models: Vec<crate::ingest::DiscoveredModel> = Vec::new();
    let mut discovery_failures = 0usize;
    let mut provider_hint: Option<String> = None;
    for source in &sources {
        match helper.discover(source) {
            Ok(resp) => {
                if provider_hint.is_none() {
                    provider_hint = resp.provider_hint.clone();
                }
                for model in resp.models {
                    discovered_models.push(model);
                }
                // Log discovery warnings
                for warning in &resp.warnings {
                    eprintln!(
                        "warning: discovery warning for '{}' in source '{}': {}",
                        warning.model, source, warning.message
                    );
                }
            }
            Err(e) => {
                eprintln!("error: discovery failed for source '{}': {}", source, e);
                discovery_failures += 1;
            }
        }
    }

    // Apply exclude patterns
    if !exclude_globs.is_empty() {
        discovered_models.retain(|m| {
            !exclude_globs.iter().any(|g| {
                let core = g.trim_start_matches("**/");
                let core = core
                    .strip_suffix("/**")
                    .or_else(|| core.strip_suffix("/*"))
                    .unwrap_or(core);
                glob_match(core, &m.module_path)
            })
        });
    }

    let total_discovered = discovered_models.len();
    if total_discovered == 0 {
        eprintln!("warning: no Zod schemas discovered in source globs");
    } else {
        eprintln!(
            "info: discovered {} Zod schema(s) in {} source glob(s)",
            total_discovered,
            sources.len()
        );
    }

    helper.shutdown();

    if discovered_models.is_empty() && discovery_failures > 0 {
        // If no profiles configured yet, show the profiles error instead of
        // the generic discovery failure — the user may have forgotten to
        // configure profiles/packages.json.
        if profile_args.is_empty() {
            eprintln!(
                "error: no profiles specified. Use --profile or configure \"schemalint\" in package.json"
            );
            return 1;
        }
        eprintln!(
            "error: all {} source(s) failed discovery",
            discovery_failures
        );
        return 1;
    }

    // -------------------------------------------------------------------
    // 5. Auto-detect profile from provider_hint if none specified
    // -------------------------------------------------------------------
    if profile_args.is_empty() {
        if let Some(ref hint) = provider_hint {
            let resolved = match hint.as_str() {
                "openai" => OPENAI_PROFILE_ID.to_string(),
                "anthropic" => ANTHROPIC_PROFILE_ID.to_string(),
                other => {
                    eprintln!("error: unknown provider hint '{}' from source files", other);
                    return 1;
                }
            };
            eprintln!(
                "info: auto-detected provider '{}' from source imports → using profile '{}'",
                hint, resolved
            );
            profile_args.push(resolved);
        } else {
            eprintln!(
                "error: no profiles specified. Use --profile or configure \"schemalint\" in package.json"
            );
            return 1;
        }
    }
    let profiles = match explicit_profiles {
        Some(profiles) => profiles,
        None => match load_profiles_from_ids(&profile_args) {
            Ok(profiles) => profiles,
            Err(e) => {
                eprintln!("error: {}", e);
                return 1;
            }
        },
    };

    let profile_rulesets: Vec<(&crate::profile::Profile, RuleSet)> = profiles
        .iter()
        .map(|p| (p, RuleSet::from_profile(p)))
        .collect();

    let profile_names: Vec<String> = profiles.iter().map(|p| p.name.clone()).collect();

    // -------------------------------------------------------------------
    // 6. Normalize and check schemas
    // -------------------------------------------------------------------
    let schema_entries: Vec<(PathBuf, String, serde_json::Value)> = discovered_models
        .iter()
        .map(|m| {
            (
                PathBuf::from(&m.module_path),
                m.name.clone(),
                m.schema.clone(),
            )
        })
        .collect();

    let results = process_schemas(schema_entries, &profile_rulesets);

    // -------------------------------------------------------------------
    // 7. Attach source spans from discovery
    // -------------------------------------------------------------------
    let all_diagnostics = attach_source_spans(results, &discovered_models);

    // -------------------------------------------------------------------
    // 8. Aggregate results
    // -------------------------------------------------------------------
    let (all_diagnostics, total_errors, total_warnings) = aggregate_results(all_diagnostics);

    // -------------------------------------------------------------------
    // 9. Emit output
    // -------------------------------------------------------------------
    let duration_ms = Some(start.elapsed().as_millis() as u64);
    if let Err(exit_code) = emit_output(
        format,
        &all_diagnostics,
        total_errors,
        total_warnings,
        &profile_names,
        duration_ms,
        args.output.as_deref(),
    ) {
        return exit_code;
    }

    if total_errors > 0 || discovery_failures > 0 {
        1
    } else {
        0
    }
}

/// Simple glob matcher for exclude patterns.
///
/// Handles `*` (match anything within a single path segment except `/`).
/// `**` is handled by the caller before this function is invoked.
/// `?` is not supported; use `*` instead.
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
/// so a valid match at a later position is never missed.
fn glob_match(pattern: &str, path: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 1 {
        // No wildcard — substring match (unanchored).
        return path.contains(parts[0]);
    }
    // Recursive backtracking matcher.  `slash_free` is false only for the
    // gap before the very first non-empty literal (unanchored start).
    match_parts(&parts, path, false)
}

/// Recursive backtracking helper for [`glob_match`].
///
/// `parts` is the remaining slice of literals split from the pattern on `*`.
/// `path`  is the remaining suffix of the original path being matched.
/// `slash_free` is `true` when the gap between the previous literal and the
/// next must not contain a `/` (i.e. after the first non-empty literal has
/// been consumed).
fn match_parts(parts: &[&str], path: &str, slash_free: bool) -> bool {
    if parts.len() == 1 {
        // Base case: last literal must end-anchor the path.
        let last = parts[0];
        if last.is_empty() {
            return true; // trailing '*' — no end anchor required
        }
        return match path.strip_suffix(last) {
            // The prefix between where we are and the final literal must not
            // contain a '/' if the previous gap was segment-local.
            Some(pre) => !slash_free || !pre.contains('/'),
            None => false,
        };
    }

    let part = parts[0];
    if part.is_empty() {
        // Leading or consecutive '*' — skip this empty part and continue.
        // The slash_free constraint only activates after a real literal, so
        // keep the current flag when the star is merely vacuous.
        return match_parts(&parts[1..], path, slash_free);
    }

    // Try every occurrence of `part` in `path`.  We must backtrack rather than
    // greedily anchoring on the first hit, because a later occurrence may be
    // the one that satisfies the remaining constraints.
    for (off, _) in path.match_indices(part) {
        if slash_free && path[..off].contains('/') {
            // All later occurrences will have even more '/' in the gap, so we
            // can stop early.
            break;
        }
        let after = off + part.len();
        // After consuming this literal the next gap is segment-local.
        if match_parts(&parts[1..], &path[after..], true) {
            return true;
        }
    }
    false
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
