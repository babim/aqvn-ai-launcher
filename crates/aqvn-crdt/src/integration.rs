//! Integration bridges for `crates/aqvn-core` and `crates/aqvn-connector`.
//!
//! Provides conversion between Loro CRDT workspace structures and AQVN Host Gateway Hub
//! remote protocol messages (`ConnectorRemoteTreeUpdate`, `ConnectorFileOp`, `AppConfig`).

use crate::error::{CrdtError, CrdtResult};
use crate::tree::{WorkspaceMovableTree, WorkspaceTreeNode};
use aqvn_connector::protocol::{ConnectorRemoteTreeUpdate, MSG_TYPE_REMOTE_TREE_UPDATE};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

/// CRDT engine configuration derived from `aqvn-core::config::AppConfig`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceCrdtConfig {
    pub gateway_id: String,
    pub peer_id: u64,
    pub workspace_path: PathBuf,
    pub server_url: String,
}

impl WorkspaceCrdtConfig {
    /// Build CRDT engine configuration from `aqvn_core::config::AppConfig`.
    pub fn from_app_config(config: &aqvn_core::config::AppConfig) -> Self {
        let gateway_id = aqvn_core::config::deterministic_gateway_id();
        let peer_id = derive_peer_id(&gateway_id);
        Self {
            gateway_id,
            peer_id,
            workspace_path: PathBuf::from(&config.workspace_path),
            server_url: config.server_url.clone(),
        }
    }
}

/// Derive a deterministic, non-zero 64-bit peer ID from a gateway or machine ID.
pub fn derive_peer_id(identifier: &str) -> u64 {
    let mut hasher = Sha256::new();
    hasher.update(identifier.as_bytes());
    let hash = hasher.finalize();
    let raw = u64::from_le_bytes(hash[0..8].try_into().unwrap());
    if raw == 0 {
        1
    } else {
        raw
    }
}

/// Format a `WorkspaceMovableTree` into a `ConnectorRemoteTreeUpdate` struct for `aqvn-connector`.
pub fn tree_to_remote_update(
    tree: &WorkspaceMovableTree,
    workspace_path: &str,
    max_depth: usize,
) -> ConnectorRemoteTreeUpdate {
    let mut flat_entries = Vec::new();
    let mut total_files = 0usize;
    let mut total_dirs = 0usize;

    fn build_json_node(
        node: &WorkspaceTreeNode,
        depth: usize,
        max_depth: usize,
        flat: &mut Vec<Value>,
        files: &mut usize,
        dirs: &mut usize,
    ) -> Value {
        if node.meta.is_dir {
            *dirs += 1;
        } else {
            *files += 1;
        }

        let entry_json = json!({
            "name": node.meta.name,
            "path": node.meta.path,
            "is_file": !node.meta.is_dir,
            "is_dir": node.meta.is_dir,
            "depth": depth,
            "size": node.meta.size,
            "modified": node.meta.modified_at,
            "crdt_id": node.meta.id,
        });
        flat.push(entry_json);

        let children_json = if node.meta.is_dir && depth < max_depth {
            node.children
                .iter()
                .map(|child| build_json_node(child, depth + 1, max_depth, flat, files, dirs))
                .collect()
        } else {
            Vec::new()
        };

        json!({
            "name": node.meta.name,
            "path": node.meta.path,
            "is_file": !node.meta.is_dir,
            "is_dir": node.meta.is_dir,
            "depth": depth,
            "size": node.meta.size,
            "modified": node.meta.modified_at,
            "crdt_id": node.meta.id,
            "children": children_json,
        })
    }

    let hierarchy = tree.build_tree_hierarchy();
    let mut tree_nodes = Vec::new();
    for root_node in &hierarchy {
        let node_json = build_json_node(
            root_node,
            1,
            max_depth,
            &mut flat_entries,
            &mut total_files,
            &mut total_dirs,
        );
        tree_nodes.push(node_json);
    }

    let total_count = flat_entries.len();

    ConnectorRemoteTreeUpdate {
        msg_type: MSG_TYPE_REMOTE_TREE_UPDATE.to_string(),
        op: MSG_TYPE_REMOTE_TREE_UPDATE.to_string(),
        id: Some(format!("crdt-tree-sync-{}", std::process::id())),
        op_id: Some("crdt-tree-op".to_string()),
        workspace_path: workspace_path.to_string(),
        root: workspace_path.to_string(),
        max_depth,
        tree: tree_nodes,
        entries: Some(flat_entries),
        count: Some(total_count),
        total_count: Some(total_count),
        total_files: Some(total_files),
        total_dirs: Some(total_dirs),
    }
}

