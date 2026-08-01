use crate::config::preset::{builtin_presets, Preset};
use crate::config::validator::{validate_preset, PresetPlatform, PresetSource};
use crate::engine::runtime_config::PreparedRuntimeConfig;
use crate::optimizer::session::OptimizerError;
use std::collections::HashSet;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) struct OptimizerCandidate {
    pub preset: Preset,
    pub prepared_config: PreparedRuntimeConfig,
    pub fingerprint: String,
}

pub(crate) fn resolve_and_deduplicate_candidates(
    candidate_ids: Option<Vec<String>>,
    app_data_dir: &std::path::Path,
) -> Result<Vec<OptimizerCandidate>, OptimizerError> {
    let all_builtins = builtin_presets();
    let _current_platform = if cfg!(target_os = "windows") {
        PresetPlatform::Windows
    } else {
        PresetPlatform::Linux
    };

    let selected_presets: Vec<Preset> = match candidate_ids {
        Some(ids) if !ids.is_empty() => {
            let mut matched = Vec::new();
            for id in ids {
                if let Some(p) = all_builtins.iter().find(|p| p.id == id) {
                    matched.push(p.clone());
                } else {
                    return Err(OptimizerError::CandidateNotFound(id));
                }
            }
            matched
        }
        _ => {
            let mut sorted = all_builtins;
            sorted.sort_by_key(|p| p.priority);
            sorted
        }
    };

    let mut result = Vec::new();
    let mut seen_fingerprints = HashSet::new();

    for preset in selected_presets {
        if let Err(e) = validate_preset(&preset, PresetSource::OptimizerCandidate) {
            tracing::warn!(
                "Skipping candidate preset {}: validation failed: {}",
                preset.id,
                e
            );
            continue;
        }

        let candidate_input = crate::engine::runtime_config::RuntimeConfigCandidate {
            preset_id: preset.id.clone(),
            preset_args: preset.args.clone(),
            bypass: crate::engine::runtime_config::RuntimeBypassCandidate {
                mode: "all".to_string(),
                domains: Vec::new(),
                kill_switch: false,
            },
            dns: crate::engine::runtime_config::RuntimeDnsCandidate {
                enabled: false,
                protocol: "doh".to_string(),
                provider: None,
                adblock: false,
                cache_enabled: false,
            },
            security: crate::engine::runtime_config::RuntimeSecurityCandidate {
                kill_switch: false,
                binary_integrity_required: false,
            },
        };

        let verified_config = match crate::engine::runtime_config::verify_runtime_config(
            candidate_input,
            crate::engine::runtime_config::ConfigRevision::new(1),
        ) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(
                    "Candidate preset {} config validation failed: {}",
                    preset.id,
                    e
                );
                continue;
            }
        };

        let (prepared, _) =
            match crate::engine::pattern_transaction::prepare_runtime_config_for_transaction(
                verified_config,
                app_data_dir,
            ) {
                Ok(res) => res,
                Err(e) => {
                    tracing::warn!("Candidate preset {} prepare failed: {}", preset.id, e);
                    continue;
                }
            };

        let fingerprint = prepared.verified.fingerprint.to_string();
        if seen_fingerprints.insert(fingerprint.clone()) {
            result.push(OptimizerCandidate {
                preset,
                prepared_config: prepared,
                fingerprint,
            });
        } else {
            tracing::info!("Deduplicated identical candidate preset: {}", preset.id);
        }
    }

    if result.is_empty() {
        return Err(OptimizerError::InvalidCandidateSet);
    }

    Ok(result)
}
