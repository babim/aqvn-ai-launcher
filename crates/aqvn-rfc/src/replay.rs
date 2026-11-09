//! Sliding-window anti-replay cache and timestamp validation for `/api/rfc`.
//!
//! Enforces:
//! 1. Timestamp drift within allowable sliding window (TTL 300s).
//! 2. Unique nonces within sliding window (maximum 10,000 nonces retained).
//! 3. Eviction of expired nonces and LRU/FIFO capacity bounding.

use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub const DEFAULT_REPLAY_TTL: Duration = Duration::from_secs(300);
pub const DEFAULT_MAX_NONCES: usize = 10_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplayError {
    MissingField(&'static str),
    InvalidTimestamp,
    InvalidNonce,
    TimestampDriftExceeded { drift_secs: u64, max_secs: u64 },
    DuplicateNonce(String),
}

impl std::fmt::Display for ReplayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingField(field) => write!(f, "missing field `{field}` in envelope"),
            Self::InvalidTimestamp => write!(f, "invalid timestamp format in envelope"),
            Self::InvalidNonce => write!(f, "invalid or empty nonce in envelope"),
            Self::TimestampDriftExceeded {
                drift_secs,
                max_secs,
            } => {
                write!(
                    f,
                    "timestamp drift ({drift_secs}s) exceeds allowable window ({max_secs}s)"
                )
            }
            Self::DuplicateNonce(nonce) => {
                write!(f, "duplicate nonce detected (replay attack): {nonce}")
            }
        }
    }
}

impl std::error::Error for ReplayError {}

#[derive(Clone)]
pub struct AntiReplayWindow {
    ttl: Duration,
    max_nonces: usize,
    inner: Arc<Mutex<SlidingWindowInner>>,
}

struct SlidingWindowInner {
    entries: HashMap<String, Instant>,
    queue: VecDeque<(Instant, String)>,
}

impl AntiReplayWindow {
    pub fn new(ttl: Duration, max_nonces: usize) -> Self {
        Self {
            ttl,
            max_nonces,
            inner: Arc::new(Mutex::new(SlidingWindowInner {
                entries: HashMap::new(),
                queue: VecDeque::new(),
            })),
        }
    }

    pub fn default_rfc() -> Self {
        Self::new(DEFAULT_REPLAY_TTL, DEFAULT_MAX_NONCES)
    }

    pub fn ttl(&self) -> Duration {
        self.ttl
    }

    pub fn max_nonces(&self) -> usize {
        self.max_nonces
    }

