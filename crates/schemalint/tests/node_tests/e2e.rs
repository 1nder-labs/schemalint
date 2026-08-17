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
fn e2e_nested_identifier_schema_reports_ancestor_source() {
    // `Inner` is a separately declared const, not an inline `z.object()`
    // literal at the `a:` call site. The TypeScript source-map builder
    // (`buildSourceMapFromObjectLiteral`) only recurses into an inline
    // literal, so it maps `/properties/a` but never
    // `/properties/a/properties/site`. The diagnostic on the nested
    // pointer must still carry a location, taken from its mapped parent.
    let tmp = TempDir::new().unwrap();
    setup_ts_project(
        tmp.path(),
        &[(
            "nested_identifier.ts",
            r#"import { z } from "zod";
const Inner = z.object({ site: z.string().url() });
export const Outer = z.object({ a: Inner });
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

    let diag = out
        .diagnostics
        .iter()
        .find(|d| d.pointer == "/properties/a/properties/site")
        .expect("should diagnose /properties/a/properties/site from Inner");

    let src = diag
        .source
        .as_ref()
        .expect("nested diagnostic should carry the ancestor's source span");
    assert!(
        src.file.ends_with("/nested_identifier.ts"),
        "file={}",
        src.file
    );
    assert_eq!(src.line, Some(3), "`a: Inner,` is on line 3");
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

    // z.intersection() is NOT discovered — an explicitly requested source
    // that yields no checkable target is incomplete, not a clean run.
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
        !output.status.success(),
        "zero-target discovery must exit 1"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let out: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(out["schema_version"], "1.1");
    assert_eq!(out["report"]["coverage"]["status"], "empty");
    assert_eq!(out["report"]["coverage"]["checked"], 0);
}

// ---------------------------------------------------------------------------
// Per-target provider auto-selection tests
//
// Direct SDK adapters carry definitive ownership across the sidecar wire.
// ---------------------------------------------------------------------------

#[test]
fn e2e_openai_target_auto_selects_openai_profile() {
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

    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    cmd.current_dir(tmp.path());
    let output = cmd
        .args(["check-node", "-S", "src/**/*.ts", "-f", "json"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("auto-selected per-target profile(s)"),
        "expected per-target selection log, got stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("openai.so.2026-04-30"),
        "expected profile name in auto-detect log, got stderr:\n{stderr}"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let out: JsonOutput = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("JSON parse failed: {e}\nstdout:\n{stdout}\nstderr:\n{stderr}"));
    assert!(
        out.profiles.iter().any(|p| p == "openai.so.2026-04-30"),
        "expected openai profile in output, got: {:?}",
        out.profiles
    );
}

#[test]
fn e2e_anthropic_target_auto_selects_anthropic_profile() {
    let tmp = TempDir::new().unwrap();
    setup_ts_project(
        tmp.path(),
        &[(
            "schema.ts",
            r#"import { z } from "zod";
import { betaZodTool } from "@anthropic-ai/sdk/helpers/beta/zod";
export const Translate = betaZodTool({
  name: "translate",
  inputSchema: z.object({ text: z.string(), target_language: z.string() }),
});
"#,
        )],
    );

    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    cmd.current_dir(tmp.path());
    let output = cmd
        .args(["check-node", "-S", "src/**/*.ts", "-f", "json"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("auto-selected per-target profile(s)"),
        "expected per-target selection log, got stderr:\n{stderr}"
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

#[test]
fn e2e_provider_inference_is_independent_of_source_partitioning() {
    let tmp = TempDir::new().unwrap();
    setup_ts_project(
        tmp.path(),
        &[
            (
                "generic.ts",
                r#"import { z } from "zod";
import { Output } from "ai";
const Generic = z.object({ value: z.string() });
Output.object({ name: "generic", schema: Generic });
"#,
            ),
            (
                "openai.ts",
                r#"import { z } from "zod";
import { zodResponseFormat } from "openai/helpers/zod";
const OpenAI = z.object({ value: z.string() });
zodResponseFormat(OpenAI, "openai_response");
"#,
            ),
            (
                "anthropic.ts",
                r#"import { z } from "zod";
import { zodOutputFormat } from "@anthropic-ai/sdk/helpers/zod";
const Anthropic = z.object({ value: z.string() });
zodOutputFormat(Anthropic);
"#,
            ),
        ],
    );

    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    cmd.current_dir(tmp.path());
    let output = cmd
        .args([
            "check-node",
            "-S",
            "src/generic.ts",
            "-S",
            "src/openai.ts",
            "-S",
            "src/anthropic.ts",
            "-f",
            "json",
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(report["report"]["coverage"]["status"], "partial");
    assert_eq!(report["report"]["coverage"]["discovered"], 3);
    assert_eq!(report["report"]["coverage"]["checked"], 2);
    assert_eq!(report["report"]["coverage"]["failed"], 1);
    assert!(report["report"]["failures"]
        .as_array()
        .unwrap()
        .iter()
        .any(|failure| failure["message"]
            .as_str()
            .is_some_and(|message| message.contains("provider is ambiguous"))));
    let generic = report["report"]["targets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|target| target["canonical_kind"] == "ai.Output.object")
        .unwrap();
    assert_eq!(generic["provider"]["certainty"], "ambiguous");
    assert_eq!(generic["effective_profiles"], serde_json::json!([]));
    assert_eq!(generic["status"], "failed");
}

// ---------------------------------------------------------------------------
// Ambiguous automatic targets fail instead of guessing from package metadata.
// ---------------------------------------------------------------------------

/// No provider hint (plain `z.object` with no provider SDK import), no
/// "schemalint" config in package.json, but package.json lists the `openai`
/// dependency → falls back to `detect_providers_from_deps` and selects the
/// openai profile.
#[test]
fn e2e_ambiguous_target_does_not_guess_from_package_json() {
    let tmp = TempDir::new().unwrap();
    setup_ts_project(
        tmp.path(),
        &[(
            "schema.ts",
            r#"import { z } from "zod";
export const Plain = z.object({ name: z.string() });
"#,
        )],
    );
    // No "schemalint" key here — only a dependency for the deps-detection
    // tier to find.
    fs::write(
        tmp.path().join("package.json"),
        r#"{"dependencies": {"openai": "^4.0.0"}}"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    cmd.current_dir(tmp.path());
    let output = cmd
        .args(["check-node", "-S", "src/**/*.ts", "-f", "json"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let out: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(out["report"]["coverage"]["status"], "partial");
    assert!(out["report"]["failures"][0]["message"]
        .as_str()
        .unwrap()
        .contains("provider is ambiguous"));
}

/// No provider hint, no "schemalint" config, and no recognized dependency in
/// package.json → falls all the way through to the openai default rather
/// than hard-erroring with "no profiles specified.".
#[test]
fn e2e_ambiguous_target_without_deps_requires_explicit_profile() {
    let tmp = TempDir::new().unwrap();
    setup_ts_project(
        tmp.path(),
        &[(
            "schema.ts",
            r#"import { z } from "zod";
export const Plain = z.object({ name: z.string() });
"#,
        )],
    );

    let mut cmd = Command::cargo_bin("schemalint").unwrap();
    cmd.current_dir(tmp.path());
    let output = cmd
        .args(["check-node", "-S", "src/**/*.ts", "-f", "json"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let out: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(out["report"]["coverage"]["status"], "partial");
}
