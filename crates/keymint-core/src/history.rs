//! Recent command results, so they can be reused after the palette closes.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::config::write_json_atomic;
use crate::error::Result;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: u64,
    pub command_id: String,
    pub command_title: String,
    pub input: String,
    pub output: String,
    /// Unix seconds.
    pub timestamp: u64,
}

/// Newest first.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct History {
    next_id: u64,
    entries: Vec<HistoryEntry>,
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

impl History {
    /// A missing or unreadable file just means an empty history.
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        write_json_atomic(path, self)
    }

    pub fn entries(&self) -> &[HistoryEntry] {
        &self.entries
    }

    /// Record a result, keeping at most `limit` entries. `limit == 0` turns history off.
    pub fn add(&mut self, command_id: &str, command_title: &str, input: &str, output: &str, limit: usize) {
        self.add_at(command_id, command_title, input, output, limit, now());
    }

    fn add_at(&mut self, command_id: &str, command_title: &str, input: &str, output: &str, limit: usize, ts: u64) {
        if limit == 0 {
            return;
        }
        // Retrying with an identical outcome should not fill the list with copies.
        if let Some(last) = self.entries.first() {
            if last.command_id == command_id && last.input == input && last.output == output {
                return;
            }
        }
        self.next_id += 1;
        self.entries.insert(
            0,
            HistoryEntry {
                id: self.next_id,
                command_id: command_id.to_string(),
                command_title: command_title.to_string(),
                input: input.to_string(),
                output: output.to_string(),
                timestamp: ts,
            },
        );
        self.truncate(limit);
    }

    pub fn truncate(&mut self, limit: usize) {
        self.entries.truncate(limit);
    }

    pub fn remove(&mut self, id: u64) {
        self.entries.retain(|e| e.id != id);
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newest_first_with_unique_ids() {
        let mut h = History::default();
        h.add_at("a", "A", "in1", "out1", 10, 1);
        h.add_at("b", "B", "in2", "out2", 10, 2);
        let e = h.entries();
        assert_eq!((e[0].command_id.as_str(), e[1].command_id.as_str()), ("b", "a"));
        assert_ne!(e[0].id, e[1].id);
    }

    #[test]
    fn respects_the_limit_and_zero_disables() {
        let mut h = History::default();
        for i in 0..5 {
            h.add_at("a", "A", &format!("in{i}"), "out", 3, i);
        }
        assert_eq!(h.entries().len(), 3);
        assert_eq!(h.entries()[0].input, "in4");
        h.add_at("a", "A", "x", "y", 0, 9);
        assert_eq!(h.entries().len(), 3, "limit 0 must not record");
        h.truncate(1);
        assert_eq!(h.entries().len(), 1);
    }

    #[test]
    fn skips_identical_consecutive_results_but_keeps_different_ones() {
        let mut h = History::default();
        h.add_at("a", "A", "in", "out", 10, 1);
        h.add_at("a", "A", "in", "out", 10, 2);
        assert_eq!(h.entries().len(), 1);
        h.add_at("a", "A", "in", "different", 10, 3);
        assert_eq!(h.entries().len(), 2);
    }

    #[test]
    fn remove_clear_and_persistence_keep_ids_unique() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.json");
        let mut h = History::default();
        h.add_at("a", "A", "i1", "o1", 10, 1);
        h.add_at("a", "A", "i2", "o2", 10, 2);
        let first = h.entries()[1].id;
        h.remove(first);
        assert_eq!(h.entries().len(), 1);
        h.save(&path).unwrap();

        let mut loaded = History::load(&path);
        loaded.add_at("a", "A", "i3", "o3", 10, 3);
        let ids: Vec<_> = loaded.entries().iter().map(|e| e.id).collect();
        assert_eq!(ids.len(), 2);
        assert_ne!(ids[0], ids[1]);
        loaded.clear();
        assert!(loaded.entries().is_empty());
    }

    #[test]
    fn unreadable_file_is_an_empty_history() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.json");
        std::fs::write(&path, "{ not json").unwrap();
        assert!(History::load(&path).entries().is_empty());
        assert!(History::load(&dir.path().join("missing.json")).entries().is_empty());
    }
}