    pub fn len(&self) -> usize {
        let inner = match self.inner.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        inner.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn reset(&self) {
        let mut inner = match self.inner.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        inner.entries.clear();
        inner.queue.clear();
    }

    /// Verifies timestamp drift against current server wall clock and checks/records nonce.
    pub fn check_and_record(&self, timestamp_secs: u64, nonce: &str) -> Result<(), ReplayError> {
        let now_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.check_and_record_with_time(timestamp_secs, nonce, now_secs, Instant::now())
    }

    /// Inner implementation parameterized with current time for deterministic testing.
    pub fn check_and_record_with_time(
        &self,
        timestamp_secs: u64,
        nonce: &str,
        now_secs: u64,
        now_instant: Instant,
    ) -> Result<(), ReplayError> {
        let trimmed_nonce = nonce.trim();
        if trimmed_nonce.is_empty() {
            return Err(ReplayError::InvalidNonce);
        }

        // Validate timestamp drift within allowable TTL window
        let max_drift = self.ttl.as_secs();
        let drift = now_secs.abs_diff(timestamp_secs);

        if drift > max_drift {
            return Err(ReplayError::TimestampDriftExceeded {
                drift_secs: drift,
                max_secs: max_drift,
            });
        }

        let mut inner = match self.inner.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        // 1. Purge expired nonces
        let cutoff = now_instant.checked_sub(self.ttl).unwrap_or(now_instant);
        while let Some((recorded_at, _)) = inner.queue.front() {
            if *recorded_at < cutoff {
                if let Some((_, old_nonce)) = inner.queue.pop_front() {
                    inner.entries.remove(&old_nonce);
                }
            } else {
                break;
            }
        }

        // 2. Check for duplicate nonce
        if inner.entries.contains_key(trimmed_nonce) {
            return Err(ReplayError::DuplicateNonce(trimmed_nonce.to_string()));
        }

        // 3. Enforce maximum capacity (max_nonces)
        while inner.entries.len() >= self.max_nonces {
            if let Some((_, oldest_nonce)) = inner.queue.pop_front() {
                inner.entries.remove(&oldest_nonce);
            } else {
                break;
            }
        }

        // 4. Insert new nonce
        inner.entries.insert(trimmed_nonce.to_string(), now_instant);
        inner
            .queue
            .push_back((now_instant, trimmed_nonce.to_string()));

        Ok(())
    }
}

/// Extracts and parses timestamp from a JSON value.
/// Supports integers (seconds or milliseconds), floats, and numeric strings.
pub fn extract_timestamp(value: &Value) -> Result<u64, ReplayError> {
    let raw = match value {
        Value::Number(n) => {
            if let Some(u) = n.as_u64() {
                u
            } else if let Some(i) = n.as_i64() {
                if i <= 0 {
                    return Err(ReplayError::InvalidTimestamp);
                }
                i as u64
            } else if let Some(f) = n.as_f64() {
                if f <= 0.0 {
                    return Err(ReplayError::InvalidTimestamp);
                }
                f as u64
            } else {
                return Err(ReplayError::InvalidTimestamp);
            }
        }
        Value::String(s) => s
            .trim()
            .parse::<u64>()
            .map_err(|_| ReplayError::InvalidTimestamp)?,
        _ => return Err(ReplayError::InvalidTimestamp),
    };

    if raw == 0 {
        return Err(ReplayError::InvalidTimestamp);
    }

    // If timestamp is given in milliseconds (greater than year ~2286 in seconds), normalize to seconds.
    if raw > 100_000_000_000 {
        Ok(raw / 1000)
    } else {
        Ok(raw)
    }
}

/// Extracts a non-empty nonce string from a JSON value.
pub fn extract_nonce(value: &Value) -> Result<&str, ReplayError> {
    match value {
        Value::String(s) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                Err(ReplayError::InvalidNonce)
            } else {
                Ok(trimmed)
            }
        }
        _ => Err(ReplayError::InvalidNonce),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_timestamp_formats() {
        assert_eq!(
            extract_timestamp(&serde_json::json!(1731170000)).unwrap(),
            1731170000
        );
        assert_eq!(
            extract_timestamp(&serde_json::json!(1731170000000u64)).unwrap(),
            1731170000
        );
        assert_eq!(
            extract_timestamp(&serde_json::json!("1731170000")).unwrap(),
            1731170000
        );
        assert_eq!(
            extract_timestamp(&serde_json::json!(" 1731170000 ")).unwrap(),
            1731170000
        );
        assert!(extract_timestamp(&serde_json::json!(0)).is_err());
        assert!(extract_timestamp(&serde_json::json!(-100)).is_err());
        assert!(extract_timestamp(&serde_json::json!("not-a-number")).is_err());
        assert!(extract_timestamp(&serde_json::json!(null)).is_err());
    }

    #[test]
    fn test_extract_nonce_formats() {
        assert_eq!(
            extract_nonce(&serde_json::json!("nonce-1234")).unwrap(),
            "nonce-1234"
        );
        assert_eq!(
            extract_nonce(&serde_json::json!("  nonce-spaced  ")).unwrap(),
            "nonce-spaced"
        );
        assert!(extract_nonce(&serde_json::json!("")).is_err());
        assert!(extract_nonce(&serde_json::json!("   ")).is_err());
        assert!(extract_nonce(&serde_json::json!(1234)).is_err());
        assert!(extract_nonce(&serde_json::json!(null)).is_err());
    }

