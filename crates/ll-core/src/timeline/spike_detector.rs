use std::collections::VecDeque;
use chrono::{DateTime, Utc, Duration};
use crate::models::log_entry::{NormalizedEntry, LogLevel};

const WINDOW_SIZE: usize = 60; // seconds
const EMA_ALPHA: f64 = 0.3;
const SPIKE_FACTOR: f64 = 3.0;

#[derive(Debug, Clone)]
pub struct ErrorSpike {
    pub detected_at: DateTime<Utc>,
    pub rate: f64,
    pub baseline: f64,
    pub factor: f64,
    pub service: Option<String>,
}

pub struct SpikeDetector {
    window: VecDeque<DateTime<Utc>>,
    ema_baseline: f64,
    last_tick: DateTime<Utc>,
}

impl SpikeDetector {
    pub fn new() -> Self {
        Self {
            window: VecDeque::new(),
            ema_baseline: 0.0,
            last_tick: Utc::now(),
        }
    }

    /// Feed an entry; returns Some(spike) if a spike is detected.
    pub fn push(&mut self, entry: &NormalizedEntry) -> Option<ErrorSpike> {
        if entry.level < LogLevel::Error {
            return None;
        }

        let now = entry.timestamp;
        self.window.push_back(now);

        // Evict old entries
        let cutoff = now - Duration::seconds(WINDOW_SIZE as i64);
        while self.window.front().is_some_and(|t| *t < cutoff) {
            self.window.pop_front();
        }

        // Update EMA every second
        if (now - self.last_tick).num_seconds() >= 1 {
            let rate = self.window.len() as f64;
            self.ema_baseline = if self.ema_baseline == 0.0 {
                rate
            } else {
                EMA_ALPHA * rate + (1.0 - EMA_ALPHA) * self.ema_baseline
            };
            self.last_tick = now;

            let current_rate = self.window.len() as f64;
            if self.ema_baseline > 1.0 && current_rate >= self.ema_baseline * SPIKE_FACTOR {
                return Some(ErrorSpike {
                    detected_at: now,
                    rate: current_rate,
                    baseline: self.ema_baseline,
                    factor: current_rate / self.ema_baseline,
                    service: entry.service.clone(),
                });
            }
        }

        None
    }
}

impl Default for SpikeDetector {
    fn default() -> Self { Self::new() }
}
