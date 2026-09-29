use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn skills_list_names_core() {
    Command::cargo_bin("schemalint")
        .unwrap()
        .arg("skills")
        .assert()
        .success()
        .stdout(predicate::str::starts_with("core "));
}

#[test]
fn skills_get_core_starts_with_frontmatter_and_full_appends_reference() {
    let plain = Command::cargo_bin("schemalint")
        .unwrap()
        .args(["skills", "get", "core"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("---\nname: core\n"))
        .get_output()
        .stdout
        .clone();
    Command::cargo_bin("schemalint")
        .unwrap()
        .args(["skills", "get", "core", "--full"])
        .assert()
        .success()
        .stdout(predicate::str::contains("## schemalint check-node"))
        .stdout(predicate::str::contains("--continue-on-discovery-error"));
    assert!(!String::from_utf8(plain)
        .unwrap()
        .contains("# Command reference"));
}

#[test]
fn skills_get_unknown_lists_valid_names() {
    Command::cargo_bin("schemalint")
        .unwrap()
        .args(["skills", "get", "nope"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("unknown skill 'nope'"))
        .stderr(predicate::str::contains("core"));
}

#[test]
fn root_help_starts_with_agent_pointer() {
    Command::cargo_bin("schemalint")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::starts_with(
            "Start here (for AI agents):\n  schemalint skills get core --full",
        ));
}
