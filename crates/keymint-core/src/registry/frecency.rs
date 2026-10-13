use std::collections::HashMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::config::write_json_atomic;
use crate::error::Result;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
struct Usage {
    count: u32,
    last_used: u64,
}

/// Remembers how often and how recently each command ran, to rank results.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Frecency(HashMap<String, Usage>);

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

impl Frecency {
    /// A missing or unreadable file just means no history; it is not worth failing startup.
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        write_json_atomic(path, self)
    }

    pub fn record(&mut self, id: &str) {
        self.record_at(id, now());
    }

    fn record_at(&mut self, id: &str, ts: u64) {
        let u = self.0.entry(id.to_string()).or_default();
        u.count = u.count.saturating_add(1);
        u.last_used = ts;
    }

    pub fn score(&self, id: &str) -> u32 {
        self.score_at(id, now())
    }

    /// 0..=~300: usage count (capped) plus a recency bonus that decays over two weeks.
    fn score_at(&self, id: &str, now: u64) -> u32 {
        let Some(u) = self.0.get(id) else { return 0 };
        let frequency = u.count.min(20) * 5;
        let age_days = now.saturating_sub(u.last_used) / 86_400;
        let recency = 200u32.saturating_sub((age_days as u32).saturating_mul(14));
        frequency + recency
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unused_is_zero_and_recent_beats_old() {
        let mut f = Frecency::default();
        f.record_at("a", 1_000_000);
        f.record_at("b", 1_000_000 + 10 * 86_400);
        let t = 1_000_000 + 10 * 86_400;
        assert_eq!(f.score_at("zzz", t), 0);
        assert!(f.score_at("b", t) > f.score_at("a", t));
    }

    #[test]
    fn frequency_is_capped() {
        let mut f = Frecency::default();
        for _ in 0..1000 {
            f.record_at("a", 0);
        }
        assert_eq!(f.score_at("a", 0), 20 * 5 + 200);
    }
}
