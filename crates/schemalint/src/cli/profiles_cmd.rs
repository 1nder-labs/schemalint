use super::PROFILES;

/// Print the built-in profile table (`schemalint profiles`).
///
/// Purely informational — always succeeds.
pub(super) fn run_profiles() -> i32 {
    println!("Built-in schemalint profiles:");
    println!();
    for meta in PROFILES {
        let alias = format!("(alias: {})", meta.provider);
        println!("  {:<24} {:<19} {}", meta.id, alias, meta.description);
    }
    println!();
    println!("Pass --profile <id> (repeatable) to `check`, `check-node`, or `check-python`.");
    println!(
        "Bare provider names (\"openai\", \"anthropic\") and \"<provider>.so.latest\" both \
         resolve to the profile above."
    );
    println!(
        "If --profile is omitted, schemalint auto-detects the provider from package.json \
         dependencies, defaulting to openai.so.2026-04-30 when none is found."
    );
    0
}
