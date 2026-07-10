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

fn result(model: &str, pointer: &str) -> SchemaCheckResult {
    (
        PathBuf::from("src/models.ts"),
        model.into(),
        Ok(vec![make_diag(pointer)]),
    )
}

#[test]
fn attaches_source_span_for_one_model() {
    let model = make_model(
        "UserSchema",
        "src/models.ts",
        vec![("/properties/email", "src/models.ts", 5)],
    );
    let output = attach_source_spans(vec![result("UserSchema", "/properties/email")], &[model]);
    let diagnostics = output[0].2.as_ref().unwrap();
    assert_eq!(diagnostics[0].source.as_ref().unwrap().line, Some(5));
}

#[test]
fn model_identity_prevents_pointer_collisions() {
    let models = [
        make_model(
            "UserSchema",
            "src/models.ts",
            vec![("/properties/name", "src/models.ts", 5)],
        ),
        make_model(
            "AddressSchema",
            "src/models.ts",
            vec![("/properties/name", "src/models.ts", 20)],
        ),
    ];
    let output = attach_source_spans(
        vec![
            result("UserSchema", "/properties/name"),
            result("AddressSchema", "/properties/name"),
        ],
        &models,
    );
    assert_eq!(
        output[0].2.as_ref().unwrap()[0]
            .source
            .as_ref()
            .unwrap()
            .line,
        Some(5)
    );
    assert_eq!(
        output[1].2.as_ref().unwrap()[0]
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
    let output = attach_source_spans(vec![result("UserSchema", "/properties/missing")], &[model]);
    assert!(output[0].2.as_ref().unwrap()[0].source.is_none());
}
