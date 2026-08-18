use serde::{Deserialize, Serialize};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
#[cfg(target_os = "windows")]
use std::process::Command;
use tauri::{AppHandle, Manager};

/// Constant for CREATE_NO_WINDOW flag on Windows to prevent console window flashing.
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Known trusted DNS providers
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsProvider {
    pub id: String,
    pub name: String,
    pub primary: String,
    pub secondary: String,
    pub emoji: String,
    pub description: String,
}

/// Windows network adapter
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkAdapter {
    pub name: String,
    pub current_primary_dns: Option<String>,
    pub current_secondary_dns: Option<String>,
    pub is_dhcp: bool,
}

/// DNS modification result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyDnsResult {
    pub success: bool,
    pub applied_adapters: Vec<String>,
    pub error: Option<String>,
}

const DNS_SNAPSHOT_FILE: &str = "dns_restore_snapshot.json";

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PersistedDnsSnapshot {
    version: u8,
    adapters: Vec<NetworkAdapter>,
}

fn dns_snapshot_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|directory| directory.join(DNS_SNAPSHOT_FILE))
        .map_err(|error| format!("DNS recovery path could not be resolved: {error}"))
}

pub fn save_dns_restore_snapshot(
    app: &AppHandle,
    adapters: &[NetworkAdapter],
) -> Result<(), String> {
    if adapters.is_empty() {
        return Err(
            "The current DNS configuration is empty; recovery snapshot was not written.".into(),
        );
    }
    let path = dns_snapshot_path(app)?;
    if path.exists() {
        let existing = std::fs::read(&path).map_err(|error| {
            format!("Existing DNS recovery snapshot could not be read: {error}")
        })?;
        let existing: PersistedDnsSnapshot = serde_json::from_slice(&existing)
            .map_err(|error| format!("Existing DNS recovery snapshot is corrupt: {error}"))?;
        if existing.version != 1 || existing.adapters.is_empty() {
            return Err(
                "Existing DNS recovery snapshot has an unsupported or empty format.".into(),
            );
        }
        return Ok(());
    }
    let payload = serde_json::to_vec_pretty(&PersistedDnsSnapshot {
        version: 1,
        adapters: adapters.to_vec(),
    })
    .map_err(|error| format!("DNS recovery snapshot could not be serialized: {error}"))?;
    crate::settings::atomic_replace_bytes(&path, &payload)
        .map_err(|error| format!("DNS recovery snapshot could not be persisted: {error}"))
}

pub fn clear_dns_restore_snapshot(app: &AppHandle) -> Result<(), String> {
    let path = dns_snapshot_path(app)?;
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!(
            "DNS recovery snapshot could not be removed: {error}"
        )),
    }
}

pub fn load_dns_restore_snapshot(app: &AppHandle) -> Result<Option<Vec<NetworkAdapter>>, String> {
    let path = dns_snapshot_path(app)?;
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("DNS recovery snapshot could not be read: {error}")),
    };
    let snapshot: PersistedDnsSnapshot = serde_json::from_slice(&bytes)
        .map_err(|error| format!("DNS recovery snapshot is corrupt: {error}"))?;
    if snapshot.version != 1 || snapshot.adapters.is_empty() {
        return Err("DNS recovery snapshot has an unsupported or empty format.".into());
    }
    Ok(Some(snapshot.adapters))
}

pub fn recover_stale_dns_snapshot(app: &AppHandle) -> Result<bool, String> {
    let path = dns_snapshot_path(app)?;
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(format!("DNS recovery snapshot could not be read: {error}")),
    };
    let snapshot: PersistedDnsSnapshot = serde_json::from_slice(&bytes)
        .map_err(|error| format!("DNS recovery snapshot is corrupt: {error}"))?;
    if snapshot.version != 1 || snapshot.adapters.is_empty() {
        return Err("DNS recovery snapshot has an unsupported or empty format.".into());
    }
    let restored = restore_dns_snapshot(&snapshot.adapters);
    if !restored.success {
        return Err(format!(
            "Saved DNS configuration could not be restored: {:?}",
            restored.error
        ));
    }
    clear_dns_restore_snapshot(app)?;
    Ok(true)
}

