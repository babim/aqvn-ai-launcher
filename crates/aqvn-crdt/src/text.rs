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

    /// Length in UTF-16 code units (for CodeMirror 6 / JavaScript compatibility).
    pub fn len_utf16(&self) -> usize {
        self.text().len_utf16()
    }

    /// Insert text at character index.
    pub fn insert(&self, pos: usize, content: &str) -> CrdtResult<()> {
        let text = self.text();
        let len = text.len_unicode();
        let safe_pos = pos.min(len);
        text.insert(safe_pos, content)?;
        Ok(())
    }

    /// Insert text at UTF-16 code-unit index.
    pub fn insert_utf16(&self, pos: usize, content: &str) -> CrdtResult<()> {
        let text = self.text();
        let len = text.len_utf16();
        let safe_pos = pos.min(len);
        text.insert_utf16(safe_pos, content)?;
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

    /// Delete text range starting at UTF-16 `pos` with length `len`.
    pub fn delete_utf16(&self, pos: usize, len: usize) -> CrdtResult<()> {
        let text = self.text();
        let total = text.len_utf16();
        if pos >= total || len == 0 {
            return Ok(());
        }
        let safe_len = len.min(total - pos);
        text.delete_utf16(pos, safe_len)?;
        Ok(())
    }

    /// Replace text range `[pos..pos+len]` with `new_content`.
    pub fn replace(&self, pos: usize, len: usize, new_content: &str) -> CrdtResult<()> {
        self.delete(pos, len)?;
        self.insert(pos, new_content)?;
        Ok(())
    }

    /// Apply a CodeMirror 6 ChangeSpec directly (`from`, `to`, `insert`) using UTF-16 code units.
    pub fn apply_codemirror_change(&self, change: &CodeMirrorChangeSpec) -> CrdtResult<()> {
        let text = self.text();
        let total = text.len_utf16();
        let from = change.from.min(total);
        let to = change.to.min(total).max(from);
        let delete_len = to - from;

        if delete_len > 0 {
            text.delete_utf16(from, delete_len)?;
        }
        if !change.insert.is_empty() {
            text.insert_utf16(from, &change.insert)?;
        }
        Ok(())
    }

    /// Apply a batch of CodeMirror 6 ChangeSpecs in reverse order to preserve offset positions.
    pub fn apply_codemirror_changes(&self, changes: &[CodeMirrorChangeSpec]) -> CrdtResult<()> {
        let mut sorted = changes.to_vec();
        sorted.sort_by_key(|b| std::cmp::Reverse(b.from));
        for change in &sorted {
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

    /// Number of lines in the buffer matching CodeMirror 6.
    pub fn line_count(&self) -> usize {
        let content = self.get_content();
        if content.is_empty() {
            return 1;
        }
        content.split('\n').count().max(1)
    }

    /// Retrieve a specific line (0-indexed).
    pub fn get_line(&self, line_idx: usize) -> Option<String> {
        let content = self.get_content();
        content.lines().nth(line_idx).map(|s| s.to_string())
    }

    /// Compute minimal ChangeSpecs between an old text snapshot and current buffer content using UTF-16 code units.
    pub fn compute_diff_changes(&self, prev_content: &str) -> Vec<CodeMirrorChangeSpec> {
        let current = self.get_content();
        if prev_content == current {
            return Vec::new();
        }

        let mut start = 0;
        let prev_u16: Vec<u16> = prev_content.encode_utf16().collect();
        let curr_u16: Vec<u16> = current.encode_utf16().collect();

        while start < prev_u16.len() && start < curr_u16.len() && prev_u16[start] == curr_u16[start]
        {
            start += 1;
        }

        let mut prev_end = prev_u16.len();
        let mut curr_end = curr_u16.len();

        while prev_end > start
            && curr_end > start
            && prev_u16[prev_end - 1] == curr_u16[curr_end - 1]
        {
            prev_end -= 1;
            curr_end -= 1;
        }

        let inserted = String::from_utf16_lossy(&curr_u16[start..curr_end]);
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
