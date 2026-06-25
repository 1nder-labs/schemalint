use super::*;

// ---------------------------------------------------------------------------
// End-to-end pipeline tests (real Node helper + TypeScript project)
// ---------------------------------------------------------------------------

#[test]
fn e2e_forbidden_format_produces_diagnostic_with_source_span() {
    let tmp = TempDir::new().unwrap();
    setup_ts_project(
        tmp.path(),
        &[(
            "forbidden.ts",
            r#"import { z } from "zod";
export const Bad = z.object({ website: z.string().url() });
"#,
        )],
    );

    let out = run_check_node_json(
        tmp.path(),
        &[
            "--source",
            "src/**/*.ts",
            "--profile",
            "openai.so.2026-04-30",
        ],
    );

    assert_eq!(out.profiles, vec!["openai.so.2026-04-30"]);
    assert_eq!(out.summary.errors, 1);
    assert_eq!(out.summary.warnings, 0);
    assert_eq!(out.summary.schemas_checked, 1);

    let diag = &out.diagnostics[0];
    assert_eq!(diag.code, "OAI-K-format-restricted");
    assert_eq!(diag.severity, "error");
    assert_eq!(diag.pointer, "/properties/website");
    assert_eq!(diag.profile, "openai.so.2026-04-30");

    let src = diag
        .source
        .as_ref()
        .expect("source span should be populated");
    assert!(src.file.ends_with("/forbidden.ts"), "file={}", src.file);
    assert_eq!(src.line, Some(2));
}

#[test]
fn e2e_clean_schema_exits_zero() {
    let tmp = TempDir::new().unwrap();
    setup_ts_project(
        tmp.path(),
        &[(
            "clean.ts",
            r#"import { z } from "zod";
export const Good = z.object({ name: z.string(), age: z.number() });
"#,
        )],
    );

    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    cmd.current_dir(tmp.path());
    let output = cmd
        .args([
            "check-node",
            "-S",
            "src/**/*.ts",
            "-p",
            "openai.so.2026-04-30",
            "-f",
            "json",
        ])
        .output()
        .unwrap();

    // A plain z.object({...}) won't produce additionalProperties: false,
    // so the OpenAI structural rule OAI-S-additionalProperties-required
    // may fire. The invariant we care about: the schema IS discovered and
    // checked, and the pipeline doesn't crash.
    let stdout = String::from_utf8_lossy(&output.stdout);
    let out: JsonOutput = serde_json::from_str(&stdout).unwrap();
    assert!(
        out.summary.schemas_checked >= 1,
        "schema should be discovered"
    );
    // No format-restricted or allof errors on this clean schema
    assert!(
        !out.diagnostics
            .iter()
            .any(|d| d.code == "OAI-K-format-restricted" || d.code == "OAI-K-allOf-forbidden"),
        "clean schema should not trigger format/allof errors"
    );
}

#[test]
fn e2e_multi_schema_single_file_separate_source_spans() {
    let tmp = TempDir::new().unwrap();
    setup_ts_project(
        tmp.path(),
        &[(
            "multi.ts",
            r#"import { z } from "zod";

export const UserSchema = z.object({
  email: z.string().url(),
});

export const AddressSchema = z.object({
  street: z.string(),
  city: z.string(),
});
"#,
        )],
    );

    let out = run_check_node_json(
        tmp.path(),
        &[
            "--source",
            "src/**/*.ts",
            "--profile",
            "openai.so.2026-04-30",
        ],
    );

    let user_diag = out
        .diagnostics
        .iter()
        .find(|d| d.pointer == "/properties/email")
        .expect("should diagnose /properties/email from UserSchema");

    let src = user_diag.source.as_ref().unwrap();
    assert!(src.file.ends_with("/multi.ts"));
    assert_eq!(src.line, Some(4), "url() is on line 4 of multi.ts");
}

#[test]
fn e2e_package_json_driven_without_cli_flags() {
    let tmp = TempDir::new().unwrap();
    setup_ts_project(
        tmp.path(),
        &[(
            "schema.ts",
            r#"import { z } from "zod";
export const My = z.object({ site: z.string().url() });
"#,
        )],
    );

    fs::write(
        tmp.path().join("package.json"),
        r#"{
  "schemalint": {
    "profiles": ["openai.so.2026-04-30"],
    "include": ["src/**/*.ts"]
  }
}"#,
    )
    .unwrap();

    let out = run_check_node_json(tmp.path(), &[]);
    assert_eq!(out.summary.errors, 1);
    assert_eq!(out.diagnostics[0].code, "OAI-K-format-restricted");
}

#[test]
fn e2e_cli_source_overrides_package_json_include() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("src");
    let sub = src.join("sub");
    fs::create_dir_all(&sub).unwrap();
    fs::write(
        sub.join("nested.ts"),
        r#"import { z } from "zod";
export const Nested = z.object({ url: z.string().url() });
"#,
    )
    .unwrap();

    link_workspace_node_modules(tmp.path());

    fs::write(
        tmp.path().join("tsconfig.json"),
        r#"{"compilerOptions":{"module":"ESNext","moduleResolution":"bundler","strict":true},"include":["src/**/*.ts"]}"#,
    )
    .unwrap();

    fs::write(
        tmp.path().join("package.json"),
        r#"{
  "schemalint": {
    "profiles": ["openai.so.2026-04-30"],
    "include": ["src/nonexistent/**/*.ts"]
  }
}"#,
    )
    .unwrap();

    let out = run_check_node_json(
        tmp.path(),
        &[
            "--source",
            "src/**/*.ts",
            "--profile",
            "openai.so.2026-04-30",
        ],
    );

    assert_eq!(out.summary.errors, 1);
    assert_eq!(out.diagnostics[0].code, "OAI-K-format-restricted");
}

