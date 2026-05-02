use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use clap::Parser;
use schemalint::profile;
use schemalint::rules::metadata::RuleCategory;
use schemalint::rules::{RuleSet, RULES};
use schemalint_profiles::{ANTHROPIC_SO_2026_04_30, OPENAI_SO_2026_04_30};

#[derive(Parser)]
#[command(name = "schemalint-docgen")]
struct Args {
    #[arg(long, default_value = "docs/book/src/rules")]
    output_dir: PathBuf,

    #[arg(long, default_value = "docs/book/src/SUMMARY.md")]
    summary_path: PathBuf,
}

/// Profile name + code prefix pair.
struct ProfileInfo {
    name: String,
    code_prefix: String,
}

/// Deduplicated rule entry across profiles.
#[derive(Debug, Clone)]
struct DedupedRule {
    name: String,
    description: String,
    rationale: String,
    severity: String,
    category: RuleCategory,
    bad_example: String,
    good_example: String,
    see_also: Vec<String>,
    /// Per-profile code expansions.
    profile_codes: Vec<(String, String)>,
    /// Profile names this rule is specific to (empty = universal).
    profiles: Vec<String>,
}

fn main() {
    let args = Args::parse();

    let profiles = load_profiles();
    let mut deduped: BTreeMap<String, DedupedRule> = BTreeMap::new();

    // Collect static rules from linkme slice.
    for rule in RULES {
        if let Some(metadata) = rule.metadata() {
            let entry = DedupedRule {
                name: metadata.name.clone(),
                description: metadata.description.clone(),
                rationale: metadata.rationale.clone(),
                severity: format!("{:?}", metadata.severity),
                category: metadata.category,
                bad_example: metadata.bad_example.clone(),
                good_example: metadata.good_example.clone(),
                see_also: metadata.see_also.clone(),
                profile_codes: profiles
                    .iter()
                    .map(|p| (p.name.clone(), expand_code(&metadata.code, &p.code_prefix)))
                    .collect(),
                profiles: Vec::new(),
            };
            deduped.insert(metadata.name.clone(), entry);
        }
    }

    // Collect dynamic rules from each profile.
    for profile_info in &profiles {
        let profile = load_toml_profile(profile_info);
        let rule_set = RuleSet::from_profile(&profile);
        for rule in rule_set.dynamic_rules() {
            let Some(metadata) = rule.metadata() else {
                continue;
            };
            deduped
                .entry(metadata.name.clone())
                .and_modify(|entry| {
                    entry.profile_codes.push((
                        profile_info.name.clone(),
                        expand_code(&metadata.code, &profile_info.code_prefix),
                    ));
                    if let Some(p) = &metadata.profile {
                        if !entry.profiles.contains(p) {
                            entry.profiles.push(p.clone());
                        }
                    }
                })
                .or_insert_with(|| DedupedRule {
                    name: metadata.name.clone(),
                    description: metadata.description.clone(),
                    rationale: metadata.rationale.clone(),
                    severity: format!("{:?}", metadata.severity),
                    category: metadata.category,
                    bad_example: metadata.bad_example.clone(),
                    good_example: metadata.good_example.clone(),
                    see_also: metadata.see_also.clone(),
                    profile_codes: vec![(
                        profile_info.name.clone(),
                        expand_code(&metadata.code, &profile_info.code_prefix),
                    )],
                    profiles: metadata.profile.into_iter().collect(),
                });
        }
    }

    // Write output.
    create_output(&args.output_dir, &args.summary_path, &deduped, &profiles);
}

fn load_profiles() -> Vec<ProfileInfo> {
    vec![
        ProfileInfo {
            name: "openai.so.2026-04-30".into(),
            code_prefix: "OAI".into(),
        },
        ProfileInfo {
            name: "anthropic.so.2026-04-30".into(),
            code_prefix: "ANT".into(),
        },
    ]
}

fn load_toml_profile(info: &ProfileInfo) -> schemalint::Profile {
    let toml_str = if info.name.starts_with("openai") {
        OPENAI_SO_2026_04_30
    } else if info.name.starts_with("anthropic") {
        ANTHROPIC_SO_2026_04_30
    } else {
        ""
    };
    profile::load(toml_str.as_bytes())
        .unwrap_or_else(|e| panic!("failed to load profile {}: {e}", info.name))
}

fn expand_code(template: &str, prefix: &str) -> String {
    template.replace("{prefix}", prefix)
}

fn create_output(
    output_dir: &Path,
    summary_path: &Path,
    rules: &BTreeMap<String, DedupedRule>,
    profiles: &[ProfileInfo],
) {
    // Create category directories.
    for cat in &[
        RuleCategory::Keyword,
        RuleCategory::Restriction,
        RuleCategory::Structural,
        RuleCategory::Semantic,
    ] {
        fs::create_dir_all(output_dir.join(cat.as_str())).unwrap_or_else(|e| {
            eprintln!("warning: {e}");
        });
    }

    // Write index page.
    let index_md = build_index(rules, profiles);
    fs::write(output_dir.join("index.md"), &index_md)
        .unwrap_or_else(|e| eprintln!("warning: failed to write index.md: {e}"));

    // Write per-rule pages.
    for rule in rules.values() {
        let page = build_rule_page(rule, profiles);
        let dir = output_dir.join(rule.category.as_str());
        let path = dir.join(format!("{}.md", rule.name));
        fs::write(&path, &page)
            .unwrap_or_else(|e| eprintln!("warning: failed to write {}: {e}", path.display()));
    }

    // Update SUMMARY.md with rule links.
    inject_summary_rules(summary_path, rules);
}

