//! Comprehensive CRDT unit and integration tests (VAL-PROD-CRDT-001).

use aqvn_crdt::awareness::{CursorPosition, PeerPresence, SessionAwareness};
use aqvn_crdt::integration::{
    derive_peer_id, ingest_remote_update, tree_to_remote_update, CrdtWireMessage,
    WorkspaceCrdtConfig,
};
use aqvn_crdt::store::CrdtSnapshotStore;
use aqvn_crdt::text::{CodeMirrorChangeSpec, TextCrdtBuffer};
use aqvn_crdt::tree::{parse_tree_id, WorkspaceMovableTree};
use aqvn_crdt::WorkspaceCrdtEngine;
use tempfile::tempdir;

#[test]
fn test_movable_tree_basic_crud() {
    let tree = WorkspaceMovableTree::new(Some(100));

    // 1. Create root directory
    let src_dir = tree
        .create_root_entry("src", true, "/src")
        .expect("create root src");
    assert_eq!(tree.roots().len(), 1);

    // 2. Create children inside src
    let lib_rs = tree
        .create_entry(
            Some(src_dir),
            "lib.rs",
            false,
            "/src/lib.rs",
            1024,
            1700000000,
        )
        .expect("create lib.rs");
    let main_rs = tree
        .create_entry(
            Some(src_dir),
            "main.rs",
            false,
            "/src/main.rs",
            2048,
            1700000001,
        )
        .expect("create main.rs");

    assert_eq!(tree.children(Some(src_dir)).len(), 2);
    assert_eq!(tree.parent(lib_rs), Some(src_dir));
    assert_eq!(tree.parent(main_rs), Some(src_dir));

    // 3. Query metadata
    let meta = tree.get_meta(lib_rs).expect("get_meta");
    assert_eq!(meta.name, "lib.rs");
    assert_eq!(meta.path, "/src/lib.rs");
    assert!(!meta.is_dir);
    assert_eq!(meta.size, 1024);
    assert_eq!(meta.modified_at, 1700000000);

    // 4. Update metadata
    tree.update_metadata(lib_rs, 2048, 1700000500)
        .expect("update_metadata");
    let updated_meta = tree.get_meta(lib_rs).expect("get_meta");
    assert_eq!(updated_meta.size, 2048);
    assert_eq!(updated_meta.modified_at, 1700000500);

    // 5. Rename entry
    tree.rename_entry(lib_rs, "core.rs", "/src/core.rs")
        .expect("rename_entry");
    let renamed_meta = tree.get_meta(lib_rs).expect("get_meta");
    assert_eq!(renamed_meta.name, "core.rs");
    assert_eq!(renamed_meta.path, "/src/core.rs");

    // 6. Find by path
    let found = tree.find_by_path("/src/core.rs");
    assert_eq!(found, Some(lib_rs));
    let not_found = tree.find_by_path("/src/nonexistent.rs");
    assert_eq!(not_found, None);

    // 7. Delete entry
    tree.delete_entry(main_rs).expect("delete_entry");
    assert_eq!(tree.children(Some(src_dir)).len(), 1);
}

#[test]
fn test_movable_tree_concurrent_cycle_prevention() {
    // Peer 1 and Peer 2 start with identical tree containing Folder A and Folder B
    let tree1 = WorkspaceMovableTree::new(Some(1));
    let folder_a = tree1
        .create_root_entry("FolderA", true, "/FolderA")
        .expect("create A");
    let folder_b = tree1
        .create_root_entry("FolderB", true, "/FolderB")
        .expect("create B");

    let tree2 = WorkspaceMovableTree::new(Some(2));
    let init_updates = tree1.export_all_updates().expect("export");
    tree2.import(&init_updates).expect("import init");

    // Peer 1 moves Folder B inside Folder A
    tree1
        .move_entry(folder_b, Some(folder_a))
        .expect("move B->A");

    // Peer 2 CONCURRENTLY moves Folder A inside Folder B (potential cycle!)
    tree2
        .move_entry(folder_a, Some(folder_b))
        .expect("move A->B");

    // Exchange delta updates
    let u1 = tree1.export_all_updates().expect("export u1");
    let u2 = tree2.export_all_updates().expect("export u2");

    tree1.import(&u2).expect("import u2 into tree1");
    tree2.import(&u1).expect("import u1 into tree2");

    // Both trees must deterministically converge without cyclic parent hangs or errors
    let p1_a = tree1.parent(folder_a);
    let p2_a = tree2.parent(folder_a);
    assert_eq!(p1_a, p2_a, "Folder A parents must match across peers");

    let p1_b = tree1.parent(folder_b);
    let p2_b = tree2.parent(folder_b);
    assert_eq!(p1_b, p2_b, "Folder B parents must match across peers");

    // Ensure they didn't create a mutual cycle (A parent of B AND B parent of A)
    let is_cycle = p1_a == Some(folder_b) && p1_b == Some(folder_a);
    assert!(!is_cycle, "Loro Movable Tree must resolve cycle conflict");
}