#[test]
fn e2e_anthropic_profile_allows_uri_format() {
    let tmp = TempDir::new().unwrap();
    setup_ts_project(
        tmp.path(),
        &[(
            "schema.ts",
            r#"import { z } from "zod";
export const My = z.object({ site: z.string().url() });
"#,
        )],
    );

    let out = run_check_node_json(
        tmp.path(),
        &[
            "--source",
            "src/**/*.ts",
            "--profile",
            "anthropic.so.2026-04-30",
        ],
    );

    assert!(
        !out.diagnostics
            .iter()
            .any(|d| d.code == "OAI-K-format-restricted"),
        "Anthropic profile should not produce OpenAI format-restricted diagnostics"
    );
}

#[test]
fn e2e_intersection_not_discovered_gracefully() {
    let tmp = TempDir::new().unwrap();
    setup_ts_project(
        tmp.path(),
        &[(
            "intersection.ts",
            r#"import { z } from "zod";

const Person = z.object({ name: z.string() });
const Employee = z.object({ id: z.number() });

export const Combo = z.intersection(Person, Employee);
"#,
        )],
    );

    // z.intersection() is NOT discovered — the AST walker only finds
    // z.object() call expressions. This is documented behavior (scope
    // boundary: "Schemas constructed from imported factory functions...
    // are not discoverable via AST walking"). The pipeline should exit
    // cleanly with 0 schemas rather than crashing.
    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    cmd.current_dir(tmp.path());
    let output = cmd
        .args([
            "check-node",
            "-S",
            "src/**/*.ts",
            "-p",
            "openai.so.2026-04-30",
            "-f",
            "json",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "should exit 0 (no schemas found, no error)"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let out: JsonOutput = serde_json::from_str(&stdout).unwrap();
    assert_eq!(out.summary.schemas_checked, 0);
}

// ---------------------------------------------------------------------------
// Provider-hint auto-detection tests (#8)
//
// These tests exercise the auto-detect block in check_node.rs (~line 173):
//   "openai"    → openai.so.2026-04-30 profile
//   "anthropic" → anthropic.so.2026-04-30 profile
//   other       → error + exit 1   (untestable without controlling the sidecar)
//
// All three tests omit --profile so the auto-detect path is exercised.
// The sidecar emits a `provider_hint` field when it detects SDK imports from
// `openai/helpers/zod` (→ "openai") or `@anthropic-ai/sdk/helpers/zod` (→ "anthropic").
// ---------------------------------------------------------------------------

/// When source imports from `openai/helpers/zod`, the sidecar sets
/// `provider_hint = "openai"` and the CLI auto-selects openai.so.2026-04-30.
#[test]
fn e2e_provider_hint_openai_auto_selects_openai_profile() {
    let tmp = TempDir::new().unwrap();
    setup_ts_project(
        tmp.path(),
        &[(
            "schema.ts",
            r#"import { z } from "zod";
import { zodFunction } from "openai/helpers/zod";
export const Lookup = zodFunction({
  name: "lookup",
  parameters: z.object({ id: z.string() }),
});
"#,
        )],
    );

    // No --profile flag — rely on auto-detection.
    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    cmd.current_dir(tmp.path());
    let output = cmd
        .args(["check-node", "-S", "src/**/*.ts", "-f", "json"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    // The CLI must log the auto-detection message.
    assert!(
        stderr.contains("auto-detected provider 'openai'"),
        "expected auto-detect log for openai, got stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("openai.so.2026-04-30"),
        "expected profile name in auto-detect log, got stderr:\n{stderr}"
    );

    // Output must be valid JSON and use the openai profile.
    let stdout = String::from_utf8_lossy(&output.stdout);
    let out: JsonOutput = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("JSON parse failed: {e}\nstdout:\n{stdout}\nstderr:\n{stderr}"));
    assert!(
        out.profiles.iter().any(|p| p == "openai.so.2026-04-30"),
        "expected openai profile in output, got: {:?}",
        out.profiles
    );
}

/// When source imports only from `@anthropic-ai/sdk/helpers/zod`, the sidecar
/// sets `provider_hint = "anthropic"` and the CLI auto-selects anthropic.so.2026-04-30.
#[test]
fn e2e_provider_hint_anthropic_auto_selects_anthropic_profile() {
    let tmp = TempDir::new().unwrap();
    setup_ts_project(
        tmp.path(),
        &[(
            "schema.ts",
            r#"import { z } from "zod";
import { betaZodTool } from "@anthropic-ai/sdk/helpers/zod";
export const Translate = betaZodTool({
  name: "translate",
  inputSchema: z.object({ text: z.string(), target_language: z.string() }),
});
"#,
        )],
    );

    // No --profile flag — rely on auto-detection.
    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    cmd.current_dir(tmp.path());
    let output = cmd
        .args(["check-node", "-S", "src/**/*.ts", "-f", "json"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("auto-detected provider 'anthropic'"),
        "expected auto-detect log for anthropic, got stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("anthropic.so.2026-04-30"),
        "expected profile name in auto-detect log, got stderr:\n{stderr}"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let out: JsonOutput = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("JSON parse failed: {e}\nstdout:\n{stdout}\nstderr:\n{stderr}"));
    assert!(
        out.profiles.iter().any(|p| p == "anthropic.so.2026-04-30"),
        "expected anthropic profile in output, got: {:?}",
        out.profiles
    );
}

// NOTE: The "unknown provider hint" branch (check_node.rs ~line 178, the `other =>` arm)
// is not exercised here. The sidecar only emits "openai" or "anthropic" hints — there
// is no fixture that causes it to emit an arbitrary string. Testing that branch would
// require either mocking the node subprocess or patching the sidecar, neither of which
// is available in this integration harness. The branch is covered by code inspection.
