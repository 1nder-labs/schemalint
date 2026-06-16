pub(crate) const DOCS_BASE_URL: &str = "https://1nder-labs.github.io/schemalint";

/// Semantic-category rule names. These share the `-S-` infix with structural
/// rules, so they cannot be distinguished from the diagnostic code alone.
/// Keep in sync with the `Semantic` rules in `crate::rules::semantic`.
const SEMANTIC_RULES: &[&str] = &[
    "empty-object",
    "additional-properties-object",
    "anyof-objects",
];

/// Build the documentation URL for a diagnostic code.
///
/// Rule reference pages are published at `/rules/{category}/{name}` (e.g.
/// `/rules/keyword/allOf`), so the diagnostic code's `{prefix}-{S|K}-{name}`
/// shape is translated into that path. Unrecognized shapes fall back to the
/// rule index so the link never 404s.
pub(crate) fn rule_url(code: &str) -> String {
    match rule_path(code) {
        Some(path) => format!("{DOCS_BASE_URL}/rules/{path}"),
        None => format!("{DOCS_BASE_URL}/rules"),
    }
}

/// Translate a diagnostic code into its `{category}/{name}` doc path.
fn rule_path(code: &str) -> Option<String> {
    // code = {PREFIX}-{S|K}-{name}; PREFIX (e.g. OAI, ANT) contains no '-'.
    let mut parts = code.splitn(3, '-');
    let _prefix = parts.next()?;
    let kind = parts.next()?;
    let name = parts.next().filter(|n| !n.is_empty())?;
    let category = match kind {
        "K" if name.ends_with("-restricted") => "restriction",
        "K" => "keyword",
        "S" if SEMANTIC_RULES.contains(&name) => "semantic",
        "S" => "structural",
        _ => return None,
    };
    Some(format!("{category}/{name}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyword_code_maps_to_keyword_page() {
        assert_eq!(
            rule_url("OAI-K-allOf"),
            format!("{DOCS_BASE_URL}/rules/keyword/allOf")
        );
    }

    #[test]
    fn restricted_code_maps_to_restriction_page() {
        assert_eq!(
            rule_url("OAI-K-format-restricted"),
            format!("{DOCS_BASE_URL}/rules/restriction/format-restricted")
        );
        assert_eq!(
            rule_url("ANT-K-minItems-restricted"),
            format!("{DOCS_BASE_URL}/rules/restriction/minItems-restricted")
        );
    }

    #[test]
    fn structural_code_maps_to_structural_page() {
        assert_eq!(
            rule_url("OAI-S-all-properties-required"),
            format!("{DOCS_BASE_URL}/rules/structural/all-properties-required")
        );
    }

    #[test]
    fn semantic_code_maps_to_semantic_page() {
        assert_eq!(
            rule_url("OAI-S-empty-object"),
            format!("{DOCS_BASE_URL}/rules/semantic/empty-object")
        );
        assert_eq!(
            rule_url("ANT-S-anyof-objects"),
            format!("{DOCS_BASE_URL}/rules/semantic/anyof-objects")
        );
    }

    #[test]
    fn unrecognized_code_falls_back_to_index() {
        assert_eq!(rule_url("garbage"), format!("{DOCS_BASE_URL}/rules"));
    }
}