#[test]
fn test_movable_tree_concurrent_rename() {
    let tree1 = WorkspaceMovableTree::new(Some(10));
    let file = tree1
        .create_root_entry("untitled.txt", false, "/untitled.txt")
        .expect("create");

    let tree2 = WorkspaceMovableTree::new(Some(20));
    tree2
        .import(&tree1.export_all_updates().expect("export"))
        .expect("import");

    // Peer 1 renames to "notes.txt"
    tree1
        .rename_entry(file, "notes.txt", "/notes.txt")
        .expect("rename 1");

    // Peer 2 renames to "todo.txt"
    tree2
        .rename_entry(file, "todo.txt", "/todo.txt")
        .expect("rename 2");

    // Exchange updates
    let u1 = tree1.export_all_updates().expect("export 1");
    let u2 = tree2.export_all_updates().expect("export 2");
    tree1.import(&u2).expect("import 2");
    tree2.import(&u1).expect("import 1");

    let meta1 = tree1.get_meta(file).expect("meta1");
    let meta2 = tree2.get_meta(file).expect("meta2");
    assert_eq!(
        meta1.name, meta2.name,
        "Renames must deterministically converge"
    );
    assert_eq!(meta1.path, meta2.path);
}

#[test]
fn test_text_crdt_buffer_basic_ops() {
    let buffer = TextCrdtBuffer::new("Hello World", 42);
    assert_eq!(buffer.get_content(), "Hello World");
    assert_eq!(buffer.len_utf8(), 11);
    assert_eq!(buffer.len_unicode(), 11);
    assert_eq!(buffer.line_count(), 1);

    // Insert
    buffer.insert(5, " Beautiful").expect("insert");
    assert_eq!(buffer.get_content(), "Hello Beautiful World");

    // Delete
    buffer.delete(5, 10).expect("delete");
    assert_eq!(buffer.get_content(), "Hello World");

    // Replace
    buffer.replace(6, 5, "Rustaceans!").expect("replace");
    assert_eq!(buffer.get_content(), "Hello Rustaceans!");

    // CodeMirror ChangeSpec application
    let change = CodeMirrorChangeSpec {
        from: 0,
        to: 5,
        insert: "Greetings".to_string(),
    };
    buffer
        .apply_codemirror_change(&change)
        .expect("apply change");
    assert_eq!(buffer.get_content(), "Greetings Rustaceans!");

    // Diff calculation
    let diff = buffer.compute_diff_changes("Greetings Everyone!");
    assert!(!diff.is_empty());
}

#[test]
fn test_text_crdt_concurrent_edits_convergence() {
    // Peer 1, Peer 2, Peer 3 start with the same document
    let b1 = TextCrdtBuffer::new("fn main() {\n    // placeholder\n}\n", 1);
    let init_delta = b1.export_all_updates().expect("export");

    let b2 = TextCrdtBuffer::new("", 2);
    b2.import(&init_delta).expect("import b2");

    let b3 = TextCrdtBuffer::new("", 3);
    b3.import(&init_delta).expect("import b3");

    // Peer 1 inserts at line 1
    b1.insert(0, "// Author: Peer 1\n").expect("p1 insert");

    // Peer 2 inserts in function body
    b2.replace(21, 14, "println!(\"Hello from Peer 2\");")
        .expect("p2 replace");

    // Peer 3 appends a comment at the end
    let b3_len = b3.len_unicode();
    b3.insert(b3_len, "// End of file\n").expect("p3 insert");

    // Cross-import all delta updates between all peers
    let d1 = b1.export_all_updates().expect("d1");
    let d2 = b2.export_all_updates().expect("d2");
    let d3 = b3.export_all_updates().expect("d3");

    b1.import(&d2).expect("b1+d2");
    b1.import(&d3).expect("b1+d3");

    b2.import(&d1).expect("b2+d1");
    b2.import(&d3).expect("b2+d3");

    b3.import(&d1).expect("b3+d1");
    b3.import(&d2).expect("b3+d2");

    // All peers must have the exact same text content!
    let s1 = b1.get_content();
    let s2 = b2.get_content();
    let s3 = b3.get_content();

    assert_eq!(s1, s2, "Peer 1 and 2 must converge");
    assert_eq!(s2, s3, "Peer 2 and 3 must converge");
    assert!(s1.contains("Author: Peer 1"));
    assert!(s1.contains("println!(\"Hello from Peer 2\");"));
    assert!(s1.contains("End of file"));
}

