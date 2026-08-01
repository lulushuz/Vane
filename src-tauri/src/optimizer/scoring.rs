use crate::optimizer::measurement::MeasurementSummary;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScoreConfidence {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateScore {
    pub eligible: bool,
    pub success_ratio: f64,
    pub median_latency_ms: Option<u64>,
    pub p95_latency_ms: Option<u64>,
    pub improvement_over_baseline: Option<f64>,
    pub confidence: ScoreConfidence,
}

impl CandidateScore {
    pub fn compute(
        summary: &MeasurementSummary,
        baseline: Option<&MeasurementSummary>,
        sample_count: usize,
        min_threshold: f64,
    ) -> Self {
        let eligible = summary.success_ratio >= min_threshold && summary.succeeded > 0;

        let confidence = if sample_count >= 5 && summary.succeeded >= 4 {
            ScoreConfidence::High
        } else if sample_count >= 3 && summary.succeeded >= 2 {
            ScoreConfidence::Medium
        } else {
            ScoreConfidence::Low
        };

        let improvement_over_baseline = match (
            summary.median_latency_ms,
            baseline.and_then(|b| b.median_latency_ms),
        ) {
            (Some(cand_lat), Some(base_lat)) if base_lat > 0 => {
                let diff = (base_lat as f64) - (cand_lat as f64);
                Some((diff / base_lat as f64) * 100.0)
            }
            _ => None,
        };

        Self {
            eligible,
            success_ratio: summary.success_ratio,
            median_latency_ms: summary.median_latency_ms,
            p95_latency_ms: summary.p95_latency_ms,
            improvement_over_baseline,
            confidence,
        }
    }
}

pub fn compare_candidate_scores(a: &CandidateScore, b: &CandidateScore) -> Ordering {
    match (a.eligible, b.eligible) {
        (true, false) => return Ordering::Greater,
        (false, true) => return Ordering::Less,
        (false, false) => return Ordering::Equal,
        (true, true) => {}
    }

    let ratio_diff = a.success_ratio - b.success_ratio;
    if ratio_diff.abs() > 0.05 {
        return if ratio_diff > 0.0 {
            Ordering::Greater
        } else {
            Ordering::Less
        };
    }

    match (a.median_latency_ms, b.median_latency_ms) {
        (Some(a_lat), Some(b_lat)) => match b_lat.cmp(&a_lat) {
            Ordering::Equal => match (a.p95_latency_ms, b.p95_latency_ms) {
                (Some(a_p95), Some(b_p95)) => b_p95.cmp(&a_p95),
                _ => Ordering::Equal,
            },
            other => other,
        },
        (Some(_), None) => Ordering::Greater,
        (None, Some(_)) => Ordering::Less,
        (None, None) => Ordering::Equal,
    }
}
