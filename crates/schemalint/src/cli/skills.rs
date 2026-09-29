use clap::CommandFactory;

use crate::cli::args::{Cli, SkillsAction};

/// A skill bundled into the binary so it always matches the CLI version.
struct Skill {
    name: &'static str,
    body: &'static str,
}

const SKILLS: &[Skill] = &[Skill {
    name: "core",
    body: include_str!("../../skills/core.md"),
}];

/// Print the bundled skill list or one skill (`schemalint skills ...`).
pub(super) fn run_skills(action: Option<SkillsAction>) -> i32 {
    match action.unwrap_or(SkillsAction::List) {
        SkillsAction::List => {
            list();
            0
        }
        SkillsAction::Get { name, full } => get(&name, full),
    }
}

fn list() {
    for skill in SKILLS {
        println!("{:<8} {}", skill.name, summary(skill.body));
    }
}

fn get(name: &str, full: bool) -> i32 {
    let Some(skill) = SKILLS.iter().find(|skill| skill.name == name) else {
        let names: Vec<&str> = SKILLS.iter().map(|skill| skill.name).collect();
        eprintln!(
            "error: unknown skill '{name}'. Available skills: {}",
            names.join(", ")
        );
        return 1;
    };
    print!("{}", skill.body);
    if full {
        println!();
        print!("{}", command_reference());
    }
    0
}

/// First sentence of the frontmatter `description`, for the list view.
fn summary(body: &str) -> &str {
    let description = body
        .lines()
        .find_map(|line| line.strip_prefix("description: "))
        .unwrap_or("");
    description
        .split_once(". ")
        .map_or(description, |(first, _)| first)
}

/// Long help for every subcommand, rendered by clap so it cannot drift.
fn command_reference() -> String {
    let mut root = Cli::command();
    root.build();
    let mut out = String::from("---\n\n# Command reference\n");
    for sub in root.get_subcommands_mut() {
        if sub.get_name() == "help" {
            continue;
        }
        out.push_str(&format!("\n## schemalint {}\n\n", sub.get_name()));
        out.push_str(&sub.render_long_help().to_string());
    }
    out
}
