use super::*;
use crate::rules::registry::{DiagnosticSeverity, SourceSpan};

fn make_diag(pointer: &str) -> crate::rules::Diagnostic {
    crate::rules::Diagnostic {
        code: "TEST-001".into(),
        severity: DiagnosticSeverity::Error,
        message: "test diagnostic".into(),
        pointer: pointer.to_string(),
        source: None,
        profile: "test".into(),
        hint: None,
    }
}

fn make_model(
    name: &str,
    module_path: &str,
    spans: Vec<(&str, &str, u32)>,
) -> crate::ingest::DiscoveredModel {
    let source_map = spans
        .into_iter()
        .map(|(pointer, file, line)| {
            (
                pointer.to_string(),
                SourceSpan {
                    file: file.to_string(),
                    line: Some(line),
                    col: Some(1),
                },
            )
        })
        .collect();
    crate::ingest::DiscoveredModel {
        name: name.to_string(),
        module_path: module_path.to_string(),
        schema: serde_json::json!({}),
        source_map,
        canonical_kind: String::new(),
        provider: Default::default(),
        envelope: Default::default(),
        usage_span: None,
    }
}

fn diagnostic(model: &crate::ingest::DiscoveredModel, pointer: &str) -> crate::rules::Diagnostic {
    let mut diagnostics = vec![make_diag(pointer)];
    attach_diagnostic_sources(&mut diagnostics, &model.source_map);
    diagnostics.pop().unwrap()
}

#[test]
fn attaches_source_span_for_one_model() {
    let model = make_model(
        "UserSchema",
        "src/models.ts",
        vec![("/properties/email", "src/models.ts", 5)],
    );
    let output = diagnostic(&model, "/properties/email");
    assert_eq!(output.source.as_ref().unwrap().line, Some(5));
}

#[test]
fn per_usage_source_maps_prevent_duplicate_name_collisions() {
    let models = [
        make_model(
            "SharedName",
            "src/models.ts",
            vec![("/properties/name", "src/models.ts", 5)],
        ),
        make_model(
            "SharedName",
            "src/models.ts",
            vec![("/properties/name", "src/models.ts", 20)],
        ),
    ];
    let output = [
        diagnostic(&models[0], "/properties/name"),
        diagnostic(&models[1], "/properties/name"),
    ];
    assert_eq!(output[0].source.as_ref().unwrap().line, Some(5));
    assert_eq!(output[1].source.as_ref().unwrap().line, Some(20));
}

#[test]
fn unmatched_pointer_stays_without_source() {
    let model = make_model(
        "UserSchema",
        "src/models.ts",
        vec![("/properties/email", "src/models.ts", 5)],
    );
    let output = diagnostic(&model, "/properties/missing");
    assert!(output.source.is_none());
}

#[test]
fn nested_pointer_takes_ancestor_line_when_only_parent_is_mapped() {
    let model = make_model(
        "Outer",
        "src/models.ts",
        vec![("/properties/a", "src/models.ts", 3)],
    );
    let output = diagnostic(&model, "/properties/a/properties/site");
    assert_eq!(output.source.as_ref().unwrap().line, Some(3));
}

#[test]
fn exact_match_wins_over_ancestor() {
    let model = make_model(
        "Outer",
        "src/models.ts",
        vec![
            ("/properties/a", "src/models.ts", 3),
            ("/properties/a/properties/site", "src/models.ts", 7),
        ],
    );
    let output = diagnostic(&model, "/properties/a/properties/site");
    assert_eq!(output.source.as_ref().unwrap().line, Some(7));
}

#[test]
fn root_pointer_with_no_map_entry_stays_without_source() {
    let model = make_model("Empty", "src/models.ts", vec![]);
    let output = diagnostic(&model, "");
    assert!(output.source.is_none());
}

#[test]
fn escaped_segment_ancestor_is_found_without_splitting_inside_the_escape() {
    // Property literally named "a/b" maps to the escaped pointer
    // `/properties/a~1b`. A diagnostic nested inside it must walk up to
    // that ancestor, not stop partway through the `~1` escape.
    let model = make_model(
        "Outer",
        "src/models.ts",
        vec![("/properties/a~1b", "src/models.ts", 4)],
    );
    let output = diagnostic(&model, "/properties/a~1b/properties/site");
    assert_eq!(output.source.as_ref().unwrap().line, Some(4));
}

#[test]
fn escaped_segment_is_not_mistaken_for_an_unescaped_ancestor() {
    // `/properties/a` is NOT an ancestor of `/properties/a~1b` — the `~1`
    // decodes to a literal `/` inside the property name "a/b", so the two
    // pointers name unrelated schema locations. A buggy implementation
    // that unescapes before splitting would wrongly match here.
    let model = make_model(
        "Outer",
        "src/models.ts",
        vec![("/properties/a", "src/models.ts", 9)],
    );
    let output = diagnostic(&model, "/properties/a~1b");
    assert!(output.source.is_none());
}

#[test]
fn existing_usage_span_is_not_overwritten_by_root_schema_map() {
    let model = make_model("Schema", "src/models.ts", vec![("", "src/models.ts", 5)]);
    let mut output = make_diag("");
    output.source = Some(SourceSpan {
        file: "src/models.ts".into(),
        line: Some(20),
        col: Some(12),
    });
    attach_diagnostic_sources(std::slice::from_mut(&mut output), &model.source_map);
    assert_eq!(output.source.as_ref().unwrap().line, Some(20));
}
