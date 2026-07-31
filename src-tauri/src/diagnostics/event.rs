use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

static MONOTONIC_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticSeverity {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticComponent {
    Engine,
    Config,
    Dns,
    Firewall,
    Optimizer,
    Security,
    System,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiagnosticEventCode {
    EngStartInit,
    EngStartSuccess,
    EngStartFailed,
    EngStopReq,
    EngStopSuccess,
    EngStopFailed,
    EngCrashExit,
    PatCacheUpdated,
    PatRollbackTriggered,
    DnsSyncReq,
    DnsSyncSuccess,
    DnsSyncFailed,
    FwApplied,
    FwRemoved,
    OptSessionStart,
    OptSessionEnd,
    SecArtifactVerified,
    SecArtifactTampered,
    HealthCheckLocal,
    HealthCheckTraffic,
}

impl DiagnosticEventCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::EngStartInit => "ENG_START_INIT",
            Self::EngStartSuccess => "ENG_START_SUCCESS",
            Self::EngStartFailed => "ENG_START_FAILED",
            Self::EngStopReq => "ENG_STOP_REQ",
            Self::EngStopSuccess => "ENG_STOP_SUCCESS",
            Self::EngStopFailed => "ENG_STOP_FAILED",
            Self::EngCrashExit => "ENG_CRASH_EXIT",
            Self::PatCacheUpdated => "PAT_CACHE_UPDATED",
            Self::PatRollbackTriggered => "PAT_ROLLBACK_TRIGGERED",
            Self::DnsSyncReq => "DNS_SYNC_REQ",
            Self::DnsSyncSuccess => "DNS_SYNC_SUCCESS",
            Self::DnsSyncFailed => "DNS_SYNC_FAILED",
            Self::FwApplied => "FW_APPLIED",
            Self::FwRemoved => "FW_REMOVED",
            Self::OptSessionStart => "OPT_SESSION_START",
            Self::OptSessionEnd => "OPT_SESSION_END",
            Self::SecArtifactVerified => "SEC_ARTIFACT_VERIFIED",
            Self::SecArtifactTampered => "SEC_ARTIFACT_TAMPERED",
            Self::HealthCheckLocal => "HEALTH_CHECK_LOCAL",
            Self::HealthCheckTraffic => "HEALTH_CHECK_TRAFFIC",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SafeDiagnosticValue {
    Text(String),
    Int(i64),
    Float(f64),
    Bool(bool),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticEvent {
    pub sequence: u64,
    pub timestamp_epoch_ms: u64,
    pub monotonic_ns: u64,
    pub component: DiagnosticComponent,
    pub code: DiagnosticEventCode,
    pub severity: DiagnosticSeverity,
    pub fields: BTreeMap<String, SafeDiagnosticValue>,
}

impl DiagnosticEvent {
    pub fn new(
        component: DiagnosticComponent,
        code: DiagnosticEventCode,
        severity: DiagnosticSeverity,
    ) -> Self {
        let sequence = MONOTONIC_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let timestamp_epoch_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let monotonic_ns = std::time::Instant::now().elapsed().as_nanos() as u64;

        Self {
            sequence,
            timestamp_epoch_ms,
            monotonic_ns,
            component,
            code,
            severity,
            fields: BTreeMap::new(),
        }
    }

    pub fn with_field(mut self, key: impl Into<String>, value: SafeDiagnosticValue) -> Self {
        self.fields.insert(key.into(), value);
        self
    }
}

pub static DIAGNOSTIC_STORE: std::sync::LazyLock<crate::diagnostics::store::DiagnosticEventStore> =
    std::sync::LazyLock::new(crate::diagnostics::store::DiagnosticEventStore::default);

pub fn emit_diagnostic_event(event: DiagnosticEvent) {
    let store = DIAGNOSTIC_STORE.clone();
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        handle.spawn(async move {
            store.push(event).await;
        });
    }
}