#[derive(Debug, thiserror::Error)]
pub enum DnsError {
    #[error("Komut çalıştırılamadı: {0}")]
    CommandFailed(String),
    #[error("Adaptör bulunamadı")]
    NoAdapterFound,
    #[error("Yetki hatası")]
    PermissionDenied,
}

pub fn builtin_providers() -> Vec<DnsProvider> {
    vec![
        DnsProvider {
            id: "cloudflare".into(),
            name: "Cloudflare".into(),
            primary: "1.1.1.1".into(),
            secondary: "1.0.0.1".into(),
            emoji: "🌩️".into(),
            description: "Speed & Privacy - Fastest DNS".into(),
        },
        DnsProvider {
            id: "google".into(),
            name: "Google".into(),
            primary: "8.8.8.8".into(),
            secondary: "8.8.4.4".into(),
            emoji: "🔵".into(),
            description: "Reliable & Stable".into(),
        },
        DnsProvider {
            id: "quad9".into(),
            name: "Quad9".into(),
            primary: "9.9.9.9".into(),
            secondary: "149.112.112.112".into(),
            emoji: "9️⃣".into(),
            description: "Security Focused - Blocks Malware".into(),
        },
        DnsProvider {
            id: "opendns".into(),
            name: "OpenDNS".into(),
            primary: "208.67.222.222".into(),
            secondary: "208.67.220.220".into(),
            emoji: "☁️".into(),
            description: "Filtering & Security".into(),
        },
        DnsProvider {
            id: "adguard".into(),
            name: "AdGuard".into(),
            primary: "94.140.14.14".into(),
            secondary: "94.140.15.15".into(),
            emoji: "🛡️".into(),
            description: "Blocks Ads & Trackers".into(),
        },
        DnsProvider {
            id: "nextdns".into(),
            name: "NextDNS".into(),
            primary: "45.90.28.167".into(),
            secondary: "45.90.30.167".into(),
            emoji: "⏩".into(),
            description: "Block Ads & Trackers".into(),
        },
        DnsProvider {
            id: "yandex".into(),
            name: "Yandex".into(),
            primary: "77.88.8.8".into(),
            secondary: "77.88.8.1".into(),
            emoji: "🔴".into(),
            description: "Fast & Reliable".into(),
        },
        DnsProvider {
            id: "mullvad".into(),
            name: "Mullvad".into(),
            primary: "194.242.2.4".into(),
            secondary: "194.242.2.5".into(),
            emoji: "🔐".into(),
            description: "Privacy First (No Logging)".into(),
        },
    ]
}

/// Reads active adapters through PowerShell networking cmdlets. Their object
/// properties are stable across Windows display languages, unlike netsh text.
#[cfg(target_os = "windows")]
pub fn get_active_adapters() -> Vec<NetworkAdapter> {
    let script = r#"$routed = @(Get-NetRoute -DestinationPrefix '0.0.0.0/0' -AddressFamily IPv4 -ErrorAction SilentlyContinue | Select-Object -ExpandProperty InterfaceIndex); @(Get-NetAdapter | Where-Object { $_.Status -eq 'Up' -and ($routed -contains $_.ifIndex -or ($routed.Count -eq 0 -and $_.InterfaceDescription -notmatch 'VirtualBox|VMware|Hyper-V|Loopback|Npcap|TAP|Wintun')) } | ForEach-Object { $i = $_; $d = @((Get-DnsClientServerAddress -InterfaceIndex $i.ifIndex -AddressFamily IPv4 -ErrorAction SilentlyContinue).ServerAddresses); $dhcp = (Get-NetIPInterface -InterfaceIndex $i.ifIndex -AddressFamily IPv4 -ErrorAction SilentlyContinue).Dhcp -eq 'Enabled'; [pscustomobject]@{ name = $i.Name; dns = $d; dhcp = $dhcp } }) | ConvertTo-Json -Compress"#;
    let output = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    let Ok(out) = output else {
        return vec![];
    };
    if !out.status.success() {
        tracing::error!(
            "Windows adapter discovery failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        return vec![];
    }
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&out.stdout) else {
        tracing::error!("Windows adapter discovery returned invalid JSON.");
        return vec![];
    };
    let items: Vec<&serde_json::Value> = match &value {
        serde_json::Value::Array(values) => values.iter().collect(),
        serde_json::Value::Object(_) => vec![&value],
        _ => vec![],
    };
    items
        .into_iter()
        .filter_map(|item| {
            let name = item.get("name")?.as_str()?.to_string();
            let dns: Vec<String> = match item.get("dns") {
                Some(serde_json::Value::Array(values)) => values
                    .iter()
                    .filter_map(|value| value.as_str().map(str::to_owned))
                    .collect(),
                Some(serde_json::Value::String(value)) => vec![value.clone()],
                _ => vec![],
            };
            Some(NetworkAdapter {
                name,
                current_primary_dns: dns.first().cloned(),
                current_secondary_dns: dns.get(1).cloned(),
                is_dhcp: item
                    .get("dhcp")
                    .and_then(|value| value.as_bool())
                    .unwrap_or(true),
            })
        })
        .collect()
}

