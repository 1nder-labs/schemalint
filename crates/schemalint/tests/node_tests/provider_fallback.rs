use super::*;

const PLAIN_SCHEMA: (&str, &str) = (
    "schema.ts",
    r#"import { z } from "zod";
export const Plain = z.object({ name: z.string() });
"#,
);

fn run_plain(deps: Option<&str>) -> (std::process::Output, serde_json::Value) {
    let tmp = TempDir::new().unwrap();
    setup_ts_project(tmp.path(), &[PLAIN_SCHEMA]);
    if let Some(deps) = deps {
        fs::write(tmp.path().join("package.json"), deps).unwrap();
    }
    let output = Command::cargo_bin("schemalint")
        .unwrap()
        .current_dir(tmp.path())
        .args(["check-node", "-S", "src/**/*.ts", "-f", "json"])
        .output()
        .unwrap();
    let out = serde_json::from_slice(&output.stdout).unwrap();
    (output, out)
}

/// A schema with no provider signal falls back to the package.json-detected
/// provider.
#[test]
fn provider_falls_back_to_package_json_dependency() {
    let (output, out) = run_plain(Some(
        r#"{"dependencies":{"zod":"^3.23.0","openai":"^4.0.0"}}"#,
    ));
    assert_eq!(output.status.code(), Some(0), "{out}");
    assert_eq!(out["report"]["coverage"]["status"], "complete");
    assert_eq!(out["profiles"][0], "openai.so.2026-04-30");
}

#[test]
fn provider_fallback_selects_anthropic_from_package_json() {
    let (output, out) = run_plain(Some(
        r#"{"dependencies":{"zod":"^3.23.0","@anthropic-ai/sdk":"^0.30.0"}}"#,
    ));
    assert_eq!(output.status.code(), Some(0), "{out}");
    assert_eq!(
        out["report"]["targets"][0]["effective_profiles"][0],
        "anthropic.so.2026-04-30"
    );
}

/// No package.json signal at all: fall through to the openai default.
#[test]
fn provider_falls_back_to_openai_without_deps() {
    let (output, out) = run_plain(None);
    assert_eq!(output.status.code(), Some(0), "{out}");
    assert_eq!(
        out["report"]["targets"][0]["effective_profiles"][0],
        "openai.so.2026-04-30"
    );
}

/// Both providers in package.json and no source signal stays ambiguous.
#[test]
fn provider_stays_ambiguous_when_package_json_lists_both() {
    let (output, out) = run_plain(Some(
        r#"{"dependencies":{"zod":"^3","openai":"^4","@anthropic-ai/sdk":"^0.30.0"}}"#,
    ));
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(out["report"]["coverage"]["status"], "partial");
    assert!(out["report"]["failures"][0]["message"]
        .as_str()
        .unwrap()
        .contains("provider is ambiguous"));
}

/// Nothing discovered: exit 3, and the human report opens with a line that
/// cannot be mistaken for a pass.
#[test]
fn empty_coverage_exits_3_with_explicit_first_line() {
    let tmp = TempDir::new().unwrap();
    setup_ts_project(tmp.path(), &[PLAIN_SCHEMA]);
    let output = Command::cargo_bin("schemalint")
        .unwrap()
        .current_dir(tmp.path())
        .args(["check-node", "-S", "nothing/**/*.ts", "-f", "human"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout.lines().next(),
        Some("checked 0 schemas: nothing was discovered")
    );
    let warnings = stdout.matches("No file on disk matched").count()
        + String::from_utf8_lossy(&output.stderr)
            .matches("No file on disk matched")
            .count();
    assert_eq!(warnings, 1, "glob warning printed once");
}