    #[test]
    fn test_replay_sliding_window_drift() {
        let window = AntiReplayWindow::new(Duration::from_secs(300), 100);
        let base_now = 1_000_000;
        let instant = Instant::now();

        // Exact match
        assert!(window
            .check_and_record_with_time(base_now, "nonce-1", base_now, instant)
            .is_ok());

        // Within 300s in past (e.g. -299s)
        assert!(window
            .check_and_record_with_time(base_now - 299, "nonce-2", base_now, instant)
            .is_ok());

        // Within 300s in future (e.g. +299s)
        assert!(window
            .check_and_record_with_time(base_now + 299, "nonce-3", base_now, instant)
            .is_ok());

        // Boundary: exactly 300s drift
        assert!(window
            .check_and_record_with_time(base_now - 300, "nonce-4", base_now, instant)
            .is_ok());

        // Exceeded in past (-301s)
        let err_past =
            window.check_and_record_with_time(base_now - 301, "nonce-5", base_now, instant);
        assert!(matches!(
            err_past,
            Err(ReplayError::TimestampDriftExceeded { .. })
        ));

        // Exceeded in future (+301s)
        let err_future =
            window.check_and_record_with_time(base_now + 301, "nonce-6", base_now, instant);
        assert!(matches!(
            err_future,
            Err(ReplayError::TimestampDriftExceeded { .. })
        ));
    }

    #[test]
    fn test_replay_duplicate_nonce_rejected() {
        let window = AntiReplayWindow::new(Duration::from_secs(300), 100);
        let base_now = 1_000_000;
        let instant = Instant::now();

        assert!(window
            .check_and_record_with_time(base_now, "unique-nonce", base_now, instant)
            .is_ok());

        // Replay same nonce
        let err = window.check_and_record_with_time(base_now, "unique-nonce", base_now, instant);
        assert_eq!(
            err,
            Err(ReplayError::DuplicateNonce("unique-nonce".to_string()))
        );
    }

    #[test]
    fn test_replay_sliding_window_capacity_eviction() {
        let max = 5;
        let window = AntiReplayWindow::new(Duration::from_secs(300), max);
        let base_now = 1_000_000;
        let instant = Instant::now();

        for i in 0..max {
            assert!(window
                .check_and_record_with_time(base_now, &format!("n-{i}"), base_now, instant)
                .is_ok());
        }
        assert_eq!(window.len(), max);

        // Insert 6th element -> oldest (n-0) should be evicted
        assert!(window
            .check_and_record_with_time(base_now, "n-5", base_now, instant)
            .is_ok());
        assert_eq!(window.len(), max);

        // n-0 was evicted, so n-0 can now be inserted again
        assert!(window
            .check_and_record_with_time(base_now, "n-0", base_now, instant)
            .is_ok());
    }

    #[test]
    fn test_replay_sliding_window_ttl_expiration() {
        let ttl = Duration::from_secs(10);
        let window = AntiReplayWindow::new(ttl, 100);
        let t0_instant = Instant::now();
        let base_now = 1_000_000;

        assert!(window
            .check_and_record_with_time(base_now, "nonce-expire", base_now, t0_instant)
            .is_ok());
        assert_eq!(window.len(), 1);

        // Fast forward instant by 11 seconds (past TTL)
        let t1_instant = t0_instant + Duration::from_secs(11);
        let t1_now = base_now + 11;

        // Inserting another nonce triggers purge of expired nonce
        assert!(window
            .check_and_record_with_time(t1_now, "nonce-fresh", t1_now, t1_instant)
            .is_ok());
        assert_eq!(window.len(), 1);

        // Since nonce-expire was purged, inserting it again at t1_now succeeds
        assert!(window
            .check_and_record_with_time(t1_now, "nonce-expire", t1_now, t1_instant)
            .is_ok());
    }
}
