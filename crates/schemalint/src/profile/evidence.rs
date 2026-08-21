use serde::{Deserialize, Serialize};

/// Provider-independent identity: the diagnostic code without its provider prefix.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuleKey(String);

impl RuleKey {
    pub fn parse(value: &str) -> Result<Self, String> {
        if (value.starts_with("K-") || value.starts_with("S-")) && value.len() > 2 {
            Ok(Self(value.to_string()))
        } else {
            Err(format!("invalid rule key '{value}'; expected K-* or S-*"))
        }
    }

    pub fn from_code(code: &str, prefix: &str) -> Option<Self> {
        code.strip_prefix(prefix)
            .and_then(|value| value.strip_prefix('-'))
            .and_then(|value| Self::parse(value).ok())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStatus {
    Documented,
    DocumentedExample,
    SdkTransform,
    LiveVerified,
    Inferred,
    Unknown,
}

impl EvidenceStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Documented => "documented",
            Self::DocumentedExample => "documented_example",
            Self::SdkTransform => "sdk_transform",
            Self::LiveVerified => "live_verified",
            Self::Inferred => "inferred",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EvidenceSource {
    pub title: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderEvidence {
    pub status: EvidenceStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<EvidenceSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub basis: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_target: Option<String>,
}

impl ProviderEvidence {
    pub fn validate(&self) -> Result<(), String> {
        if self
            .sources
            .iter()
            .any(|source| !source.url.starts_with("https://") || source.title.is_empty())
        {
            return Err("sources require a title and canonical HTTPS URL".into());
        }
        let has_basis = self
            .basis
            .as_ref()
            .is_some_and(|value| !value.trim().is_empty());
        if matches!(
            self.status,
            EvidenceStatus::Documented
                | EvidenceStatus::DocumentedExample
                | EvidenceStatus::SdkTransform
        ) && self.sources.is_empty()
        {
            return Err(format!(
                "{} evidence requires a source",
                self.status.as_str()
            ));
        }
        if matches!(
            self.status,
            EvidenceStatus::DocumentedExample
                | EvidenceStatus::SdkTransform
                | EvidenceStatus::Inferred
                | EvidenceStatus::Unknown
        ) && !has_basis
        {
            return Err(format!(
                "{} evidence requires a basis",
                self.status.as_str()
            ));
        }
        if self.status == EvidenceStatus::Unknown && !self.sources.is_empty() {
            return Err("unknown evidence cannot cite a source".into());
        }
        if self.status == EvidenceStatus::LiveVerified {
            if !has_basis || self.verified_at.is_none() || self.verification_target.is_none() {
                return Err(
                    "live_verified evidence requires basis, verifiedAt, and verificationTarget"
                        .into(),
                );
            }
        } else if self.verified_at.is_some() || self.verification_target.is_some() {
            return Err(format!(
                "{} evidence cannot include verification fields",
                self.status.as_str()
            ));
        }
        Ok(())
    }

    pub fn primary_url(&self) -> Option<&str> {
        self.sources.first().map(|source| source.url.as_str())
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawEvidence {
    pub key: String,
    #[serde(flatten)]
    pub evidence: ProviderEvidence,
}
