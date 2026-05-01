use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn diag_key(v: &serde_json::Value) -> (String, String) {
    (
        v["code"].as_str().unwrap_or("").to_string(),
        v["pointer"].as_str().unwrap_or("").to_string(),
    )
}

/// Compare two diagnostic arrays, ignoring `filePath` which depends on cwd.
fn diagnostics_match(actual: &[serde_json::Value], expected: &[serde_json::Value]) -> bool {
    if actual.len() != expected.len() {
        return false;
    }
    let mut a_sorted: Vec<_> = actual.to_vec();
    let mut e_sorted: Vec<_> = expected.to_vec();
    a_sorted.sort_by(|a, b| diag_key(a).cmp(&diag_key(b)));
    e_sorted.sort_by(|a, b| diag_key(a).cmp(&diag_key(b)));
    for (a, e) in a_sorted.iter().zip(e_sorted.iter()) {
        for field in ["code", "severity", "message", "pointer", "profile"] {
            if a.get(field) != e.get(field) {
                return false;
            }
        }
    }
    true
}

fn run_corpus(corpus_dir: &PathBuf, bin: &PathBuf, profile: &str, prefix: &str) -> Vec<String> {
    let mut schemas: Vec<PathBuf> = fs::read_dir(corpus_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            let name = p.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            p.extension().and_then(|s| s.to_str()) == Some("json") && name.starts_with(prefix)
        })
        .collect();
    schemas.sort();

    let mut failures = Vec::new();

    for schema_path in schemas {
        let expected_path = schema_path.with_extension("expected");
        if !expected_path.exists() {
            failures.push(format!(
                "{}: missing expected file",
                schema_path.file_name().unwrap().to_string_lossy()
            ));
            continue;
        }

        let output = Command::new(bin)
            .arg("check")
            .arg("--profile")
            .arg(profile)
            .arg("--format")
            .arg("json")
            .arg(&schema_path)
            .output()
            .expect("failed to run schemalint");

        let stdout = String::from_utf8(output.stdout).unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();
        let result: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
            panic!(
                "invalid JSON output for {} (exit={}):\nstdout: {}\nstderr: {}\nerror: {}",
                schema_path.display(),
                output.status.code().unwrap_or(-1),
                stdout,
                stderr,
                e
            )
        });

        let actual_diagnostics = result["diagnostics"].as_array().unwrap();
        let expected_raw = fs::read_to_string(&expected_path).unwrap();
        let expected_diagnostics: Vec<serde_json::Value> =
            serde_json::from_str(&expected_raw).unwrap();

        if !diagnostics_match(actual_diagnostics, &expected_diagnostics) {
            failures.push(format!(
                "{}: mismatch\n  expected: {}\n  actual: {}",
                schema_path.file_name().unwrap().to_string_lossy(),
                serde_json::to_string_pretty(&expected_diagnostics).unwrap(),
                serde_json::to_string_pretty(actual_diagnostics).unwrap()
            ));
        }
    }

    failures
}

#[test]
fn corpus_openai_schemas_match_expected() {
    let corpus_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/corpus");
    let bin = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/schemalint");

    let failures = run_corpus(&corpus_dir, &bin, "openai.so.2026-04-30", "schema_");

    if !failures.is_empty() {
        panic!(
            "{} OpenAI corpus schema(s) failed:\n\n{}",
            failures.len(),
            failures.join("\n\n")
        );
    }
}

#[test]
fn corpus_anthropic_schemas_match_expected() {
    let corpus_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/corpus");
    let bin = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/schemalint");

    let failures = run_corpus(&corpus_dir, &bin, "anthropic.so.2026-04-30", "ant_schema_");

    if !failures.is_empty() {
        panic!(
            "{} Anthropic corpus schema(s) failed:\n\n{}",
            failures.len(),
            failures.join("\n\n")
        );
    }
}
