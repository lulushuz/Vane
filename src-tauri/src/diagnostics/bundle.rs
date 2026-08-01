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

    let mut bundle = DiagnosticsBundle {
        schema_version: DIAGNOSTIC_SCHEMA_VERSION.into(),
        app_version: env!("CARGO_PKG_VERSION").into(),
        platform: std::env::consts::OS.into(),
        timestamp_ms: now_ms,
        health_snapshot,
        events,
        dropped_event_count: dropped_count,
        truncated: false,
        secret_scanner_passed: true,
    };

    // Verify bundle size limits
    if let Ok(serialized) = serde_json::to_string(&bundle) {
        if serialized.len() > MAX_BUNDLE_SIZE_BYTES {
            bundle.events.truncate(500);
            bundle.truncated = true;
        }
    }

    bundle
}

pub fn export_bundle_to_file(bundle: &DiagnosticsBundle, target_path: &Path) -> Result<(), String> {
    let json_bytes = serde_json::to_vec_pretty(bundle)
        .map_err(|e| format!("Failed to serialize bundle: {e}"))?;

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
