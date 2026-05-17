pub(crate) const DOCS_BASE_URL: &str = "https://1nder-labs.github.io/schemalint";

pub(crate) fn rule_url(code: &str) -> String {
    format!("{DOCS_BASE_URL}/rules/{code}")
}