#[cfg(not(target_os = "windows"))]
pub fn get_active_adapters() -> Vec<NetworkAdapter> {
    let mut adapters = vec![];

    // nmcli -t -f NAME,DEVICE,STATE connection show --active
    let output = std::process::Command::new("nmcli")
        .args([
            "-t",
            "-f",
            "NAME,DEVICE,STATE",
            "connection",
            "show",
            "--active",
        ])
        .output();

    if let Ok(out) = output {
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 3 && parts[2] == "activated" {
                let name = parts[0].to_string();
                let device = parts[1].to_string();

                let dns_out = std::process::Command::new("nmcli")
                    .args(["-t", "-f", "IP4.DNS", "device", "show", &device])
                    .output();

                let mut primary = None;
                let mut secondary = None;

                if let Ok(d_out) = dns_out {
                    let d_text = String::from_utf8_lossy(&d_out.stdout);
                    for d_line in d_text.lines() {
                        let d_parts: Vec<&str> = d_line.split(':').collect();
                        if d_parts.len() == 2 {
                            if primary.is_none() {
                                primary = Some(d_parts[1].to_string());
                            } else if secondary.is_none() {
                                secondary = Some(d_parts[1].to_string());
                            }
                        }
                    }
                }

                adapters.push(NetworkAdapter {
                    name,
                    current_primary_dns: primary,
                    current_secondary_dns: secondary,
                    is_dhcp: false,
                });
            }
        }
    }
    adapters
}

