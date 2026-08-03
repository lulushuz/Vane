use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DpiBypassAssessment {
    Inconclusive,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetProbeResult {
    pub target_id: String,
    pub success: bool,
    pub status_code: Option<u16>,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrafficProbeReport {
    pub targets: Vec<TargetProbeResult>,
    pub success_ratio: f64,
    pub median_latency_ms: Option<u64>,
    pub assessment: DpiBypassAssessment,
    pub timestamp_ms: u64,
}

pub struct TrafficProbeRunner {
    is_running: Arc<Mutex<bool>>,
    cancel_flag: Arc<AtomicBool>,
}

impl Default for TrafficProbeRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl TrafficProbeRunner {
    pub fn new() -> Self {
        Self {
            is_running: Arc::new(Mutex::new(false)),
            cancel_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) -> bool {
        self.cancel_flag.store(true, Ordering::SeqCst);
        true
    }

    pub fn is_running(&self) -> bool {
        let guard = self.is_running.try_lock();
        match guard {
            Ok(g) => *g,
            Err(_) => true,
        }
    }

    pub async fn run_probes(&self, targets: &[String]) -> Result<TrafficProbeReport, String> {
        let mut running_guard = self
            .is_running
            .try_lock()
            .map_err(|_| "Traffic probe already running".to_string())?;

        if *running_guard {
            return Err("Traffic probe already running".to_string());
        }

        *running_guard = true;
        self.cancel_flag.store(false, Ordering::SeqCst);

        let registry = crate::optimizer::targets::default_measurement_targets();
        let requested = if targets.is_empty() {
            vec!["youtube".into(), "discord".into(), "x".into()]
        } else {
            targets.to_vec()
        };
        let mut resolved_targets = Vec::new();
        for target_id in requested {
            let target = registry
                .iter()
                .find(|item| item.id.0 == target_id)
                .ok_or_else(|| format!("Unknown traffic target ID: {target_id}"))?;
            resolved_targets.push((target_id, target.host.clone()));
        }

        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(3000))
            .redirect(reqwest::redirect::Policy::limited(3))
            .danger_accept_invalid_certs(false) // Strict TLS verification enforced
            .build()
            .map_err(|e| format!("Failed to create reqwest client: {e}"))?;

        let mut results = Vec::new();
        let mut latencies = Vec::new();

        for (target_id, host) in resolved_targets {
            if self.cancel_flag.load(Ordering::SeqCst) {
                *running_guard = false;
                return Err("Traffic probe cancelled by user".to_string());
            }

            let url = format!("https://{host}/");
            let start = Instant::now();
            let res = client.get(&url).send().await;
            let elapsed = start.elapsed().as_millis() as u64;

            match res {
                Ok(resp) => {
                    let status = resp.status().as_u16();
                    let success = resp.status().is_success() || resp.status().is_redirection();
                    if success {
                        latencies.push(elapsed);
                    }

                    results.push(TargetProbeResult {
                        target_id,
                        success,
                        status_code: Some(status),
                        latency_ms: Some(elapsed),
                        error: None,
                    });
                }
                Err(_e) => {
                    results.push(TargetProbeResult {
                        target_id,
                        success: false,
                        status_code: None,
                        latency_ms: None,
                        error: Some("HTTPS Probe Failed".into()), // Redacted generic error
                    });
                }
            }
        }

        *running_guard = false;

        let total = results.len();
        let succeeded = results.iter().filter(|r| r.success).count();
        let success_ratio = if total > 0 {
            succeeded as f64 / total as f64
        } else {
            0.0
        };

        latencies.sort_unstable();
        let median_latency_ms = if !latencies.is_empty() {
            Some(latencies[latencies.len() / 2])
        } else {
            None
        };

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Ok(TrafficProbeReport {
            targets: results,
            success_ratio,
            median_latency_ms,
            assessment: DpiBypassAssessment::Inconclusive, // STRICT RULE: Never report confirmed DPI bypass
            timestamp_ms: now_ms,
        })
    }
}
