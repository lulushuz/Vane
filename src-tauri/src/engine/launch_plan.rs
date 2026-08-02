use crate::config::preset::Preset;
use crate::engine::sanitizer::validate_preset_args;
use crate::engine::EngineError;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EnginePlatform {
    Windows,
    Linux,
}

impl EnginePlatform {
    pub(crate) const fn current() -> Self {
        if cfg!(target_os = "windows") {
            Self::Windows
        } else {
            Self::Linux
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EngineBinaryKind {
    Winws,
    Nfqws,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EngineBinaryPlan {
    pub executable: PathBuf,
    pub working_directory: PathBuf,
    pub kind: EngineBinaryKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LaunchBypassMode {
    All,
    Whitelist,
    Blacklist,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LaunchBypassInput {
    pub mode: LaunchBypassMode,
    pub domain_list: String,
    pub hostlist_path: Option<PathBuf>,
    pub kill_switch: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TrafficFilterPlan {
    pub declared_tcp_spec: Option<String>,
    pub declared_udp_spec: Option<String>,
    pub effective_linux_tcp_spec: Option<String>,
    pub effective_linux_udp_spec: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum HostlistPlan {
    None,
    Include { path: PathBuf, domain_count: usize },
    Exclude { path: PathBuf, domain_count: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KillSwitchRequirement {
    Disabled,
    Required,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LinuxFirewallBehavior {
    pub tcp_ports: Vec<u16>,
    pub udp_ports: Vec<u16>,
    pub uses_nftables_fallback: bool,
    pub performs_global_process_cleanup: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PlatformLaunchPlan {
    Windows {
        arguments: Vec<String>,
    },
    Linux {
        arguments: Vec<String>,
        queue_number: u16,
        current_firewall_behavior: LinuxFirewallBehavior,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EngineLaunchPlan {
    pub preset_id: String,
    pub binary: EngineBinaryPlan,
    pub hostlist: HostlistPlan,
    pub kill_switch: KillSwitchRequirement,
    pub platform: EnginePlatform,
    pub traffic_filter: TrafficFilterPlan,
    pub platform_launch: PlatformLaunchPlan,
    pub final_arguments: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct EngineLaunchInput<'a> {
    pub preset: &'a Preset,
    pub platform: EnginePlatform,
    pub executable: PathBuf,
    pub bypass: LaunchBypassInput,
}

pub(crate) fn build_engine_launch_plan(
    input: EngineLaunchInput<'_>,
) -> Result<EngineLaunchPlan, EngineError> {
    // 1. Validate preset args using existing sanitizer
    validate_preset_args(&input.preset.args)?;

    // 2. Binary kind and working directory
    let binary_kind = match input.platform {
        EnginePlatform::Windows => EngineBinaryKind::Winws,
        EnginePlatform::Linux => EngineBinaryKind::Nfqws,
    };
    let working_directory = match input.platform {
        EnginePlatform::Windows => input.executable.to_str().and_then(|path| {
            path.rsplit_once(['\\', '/'])
                .map(|(parent, _)| PathBuf::from(parent))
                .or_else(|| (!path.is_empty()).then(PathBuf::new))
        }),
        EnginePlatform::Linux => input
            .executable
            .parent()
            .map(Path::to_path_buf)
            .or_else(|| (!input.executable.as_os_str().is_empty()).then(PathBuf::new)),
    }
    .ok_or_else(|| {
        EngineError::BinaryNotFound(format!(
            "Binary path'in parent klasörü alınamadı: {:?}",
            input.executable
        ))
    })?;

    let binary_plan = EngineBinaryPlan {
        executable: input.executable.clone(),
        working_directory,
        kind: binary_kind,
    };

    // 3. Extract declared traffic filter specs from preset args
    let mut declared_tcp_spec = None;
    let mut declared_udp_spec = None;
    for arg in &input.preset.args {
        if let Some(val) = arg.strip_prefix("--wf-tcp=") {
            declared_tcp_spec = Some(val.to_string());
        } else if let Some(val) = arg.strip_prefix("--wf-udp=") {
            declared_udp_spec = Some(val.to_string());
        }
    }

    // 4. Platform argument preparation & Linux stripping
    let mut prepared_args = Vec::new();
    match input.platform {
        EnginePlatform::Windows => {
            prepared_args.extend(input.preset.args.iter().cloned());
        }
        EnginePlatform::Linux => {
            for arg in &input.preset.args {
                if arg.starts_with("--wf-")
                    || arg.starts_with("--windivert")
                    || arg.starts_with("tcp.")
                    || arg.starts_with("udp.")
                    || arg.starts_with("icmp.")
                {
                    continue;
                }
                prepared_args.push(arg.clone());
            }
        }
    }

    // 5. Hostlist plan & argument addition
    let (hostlist_plan, final_args) = match input.bypass.mode {
        LaunchBypassMode::Whitelist => {
            if input.bypass.domain_list.trim().is_empty() {
                return Err(EngineError::ConfigParseError(
                    "Whitelist mode is selected, but the whitelist has no valid domains. DPI bypass was not started."
                        .to_string(),
                ));
            }
            let path = input.bypass.hostlist_path.ok_or_else(|| {
                EngineError::IoError(
                    "Pattern storage path is unavailable for whitelist hostlist".into(),
                )
            })?;
            let domain_count = input
                .bypass
                .domain_list
                .lines()
                .filter(|line| !line.trim().is_empty())
                .count();

            let mut args = prepared_args;
            args.push(format!("--hostlist={}", path.to_string_lossy()));
            (HostlistPlan::Include { path, domain_count }, args)
        }
        LaunchBypassMode::Blacklist => {
            let path = input.bypass.hostlist_path.ok_or_else(|| {
                EngineError::IoError(
                    "Pattern storage path is unavailable for blacklist hostlist".into(),
                )
            })?;
            let domain_count = input
                .bypass
                .domain_list
                .lines()
                .filter(|line| !line.trim().is_empty())
                .count();

            let mut args = prepared_args;
            args.push(format!("--hostlist-exclude={}", path.to_string_lossy()));
            (HostlistPlan::Exclude { path, domain_count }, args)
        }
        LaunchBypassMode::All => (HostlistPlan::None, prepared_args),
    };

    // 6. Kill Switch requirement
    let kill_switch = if input.bypass.kill_switch {
        KillSwitchRequirement::Required
    } else {
        KillSwitchRequirement::Disabled
    };

    // 7. Traffic filter plan
    let traffic_filter = match input.platform {
        EnginePlatform::Windows => TrafficFilterPlan {
            declared_tcp_spec: declared_tcp_spec.clone(),
            declared_udp_spec: declared_udp_spec.clone(),
            effective_linux_tcp_spec: None,
            effective_linux_udp_spec: None,
        },
        EnginePlatform::Linux => TrafficFilterPlan {
            declared_tcp_spec: declared_tcp_spec.clone(),
            declared_udp_spec: declared_udp_spec.clone(),
            effective_linux_tcp_spec: declared_tcp_spec
                .clone()
                .or_else(|| Some("80,443".to_string())),
            effective_linux_udp_spec: declared_udp_spec.clone(),
        },
    };

    let parsed_tcp_ports = crate::platform::linux::filter_intent::parse_port_spec(
        traffic_filter
            .effective_linux_tcp_spec
            .as_deref()
            .unwrap_or("80,443"),
    )
    .into_iter()
    .map(|r| r.start)
    .collect();

    let parsed_udp_ports = crate::platform::linux::filter_intent::parse_port_spec(
        traffic_filter
            .effective_linux_udp_spec
            .as_deref()
            .unwrap_or(""),
    )
    .into_iter()
    .map(|r| r.start)
    .collect();

    // 8. Platform Launch Plan
    let platform_launch = match input.platform {
        EnginePlatform::Windows => PlatformLaunchPlan::Windows {
            arguments: final_args.clone(),
        },
        EnginePlatform::Linux => PlatformLaunchPlan::Linux {
            arguments: final_args.clone(),
            queue_number: 0,
            current_firewall_behavior: LinuxFirewallBehavior {
                tcp_ports: parsed_tcp_ports,
                udp_ports: parsed_udp_ports,
                uses_nftables_fallback: false,
                performs_global_process_cleanup: false,
            },
        },
    };

    Ok(EngineLaunchPlan {
        preset_id: input.preset.id.clone(),
        binary: binary_plan,
        hostlist: hostlist_plan,
        kill_switch,
        platform: input.platform,
        traffic_filter,
        platform_launch,
        final_arguments: final_args,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_planner_purity_no_side_effects() {
        let preset = Preset {
            id: "default".to_string(),
            label: "Default".to_string(),
            description: "Desc".to_string(),
            icon: "zap".to_string(),
            args: vec![
                "--wf-tcp=80,443".to_string(),
                "--dpi-desync=fake".to_string(),
            ],
            is_custom: false,
            priority: 1,
            category: Default::default(),
        };

        let input = EngineLaunchInput {
            preset: &preset,
            platform: EnginePlatform::Windows,
            executable: PathBuf::from("C:\\Program Files\\Vane\\winws.exe"),
            bypass: LaunchBypassInput {
                mode: LaunchBypassMode::All,
                domain_list: String::new(),
                hostlist_path: None,
                kill_switch: false,
            },
        };

        let plan = build_engine_launch_plan(input).unwrap();
        assert_eq!(plan.preset_id, "default");
        assert_eq!(plan.binary.kind, EngineBinaryKind::Winws);
        assert_eq!(
            plan.binary.working_directory,
            PathBuf::from("C:\\Program Files\\Vane")
        );
        assert_eq!(
            plan.final_arguments,
            vec!["--wf-tcp=80,443", "--dpi-desync=fake"]
        );
    }
}