/// Applies the given DNS to all active adapters.
/// netsh interface ip set dns "AdapterName" static 1.1.1.1
#[cfg(target_os = "windows")]
pub fn apply_dns(primary: &str, secondary: &str) -> ApplyDnsResult {
    if primary.parse::<std::net::Ipv4Addr>().is_err()
        || secondary.parse::<std::net::Ipv4Addr>().is_err()
    {
        return ApplyDnsResult {
            success: false,
            applied_adapters: vec![],
            error: Some("DNS addresses must be valid IPv4 addresses.".into()),
        };
    }
    let adapters = get_active_adapters();

    if adapters.is_empty() {
        return ApplyDnsResult {
            success: false,
            applied_adapters: vec![],
            error: Some("Aktif ağ adaptörü bulunamadı.".into()),
        };
    }

    let mut applied = vec![];
    let mut errors = Vec::new();

    for adapter in &adapters {
        // Primary DNS
        let primary_res = Command::new("netsh")
            .args([
                "interface",
                "ip",
                "set",
                "dns",
                &adapter.name,
                "static",
                primary,
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .output();

        let primary_ok = primary_res
            .as_ref()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if primary_ok {
            // Secondary DNS (index=2)
            let secondary_ok = if secondary == primary {
                true
            } else {
                Command::new("netsh")
                    .args([
                        "interface",
                        "ip",
                        "add",
                        "dns",
                        &adapter.name,
                        secondary,
                        "index=2",
                    ])
                    .creation_flags(CREATE_NO_WINDOW)
                    .output()
                    .map(|output| output.status.success())
                    .unwrap_or(false)
            };
            if secondary_ok {
                applied.push(adapter.name.clone());
            } else {
                errors.push(format!(
                    "Secondary DNS could not be applied to '{}'.",
                    adapter.name
                ));
            }
        } else {
            let err = primary_res
                .map(|o| String::from_utf8_lossy(&o.stderr).to_string())
                .unwrap_or_else(|e| e.to_string());
            errors.push(format!("Adapter '{}' failed: {}", adapter.name, err.trim()));
        }
    }

    if errors.is_empty() {
        let verified = get_active_adapters();
        for adapter in &adapters {
            let matches = verified
                .iter()
                .find(|item| item.name == adapter.name)
                .is_some_and(|item| {
                    item.current_primary_dns.as_deref() == Some(primary)
                        && (secondary == primary
                            || item.current_secondary_dns.as_deref() == Some(secondary))
                });
            if !matches {
                errors.push(format!(
                    "Windows did not report the expected DNS values for '{}'.",
                    adapter.name
                ));
            }
        }
    }

    ApplyDnsResult {
        success: errors.is_empty() && applied.len() == adapters.len(),
        applied_adapters: applied,
        error: (!errors.is_empty()).then(|| errors.join(" ")),
    }
}

#[cfg(not(target_os = "windows"))]
pub fn apply_dns(primary: &str, secondary: &str) -> ApplyDnsResult {
    let adapters = get_active_adapters();
    if adapters.is_empty() {
        return ApplyDnsResult {
            success: false,
            applied_adapters: vec![],
            error: Some("nmcli komutu bulunamadı veya aktif ağ bağlantısı yok.".into()),
        };
    }

    let mut applied = vec![];
    let mut last_error = None;

    let dns_string = format!("{} {}", primary, secondary);

    for adapter in &adapters {
        let mod_res = std::process::Command::new("nmcli")
            .args([
                "con",
                "mod",
                &adapter.name,
                "ipv4.dns",
                &dns_string,
                "ipv4.ignore-auto-dns",
                "yes",
            ])
            .output();

        if mod_res.map(|o| o.status.success()).unwrap_or(false) {
            let _ = std::process::Command::new("nmcli")
                .args(["con", "up", &adapter.name])
                .output();
            applied.push(adapter.name.clone());
        } else {
            last_error = Some(format!(
                "{} adaptörü için nmcli kuralı uygulanamadı.",
                adapter.name
            ));
        }
    }

    ApplyDnsResult {
        success: !applied.is_empty(),
        applied_adapters: applied,
        error: last_error,
    }
}

/// Reverts DNS back to DHCP (automatic).
#[cfg(target_os = "windows")]
pub fn reset_dns_to_dhcp() -> ApplyDnsResult {
    let adapters = get_active_adapters();
    if adapters.is_empty() {
        return ApplyDnsResult {
            success: false,
            applied_adapters: vec![],
            error: Some("No active network adapter was found for DHCP DNS restore.".into()),
        };
    }
    let mut applied = vec![];
    let mut errors = Vec::new();

    for adapter in &adapters {
        let res = Command::new("netsh")
            .args(["interface", "ip", "set", "dns", &adapter.name, "dhcp"])
            .creation_flags(CREATE_NO_WINDOW)
            .output();

        match res {
            Ok(output) if output.status.success() => applied.push(adapter.name.clone()),
            Ok(output) => errors.push(format!(
                "DHCP DNS restore failed for '{}': {}",
                adapter.name,
                String::from_utf8_lossy(&output.stderr).trim()
            )),
            Err(error) => errors.push(format!(
                "DHCP DNS restore failed for '{}': {error}",
                adapter.name
            )),
        }
    }

    ApplyDnsResult {
        success: !adapters.is_empty() && errors.is_empty() && applied.len() == adapters.len(),
        applied_adapters: applied,
        error: (!errors.is_empty()).then(|| errors.join(" ")),
    }
}

#[cfg(target_os = "windows")]
pub fn restore_dns_snapshot(adapters: &[NetworkAdapter]) -> ApplyDnsResult {
    let mut applied = Vec::new();
    let mut errors = Vec::new();
    for adapter in adapters {
        let result = if adapter.is_dhcp || adapter.current_primary_dns.is_none() {
            Command::new("netsh")
                .args(["interface", "ip", "set", "dns", &adapter.name, "dhcp"])
                .creation_flags(CREATE_NO_WINDOW)
                .output()
        } else {
            Command::new("netsh")
                .args([
                    "interface",
                    "ip",
                    "set",
                    "dns",
                    &adapter.name,
                    "static",
                    adapter.current_primary_dns.as_deref().unwrap_or_default(),
                ])
                .creation_flags(CREATE_NO_WINDOW)
                .output()
        };
        match result {
            Ok(output) if output.status.success() => {
                if let Some(secondary) = adapter.current_secondary_dns.as_deref() {
                    let secondary_ok = Command::new("netsh")
                        .args([
                            "interface",
                            "ip",
                            "add",
                            "dns",
                            &adapter.name,
                            secondary,
                            "index=2",
                        ])
                        .creation_flags(CREATE_NO_WINDOW)
                        .output()
                        .map(|output| output.status.success())
                        .unwrap_or(false);
                    if !secondary_ok {
                        errors.push(format!(
                            "Secondary DNS restore failed for '{}'.",
                            adapter.name
                        ));
                        continue;
                    }
                }
                applied.push(adapter.name.clone());
            }
            Ok(output) => errors.push(format!(
                "DNS restore failed for '{}': {}",
                adapter.name,
                String::from_utf8_lossy(&output.stderr).trim()
            )),
            Err(error) => errors.push(format!(
                "DNS restore failed for '{}': {error}",
                adapter.name
            )),
        }
    }
    if errors.is_empty() {
        let current = get_active_adapters();
        for expected in adapters {
            let verified = current
                .iter()
                .find(|adapter| adapter.name == expected.name)
                .is_some_and(|adapter| {
                    if expected.is_dhcp || expected.current_primary_dns.is_none() {
                        adapter.is_dhcp
                    } else {
                        adapter.current_primary_dns == expected.current_primary_dns
                            && adapter.current_secondary_dns == expected.current_secondary_dns
                    }
                });
            if !verified {
                errors.push(format!(
                    "Windows did not verify the restored DNS snapshot for '{}'.",
                    expected.name
                ));
            }
        }
    }
    ApplyDnsResult {
        success: !adapters.is_empty() && errors.is_empty() && applied.len() == adapters.len(),
        applied_adapters: applied,
        error: (!errors.is_empty()).then(|| errors.join(" ")),
    }
}

#[cfg(not(target_os = "windows"))]
pub fn restore_dns_snapshot(_adapters: &[NetworkAdapter]) -> ApplyDnsResult {
    reset_dns_to_dhcp()
}

#[cfg(not(target_os = "windows"))]
pub fn reset_dns_to_dhcp() -> ApplyDnsResult {
    let adapters = get_active_adapters();
    let mut applied = vec![];

    for adapter in &adapters {
        let mod_res = std::process::Command::new("nmcli")
            .args([
                "con",
                "mod",
                &adapter.name,
                "ipv4.dns",
                "",
                "ipv4.ignore-auto-dns",
                "no",
            ])
            .output();

        if mod_res.map(|o| o.status.success()).unwrap_or(false) {
            let _ = std::process::Command::new("nmcli")
                .args(["con", "up", &adapter.name])
                .output();
            applied.push(adapter.name.clone());
        }
    }

    ApplyDnsResult {
        success: !applied.is_empty(),
        applied_adapters: applied,
        error: None,
    }
}

/// Checks if the current DNS is a known trusted DNS.
/// Returns `false` if ISP DNS is used.
#[cfg(target_os = "windows")]
pub fn is_using_trusted_dns() -> bool {
    let trusted = [
        "1.1.1.1",
        "1.0.0.1", // Cloudflare
        "8.8.8.8",
        "8.8.4.4", // Google
        "9.9.9.9",
        "149.112.112.112", // Quad9
        "208.67.222.222",
        "208.67.220.220", // OpenDNS
        "94.140.14.14",
        "94.140.15.15", // AdGuard
    ];

    let adapters = get_active_adapters();
    for adapter in adapters {
        if let Some(primary) = &adapter.current_primary_dns {
            if trusted.contains(&primary.as_str()) {
                return true;
            }
        }
    }
    false
}

#[cfg(not(target_os = "windows"))]
pub fn is_using_trusted_dns() -> bool {
    let trusted = [
        "1.1.1.1",
        "1.0.0.1",
        "8.8.8.8",
        "8.8.4.4",
        "9.9.9.9",
        "149.112.112.112",
        "208.67.222.222",
        "208.67.220.220",
        "94.140.14.14",
        "94.140.15.15",
    ];

    let adapters = get_active_adapters();
    for adapter in adapters {
        if let Some(primary) = &adapter.current_primary_dns {
            if trusted.contains(&primary.as_str()) {
                return true;
            }
        }
    }
    false
}
