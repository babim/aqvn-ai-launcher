//! Text CRDT Buffer for CodeMirror 6 real-time collaborative editing.
//!
//! Provides conflict-free collaborative text editing, char-level insertion and deletion,
//! delta updates, and bidirectional integration with CodeMirror 6 ChangeSpec.

use crate::error::CrdtResult;
use loro::{ExportMode, LoroDoc, LoroText, VersionVector};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub const DEFAULT_TEXT_CONTAINER: &str = "editor_buffer";

/// Representation of a CodeMirror 6 ChangeSpec (`{ from, to, insert }`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeMirrorChangeSpec {
    pub from: usize,
    pub to: usize,
    pub insert: String,
}

/// JSON-serializable delta message for sending updates to Webview/CodeMirror 6 over IPC/WebSocket.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextDeltaMessage {
    pub buffer_id: String,
    pub peer_id: u64,
    pub changes: Vec<CodeMirrorChangeSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crdt_update_base64: Option<String>,
}

#[derive(Clone)]
pub struct TextCrdtBuffer {
    doc: Arc<LoroDoc>,
    container_id: String,
    peer_id: u64,
}

impl TextCrdtBuffer {
    /// Create a new Text CRDT buffer with initial content and peer ID.
    pub fn new(initial_text: &str, peer_id: u64) -> Self {
        let doc = LoroDoc::new();
        let _ = doc.set_peer_id(peer_id);
        let buffer = Self {
            doc: Arc::new(doc),
            container_id: DEFAULT_TEXT_CONTAINER.to_string(),
            peer_id,
        };
        if !initial_text.is_empty() {
            let _ = buffer.text().insert(0, initial_text);
        }
        buffer
    }

    /// Create a Text CRDT buffer bound to a specific container name in an existing `LoroDoc`.
    pub fn from_doc(doc: Arc<LoroDoc>, container_id: &str, peer_id: u64) -> Self {
        Self {
            doc,
            container_id: container_id.to_string(),
            peer_id,
        }
    }

    /// Access the underlying `LoroDoc`.
    pub fn doc(&self) -> &LoroDoc {
        &self.doc
    }

    /// Current peer ID.
    pub fn peer_id(&self) -> u64 {
        self.peer_id
    }

    /// Buffer / container identifier.
    pub fn buffer_id(&self) -> &str {
        &self.container_id
    }

    /// Access the underlying `LoroText`.
    pub fn text(&self) -> LoroText {
        self.doc.get_text(self.container_id.as_str())
    }

    /// Insert text at character index.
    pub fn insert(&self, pos: usize, content: &str) -> CrdtResult<()> {
        let text = self.text();
        let len = text.len_unicode();
        let safe_pos = pos.min(len);
        text.insert(safe_pos, content)?;
        Ok(())
    }

    /// Delete text range starting at `pos` with length `len`.
    pub fn delete(&self, pos: usize, len: usize) -> CrdtResult<()> {
        let text = self.text();
        let total = text.len_unicode();
        if pos >= total || len == 0 {
            return Ok(());
        }
        let safe_len = len.min(total - pos);
        text.delete(pos, safe_len)?;
        Ok(())
    }

    /// Replace text range `[pos..pos+len]` with `new_content`.
    pub fn replace(&self, pos: usize, len: usize, new_content: &str) -> CrdtResult<()> {
        self.delete(pos, len)?;
        self.insert(pos, new_content)?;
        Ok(())
    }

    /// Apply a CodeMirror 6 ChangeSpec directly (`from`, `to`, `insert`).
    pub fn apply_codemirror_change(&self, change: &CodeMirrorChangeSpec) -> CrdtResult<()> {
        let text = self.text();
        let total = text.len_unicode();
        let from = change.from.min(total);
        let to = change.to.min(total).max(from);
        let delete_len = to - from;

        if delete_len > 0 {
            text.delete(from, delete_len)?;
        }
        if !change.insert.is_empty() {
            text.insert(from, &change.insert)?;
        }
        Ok(())
    }

    /// Apply a batch of CodeMirror 6 ChangeSpecs.
    pub fn apply_codemirror_changes(&self, changes: &[CodeMirrorChangeSpec]) -> CrdtResult<()> {
        for change in changes {
            self.apply_codemirror_change(change)?;
        }
        Ok(())
    }

    /// Get current full text content as a `String`.
    pub fn get_content(&self) -> String {
        self.text().to_string()
    }

    /// Length in UTF-8 bytes.
    pub fn len_utf8(&self) -> usize {
        self.text().len_utf8()
    }

    /// Length in Unicode code points / characters.
    pub fn len_unicode(&self) -> usize {
        self.text().len_unicode()
    }

    /// Number of lines in the buffer.
    pub fn line_count(&self) -> usize {
        let content = self.get_content();
        if content.is_empty() {
            return 1;
        }
        content.lines().count().max(1)
    }

    /// Retrieve a specific line (0-indexed).
    pub fn get_line(&self, line_idx: usize) -> Option<String> {
        let content = self.get_content();
        content.lines().nth(line_idx).map(|s| s.to_string())
    }

    /// Compute minimal ChangeSpecs between an old text snapshot and current buffer content.
    pub fn compute_diff_changes(&self, prev_content: &str) -> Vec<CodeMirrorChangeSpec> {
        let current = self.get_content();
        if prev_content == current {
            return Vec::new();
        }

        let mut start = 0;
        let prev_chars: Vec<char> = prev_content.chars().collect();
        let curr_chars: Vec<char> = current.chars().collect();

        while start < prev_chars.len()
            && start < curr_chars.len()
            && prev_chars[start] == curr_chars[start]
        {
            start += 1;
        }

        let mut prev_end = prev_chars.len();
        let mut curr_end = curr_chars.len();

        while prev_end > start
            && curr_end > start
            && prev_chars[prev_end - 1] == curr_chars[curr_end - 1]
        {
            prev_end -= 1;
            curr_end -= 1;
        }

        let inserted: String = curr_chars[start..curr_end].iter().collect();
        vec![CodeMirrorChangeSpec {
            from: start,
            to: prev_end,
            insert: inserted,
        }]
    }

    /// Export all updates for peer synchronization.
    pub fn export_all_updates(&self) -> CrdtResult<Vec<u8>> {
        let bytes = self.doc.export(ExportMode::all_updates())?;
        Ok(bytes)
    }

    /// Export updates since peer's version vector.
    pub fn export_updates_from(&self, vv: &VersionVector) -> CrdtResult<Vec<u8>> {
        let bytes = self.doc.export(ExportMode::updates(vv))?;
        Ok(bytes)
    }

    /// Export a full snapshot of the text document.
    pub fn export_snapshot(&self) -> CrdtResult<Vec<u8>> {
        let bytes = self.doc.export(ExportMode::Snapshot)?;
        Ok(bytes)
    }

    /// Import CRDT updates or snapshot from a peer.
    pub fn import(&self, bytes: &[u8]) -> CrdtResult<()> {
        self.doc.import(bytes)?;
        Ok(())
    }

    /// Current version vector.
    pub fn version_vector(&self) -> VersionVector {
        self.doc.oplog_vv()
    }
}
