use serde::Deserialize;

/// Top-level truth declaration for a single provider.
#[derive(Debug, Clone, Deserialize)]
pub struct ProviderTruth {
    pub provider: ProviderInfo,
    #[serde(default)]
    pub keywords: Vec<KeywordTruth>,
    #[serde(default, rename = "structural_tests")]
    pub structural_tests: Vec<StructuralTest>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProviderInfo {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub endpoint: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    pub behavior: ProviderBehavior,
}

/// Provider's declared handling strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderBehavior {
    Strict,
    Permissive,
    Stripping,
}

/// Per-keyword truth entry.
#[derive(Debug, Clone, Deserialize)]
pub struct KeywordTruth {
    pub name: String,
    pub behavior: KeywordBehavior,
    /// Inline JSON schema exercising this keyword.
    pub test_schema: String,
    /// Expected error message when behavior is Reject.
    #[serde(default)]
    pub expected_error: Option<String>,
    /// Expected JSON Pointer path for the error.
    #[serde(default, rename = "expected_error_path")]
    pub expected_error_path: Option<String>,
    /// Expected transformed schema when behavior is Strip.
    #[serde(default, rename = "expected_transformed")]
    pub expected_transformed: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KeywordBehavior {
    Accept,
    Reject,
    Strip,
}

/// Structural limit test case.
#[derive(Debug, Clone, Deserialize)]
pub struct StructuralTest {
    pub limit_name: String,
    pub test_schema: String,
    pub expected_behavior: KeywordBehavior,
    #[serde(default, rename = "expected_error_path")]
    pub expected_error_path: Option<String>,
}