#[test]
fn test_session_awareness() {
    let awareness = SessionAwareness::new(101, "Alice");
    assert_eq!(awareness.local_peer_id(), 101);
    assert_eq!(awareness.local_label(), "Alice");

    // Update local cursor & thought
    awareness.update_local_cursor(
        Some("src/main.rs"),
        Some(CursorPosition {
            line: 12,
            ch: 4,
            head: 154,
            anchor: 154,
        }),
    );
    awareness.update_local_thought(Some("Refactoring tree sync handler"));

    // Add remote peer
    let mut remote = PeerPresence::new(202, "Bob");
    remote.active_file = Some("src/main.rs".to_string());
    awareness.update_remote_peer(remote);

    assert_eq!(awareness.active_peers().len(), 2);
    let collabs = awareness.file_collaborators("src/main.rs");
    assert_eq!(collabs.len(), 2);

    // Serialization to broadcast payload
    let payload = awareness.to_broadcast_payload();
    assert_eq!(payload["type"], "crdt_awareness_update");
    assert_eq!(payload["sender_peer_id"], 101);

    // Remote peer awareness ingestion
    let peer_b_awareness = SessionAwareness::new(303, "Charlie");
    let ingested = peer_b_awareness
        .apply_broadcast_payload(&payload)
        .expect("ingest");
    assert!(ingested >= 1);
    assert_eq!(peer_b_awareness.active_peers().len(), 3);
}

#[test]
fn test_crdt_snapshot_store_integrity() {
    let temp = tempdir().expect("tempdir");
    let store = CrdtSnapshotStore::for_workspace(temp.path());

    let payload = b"LORO_CRDT_SNAPSHOT_TEST_DATA_PAYLOAD_123456789";
    let sha_hex = store.save_snapshot(payload).expect("save_snapshot");
    assert_eq!(sha_hex.len(), 64);
    assert!(store.path().exists());

    // Read metadata
    let meta = store.read_metadata().expect("read_metadata");
    assert_eq!(meta.version, 1);
    assert_eq!(meta.payload_len, payload.len());
    assert_eq!(meta.sha256_hex, sha_hex);

    // Reload snapshot
    let loaded = store.load_snapshot().expect("load_snapshot");
    assert_eq!(&loaded[..], payload);

    // Tamper detection
    let mut file_bytes = std::fs::read(store.path()).expect("read");
    let last = file_bytes.len() - 1;
    file_bytes[last] ^= 0xFF; // Flip bits
    std::fs::write(store.path(), &file_bytes).expect("write tampered");

    let err = store.load_snapshot().unwrap_err();
    match err {
        aqvn_crdt::CrdtError::IntegrityMismatch { .. } => {}
        other => panic!("expected IntegrityMismatch, got: {other:?}"),
    }
}

#[test]
fn test_connector_integration_tree_update() {
    let tree = WorkspaceMovableTree::new(Some(1));
    let dir_src = tree.create_root_entry("src", true, "/src").unwrap();
    let _ = tree.create_entry(Some(dir_src), "lib.rs", false, "/src/lib.rs", 512, 100);
    let _ = tree.create_entry(Some(dir_src), "main.rs", false, "/src/main.rs", 1024, 101);

    // Export to ConnectorRemoteTreeUpdate
    let update = tree_to_remote_update(&tree, "/a0", 3);
    assert_eq!(update.msg_type, "connector_remote_tree_update");
    assert_eq!(update.workspace_path, "/a0");
    assert_eq!(update.total_count, Some(3));
    assert_eq!(update.total_files, Some(2));
    assert_eq!(update.total_dirs, Some(1));

    // Ingest into another tree
    let tree_clone = WorkspaceMovableTree::new(Some(2));
    let count = ingest_remote_update(&tree_clone, &update).expect("ingest");
    assert_eq!(count, 3);
    assert!(tree_clone.find_by_path("/src/lib.rs").is_some());
    assert!(tree_clone.find_by_path("/src/main.rs").is_some());
}

