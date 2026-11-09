//! Human-in-the-Loop (HITL) Action Approval Modal Gate for Sensitive SSH commands.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub const DEFAULT_HITL_TIMEOUT_SECS: u64 = 30;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HitlDecision {
    Approved,
    Rejected(String),
    TimedOut,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalRequest {
    pub id: String,
    pub command: String,
    pub reason: String,
    pub created_at: Instant,
    pub timestamp_millis: u64,
    pub timeout_seconds: u64,
}

type ApprovalHandler = Box<dyn Fn(&ApprovalRequest) -> HitlDecision + Send + Sync + 'static>;

/// Coordinates Human-in-the-Loop sensitive command approvals with a 30s fail-closed timeout.
#[derive(Clone)]
pub struct HitlApprovalManager {
    timeout: Duration,
    #[allow(dead_code)]
    pending: Arc<Mutex<HashMap<String, ApprovalRequest>>>,
    handler: Arc<Mutex<Option<ApprovalHandler>>>,
    default_policy: Arc<Mutex<Option<HitlDecision>>>,
}

impl Default for HitlApprovalManager {
    fn default() -> Self {
        Self::with_default_timeout()
    }
}

impl HitlApprovalManager {
    pub fn new(timeout_seconds: u64) -> Self {
        Self {
            timeout: Duration::from_secs(timeout_seconds),
            pending: Arc::new(Mutex::new(HashMap::new())),
            handler: Arc::new(Mutex::new(None)),
            default_policy: Arc::new(Mutex::new(None)),
        }
    }

    pub fn with_default_timeout() -> Self {
        Self::new(DEFAULT_HITL_TIMEOUT_SECS)
    }

    /// Sets an interactive or automated approval handler.
    pub fn set_handler<F>(&self, handler: F)
    where
        F: Fn(&ApprovalRequest) -> HitlDecision + Send + Sync + 'static,
    {
        let mut h = self.handler.lock().unwrap();
        *h = Some(Box::new(handler));
    }

    /// Sets a fixed fallback policy (useful for testing approval/rejection branches).
    pub fn set_default_policy(&self, policy: HitlDecision) {
        let mut p = self.default_policy.lock().unwrap();
        *p = Some(policy);
    }

    /// Requests approval for a sensitive command. Fails closed on timeout or rejection.
    pub fn request_approval(&self, command: &str, reason: &str) -> HitlDecision {
        let id = format!("{:x}", rand::random::<u128>());
        let now_millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let req = ApprovalRequest {
            id: id.clone(),
            command: command.to_string(),
            reason: reason.to_string(),
            created_at: Instant::now(),
            timestamp_millis: now_millis,
            timeout_seconds: self.timeout.as_secs(),
        };

        // If an explicit handler is registered, invoke it
        if let Some(ref handler) = *self.handler.lock().unwrap() {
            let start = Instant::now();
            let decision = handler(&req);
            if start.elapsed() > self.timeout {
                return HitlDecision::TimedOut;
            }
            return decision;
        }

        // If a default policy is set (e.g. in test fixtures), return it
        if let Some(ref policy) = *self.default_policy.lock().unwrap() {
            return policy.clone();
        }

        // Fail-closed default: if no interactive UI/handler responded, reject or timeout
        HitlDecision::Rejected(format!(
            "Operation requires human approval (modal timed out or unhandled, fail-closed after {}s)",
            self.timeout.as_secs()
        ))
    }

    pub fn timeout_duration(&self) -> Duration {
        self.timeout
    }
}
