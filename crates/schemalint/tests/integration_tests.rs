use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn minimal_profile() -> &'static str {
    r##"
name = "test"
version = "1.0"

[structural]
require_object_root = false
"##
}

fn profile_with_forbid_allof() -> &'static str {
    r##"
name = "test"
version = "1.0"
allOf = "forbid"

[structural]
require_object_root = false
"##
}

fn cmd() -> Command {
    Command::cargo_bin("schemalint").unwrap()
}

// ---------------------------------------------------------------------------
// Happy path
// ---------------------------------------------------------------------------

include!("integration_tests/part_01.rs");
include!("integration_tests/part_02.rs");
