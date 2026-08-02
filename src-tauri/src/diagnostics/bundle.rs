use crate::diagnostics::event::DiagnosticEvent;
use crate::diagnostics::health::SystemHealthSnapshot;
use crate::diagnostics::redaction::DiagnosticRedactor;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::path::Path;

const DIAGNOSTIC_SCHEMA_VERSION: &str = "1.0";
const MAX_BUNDLE_SIZE_BYTES: usize = 5 * 1024 * 1024; // 5 MiB

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsBundle {
    pub schema_version: String,
    pub app_version: String,
    pub platform: String,
    pub timestamp_ms: u64,
    pub health_snapshot: SystemHealthSnapshot,
    pub events: Vec<DiagnosticEvent>,
    pub dropped_event_count: u64,
    pub original_event_count: usize,
    pub exported_event_count: usize,
    pub truncated_count: usize,
    pub truncated: bool,
    pub secret_scanner_passed: bool,
}

pub fn create_diagnostics_bundle(
    health_snapshot: SystemHealthSnapshot,
    mut events: Vec<DiagnosticEvent>,
    dropped_count: u64,
) -> DiagnosticsBundle {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    // Sanitize string fields in events using DiagnosticRedactor
    for evt in &mut events {
        for val in evt.fields.values_mut() {
            if let crate::diagnostics::event::SafeDiagnosticValue::Text(t) = val {
                *t = DiagnosticRedactor::sanitize_text(t);
            }
        }
    }

    let original_event_count = events.len();
    let mut bundle = DiagnosticsBundle {
        schema_version: DIAGNOSTIC_SCHEMA_VERSION.into(),
        app_version: env!("CARGO_PKG_VERSION").into(),
        platform: std::env::consts::OS.into(),
        timestamp_ms: now_ms,
        health_snapshot,
        events,
        dropped_event_count: dropped_count,
        original_event_count,
        exported_event_count: original_event_count,
        truncated_count: 0,
        truncated: false,
        secret_scanner_passed: false,
    };

    while serde_json::to_vec(&bundle)
        .map(|bytes| bytes.len() > MAX_BUNDLE_SIZE_BYTES)
        .unwrap_or(true)
        && !bundle.events.is_empty()
    {
        let remove_count = (bundle.events.len() / 4).max(1);
        bundle.events.drain(..remove_count);
        bundle.truncated = true;
        bundle.exported_event_count = bundle.events.len();
        bundle.truncated_count = original_event_count - bundle.events.len();
    }

    bundle.secret_scanner_passed = final_privacy_scan(&bundle).is_ok();

    bundle
}

pub fn export_bundle_to_file(bundle: &DiagnosticsBundle, target_path: &Path) -> Result<(), String> {
    if target_path.extension().and_then(|value| value.to_str()) != Some("json")
        || !target_path
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|name| name.ends_with(".vane-diag.json"))
    {
        return Err("Diagnostics exports must use the .vane-diag.json extension".into());
    }
    if target_path.exists() {
        return Err("Refusing to overwrite an existing diagnostics file".into());
    }
    final_privacy_scan(bundle)?;
    let json_bytes = serde_json::to_vec_pretty(bundle)
        .map_err(|e| format!("Failed to serialize bundle: {e}"))?;
    if json_bytes.len() > MAX_BUNDLE_SIZE_BYTES {
        return Err("Serialized diagnostics bundle exceeds the 5 MiB limit".into());
    }

    let temp_path = target_path.with_extension("tmp");
    {
        let mut file = File::create(&temp_path)
            .map_err(|e| format!("Failed to create temporary bundle file: {e}"))?;
        file.write_all(&json_bytes)
            .map_err(|e| format!("Failed to write bundle file: {e}"))?;
        file.sync_all()
            .map_err(|e| format!("Failed to sync bundle file: {e}"))?;
    }

    std::fs::rename(&temp_path, target_path)
        .map_err(|e| format!("Failed to rename temporary bundle file: {e}"))?;

    Ok(())
}

fn final_privacy_scan(bundle: &DiagnosticsBundle) -> Result<(), String> {
    let bytes = serde_json::to_vec(bundle)
        .map_err(|error| format!("Failed to serialize diagnostics bundle for scanning: {error}"))?;
    if bytes.len() > MAX_BUNDLE_SIZE_BYTES {
        return Err("Serialized diagnostics bundle exceeds the 5 MiB limit".into());
    }
    let text = String::from_utf8_lossy(&bytes).to_ascii_lowercase();
    const FORBIDDEN: &[&str] = &[
        "-----begin private key-----",
        "-----begin rsa private key-----",
        "authorization: bearer ",
        "password=",
        "proxy-authorization",
        "\\\\?\\",
        "\\\\.\\",
        "c:\\\\users\\\\",
        "/home/",
        "/users/",
        "http://",
        "https://",
        "bearer ",
        "proxy://",
        "\"token\"",
        "\"password\"",
        "\"environment_variables\"",
        "-----begin ",
        " --",
    ];
    if let Some(pattern) = FORBIDDEN.iter().find(|pattern| text.contains(**pattern)) {
        return Err(format!(
            "Diagnostics privacy scan rejected forbidden pattern: {pattern}"
        ));
    }
    static IPV4: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(r"\b(?:\d{1,3}\.){3}\d{1,3}\b").expect("valid IPv4 scanner")
    });
    static IPV6: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(r"(?i)\b[0-9a-f]{0,4}:[0-9a-f:]{2,}\b").expect("valid IPv6 scanner")
    });
    static DOMAIN: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(
            r"(?i)\b[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?(?:\.[a-z0-9-]{1,63})+\.[a-z]{2,63}\b",
        )
        .expect("valid domain scanner")
    });
    if IPV4.is_match(&text) || IPV6.is_match(&text) || DOMAIN.is_match(&text) {
        return Err("Diagnostics privacy scan rejected network identifier".into());
    }
    Ok(())
}
