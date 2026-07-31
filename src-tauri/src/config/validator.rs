#![allow(dead_code)]

use std::collections::HashSet;
use std::fmt;

use crate::config::preset::{Preset, PresetCategory};

pub const MAX_DESYNC_METHODS: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum PresetSource {
    BuiltIn,
    Custom,
    ImportedVane,
    ImportedLegacyJson,
    RemoteSigned,
    OptimizerCandidate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum PresetPlatform {
    Windows,
    Linux,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) enum PlatformSupport {
    Supported,
    Experimental,
    Unsupported { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct PlatformSupportMatrix {
    pub windows: PlatformSupport,
    pub linux: PlatformSupport,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityStatus {
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl CapabilityStatus {
    pub fn supported() -> Self {
        Self {
            state: "supported".into(),
            reason: None,
        }
    }

    pub fn experimental(reason: impl Into<String>) -> Self {
        Self {
            state: "experimental".into(),
            reason: Some(reason.into()),
        }
    }

    pub fn unsupported(reason: impl Into<String>) -> Self {
        Self {
            state: "unsupported".into(),
            reason: Some(reason.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrafficCapabilities {
    pub tcp_filtering: CapabilityStatus,
    pub udp_filtering: CapabilityStatus,
    pub custom_tcp_ports: CapabilityStatus,
    pub custom_udp_ports: CapabilityStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OptionCapabilities {
    pub auto_ttl: CapabilityStatus,
    pub fixed_ttl: CapabilityStatus,
    pub repeats: CapabilityStatus,
    pub fooling: CapabilityStatus,
    pub split_position: CapabilityStatus,
    pub window_size: CapabilityStatus,
    pub mss: CapabilityStatus,
    pub fake_payload: CapabilityStatus,
    pub fake_tls_sni: CapabilityStatus,
    pub bind_address: CapabilityStatus,
    pub ipset: CapabilityStatus,
    pub tpws: CapabilityStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdvancedCapabilities {
    pub platform: String,
    pub methods: std::collections::HashMap<String, CapabilityStatus>,
    pub traffic: TrafficCapabilities,
    pub options: OptionCapabilities,
}

impl AdvancedCapabilities {
    pub fn for_current_platform() -> Self {
        let is_windows = cfg!(target_os = "windows");
        let platform = if is_windows { "windows" } else { "linux" };

        let mut methods = std::collections::HashMap::new();
        let all_methods = vec![
            "syndata",
            "rst",
            "rstack",
            "fake",
            "fakeknown",
            "split",
            "split2",
            "multisplit",
            "disorder",
            "multidisorder",
            "hostfake",
            "fakedsplit",
            "destopt",
            "ipfrag1",
            "ipfrag2",
            "udplen",
            "tamper",
            "none",
        ];

        for m in all_methods {
            methods.insert(m.to_string(), CapabilityStatus::supported());
        }

        let traffic = if is_windows {
            TrafficCapabilities {
                tcp_filtering: CapabilityStatus::supported(),
                udp_filtering: CapabilityStatus::supported(),
                custom_tcp_ports: CapabilityStatus::supported(),
                custom_udp_ports: CapabilityStatus::supported(),
            }
        } else {
            let exp_msg = "Experimental — automated plan/executor tests passed; pending privileged live acceptance";
            TrafficCapabilities {
                tcp_filtering: CapabilityStatus::experimental(exp_msg),
                udp_filtering: CapabilityStatus::experimental(exp_msg),
                custom_tcp_ports: CapabilityStatus::experimental(exp_msg),
                custom_udp_ports: CapabilityStatus::experimental(exp_msg),
            }
        };

        let not_supported_msg = "Not supported by bundled DPI engine";

        let options = OptionCapabilities {
            auto_ttl: CapabilityStatus::supported(),
            fixed_ttl: CapabilityStatus::supported(),
            repeats: CapabilityStatus::supported(),
            fooling: CapabilityStatus::supported(),
            split_position: CapabilityStatus::supported(),
            window_size: CapabilityStatus::supported(),
            mss: CapabilityStatus::unsupported(not_supported_msg),
            fake_payload: CapabilityStatus::unsupported(not_supported_msg),
            fake_tls_sni: CapabilityStatus::unsupported(not_supported_msg),
            bind_address: CapabilityStatus::unsupported(not_supported_msg),
            ipset: CapabilityStatus::unsupported(not_supported_msg),
            tpws: CapabilityStatus::unsupported(not_supported_msg),
        };

        Self {
            platform: platform.to_string(),
            methods,
            traffic,
            options,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum DesyncPhase {
    Phase0 = 0,
    Phase1 = 1,
    Phase2 = 2,
}

impl fmt::Display for DesyncPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Phase0 => write!(f, "Phase0(SYN/SYN-ACK)"),
            Self::Phase1 => write!(f, "Phase1(Payload/Split)"),
            Self::Phase2 => write!(f, "Phase2(Option/Post-handshake)"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum DesyncMethod {
    Syndata,
    Fake,
    FakeKnown,
    Split,
    Split2,
    MultiSplit,
    Disorder,
    MultiDisorder,
    HostFake,
    FakedSplit,
    Rst,
    Rstack,
    DestOpt,
    IpFrag1,
    IpFrag2,
    UdpLen,
    Tamper,
    NoneMethod,
}

impl DesyncMethod {
    pub(crate) fn parse(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "syndata" => Some(Self::Syndata),
            "fake" => Some(Self::Fake),
            "fakeknown" => Some(Self::FakeKnown),
            "split" => Some(Self::Split),
            "split2" => Some(Self::Split2),
            "multisplit" => Some(Self::MultiSplit),
            "disorder" => Some(Self::Disorder),
            "multidisorder" => Some(Self::MultiDisorder),
            "hostfake" => Some(Self::HostFake),
            "fakedsplit" => Some(Self::FakedSplit),
            "rst" => Some(Self::Rst),
            "rstack" => Some(Self::Rstack),
            "destopt" => Some(Self::DestOpt),
            "ipfrag1" => Some(Self::IpFrag1),
            "ipfrag2" => Some(Self::IpFrag2),
            "udplen" => Some(Self::UdpLen),
            "tamper" => Some(Self::Tamper),
            "none" => Some(Self::NoneMethod),
            _ => None,
        }
    }

    pub(crate) fn phase(&self) -> DesyncPhase {
        match self {
            Self::Syndata | Self::Rst | Self::Rstack => DesyncPhase::Phase0,
            Self::Fake
            | Self::FakeKnown
            | Self::Split
            | Self::Split2
            | Self::MultiSplit
            | Self::Disorder
            | Self::MultiDisorder
            | Self::HostFake
            | Self::FakedSplit => DesyncPhase::Phase1,
            Self::DestOpt
            | Self::IpFrag1
            | Self::IpFrag2
            | Self::UdpLen
            | Self::Tamper
            | Self::NoneMethod => DesyncPhase::Phase2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum PresetValidationError {
    #[error("invalid preset ID: {0}")]
    InvalidId(String),
    #[error("preset argument limit exceeded: {count} > {max}")]
    TooManyArguments { count: usize, max: usize },
    #[error("preset argument too long ({len} chars > {max} limit): {arg}")]
    ArgumentTooLong { arg: String, len: usize, max: usize },
    #[error("unsafe character '{ch}' in argument: {arg}")]
    UnsafeArgument { arg: String, ch: char },
    #[error("unknown or forbidden argument: {arg}")]
    UnknownArgument { arg: String },
    #[error("duplicate single-value argument: {arg}")]
    DuplicateArgument { arg: String },
    #[error("unknown desync method '{method}' in argument '{arg}'")]
    UnknownDesyncMethod { arg: String, method: String },
    #[error("too many desync methods ({count} > {max})")]
    TooManyDesyncMethods { count: usize, max: usize },
    #[error("invalid desync phase sequence in '--dpi-desync={spec}': {prev_method} ({prev_phase}) precedes {next_method} ({next_phase})")]
    InvalidPhaseOrder {
        spec: String,
        prev_method: String,
        prev_phase: DesyncPhase,
        next_method: String,
        next_phase: DesyncPhase,
    },
    #[error("duplicate desync method '{method}' in argument '{arg}'")]
    DuplicateDesyncMethod { arg: String, method: String },
    #[error("desync method 'none' cannot be combined with other methods in '{arg}'")]
    NoneCombinedWithStrategy { arg: String },
    #[error("conflicting TTL options: both fixed TTL and auto-TTL specified")]
    ConflictingTtl,
    #[error("dangling split position argument without a split-based desync method")]
    DanglingSplitPosition,
    #[error("dangling repeats argument without a desync method")]
    DanglingRepeats,
    #[error("preset contains forbidden hostlist path argument: {arg}")]
    ForbiddenHostlistPath { arg: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VerifiedPreset {
    pub id: String,
    pub label: String,
    pub description: String,
    pub icon: String,
    pub arguments: Vec<String>,
    pub is_custom: bool,
    pub priority: u8,
    pub category: PresetCategory,
    pub source: PresetSource,
    pub supported_platforms: PlatformSupportMatrix,
    pub parsed_desync_methods: Vec<DesyncMethod>,
}

pub(crate) fn validate_preset(
    preset: &Preset,
    source: PresetSource,
) -> Result<VerifiedPreset, PresetValidationError> {
    // 1. Structural ID check
    if preset.id.is_empty()
        || !preset
            .id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(PresetValidationError::InvalidId(preset.id.clone()));
    }

    // 2. Argument count & length limits
    if preset.args.len() > 30 {
        return Err(PresetValidationError::TooManyArguments {
            count: preset.args.len(),
            max: 30,
        });
    }

    let forbidden_chars: &[char] = &[
        '&', ';', '|', '`', '$', '<', '>', '\'', '"', '\\', '/', '\n', '\r', '\0',
    ];

    let mut seen_prefixes: HashSet<String> = HashSet::new();
    let mut parsed_methods: Vec<DesyncMethod> = Vec::new();
    let mut has_auto_ttl = false;
    let mut has_fixed_ttl = false;
    let mut has_split_pos = false;
    let mut has_repeats = false;
    let mut has_desync_spec = false;
    let mut has_split_method = false;
    let mut is_udp_quic_only = false;
    let mut has_custom_ports = false;

    for arg in &preset.args {
        if arg.len() > 128 {
            return Err(PresetValidationError::ArgumentTooLong {
                arg: arg.clone(),
                len: arg.len(),
                max: 128,
            });
        }
        if arg.is_empty() {
            return Err(PresetValidationError::UnknownArgument { arg: arg.clone() });
        }

        for &ch in forbidden_chars {
            if arg.contains(ch) {
                return Err(PresetValidationError::UnsafeArgument {
                    arg: arg.clone(),
                    ch,
                });
            }
        }

        if arg.starts_with("--hostlist=") || arg.starts_with("--hostlist-exclude=") {
            return Err(PresetValidationError::ForbiddenHostlistPath { arg: arg.clone() });
        }

        // Single value duplicate check
        let prefix = arg.split('=').next().unwrap_or(arg).to_string();
        if (prefix == "--dpi-desync"
            || prefix == "--wf-tcp"
            || prefix == "--wf-udp"
            || prefix == "--dpi-desync-split-pos"
            || prefix == "--dpi-desync-ttl")
            && !seen_prefixes.insert(prefix.clone())
        {
            return Err(PresetValidationError::DuplicateArgument { arg: arg.clone() });
        }

        if arg == "--dpi-desync-autottl" {
            has_auto_ttl = true;
        }
        if arg.starts_with("--dpi-desync-ttl=") {
            has_fixed_ttl = true;
        }
        if arg.starts_with("--dpi-desync-split-pos=") {
            has_split_pos = true;
        }
        if arg.starts_with("--dpi-desync-repeats=") {
            has_repeats = true;
        }

        if let Some(spec) = arg.strip_prefix("--dpi-desync=") {
            has_desync_spec = true;
            let raw_methods: Vec<&str> = spec.split(',').map(|s| s.trim()).collect();

            if raw_methods.len() > MAX_DESYNC_METHODS {
                return Err(PresetValidationError::TooManyDesyncMethods {
                    count: raw_methods.len(),
                    max: MAX_DESYNC_METHODS,
                });
            }

            let mut method_list: Vec<DesyncMethod> = Vec::new();
            let mut seen_methods_in_arg: HashSet<DesyncMethod> = HashSet::new();

            for raw_m in &raw_methods {
                let m = DesyncMethod::parse(raw_m).ok_or_else(|| {
                    PresetValidationError::UnknownDesyncMethod {
                        arg: arg.clone(),
                        method: raw_m.to_string(),
                    }
                })?;

                if m == DesyncMethod::NoneMethod && raw_methods.len() > 1 {
                    return Err(PresetValidationError::NoneCombinedWithStrategy {
                        arg: arg.clone(),
                    });
                }

                if !seen_methods_in_arg.insert(m.clone()) {
                    return Err(PresetValidationError::DuplicateDesyncMethod {
                        arg: arg.clone(),
                        method: raw_m.to_string(),
                    });
                }

                if matches!(
                    m,
                    DesyncMethod::Split
                        | DesyncMethod::Split2
                        | DesyncMethod::MultiSplit
                        | DesyncMethod::Disorder
                        | DesyncMethod::MultiDisorder
                        | DesyncMethod::FakedSplit
                ) {
                    has_split_method = true;
                }

                method_list.push(m);
            }

            // Phase order check
            for i in 0..method_list.len().saturating_sub(1) {
                let curr = &method_list[i];
                let next = &method_list[i + 1];

                if curr.phase() > next.phase() {
                    return Err(PresetValidationError::InvalidPhaseOrder {
                        spec: spec.to_string(),
                        prev_method: raw_methods[i].to_string(),
                        prev_phase: curr.phase(),
                        next_method: raw_methods[i + 1].to_string(),
                        next_phase: next.phase(),
                    });
                }
            }

            parsed_methods.extend(method_list);
        }

        if let Some(ports) = arg.strip_prefix("--wf-udp=") {
            if ports == "443" || ports == "50000-65535" {
                if !preset.args.iter().any(|a| a.starts_with("--wf-tcp=")) {
                    is_udp_quic_only = true;
                }
            } else {
                has_custom_ports = true;
            }
        }
    }

    // 3. Cross-argument compatibility checks
    if has_auto_ttl && has_fixed_ttl {
        return Err(PresetValidationError::ConflictingTtl);
    }
    if has_split_pos && !has_split_method {
        return Err(PresetValidationError::DanglingSplitPosition);
    }
    if has_repeats && !has_desync_spec {
        return Err(PresetValidationError::DanglingRepeats);
    }

    // 4. Platform capability analysis
    let linux_support = if is_udp_quic_only {
        PlatformSupport::Unsupported {
            reason: "Linux effective UDP packet filtering is not yet supported in current kernel netfilter rules".into(),
        }
    } else if has_custom_ports {
        PlatformSupport::Experimental
    } else {
        PlatformSupport::Supported
    };

    let supported_platforms = PlatformSupportMatrix {
        windows: PlatformSupport::Supported,
        linux: linux_support,
    };

    Ok(VerifiedPreset {
        id: preset.id.clone(),
        label: preset.label.clone(),
        description: preset.description.clone(),
        icon: preset.icon.clone(),
        arguments: preset.args.clone(),
        is_custom: preset.is_custom,
        priority: preset.priority,
        category: preset.category.clone(),
        source,
        supported_platforms,
        parsed_desync_methods: parsed_methods,
    })
}