#[test]
fn test_crdt_wire_message_delta_and_snapshot() {
    let delta_data = b"sample_delta_bytes";
    let wire = CrdtWireMessage::delta("text_edit", Some("buffer:lib.rs"), 42, delta_data);
    assert_eq!(wire.action, "text_edit");
    assert_eq!(wire.peer_id, 42);

    let decoded = wire.decode_delta().expect("decode").unwrap();
    assert_eq!(&decoded[..], delta_data);

    let snapshot_data = b"sample_snapshot_bytes";
    let wire_snap = CrdtWireMessage::snapshot("full_sync", None, 42, snapshot_data);
    let decoded_snap = wire_snap.decode_snapshot().expect("decode").unwrap();
    assert_eq!(&decoded_snap[..], snapshot_data);
}

#[test]
fn test_workspace_engine_coordinator() {
    let temp = tempdir().expect("tempdir");
    let engine1 = WorkspaceCrdtEngine::new(1, "Agent-1", temp.path());
    let engine2 = WorkspaceCrdtEngine::new(2, "Agent-2", temp.path().join("peer2"));

    // Tree operation in engine 1
    let dir = engine1
        .tree()
        .create_root_entry("docs", true, "/docs")
        .unwrap();
    let _file =
        engine1
            .tree()
            .create_entry(Some(dir), "README.md", false, "/docs/README.md", 256, 1);

    // Text buffer operation in engine 1
    let buf1 = engine1.get_or_open_buffer("docs/README.md", Some("# Project Docs\n"));
    buf1.insert(14, "\nWelcome to AQVN CRDT!\n").unwrap();

    // Export workspace delta
    let delta = engine1.export_all_updates().unwrap();
    engine2.import(&delta).unwrap();

    // Verify engine 2 received both tree and text buffer updates
    assert!(engine2.tree().find_by_path("/docs/README.md").is_some());
    let buf2 = engine2.get_or_open_buffer("docs/README.md", None);
    assert_eq!(buf1.get_content(), buf2.get_content());
    assert!(buf2.get_content().contains("Welcome to AQVN CRDT!"));

    // Test saving and loading snapshot
    let sha = engine1
        .save_snapshot_to_disk()
        .expect("save_snapshot_to_disk");
    assert_eq!(sha.len(), 64);
    assert!(engine1.store().path().exists());

    let engine3 = WorkspaceCrdtEngine::new(3, "Agent-3", temp.path());
    engine3
        .load_snapshot_from_disk()
        .expect("load_snapshot_from_disk");
    let buf3 = engine3.get_or_open_buffer("docs/README.md", None);
    assert_eq!(buf3.get_content(), buf1.get_content());
}

#[test]
fn test_parse_tree_id_helper() {
    let id_str = "10@42";
    let parsed = parse_tree_id(id_str).expect("parse @");
    assert_eq!(parsed.to_string(), "10@42");

    let colon_str = "42:10";
    let parsed_colon = parse_tree_id(colon_str).expect("parse :");
    assert_eq!(parsed_colon.to_string(), "10@42");

    assert!(parse_tree_id("invalid").is_none());
}

#[test]
fn test_derive_peer_id_from_gateway_id() {
    let id1 = derive_peer_id("aqvn-gw-0123456789ab");
    let id2 = derive_peer_id("aqvn-gw-0123456789ab");
    let id3 = derive_peer_id("aqvn-gw-fedcba987654");

    assert_eq!(id1, id2, "Deterministic ID must match for same input");
    assert_ne!(id1, id3, "Distinct inputs must produce distinct peer IDs");
    assert_ne!(id1, 0, "Peer ID must not be zero");

    let cfg = aqvn_core::config::AppConfig::default();
    let crdt_cfg = WorkspaceCrdtConfig::from_app_config(&cfg);
    assert!(crdt_cfg.peer_id > 0);
    assert_eq!(crdt_cfg.server_url, cfg.server_url);
}
