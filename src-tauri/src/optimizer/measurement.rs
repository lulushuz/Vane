use crate::optimizer::targets::{MeasurementTargetId, ResolvedMeasurementTarget};
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeasurementErrorCategory {
    Timeout,
    DnsResolution,
    ConnectionRefused,
    Tls,
    HttpStatus,
    NetworkUnavailable,
    Cancelled,
    EnvironmentChanged,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeasurementSample {
    pub target_id: MeasurementTargetId,
    pub success: bool,
    pub latency_ms: Option<u64>,
    pub error: Option<MeasurementErrorCategory>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeasurementSummary {
    pub attempted: usize,
    pub succeeded: usize,
    pub success_ratio: f64,
    pub median_latency_ms: Option<u64>,
    pub p95_latency_ms: Option<u64>,
    pub min_latency_ms: Option<u64>,
    pub max_latency_ms: Option<u64>,
}

impl MeasurementSummary {
    pub fn compute(samples: &[MeasurementSample]) -> Self {
        let attempted = samples.len();
        let successful_samples: Vec<u64> = samples
            .iter()
            .filter(|s| s.success)
            .filter_map(|s| s.latency_ms)
            .collect();

        let succeeded = successful_samples.len();
        let success_ratio = if attempted > 0 {
            succeeded as f64 / attempted as f64
        } else {
            0.0
        };

        if successful_samples.is_empty() {
            return Self {
                attempted,
                succeeded: 0,
                success_ratio: 0.0,
                median_latency_ms: None,
                p95_latency_ms: None,
                min_latency_ms: None,
                max_latency_ms: None,
            };
        }

        let mut sorted = successful_samples;
        sorted.sort_unstable();

        let min_latency_ms = Some(sorted[0]);
        let max_latency_ms = Some(sorted[sorted.len() - 1]);

        let median_latency_ms = if sorted.len() % 2 == 1 {
            Some(sorted[sorted.len() / 2])
        } else {
            let mid = sorted.len() / 2;
            Some((sorted[mid - 1] + sorted[mid]) / 2)
        };

        let p95_idx = ((sorted.len() as f64 * 0.95).ceil() as usize).saturating_sub(1);
        let p95_latency_ms = Some(sorted[p95_idx.min(sorted.len() - 1)]);

        Self {
            attempted,
            succeeded,
            success_ratio,
            median_latency_ms,
            p95_latency_ms,
            min_latency_ms,
            max_latency_ms,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MeasurementPolicy {
    pub warmup_count: usize,
    pub sample_count: usize,
    pub request_timeout: Duration,
    pub inter_sample_delay: Duration,
    pub minimum_success_ratio: f64,
}

impl Default for MeasurementPolicy {
    fn default() -> Self {
        Self {
            warmup_count: 1,
            sample_count: 3,
            request_timeout: Duration::from_secs(5),
            inter_sample_delay: Duration::from_millis(300),
            minimum_success_ratio: 0.33,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NetworkEnvironmentSnapshot {
    pub targets: Vec<ResolvedMeasurementTarget>,
    pub captured_at: SystemTime,
}