/// Recursively ingest a connector tree update into `WorkspaceMovableTree`.
pub fn ingest_remote_update(
    tree: &WorkspaceMovableTree,
    update: &ConnectorRemoteTreeUpdate,
) -> CrdtResult<usize> {
    let mut count = 0;

    fn ingest_node(
        tree: &WorkspaceMovableTree,
        node: &Value,
        parent_id: Option<loro::TreeID>,
        count: &mut usize,
    ) -> CrdtResult<()> {
        let name = node.get("name").and_then(Value::as_str).unwrap_or_default();
        let path = node.get("path").and_then(Value::as_str).unwrap_or_default();
        let is_dir = node.get("is_dir").and_then(Value::as_bool).unwrap_or(false);
        let size = node.get("size").and_then(Value::as_u64).unwrap_or(0);
        let modified = node
            .get("modified")
            .and_then(Value::as_f64)
            .map(|f| f as u64)
            .unwrap_or(0);

        if name.is_empty() {
            return Ok(());
        }

        let existing = tree.find_by_path(path);
        let current_node = if let Some(id) = existing {
            let _ = tree.update_metadata(id, size, modified);
            id
        } else {
            tree.create_entry(parent_id, name, is_dir, path, size, modified)?
        };
        *count += 1;

        if is_dir {
            if let Some(children) = node.get("children").and_then(Value::as_array) {
                for child in children {
                    ingest_node(tree, child, Some(current_node), count)?;
                }
            }
        }

        Ok(())
    }

    for root_item in &update.tree {
        ingest_node(tree, root_item, None, &mut count)?;
    }

    Ok(count)
}

/// CRDT Wire Envelope for WebSocket sync operations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CrdtWireMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub op: String,
    pub action: String,
    pub buffer_id: Option<String>,
    pub peer_id: u64,
    pub delta_base64: Option<String>,
    pub snapshot_base64: Option<String>,
}

impl CrdtWireMessage {
    /// Create a binary delta sync message.
    pub fn delta(action: &str, buffer_id: Option<&str>, peer_id: u64, delta_bytes: &[u8]) -> Self {
        Self {
            msg_type: "crdt_sync_message".to_string(),
            op: "crdt_sync".to_string(),
            action: action.to_string(),
            buffer_id: buffer_id.map(str::to_string),
            peer_id,
            delta_base64: Some(BASE64.encode(delta_bytes)),
            snapshot_base64: None,
        }
    }

    /// Create a full snapshot sync message.
    pub fn snapshot(
        action: &str,
        buffer_id: Option<&str>,
        peer_id: u64,
        snapshot_bytes: &[u8],
    ) -> Self {
        Self {
            msg_type: "crdt_sync_message".to_string(),
            op: "crdt_sync".to_string(),
            action: action.to_string(),
            buffer_id: buffer_id.map(str::to_string),
            peer_id,
            delta_base64: None,
            snapshot_base64: Some(BASE64.encode(snapshot_bytes)),
        }
    }

    /// Decode the binary delta payload if present.
    pub fn decode_delta(&self) -> CrdtResult<Option<Vec<u8>>> {
        if let Some(ref b64) = self.delta_base64 {
            let bytes = BASE64
                .decode(b64.trim())
                .map_err(|e| CrdtError::SyncError(format!("invalid delta base64: {e}")))?;
            Ok(Some(bytes))
        } else {
            Ok(None)
        }
    }

    /// Decode the binary snapshot payload if present.
    pub fn decode_snapshot(&self) -> CrdtResult<Option<Vec<u8>>> {
        if let Some(ref b64) = self.snapshot_base64 {
            let bytes = BASE64
                .decode(b64.trim())
                .map_err(|e| CrdtError::SyncError(format!("invalid snapshot base64: {e}")))?;
            Ok(Some(bytes))
        } else {
            Ok(None)
        }
    }
}
