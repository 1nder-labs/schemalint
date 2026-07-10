use crate::ingest::{DiscoveredModel, EnvelopeField};
use crate::profile::Profile;
use crate::rules::registry::{Diagnostic, DiagnosticSeverity};

const MAX_PROVIDER_NAME_CHARS: usize = 64;

/// Validate provider request-envelope metadata at the SDK usage site.
///
/// Envelope fields are not JSON Schema keywords and therefore must not be
/// inserted into the normalized schema arena. Diagnostics retain the span
/// emitted by the source adapter.
pub fn check_envelope(model: &DiscoveredModel, profile: &Profile) -> Vec<Diagnostic> {
    check_envelope_fields(&model.envelope, profile)
}

pub(crate) fn check_envelope_fields(
    envelope: &std::collections::HashMap<String, EnvelopeField>,
    profile: &Profile,
) -> Vec<Diagnostic> {
    if !profile_has_provider_name_rules(profile) {
        return Vec::new();
    }

    envelope
        .get("name")
        .and_then(|field| validate_name(field, profile))
        .into_iter()
        .collect()
}

fn profile_has_provider_name_rules(profile: &Profile) -> bool {
    profile.name.starts_with("openai.") || profile.name.starts_with("anthropic.")
}

fn validate_name(field: &EnvelopeField, profile: &Profile) -> Option<Diagnostic> {
    let issue = if field.required && field.value.is_none() {
        Some("required provider name is not statically resolvable".to_string())
    } else if let Some(name) = field.value.as_deref() {
        let count = name.chars().count();
        if count == 0 {
            Some("provider name must not be empty".to_string())
        } else if count > MAX_PROVIDER_NAME_CHARS {
            Some(format!(
                "provider name has {count} characters; maximum is {MAX_PROVIDER_NAME_CHARS}"
            ))
        } else if !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        {
            Some(
                "provider name may contain only ASCII letters, digits, underscores, and hyphens"
                    .to_string(),
            )
        } else {
            None
        }
    } else {
        None
    };

    issue.map(|message| Diagnostic {
        code: format!("{}-S-envelope-name", profile.code_prefix),
        severity: DiagnosticSeverity::Error,
        message,
        pointer: String::new(),
        source: Some(field.span.clone()),
        profile: profile.name.clone(),
        hint: Some("Use a 1-64 character provider-safe name".into()),
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::ingest::ProviderResolution;
    use crate::profile::load;
    use crate::rules::registry::SourceSpan;

    fn profile(name: &str) -> Profile {
        load(format!("name = \"{name}\"\ncode_prefix = \"T\"\n[structural]\n").as_bytes()).unwrap()
    }

    fn model(name: &str) -> DiscoveredModel {
        DiscoveredModel {
            name: "schema".into(),
            module_path: "schema.ts".into(),
            schema: serde_json::json!({}),
            source_map: HashMap::new(),
            canonical_kind: "openai.zodTextFormat".into(),
            provider: ProviderResolution::Definitive {
                provider: crate::ingest::Provider::Openai,
            },
            envelope: HashMap::from([(
                "name".into(),
                EnvelopeField {
                    required: true,
                    value: Some(name.into()),
                    span: SourceSpan {
                        file: "schema.ts".into(),
                        line: Some(7),
                        col: Some(12),
                    },
                },
            )]),
            usage_span: None,
        }
    }

    #[test]
    fn accepts_provider_safe_name_at_limit() {
        assert!(check_envelope(&model(&"a".repeat(64)), &profile("openai.test")).is_empty());
    }

    #[test]
    fn rejects_invalid_name_at_emitted_span() {
        let diagnostics = check_envelope(&model("bad name"), &profile("openai.test"));
        assert_eq!(diagnostics[0].code, "T-S-envelope-name");
        assert_eq!(diagnostics[0].source.as_ref().unwrap().line, Some(7));
    }
}
