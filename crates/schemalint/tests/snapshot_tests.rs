use assert_cmd::Command;
use schemalint::cli::emit_gha::emit_gha_to_string;
use schemalint::cli::emit_human::emit_human_to_string;
use schemalint::cli::emit_json::emit_json_to_string;
use schemalint::cli::emit_junit::emit_junit_to_string;
use schemalint::cli::emit_sarif::emit_sarif_to_string;
use schemalint::rules::registry::{DiagnosticSeverity, SourceSpan};
use schemalint::rules::Diagnostic;
use std::fs;

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

/// Replace temp directory paths and non-deterministic durations in output with stable placeholders.
fn normalize_temp_paths(output: &str, temp_dir: &std::path::Path) -> String {
    let mut out = output.replace(&temp_dir.to_string_lossy().to_string(), "[TEMP_DIR]");
    // Strip human footer duration: " in 0ms" -> " in [DURATION]ms"
    let re_human = regex::Regex::new(r" in \d+ms").unwrap();
    out = re_human.replace_all(&out, " in [DURATION]ms").to_string();
    // Strip JSON duration_ms
    let re_json = regex::Regex::new("duration_ms\": [0-9]+").unwrap();
    out = re_json
        .replace_all(&out, "duration_ms\": [DURATION]")
        .to_string();
    out
}

macro_rules! assert_snapshot_stable {
    ($value:expr) => {{
        let mut settings = insta::Settings::clone_current();
        settings.set_snapshot_path(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots"),
        );
        settings.bind(|| {
            insta::assert_snapshot!($value);
        });
    }};
}

// ---------------------------------------------------------------------------
// Snapshot: human output
// ---------------------------------------------------------------------------

// ===========================================================================
// Direct emitter coverage tests — U2 (Phase 6)
// ===========================================================================

fn diag(
    code: &str,
    severity: DiagnosticSeverity,
    message: &str,
    pointer: &str,
    source: Option<SourceSpan>,
    profile: &str,
    hint: Option<&str>,
) -> Diagnostic {
    Diagnostic {
        code: code.to_string(),
        severity,
        message: message.to_string(),
        pointer: pointer.to_string(),
        source,
        profile: profile.to_string(),
        hint: hint.map(|s| s.to_string()),
    }
}

fn src_span(file: &str, line: Option<u32>, col: Option<u32>) -> Option<SourceSpan> {
    Some(SourceSpan {
        file: file.to_string(),
        line,
        col,
    })
}

include!("snapshot_tests/part_01.rs");
include!("snapshot_tests/part_02.rs");
include!("snapshot_tests/part_03.rs");
