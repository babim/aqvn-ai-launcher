//! Session Awareness & Cursor Presence CRDT for real-time collaborative editing.
//!
//! Synchronizes peer presence, CodeMirror 6 cursor positions, selection ranges,
//! active workspace file, and live agent thoughts across the cluster.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub const DEFAULT_AWARENESS_TIMEOUT_MS: u64 = 30_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CursorPosition {
    pub line: usize,
    pub ch: usize,
    pub head: usize,
    pub anchor: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerPresence {
    pub peer_id: u64,
    pub user_label: String,
    pub active_file: Option<String>,
    pub cursor: Option<CursorPosition>,
    pub agent_thought: Option<String>,
    pub last_active_epoch_ms: u64,
    pub color: String,
}

impl PeerPresence {
    pub fn new(peer_id: u64, user_label: impl Into<String>) -> Self {
        let label = user_label.into();
        let color = generate_peer_color(peer_id);
        Self {
            peer_id,
            user_label: label,
            active_file: None,
            cursor: None,
            agent_thought: None,
            last_active_epoch_ms: current_epoch_ms(),
            color,
        }
    }
}

pub fn generate_peer_color(peer_id: u64) -> String {
    // Generate high-visibility pastel color palette for editor cursors
    const PALETTE: [&str; 8] = [
        "#3b82f6", // Blue
        "#10b981", // Emerald
        "#f59e0b", // Amber
        "#8b5cf6", // Purple
        "#ec4899", // Pink
        "#06b6d4", // Cyan
        "#ef4444", // Red
        "#84cc16", // Lime
    ];
    let idx = (peer_id as usize) % PALETTE.len();
    PALETTE[idx].to_string()
}

pub fn current_epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[derive(Clone)]
pub struct SessionAwareness {
    local_peer_id: u64,
    local_label: String,
    peers: Arc<RwLock<HashMap<u64, PeerPresence>>>,
}

impl SessionAwareness {
    pub fn new(local_peer_id: u64, local_label: impl Into<String>) -> Self {
        let label = local_label.into();
        let local_presence = PeerPresence::new(local_peer_id, &label);
        let mut map = HashMap::new();
        map.insert(local_peer_id, local_presence);

        Self {
            local_peer_id,
            local_label: label,
            peers: Arc::new(RwLock::new(map)),
        }
    }

    pub fn local_peer_id(&self) -> u64 {
        self.local_peer_id
    }

    pub fn local_label(&self) -> &str {
        &self.local_label
    }

    /// Update local user active file and cursor position.
    pub fn update_local_cursor(&self, active_file: Option<&str>, cursor: Option<CursorPosition>) {
        if let Ok(mut peers) = self.peers.write() {
            if let Some(p) = peers.get_mut(&self.local_peer_id) {
                p.active_file = active_file.map(str::to_string);
                p.cursor = cursor;
                p.last_active_epoch_ms = current_epoch_ms();
            }
        }
    }

    /// Update local agent streaming thought.
    pub fn update_local_thought(&self, thought: Option<&str>) {
        if let Ok(mut peers) = self.peers.write() {
            if let Some(p) = peers.get_mut(&self.local_peer_id) {
                p.agent_thought = thought.map(str::to_string);
                p.last_active_epoch_ms = current_epoch_ms();
            }
        }
    }

    /// Update presence state for a remote peer.
    pub fn update_remote_peer(&self, presence: PeerPresence) {
        if let Ok(mut peers) = self.peers.write() {
            peers.insert(presence.peer_id, presence);
        }
    }

    /// Remove a disconnected peer.
    pub fn remove_peer(&self, peer_id: u64) -> Option<PeerPresence> {
        if let Ok(mut peers) = self.peers.write() {
            peers.remove(&peer_id)
        } else {
            None
        }
    }

    /// Prune inactive peers whose last heartbeat exceeds `timeout_ms`.
    pub fn prune_stale_peers(&self, timeout_ms: u64, now_ms: u64) -> usize {
        if let Ok(mut peers) = self.peers.write() {
            let initial = peers.len();
            peers.retain(|&id, p| {
                if id == self.local_peer_id {
                    return true;
                }
                now_ms.saturating_sub(p.last_active_epoch_ms) <= timeout_ms
            });
            initial - peers.len()
        } else {
            0
        }
    }

    /// Retrieve all currently active peer presences.
    pub fn active_peers(&self) -> Vec<PeerPresence> {
        if let Ok(peers) = self.peers.read() {
            peers.values().cloned().collect()
        } else {
            Vec::new()
        }
    }

    /// Retrieve active peers editing a specific workspace file.
    pub fn file_collaborators(&self, file_path: &str) -> Vec<PeerPresence> {
        let norm = file_path.trim().trim_start_matches('/');
        if let Ok(peers) = self.peers.read() {
            peers
                .values()
                .filter(|p| {
                    p.active_file
                        .as_deref()
                        .map(|f| f.trim().trim_start_matches('/') == norm)
                        .unwrap_or(false)
                })
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Serialize presence into WebSocket broadcast payload.
    pub fn to_broadcast_payload(&self) -> serde_json::Value {
        let peers = self.active_peers();
        serde_json::json!({
            "type": "crdt_awareness_update",
            "op": "crdt_awareness_update",
            "sender_peer_id": self.local_peer_id,
            "peers": peers,
            "timestamp": current_epoch_ms(),
        })
    }

    /// Ingest a remote broadcast payload.
    pub fn apply_broadcast_payload(&self, payload: &serde_json::Value) -> Result<usize, String> {
        let peers_val = payload
            .get("peers")
            .and_then(|v| v.as_array())
            .ok_or_else(|| "missing peers array in awareness payload".to_string())?;

        let mut count = 0;
        for item in peers_val {
            if let Ok(presence) = serde_json::from_value::<PeerPresence>(item.clone()) {
                if presence.peer_id != self.local_peer_id {
                    self.update_remote_peer(presence);
                    count += 1;
                }
            }
        }
        Ok(count)
    }
}
