//! Durable Crash-Safe CRDT Snapshot Store.
//!
//! Persists binary Loro CRDT snapshots to `.aqvn/workspace.crdt` with atomic
//! temp-file writes and SHA-256 integrity envelopes to prevent partial-write corruption.

use crate::error::{CrdtError, CrdtResult};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const SNAPSHOT_MAGIC: &[u8; 8] = b"AQVNCRDT";
const SNAPSHOT_VERSION: u16 = 1;
const HEADER_LEN: usize = 8 + 2 + 2 + 8 + 32 + 8; // 60 bytes

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotMetadata {
    pub version: u16,
    pub timestamp_secs: u64,
    pub sha256_hex: String,
    pub payload_len: usize,
}

pub struct CrdtSnapshotStore {
    snapshot_path: PathBuf,
}

impl CrdtSnapshotStore {
    /// Create a snapshot store pointing to `<workspace_root>/.aqvn/workspace.crdt`.
    pub fn for_workspace(workspace_root: impl AsRef<Path>) -> Self {
        let snapshot_path = workspace_root.as_ref().join(".aqvn").join("workspace.crdt");
        Self { snapshot_path }
    }

    /// Create a snapshot store at an explicit file path.
    pub fn at_path(path: impl AsRef<Path>) -> Self {
        Self {
            snapshot_path: path.as_ref().to_path_buf(),
        }
    }

    /// Path to the persisted snapshot file.
    pub fn path(&self) -> &Path {
        &self.snapshot_path
    }

    /// Save snapshot bytes atomically with SHA-256 verification header.
    /// Returns the SHA-256 hex digest of the stored payload.
    pub fn save_snapshot(&self, payload: &[u8]) -> CrdtResult<String> {
        if let Some(parent) = self.snapshot_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut hasher = Sha256::new();
        hasher.update(payload);
        let digest: [u8; 32] = hasher.finalize().into();
        let digest_hex = hex_encode(&digest);

        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let mut buf = Vec::with_capacity(HEADER_LEN + payload.len());
        buf.extend_from_slice(SNAPSHOT_MAGIC);
        buf.extend_from_slice(&SNAPSHOT_VERSION.to_le_bytes());
        buf.extend_from_slice(&0u16.to_le_bytes()); // reserved flags
        buf.extend_from_slice(&ts.to_le_bytes());
        buf.extend_from_slice(&digest);
        buf.extend_from_slice(&(payload.len() as u64).to_le_bytes());
        buf.extend_from_slice(payload);

        // Atomic write via temp file in same directory
        let parent = self
            .snapshot_path
            .parent()
            .unwrap_or_else(|| Path::new("."));
        let file_name = self
            .snapshot_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("workspace.crdt");

        static TMP_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let seq = TMP_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let tmp_path = parent.join(format!(".{file_name}.tmp.{}.{seq}", std::process::id()));

        {
            let mut f = File::create(&tmp_path)?;
            f.write_all(&buf)?;
            f.sync_all()?;
        }

        if let Err(err) = fs::rename(&tmp_path, &self.snapshot_path) {
            let _ = fs::remove_file(&tmp_path);
            return Err(CrdtError::Io(err));
        }

        Ok(digest_hex)
    }

    /// Load and integrity-verify snapshot bytes from disk.
    pub fn load_snapshot(&self) -> CrdtResult<Vec<u8>> {
        if !self.snapshot_path.exists() {
            return Err(CrdtError::PersistenceError(format!(
                "snapshot file does not exist: {}",
                self.snapshot_path.display()
            )));
        }

        let mut raw = Vec::new();
        File::open(&self.snapshot_path)?.read_to_end(&mut raw)?;

        if raw.len() < HEADER_LEN {
            return Err(CrdtError::PersistenceError(
                "snapshot file is truncated (smaller than header)".to_string(),
            ));
        }

        if &raw[0..8] != SNAPSHOT_MAGIC {
            return Err(CrdtError::PersistenceError(
                "invalid snapshot magic header".to_string(),
            ));
        }

        let version = u16::from_le_bytes([raw[8], raw[9]]);
        if version != SNAPSHOT_VERSION {
            return Err(CrdtError::PersistenceError(format!(
                "unsupported snapshot version: {version}"
            )));
        }

        let expected_digest = &raw[20..52];
        let payload_len_u64 = u64::from_le_bytes(raw[52..60].try_into().unwrap());
        let payload_len = usize::try_from(payload_len_u64).map_err(|_| {
            CrdtError::PersistenceError("snapshot payload length overflows usize".to_string())
        })?;

        if raw.len() != HEADER_LEN + payload_len {
            return Err(CrdtError::PersistenceError(format!(
                "snapshot payload length mismatch: header declares {payload_len}, got {}",
                raw.len().saturating_sub(HEADER_LEN)
            )));
        }

        let payload = &raw[HEADER_LEN..];
        let mut hasher = Sha256::new();
        hasher.update(payload);
        let actual_digest: [u8; 32] = hasher.finalize().into();

        if actual_digest != expected_digest {
            return Err(CrdtError::IntegrityMismatch {
                expected: hex_encode(expected_digest),
                actual: hex_encode(&actual_digest),
            });
        }

        Ok(payload.to_vec())
    }

    /// Read snapshot metadata without loading full payload into memory if valid.
    pub fn read_metadata(&self) -> CrdtResult<SnapshotMetadata> {
        if !self.snapshot_path.exists() {
            return Err(CrdtError::PersistenceError(format!(
                "snapshot file does not exist: {}",
                self.snapshot_path.display()
            )));
        }

        let mut header = [0u8; HEADER_LEN];
        let mut file = File::open(&self.snapshot_path)?;
        file.read_exact(&mut header)
            .map_err(|_| CrdtError::PersistenceError("snapshot header incomplete".to_string()))?;

        if &header[0..8] != SNAPSHOT_MAGIC {
            return Err(CrdtError::PersistenceError(
                "invalid snapshot magic header".to_string(),
            ));
        }

        let version = u16::from_le_bytes([header[8], header[9]]);
        let timestamp_secs = u64::from_le_bytes(header[12..20].try_into().unwrap());
        let sha256_hex = hex_encode(&header[20..52]);
        let payload_len = u64::from_le_bytes(header[52..60].try_into().unwrap()) as usize;

        Ok(SnapshotMetadata {
            version,
            timestamp_secs,
            sha256_hex,
            payload_len,
        })
    }

    /// Remove persisted snapshot from disk.
    pub fn clear(&self) -> CrdtResult<()> {
        if self.snapshot_path.exists() {
            fs::remove_file(&self.snapshot_path)?;
        }
        Ok(())
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(&mut s, "{:02x}", b);
    }
    s
}
