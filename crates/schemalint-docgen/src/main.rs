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
    #[arg(long, default_value = "docs/docs/rules")]
    output_dir: PathBuf,
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
    create_output(&args.output_dir, &deduped, &profiles);
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
    rules: &BTreeMap<String, DedupedRule>,
    profiles: &[ProfileInfo],
) {
    // Create category directories and _category_.json for Docusaurus.
    let categories = [
        ("Keyword Rules", RuleCategory::Keyword, 1),
        ("Restriction Rules", RuleCategory::Restriction, 2),
        ("Structural Rules", RuleCategory::Structural, 3),
        ("Semantic Rules", RuleCategory::Semantic, 4),
    ];

    for &(label, cat, position) in &categories {
        let dir = output_dir.join(cat.as_str());
        fs::create_dir_all(&dir).unwrap_or_else(|e| {
            eprintln!("warning: {e}");
        });

        // Write _category_.json for Docusaurus sidebar auto-generation.
        let category_json = serde_json::json!({
            "label": label,
            "position": position,
        });
        let json_path = dir.join("_category_.json");
        fs::write(
            &json_path,
            serde_json::to_string_pretty(&category_json).unwrap(),
        )
        .unwrap_or_else(|e| eprintln!("warning: failed to write {}: {e}", json_path.display()));
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
