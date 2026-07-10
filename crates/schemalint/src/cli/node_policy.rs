use crate::cli::pipeline::{model_target_input, EnvelopePolicy, TargetInput};
use crate::ingest::{DiscoveredModel, Provider, ProviderResolution};
use crate::rules::registry::RuleSet;

use super::{ANTHROPIC_PROFILE_ID, OPENAI_PROFILE_ID};

pub(crate) fn automatic_profile_ids(models: &[DiscoveredModel]) -> Vec<String> {
    let has_openai = models
        .iter()
        .any(|model| model.provider.provider() == Some(Provider::Openai));
    let has_anthropic = models
        .iter()
        .any(|model| model.provider.provider() == Some(Provider::Anthropic));
    [
        (has_openai, OPENAI_PROFILE_ID),
        (has_anthropic, ANTHROPIC_PROFILE_ID),
    ]
    .into_iter()
    .filter(|(present, _)| *present)
    .map(|(_, profile)| profile.to_string())
    .collect()
}

pub(crate) fn automatic_target_inputs(
    models: &[DiscoveredModel],
    profile_rulesets: &[(&crate::profile::Profile, RuleSet)],
) -> Vec<TargetInput> {
    let mut targets = Vec::with_capacity(models.len());
    let inferred_provider = single_owned_provider(models);
    for model in models {
        let Some((profile_id, provider)) = effective_provider(model, inferred_provider) else {
            targets.push(model_target_input(
                model,
                model.provider,
                vec![],
                Err(format!(
                    "provider is ambiguous for target kind '{}'; pass --profile explicitly",
                    model.canonical_kind
                )),
                EnvelopePolicy::Validate,
            ));
            continue;
        };

        let Some(index) = profile_rulesets
            .iter()
            .position(|(profile, _)| profile.name == profile_id)
        else {
            targets.push(model_target_input(
                model,
                provider,
                vec![profile_id.to_string()],
                Err(format!(
                    "no ruleset loaded for provider profile '{profile_id}'"
                )),
                EnvelopePolicy::Validate,
            ));
            continue;
        };
        targets.push(model_target_input(
            model,
            provider,
            vec![profile_id.to_string()],
            Ok(vec![index]),
            EnvelopePolicy::Validate,
        ));
    }
    targets
}

fn effective_provider(
    model: &DiscoveredModel,
    inferred_provider: Option<Provider>,
) -> Option<(&'static str, ProviderResolution)> {
    match model.provider {
        ProviderResolution::Definitive {
            provider: Provider::Openai,
        }
        | ProviderResolution::Inferred {
            provider: Provider::Openai,
        } => Some((OPENAI_PROFILE_ID, model.provider)),
        ProviderResolution::Definitive {
            provider: Provider::Anthropic,
        }
        | ProviderResolution::Inferred {
            provider: Provider::Anthropic,
        } => Some((ANTHROPIC_PROFILE_ID, model.provider)),
        ProviderResolution::Ambiguous {} => inferred_provider.map(|provider| match provider {
            Provider::Openai => (OPENAI_PROFILE_ID, ProviderResolution::Inferred { provider }),
            Provider::Anthropic => (
                ANTHROPIC_PROFILE_ID,
                ProviderResolution::Inferred { provider },
            ),
        }),
    }
}

fn single_owned_provider(models: &[DiscoveredModel]) -> Option<Provider> {
    let has_openai = models
        .iter()
        .any(|model| model.provider.provider() == Some(Provider::Openai));
    let has_anthropic = models
        .iter()
        .any(|model| model.provider.provider() == Some(Provider::Anthropic));
    match (has_openai, has_anthropic) {
        (true, false) => Some(Provider::Openai),
        (false, true) => Some(Provider::Anthropic),
        _ => None,
    }
}