fn build_index(rules: &BTreeMap<String, DedupedRule>, profiles: &[ProfileInfo]) -> String {
    let mut out = String::from("# Rule Reference\n\n");
    out.push_str("This page lists all lint rules grouped by category.\n\n");

    out.push_str("| Rule | Category | Severity | ");
    for p in profiles {
        out.push_str(&format!("{} | ", p.name));
    }
    out.push_str("\n|------|----------|----------|");
    for _ in profiles {
        out.push_str("------|");
    }
    out.push('\n');

    for rule in rules.values() {
        out.push_str(&format!(
            "| [{}](./{}/{}.md) | {} | {} | ",
            rule.name,
            rule.category.as_str(),
            rule.name,
            rule.category.as_str(),
            rule.severity
        ));
        for p in profiles {
            let code = rule
                .profile_codes
                .iter()
                .find(|(pn, _)| pn == &p.name)
                .map(|(_, c)| c.as_str())
                .unwrap_or("—");
            out.push_str(&format!("`{code}` | "));
        }
        out.push('\n');
    }
    out
}

fn build_rule_page(rule: &DedupedRule, _profiles: &[ProfileInfo]) -> String {
    let mut out = format!("# {}\n\n", rule.name);
    out.push_str("> Category: ");

    let cat_label = match rule.category {
        RuleCategory::Keyword => {
            "**Keyword** — presence of a specific JSON Schema keyword triggers this rule"
        }
        RuleCategory::Restriction => {
            "**Restriction** — a keyword value outside the allowed set triggers this rule"
        }
        RuleCategory::Structural => "**Structural** — overall schema structure triggers this rule",
        RuleCategory::Semantic => "**Semantic** — schema semantics trigger this rule",
    };
    out.push_str(cat_label);
    out.push('\n');

    out.push_str("\n## Error Codes\n\n");
    out.push_str("| Profile | Code |\n|---------|------|\n");
    for (profile_name, code) in &rule.profile_codes {
        out.push_str(&format!("| {} | `{}` |\n", profile_name, code));
    }

    out.push_str("\n## Description\n\n");
    out.push_str(&rule.description);
    out.push_str("\n\n");

    out.push_str("## Rationale\n\n");
    out.push_str(&rule.rationale);
    out.push_str("\n\n");

    if !rule.see_also.is_empty() {
        out.push_str("## See Also\n\n");
        for link in &rule.see_also {
            out.push_str(&format!("- {link}\n"));
        }
        out.push('\n');
    }

    out.push_str("## Bad Example\n\n");
    out.push_str("```json\n");
    out.push_str(&rule.bad_example);
    out.push_str("\n```\n\n");

    out.push_str("## Good Example\n\n");
    out.push_str("```json\n");
    out.push_str(&rule.good_example);
    out.push_str("\n```\n");

    out
}

fn inject_summary_rules(summary_path: &Path, rules: &BTreeMap<String, DedupedRule>) {
    let old_content = fs::read_to_string(summary_path).unwrap_or_default();

    let marker_start = "<!-- AUTO-GENERATED RULES -->";
    let marker_end = "<!-- END AUTO-GENERATED RULES -->";

    let (prefix, _old_rules_section) = match old_content.split_once(marker_start) {
        Some((pre, _)) => (pre.to_string(), String::new()),
        None => (old_content.clone(), String::new()),
    };

    let suffix = old_content
        .split_once(marker_end)
        .map(|(_, s)| s.to_string())
        .unwrap_or_default();

    let mut new_content = prefix;
    if !new_content.ends_with('\n') {
        new_content.push('\n');
    }
    new_content.push_str(marker_start);
    new_content.push('\n');

    // Generate rules section sorted by category then name.
    let categories = [
        ("Keyword Rules", RuleCategory::Keyword),
        ("Restriction Rules", RuleCategory::Restriction),
        ("Structural Rules", RuleCategory::Structural),
        ("Semantic Rules", RuleCategory::Semantic),
    ];

    new_content.push_str("- [Rule Reference](./rules/index.md)\n");

    for (heading, cat) in &categories {
        let cat_rules: Vec<_> = rules.values().filter(|r| r.category == *cat).collect();
        if cat_rules.is_empty() {
            continue;
        }
        new_content.push_str(&format!("  - [{heading}]()\n"));
        for rule in &cat_rules {
            new_content.push_str(&format!(
                "    - [{}](./rules/{}/{}.md)\n",
                rule.name,
                cat.as_str(),
                rule.name
            ));
        }
    }

    new_content.push_str(marker_end);
    new_content.push('\n');

    // Preserve anything after the end marker in the original.
    new_content.push_str(&suffix);

    fs::write(summary_path, &new_content)
        .unwrap_or_else(|e| eprintln!("warning: failed to write SUMMARY.md: {e}"));
}
