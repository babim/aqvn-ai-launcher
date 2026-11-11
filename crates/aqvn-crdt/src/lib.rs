//! Distributed CRDT Workspace engine powered by Loro for AQVN Native Workspace.
//!
//! Provides conflict-free replicated data types (CRDTs) for:
//! 1. Movable Tree for hierarchical Workspace file explorer synchronization.
//! 2. Text CRDT Buffer for real-time collaborative editing in CodeMirror 6.
//! 3. Session awareness and cursor presence sharing across multi-host pool agents.
//! 4. Crash-safe durable snapshot persistence and delta sync over WebSockets.

pub mod awareness;
pub mod error;
pub mod integration;
pub mod store;
pub mod text;
pub mod tree;

pub use awareness::{CursorPosition, PeerPresence, SessionAwareness};
pub use error::{CrdtError, CrdtResult};
pub use integration::{
    derive_peer_id, ingest_remote_update, tree_to_remote_update, CrdtWireMessage,
    WorkspaceCrdtConfig,
};
pub use store::{CrdtSnapshotStore, SnapshotMetadata};
pub use text::{CodeMirrorChangeSpec, TextCrdtBuffer, TextDeltaMessage};
pub use tree::{parse_tree_id, WorkspaceMovableTree, WorkspaceNodeMeta, WorkspaceTreeNode};

use loro::{ExportMode, LoroDoc, VersionVector};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};

pub const DEFAULT_TREE_NAME: &str = "workspace_file_tree";

/// Main coordinated CRDT Workspace Engine.
#[derive(Clone)]
pub struct WorkspaceCrdtEngine {
    doc: Arc<LoroDoc>,
    tree: WorkspaceMovableTree,
    awareness: SessionAwareness,
    store: Arc<CrdtSnapshotStore>,
    buffers: Arc<RwLock<HashMap<String, TextCrdtBuffer>>>,
    peer_id: u64,
}

impl WorkspaceCrdtEngine {
    /// Initialize a new CRDT engine for a workspace.
    pub fn new(peer_id: u64, user_label: &str, workspace_root: impl AsRef<Path>) -> Self {
        let doc = Arc::new(LoroDoc::new());
        let _ = doc.set_peer_id(peer_id);

        let tree = WorkspaceMovableTree::from_doc(Arc::clone(&doc), DEFAULT_TREE_NAME);
        let awareness = SessionAwareness::new(peer_id, user_label);
        let store = Arc::new(CrdtSnapshotStore::for_workspace(workspace_root));

        Self {
            doc,
            tree,
            awareness,
            store,
            buffers: Arc::new(RwLock::new(HashMap::new())),
            peer_id,
        }
    }

    /// Access the underlying `LoroDoc`.
    pub fn doc(&self) -> &LoroDoc {
        &self.doc
    }

    /// Local peer ID.
    pub fn peer_id(&self) -> u64 {
        self.peer_id
    }

    /// Access the file explorer Movable Tree.
    pub fn tree(&self) -> &WorkspaceMovableTree {
        &self.tree
    }

    /// Access session awareness and cursor presence.
    pub fn awareness(&self) -> &SessionAwareness {
        &self.awareness
    }

    /// Access durable snapshot store.
    pub fn store(&self) -> &CrdtSnapshotStore {
        &self.store
    }

    /// Open or access a collaborative text buffer for a specific file path.
    pub fn get_or_open_buffer(
        &self,
        file_path: &str,
        initial_content: Option<&str>,
    ) -> TextCrdtBuffer {
        let container_name = format!("buffer:{}", file_path.trim().trim_start_matches('/'));

        if let Ok(buffers) = self.buffers.read() {
            if let Some(buf) = buffers.get(&container_name) {
                return buf.clone();
            }
        }

        let mut buffers = self.buffers.write().unwrap_or_else(|e| e.into_inner());
        if let Some(buf) = buffers.get(&container_name) {
            return buf.clone();
        }

        let buffer = TextCrdtBuffer::from_doc(Arc::clone(&self.doc), &container_name, self.peer_id);

        if let Some(init) = initial_content {
            if buffer.get_content().is_empty() && !init.is_empty() {
                let _ = buffer.insert(0, init);
            }
        }

        buffers.insert(container_name, buffer.clone());
        buffer
    }

    /// Close an open text buffer from the active registry.
    pub fn close_buffer(&self, file_path: &str) -> Option<TextCrdtBuffer> {
        let container_name = format!("buffer:{}", file_path.trim().trim_start_matches('/'));
        if let Ok(mut buffers) = self.buffers.write() {
            buffers.remove(&container_name)
        } else {
            None
        }
    }

    /// List all currently active buffer keys.
    pub fn active_buffers(&self) -> Vec<String> {
        if let Ok(buffers) = self.buffers.read() {
            buffers.keys().cloned().collect()
        } else {
            Vec::new()
        }
    }

    /// Export delta updates since a specific peer version vector.
    pub fn export_delta_from(&self, vv: &VersionVector) -> CrdtResult<Vec<u8>> {
        let bytes = self.doc.export(ExportMode::updates(vv))?;
        Ok(bytes)
    }

    /// Export all updates for full state exchange.
    pub fn export_all_updates(&self) -> CrdtResult<Vec<u8>> {
        let bytes = self.doc.export(ExportMode::all_updates())?;
        Ok(bytes)
    }

    /// Export a full snapshot of all containers (tree + all text buffers).
    pub fn export_snapshot(&self) -> CrdtResult<Vec<u8>> {
        let bytes = self.doc.export(ExportMode::Snapshot)?;
        Ok(bytes)
    }

    /// Ingest updates or snapshot received from a remote peer or disk.
    pub fn import(&self, bytes: &[u8]) -> CrdtResult<()> {
        self.doc.import(bytes)?;
        Ok(())
    }

    /// Current version vector of the workspace document.
    pub fn version_vector(&self) -> VersionVector {
        self.doc.oplog_vv()
    }

    /// Persist the complete CRDT state to the snapshot file on disk.
    pub fn save_snapshot_to_disk(&self) -> CrdtResult<String> {
        let snapshot = self.export_snapshot()?;
        self.store.save_snapshot(&snapshot)
    }

    /// Reload the complete CRDT state from the snapshot file on disk.
    pub fn load_snapshot_from_disk(&self) -> CrdtResult<()> {
        let snapshot = self.store.load_snapshot()?;
        self.import(&snapshot)?;
        Ok(())
    }
}
