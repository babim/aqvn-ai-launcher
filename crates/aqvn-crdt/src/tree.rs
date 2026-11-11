//! Movable Tree CRDT for Workspace File Explorer synchronization.
//!
//! Provides conflict-free collaborative file tree operations (create, move, rename, delete)
//! preventing cycle-orphan anomalies when peers move directories concurrently.

use crate::error::CrdtResult;
use loro::{ExportMode, LoroDoc, LoroTree, TreeID, TreeParentId, VersionVector};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub const DEFAULT_TREE_NAME: &str = "workspace_file_tree";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceNodeMeta {
    pub id: String,
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified_at: u64,
    pub parent_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceTreeNode {
    pub meta: WorkspaceNodeMeta,
    pub children: Vec<WorkspaceTreeNode>,
}

#[derive(Clone)]
pub struct WorkspaceMovableTree {
    doc: Arc<LoroDoc>,
    tree_name: String,
}

impl WorkspaceMovableTree {
    /// Creates a new workspace tree backed by a fresh `LoroDoc` with an optional peer ID.
    pub fn new(peer_id: Option<u64>) -> Self {
        let doc = LoroDoc::new();
        if let Some(id) = peer_id {
            let _ = doc.set_peer_id(id);
        }
        Self {
            doc: Arc::new(doc),
            tree_name: DEFAULT_TREE_NAME.to_string(),
        }
    }

    /// Creates a tree view bound to an existing shared `LoroDoc`.
    pub fn from_doc(doc: Arc<LoroDoc>, tree_name: &str) -> Self {
        Self {
            doc,
            tree_name: tree_name.to_string(),
        }
    }

    /// Access the underlying `LoroDoc`.
    pub fn doc(&self) -> &LoroDoc {
        &self.doc
    }

    /// Access the underlying `LoroTree`.
    pub fn tree(&self) -> LoroTree {
        self.doc.get_tree(self.tree_name.as_str())
    }

    /// Create a new entry in the workspace tree.
    pub fn create_entry(
        &self,
        parent: Option<TreeID>,
        name: &str,
        is_dir: bool,
        path: &str,
        size: u64,
        modified_at: u64,
    ) -> CrdtResult<TreeID> {
        let tree = self.tree();
        let node_id = tree.create(parent)?;
        let meta = tree.get_meta(node_id)?;

        meta.insert("name", name.to_string())?;
        meta.insert("path", path.to_string())?;
        meta.insert("is_dir", is_dir)?;
        meta.insert("size", size as i64)?;
        meta.insert("modified_at", modified_at as i64)?;

        Ok(node_id)
    }

    /// Create a root-level entry in the workspace tree.
    pub fn create_root_entry(&self, name: &str, is_dir: bool, path: &str) -> CrdtResult<TreeID> {
        self.create_entry(None, name, is_dir, path, 0, 0)
    }

    /// Move an existing node to a new parent.
    /// Conflict-free: concurrent parent cyclic moves are deterministically resolved by Loro.
    pub fn move_entry(&self, target: TreeID, new_parent: Option<TreeID>) -> CrdtResult<()> {
        let tree = self.tree();
        tree.mov(target, new_parent)?;
        Ok(())
    }

    /// Move an existing node to a new parent at a specific child index.
    pub fn move_entry_to_index(
        &self,
        target: TreeID,
        new_parent: Option<TreeID>,
        index: usize,
    ) -> CrdtResult<()> {
        let tree = self.tree();
        tree.mov_to(target, new_parent, index)?;
        Ok(())
    }

    /// Rename an existing entry and update its recorded relative path.
    pub fn rename_entry(&self, target: TreeID, new_name: &str, new_path: &str) -> CrdtResult<()> {
        let tree = self.tree();
        let meta = tree.get_meta(target)?;
        meta.insert("name", new_name.to_string())?;
        meta.insert("path", new_path.to_string())?;
        Ok(())
    }

    /// Update size and modification timestamp metadata for a node.
    pub fn update_metadata(&self, target: TreeID, size: u64, modified_at: u64) -> CrdtResult<()> {
        let tree = self.tree();
        let meta = tree.get_meta(target)?;
        meta.insert("size", size as i64)?;
        meta.insert("modified_at", modified_at as i64)?;
        Ok(())
    }

    /// Delete an entry from the tree.
    pub fn delete_entry(&self, target: TreeID) -> CrdtResult<()> {
        let tree = self.tree();
        tree.delete(target)?;
        Ok(())
    }

    /// Get the parent of a node, if any.
    pub fn parent(&self, target: TreeID) -> Option<TreeID> {
        let tree = self.tree();
        match tree.parent(target) {
            Some(TreeParentId::Node(id)) => Some(id),
            _ => None,
        }
    }

    /// Query metadata of a specific node.
    pub fn get_meta(&self, target: TreeID) -> CrdtResult<WorkspaceNodeMeta> {
        let tree = self.tree();
        let meta_map = tree.get_meta(target)?;

        let name = meta_map
            .get("name")
            .and_then(|v| v.into_value().ok())
            .and_then(|v| v.into_string().ok())
            .map(|s| s.to_string())
            .unwrap_or_default();

        let path = meta_map
            .get("path")
            .and_then(|v| v.into_value().ok())
            .and_then(|v| v.into_string().ok())
            .map(|s| s.to_string())
            .unwrap_or_default();

        let is_dir = meta_map
            .get("is_dir")
            .and_then(|v| v.into_value().ok())
            .and_then(|v| v.into_bool().ok())
            .unwrap_or(false);

        let size = meta_map
            .get("size")
            .and_then(|v| v.into_value().ok())
            .and_then(|v| v.into_i64().ok())
            .map(|n| n.max(0) as u64)
            .unwrap_or(0);

        let modified_at = meta_map
            .get("modified_at")
            .and_then(|v| v.into_value().ok())
            .and_then(|v| v.into_i64().ok())
            .map(|n| n.max(0) as u64)
            .unwrap_or(0);

        let parent_id = self.parent(target).map(|p| p.to_string());

        Ok(WorkspaceNodeMeta {
            id: target.to_string(),
            name,
            path,
            is_dir,
            size,
            modified_at,
            parent_id,
        })
    }

    /// Find an active node by its path attribute.
    pub fn find_by_path(&self, path: &str) -> Option<TreeID> {
        let clean = path.trim().trim_start_matches('/');
        for node_id in self.all_nodes() {
            if let Ok(meta) = self.get_meta(node_id) {
                let node_path = meta.path.trim().trim_start_matches('/');
                if node_path == clean {
                    return Some(node_id);
                }
            }
        }
        None
    }

    /// Get all root node IDs.
    pub fn roots(&self) -> Vec<TreeID> {
        self.tree().roots()
    }

    /// Get children of a node, or roots if `parent` is None.
    pub fn children(&self, parent: Option<TreeID>) -> Vec<TreeID> {
        self.tree().children(parent).unwrap_or_default()
    }

    /// Get all active (non-deleted) node IDs in the tree.
    pub fn all_nodes(&self) -> Vec<TreeID> {
        let tree = self.tree();
        tree.nodes()
            .into_iter()
            .filter(|id| !tree.is_node_deleted(id).unwrap_or(true))
            .collect()
    }

    /// Build a hierarchical tree representation.
    pub fn build_tree_hierarchy(&self) -> Vec<WorkspaceTreeNode> {
        let roots = self.roots();
        roots
            .into_iter()
            .map(|r| self.build_node_recursive(r))
            .collect()
    }

    fn build_node_recursive(&self, node_id: TreeID) -> WorkspaceTreeNode {
        let meta = self
            .get_meta(node_id)
            .unwrap_or_else(|_| WorkspaceNodeMeta {
                id: node_id.to_string(),
                name: String::new(),
                path: String::new(),
                is_dir: false,
                size: 0,
                modified_at: 0,
                parent_id: None,
            });

        let children_ids = self.children(Some(node_id));
        let children = children_ids
            .into_iter()
            .map(|child_id| self.build_node_recursive(child_id))
            .collect();

        WorkspaceTreeNode { meta, children }
    }

    /// Convert tree hierarchy into JSON format matching `ConnectorRemoteTreeUpdate`.
    pub fn to_connector_tree_json(&self, workspace_root: &str) -> serde_json::Value {
        fn node_to_json(node: &WorkspaceTreeNode) -> serde_json::Value {
            let mut val = serde_json::json!({
                "name": node.meta.name,
                "path": node.meta.path,
                "is_dir": node.meta.is_dir,
                "size": node.meta.size,
                "modified": node.meta.modified_at,
                "crdt_id": node.meta.id,
            });
            if node.meta.is_dir {
                let children_json: Vec<serde_json::Value> =
                    node.children.iter().map(node_to_json).collect();
                val["children"] = serde_json::Value::Array(children_json);
            }
            val
        }

        let hierarchy = self.build_tree_hierarchy();
        let tree_array: Vec<serde_json::Value> = hierarchy.iter().map(node_to_json).collect();
        let total_nodes = self.all_nodes().len();

        serde_json::json!({
            "type": "connector_remote_tree_update",
            "op": "connector_remote_tree_update",
            "workspace_path": workspace_root,
            "root": workspace_root,
            "max_depth": 10,
            "tree": tree_array,
            "total_count": total_nodes,
        })
    }

    /// Export all updates for syncing with peers.
    pub fn export_all_updates(&self) -> CrdtResult<Vec<u8>> {
        let bytes = self.doc.export(ExportMode::all_updates())?;
        Ok(bytes)
    }

    /// Export delta updates since the given peer version vector.
    pub fn export_updates_from(&self, vv: &VersionVector) -> CrdtResult<Vec<u8>> {
        let bytes = self.doc.export(ExportMode::updates(vv))?;
        Ok(bytes)
    }

    /// Export a full snapshot of the CRDT document.
    pub fn export_snapshot(&self) -> CrdtResult<Vec<u8>> {
        let bytes = self.doc.export(ExportMode::Snapshot)?;
        Ok(bytes)
    }

    /// Import CRDT updates or snapshot from a peer or disk.
    pub fn import(&self, bytes: &[u8]) -> CrdtResult<()> {
        self.doc.import(bytes)?;
        Ok(())
    }

    /// Current version vector of this CRDT document.
    pub fn version_vector(&self) -> VersionVector {
        self.doc.oplog_vv()
    }
}

/// Helper function to parse TreeID from string format ("counter@peer" or "peer:counter").
pub fn parse_tree_id(s: &str) -> Option<TreeID> {
    let trimmed = s.trim();
    if let Some((c_str, p_str)) = trimmed.split_once('@') {
        let counter = c_str.parse::<i32>().ok()?;
        let peer = p_str.parse::<u64>().ok()?;
        Some(TreeID::new(peer, counter))
    } else if let Some((p_str, c_str)) = trimmed.split_once(':') {
        let peer = p_str.parse::<u64>().ok()?;
        let counter = c_str.parse::<i32>().ok()?;
        Some(TreeID::new(peer, counter))
    } else {
        None
    }
}
