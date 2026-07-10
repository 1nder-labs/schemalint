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

fn result(model: &crate::ingest::DiscoveredModel, pointer: &str) -> SchemaCheckResult {
    let entry = schema_entry(model, &["test".into()], model.provider);
    SchemaCheckResult {
        path: entry.path,
        model_name: entry.model_name,
        diagnostics: Ok(vec![make_diag(pointer)]),
        source_map: entry.source_map,
        target: entry.target,
    }
}

#[test]
fn attaches_source_span_for_one_model() {
    let model = make_model(
        "UserSchema",
        "src/models.ts",
        vec![("/properties/email", "src/models.ts", 5)],
    );
    let output = attach_source_spans(vec![result(&model, "/properties/email")]);
    let diagnostics = output[0].diagnostics.as_ref().unwrap();
    assert_eq!(diagnostics[0].source.as_ref().unwrap().line, Some(5));
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
    let output = attach_source_spans(vec![
        result(&models[0], "/properties/name"),
        result(&models[1], "/properties/name"),
    ]);
    assert_eq!(
        output[0].diagnostics.as_ref().unwrap()[0]
            .source
            .as_ref()
            .unwrap()
            .line,
        Some(5)
    );
    assert_eq!(
        output[1].diagnostics.as_ref().unwrap()[0]
            .source
            .as_ref()
            .unwrap()
            .line,
        Some(20)
    );
}

#[test]
fn unmatched_pointer_stays_without_source() {
    let model = make_model(
        "UserSchema",
        "src/models.ts",
        vec![("/properties/email", "src/models.ts", 5)],
    );
    let output = attach_source_spans(vec![result(&model, "/properties/missing")]);
    assert!(output[0].diagnostics.as_ref().unwrap()[0].source.is_none());
}

#[test]
fn existing_usage_span_is_not_overwritten_by_root_schema_map() {
    let model = make_model("Schema", "src/models.ts", vec![("", "src/models.ts", 5)]);
    let mut check = result(&model, "");
    check.diagnostics.as_mut().unwrap()[0].source = Some(SourceSpan {
        file: "src/models.ts".into(),
        line: Some(20),
        col: Some(12),
    });

    let output = attach_source_spans(vec![check]);
    assert_eq!(
        output[0].diagnostics.as_ref().unwrap()[0]
            .source
            .as_ref()
            .unwrap()
            .line,
        Some(20)
    );
}
