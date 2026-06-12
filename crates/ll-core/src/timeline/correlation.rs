use std::collections::HashMap;
use crate::models::log_entry::NormalizedEntry;
use crate::models::query::ServiceCorrelation;

pub struct ServiceCorrelator {
    // service → list of timestamps (error-only)
    timelines: HashMap<String, Vec<i64>>,
}

impl ServiceCorrelator {
    pub fn new() -> Self {
        Self { timelines: HashMap::new() }
    }

    pub fn push(&mut self, entry: &NormalizedEntry) {
        if let Some(ref svc) = entry.service {
            self.timelines
                .entry(svc.clone())
                .or_default()
                .push(entry.timestamp.timestamp());
        }
    }

    /// Pearson-style co-occurrence score within time windows.
    pub fn compute(&self, window_secs: i64) -> Vec<ServiceCorrelation> {
        let services: Vec<&str> = self.timelines.keys().map(|s| s.as_str()).collect();
        let mut results = Vec::new();

        for i in 0..services.len() {
            for j in (i + 1)..services.len() {
                let a = services[i];
                let b = services[j];
                let score = self.co_occurrence_score(a, b, window_secs);
                if score > 0.3 {
                    results.push(ServiceCorrelation {
                        service_a: a.to_string(),
                        service_b: b.to_string(),
                        correlation_score: score as f32,
                        description: format!(
                            "{} and {} show correlated errors within {}s windows",
                            a, b, window_secs
                        ),
                    });
                }
            }
        }

        results.sort_by(|a, b| b.correlation_score.partial_cmp(&a.correlation_score).unwrap());
        results
    }

    fn co_occurrence_score(&self, svc_a: &str, svc_b: &str, window_secs: i64) -> f64 {
        let a_ts = match self.timelines.get(svc_a) { Some(v) => v, None => return 0.0 };
        let b_ts = match self.timelines.get(svc_b) { Some(v) => v, None => return 0.0 };

        let mut co = 0usize;
        for &ta in a_ts {
            if b_ts.iter().any(|&tb| (ta - tb).abs() <= window_secs) {
                co += 1;
            }
        }

        let min_len = a_ts.len().min(b_ts.len()) as f64;
        if min_len == 0.0 { 0.0 } else { co as f64 / min_len }
    }
}

impl Default for ServiceCorrelator {
    fn default() -> Self { Self::new() }
}
