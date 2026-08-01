use crate::config::preset::Preset;
use crate::engine::launch_plan::{LaunchBypassInput, LaunchBypassMode};
use crate::engine::EngineError;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum RuntimeBypassMode {
    All,
    Whitelist,
    Blacklist,
}

impl From<RuntimeBypassMode> for LaunchBypassMode {
    fn from(mode: RuntimeBypassMode) -> Self {
        match mode {
            RuntimeBypassMode::All => LaunchBypassMode::All,
            RuntimeBypassMode::Whitelist => LaunchBypassMode::Whitelist,
            RuntimeBypassMode::Blacklist => LaunchBypassMode::Blacklist,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuntimeBypassCandidate {
    pub mode: String,
    pub domains: Vec<String>,
    pub kill_switch: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuntimeDnsCandidate {
    pub enabled: bool,
    pub protocol: String,
    pub provider: Option<String>,
    pub adblock: bool,
    pub cache_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuntimeSecurityCandidate {
    pub kill_switch: bool,
    pub binary_integrity_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuntimeConfigCandidate {
    pub preset_id: String,
    pub preset_args: Vec<String>,
    pub bypass: RuntimeBypassCandidate,
    pub dns: RuntimeDnsCandidate,
    pub security: RuntimeSecurityCandidate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ConfigRevision(u64);

impl ConfigRevision {
    pub(crate) const fn new(value: u64) -> Self {
        Self(value)
    }

    pub(crate) const fn get(self) -> u64 {
        self.0
    }

    #[allow(dead_code)]
    pub(crate) fn checked_next(self) -> Result<Self, RuntimeConfigError> {
        self.0
            .checked_add(1)
            .map(Self)
            .ok_or(RuntimeConfigError::RevisionOverflow)
    }

    pub(crate) fn next(self) -> Result<Self, RuntimeConfigError> {
        self.checked_next()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ConfigFingerprint(String);

impl ConfigFingerprint {
    #[allow(dead_code)]
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn prefix(&self, len: usize) -> &str {
        if self.0.len() >= len {
            &self.0[..len]
        } else {
            &self.0
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum RuntimeDnsProtocol {
    Doh,
    Dot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VerifiedPresetConfig {
    pub id: String,
    pub arguments: Vec<String>,
}

impl VerifiedPresetConfig {
    pub(crate) fn to_preset(&self) -> crate::config::preset::Preset {
        crate::config::preset::Preset {
            id: self.id.clone(),
            label: self.id.clone(),
            description: String::new(),
            icon: String::new(),
            args: self.arguments.clone(),
            is_custom: true,
            priority: 0,
            category: crate::config::preset::PresetCategory::Custom,
        }
    }
}

impl std::fmt::Display for RuntimeBypassMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeBypassMode::All => write!(f, "all"),
            RuntimeBypassMode::Whitelist => write!(f, "whitelist"),
            RuntimeBypassMode::Blacklist => write!(f, "blacklist"),
        }
    }
}

impl std::fmt::Display for ConfigFingerprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VerifiedBypassConfig {
    pub mode: RuntimeBypassMode,
    pub domains: Vec<String>,
    pub domain_count: usize,
    pub kill_switch: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VerifiedDnsConfig {
    pub enabled: bool,
    pub protocol: RuntimeDnsProtocol,
    pub provider: Option<String>,
    pub adblock: bool,
    pub cache_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VerifiedSecurityConfig {
    pub kill_switch: bool,
    pub binary_integrity_required: bool,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct VerifiedRuntimeConfig {
    pub revision: ConfigRevision,
    pub fingerprint: ConfigFingerprint,
    pub preset: VerifiedPresetConfig,
    pub bypass: VerifiedBypassConfig,
    pub dns: VerifiedDnsConfig,
    pub security: VerifiedSecurityConfig,
}

impl VerifiedRuntimeConfig {
    pub(crate) fn to_launch_bypass_input(
        &self,
        hostlist_path: Option<PathBuf>,
    ) -> LaunchBypassInput {
        LaunchBypassInput {
            mode: self.bypass.mode.into(),
            domain_list: self.bypass.domains.join("\n"),
            hostlist_path,
            kill_switch: self.bypass.kill_switch,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn summary(&self) -> RuntimeConfigSummary {
        RuntimeConfigSummary {
            revision: self.revision.get(),
            fingerprint_prefix: self.fingerprint.prefix(8).to_string(),
            preset_id: self.preset.id.clone(),
            bypass_mode: self.bypass.mode,
            domain_count: self.bypass.domain_count,
            dns_protocol: self.dns.protocol,
            kill_switch: self.bypass.kill_switch,
        }
    }
}

impl std::fmt::Debug for VerifiedRuntimeConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VerifiedRuntimeConfig")
            .field("revision", &self.revision.get())
            .field("fingerprint_prefix", &self.fingerprint.prefix(8))
            .field("preset_id", &self.preset.id)
            .field("bypass_mode", &self.bypass.mode)
            .field("domain_count", &self.bypass.domain_count)
            .field("dns_protocol", &self.dns.protocol)
            .field("kill_switch", &self.bypass.kill_switch)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PreparedHostlist {
    NotRequired,
    Planned { path: PathBuf, domain_count: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreparedRuntimeConfig {
    pub verified: VerifiedRuntimeConfig,
    pub launch_plan: crate::engine::launch_plan::EngineLaunchPlan,
    pub hostlist: PreparedHostlist,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AppliedVerification {
    ProcessStarted,
    ProcessAlive,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AppliedRuntimeConfig {
    pub verified: VerifiedRuntimeConfig,
    pub process_id: u32,
    pub applied_at: std::time::SystemTime,
    pub verification: AppliedVerification,
}

impl AppliedRuntimeConfig {
    #[allow(dead_code)]
    pub(crate) fn process_started(verified: VerifiedRuntimeConfig, pid: u32) -> Self {
        Self {
            verified,
            process_id: pid,
            applied_at: std::time::SystemTime::now(),
            verification: AppliedVerification::ProcessStarted,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuntimeConfigSummary {
    pub revision: u64,
    pub fingerprint_prefix: String,
    pub preset_id: String,
    pub bypass_mode: RuntimeBypassMode,
    pub domain_count: usize,
    pub dns_protocol: RuntimeDnsProtocol,
    pub kill_switch: bool,
}

#[allow(dead_code)]
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub(crate) enum RuntimeConfigError {
    #[error("unsupported bypass mode: {0}")]
    UnsupportedBypassMode(String),

    #[error("whitelist mode requires at least one valid domain")]
    EmptyWhitelist,

    #[error("invalid domain configuration: {0}")]
    InvalidDomains(String),

    #[error("invalid preset configuration: {0}")]
    InvalidPreset(String),

    #[error("unsupported DNS protocol: {0}")]
    UnsupportedDnsProtocol(String),

    #[error("configuration revision overflow")]
    RevisionOverflow,

    #[error("configuration fingerprint failed: {0}")]
    FingerprintFailure(String),
}

impl From<RuntimeConfigError> for EngineError {
    fn from(err: RuntimeConfigError) -> Self {
        EngineError::ConfigParseError(err.to_string())
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn compute_config_fingerprint(
    preset_id: &str,
    preset_args: &[String],
    bypass_mode: RuntimeBypassMode,
    canonical_domains: &[String],
    kill_switch: bool,
    dns_enabled: bool,
    dns_protocol: RuntimeDnsProtocol,
    dns_provider: Option<&str>,
    dns_adblock: bool,
    dns_cache: bool,
) -> ConfigFingerprint {
    use sha2::{Digest, Sha256};

    let mode_str = match bypass_mode {
        RuntimeBypassMode::All => "all",
        RuntimeBypassMode::Whitelist => "whitelist",
        RuntimeBypassMode::Blacklist => "blacklist",
    };

    let proto_str = match dns_protocol {
        RuntimeDnsProtocol::Doh => "doh",
        RuntimeDnsProtocol::Dot => "dot",
    };

    let mut sorted_domains = canonical_domains.to_vec();
    sorted_domains.sort();

    let mut hasher = Sha256::new();
    hasher.update(b"schema:1;");
    hasher.update(format!("preset_id:{preset_id};").as_bytes());
    hasher.update(format!("args:{};", preset_args.join(",")).as_bytes());
    hasher.update(format!("mode:{mode_str};").as_bytes());
    hasher.update(format!("domains:{};", sorted_domains.join(",")).as_bytes());
    hasher.update(format!("ks:{kill_switch};").as_bytes());
    hasher.update(format!("dns_en:{dns_enabled};").as_bytes());
    hasher.update(format!("dns_proto:{proto_str};").as_bytes());
    hasher.update(format!("dns_prov:{};", dns_provider.unwrap_or("")).as_bytes());
    hasher.update(format!("dns_ab:{dns_adblock};").as_bytes());
    hasher.update(format!("dns_cache:{dns_cache};").as_bytes());

    let result = hasher.finalize();
    ConfigFingerprint(format!("{:x}", result))
}

pub(crate) fn verify_runtime_config(
    candidate: RuntimeConfigCandidate,
    revision: ConfigRevision,
) -> Result<VerifiedRuntimeConfig, RuntimeConfigError> {
    let dummy_preset = crate::config::preset::Preset {
        id: candidate.preset_id.clone(),
        label: candidate.preset_id.clone(),
        description: String::new(),
        icon: String::new(),
        args: candidate.preset_args.clone(),
        is_custom: true,
        priority: 0,
        category: Default::default(),
    };
    crate::config::validator::validate_preset(
        &dummy_preset,
        crate::config::validator::PresetSource::Custom,
    )
    .map_err(|e| RuntimeConfigError::InvalidPreset(e.to_string()))?;

    let bypass_mode = match candidate.bypass.mode.as_str() {
        "all" => RuntimeBypassMode::All,
        "whitelist" => RuntimeBypassMode::Whitelist,
        "blacklist" => RuntimeBypassMode::Blacklist,
        other => return Err(RuntimeConfigError::UnsupportedBypassMode(other.to_string())),
    };

    let canonical_domains =
        crate::config::domain::canonicalize_domain_rules(&candidate.bypass.domains)
            .map_err(|e| RuntimeConfigError::InvalidDomains(e.to_string()))?;

    let domain_count = canonical_domains.len();

    if bypass_mode == RuntimeBypassMode::Whitelist && domain_count == 0 {
        return Err(RuntimeConfigError::EmptyWhitelist);
    }

    let dns_protocol = match candidate.dns.protocol.to_lowercase().as_str() {
        "doh" | "doq" => RuntimeDnsProtocol::Doh,
        "dot" => RuntimeDnsProtocol::Dot,
        other => {
            return Err(RuntimeConfigError::UnsupportedDnsProtocol(
                other.to_string(),
            ))
        }
    };

    let fingerprint = compute_config_fingerprint(
        &candidate.preset_id,
        &candidate.preset_args,
        bypass_mode,
        &canonical_domains,
        candidate.bypass.kill_switch,
        candidate.dns.enabled,
        dns_protocol,
        candidate.dns.provider.as_deref(),
        candidate.dns.adblock,
        candidate.dns.cache_enabled,
    );

    let verified_preset = VerifiedPresetConfig {
        id: candidate.preset_id,
        arguments: candidate.preset_args,
    };

    let verified_bypass = VerifiedBypassConfig {
        mode: bypass_mode,
        domains: canonical_domains,
        domain_count,
        kill_switch: candidate.bypass.kill_switch,
    };

    let verified_dns = VerifiedDnsConfig {
        enabled: candidate.dns.enabled,
        protocol: dns_protocol,
        provider: candidate.dns.provider,
        adblock: candidate.dns.adblock,
        cache_enabled: candidate.dns.cache_enabled,
    };

    let verified_security = VerifiedSecurityConfig {
        kill_switch: candidate.security.kill_switch,
        binary_integrity_required: candidate.security.binary_integrity_required,
    };

    Ok(VerifiedRuntimeConfig {
        revision,
        fingerprint,
        preset: verified_preset,
        bypass: verified_bypass,
        dns: verified_dns,
        security: verified_security,
    })
}

pub(crate) fn candidate_from_preset_and_sources(
    preset: &Preset,
    bypass_mode: &str,
    domain_list: &str,
    kill_switch: bool,
) -> RuntimeConfigCandidate {
    let domains: Vec<String> = domain_list
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect();

    RuntimeConfigCandidate {
        preset_id: preset.id.clone(),
        preset_args: preset.args.clone(),
        bypass: RuntimeBypassCandidate {
            mode: bypass_mode.to_string(),
            domains,
            kill_switch,
        },
        dns: RuntimeDnsCandidate {
            enabled: true,
            protocol: "doh".to_string(),
            provider: Some("cloudflare".to_string()),
            adblock: true,
            cache_enabled: true,
        },
        security: RuntimeSecurityCandidate {
            kill_switch,
            binary_integrity_required: true,
        },
    }
}
