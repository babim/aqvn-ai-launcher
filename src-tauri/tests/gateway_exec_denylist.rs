//! Outbound Gateway PTY exec command denylist integration tests (VAL-SEC-002).
//!
//! Verifies:
//! - Dangerous shell commands (fork bombs, rm -rf /, mkfs, etc.) are rejected before shell execution
//! - Rejected commands return error with status="denied"
//! - Safe shell commands execute normally in PTY session
//! - PTY session state persists safely across rejected dangerous commands

use aqvn_workspace_lib::connector::handlers::exec;
use aqvn_workspace_lib::connector::protocol::{
    MSG_TYPE_EXEC_OP, MSG_TYPE_EXEC_OP_RESULT, RUNTIME_TERMINAL,
};
use serde_json::{json, Value};
use tokio::sync::{mpsc, Mutex};

static TEST_SERIAL_MUTEX: Mutex<()> = Mutex::const_new(());

#[tokio::test]
async fn test_gateway_exec_denylist() {
    let _serial_guard = TEST_SERIAL_MUTEX.lock().await;
    let session_id = "test-session-denylist-gate";
    exec::reset_session(session_id);

    // 1. Destructive commands population from validation contract & guard
    let dangerous_commands = [
        ":(){ :|:& };:",
        " : ( ) { : | : & } ; :",
        "rm -rf /",
        "rm -fr /",
        "rm -r -f /",
        "rm -rf --no-preserve-root /",
        "sudo rm -rf /",
        "rm -rf /*",
        "rm -rf ~",
        "rm -rf ~/",
        "mkfs.ext4 /dev/sda",
        "mkfs -t ext4 /dev/sdb1",
        "dd if=/dev/zero of=/dev/sda",
        "shutdown now",
        "reboot",
        "poweroff",
        "echo safe_prefix; rm -rf /",
        "true && :(){ :|:& };:",
        "echo test | mkfs /dev/sdb",
    ];

    for (idx, &cmd) in dangerous_commands.iter().enumerate() {
        let op = json!({
            "type": MSG_TYPE_EXEC_OP,
            "op": MSG_TYPE_EXEC_OP,
            "id": format!("req-deny-{idx}"),
            "runtime": RUNTIME_TERMINAL,
            "command": cmd,
            "session_id": session_id,
        });

        let reply_str = exec::handle_exec_op(&op).await;
        let reply: Value = serde_json::from_str(&reply_str)
            .unwrap_or_else(|e| panic!("failed to parse reply for '{cmd}': {e}"));

        assert_eq!(
            reply["type"], MSG_TYPE_EXEC_OP_RESULT,
            "type mismatch for '{cmd}'"
        );
        assert_eq!(reply["id"], format!("req-deny-{idx}"));
        assert_eq!(reply["session_id"], session_id);
        assert_eq!(
            reply["success"], false,
            "dangerous command '{cmd}' must not succeed"
        );
        assert_eq!(
            reply["status"], "denied",
            "dangerous command '{cmd}' must return 'denied' status"
        );
        assert_eq!(reply["done"], true);
        assert_eq!(reply["exit_code"], 1);
        let err_msg = reply["error"].as_str().unwrap_or_default();
        assert!(
            err_msg.contains("denied") || err_msg.contains("refused"),
            "error message for '{cmd}' must state command is denied/refused, got: {err_msg}"
        );
    }

    // 2. Safe shell commands execute normally in PTY session
    let safe_commands = [
        ("echo 'Safe Execution 1'", "Safe Execution 1"),
        ("pwd", ""),
        ("echo safe_part_1 && echo safe_part_2", "safe_part_2"),
        ("rm -f /tmp/safe_probe_nonexistent_test.txt", ""),
    ];

    for (idx, &(cmd, expected_fragment)) in safe_commands.iter().enumerate() {
        let op = json!({
            "type": MSG_TYPE_EXEC_OP,
            "id": format!("req-safe-{idx}"),
            "runtime": RUNTIME_TERMINAL,
            "command": cmd,
            "session_id": session_id,
        });

        let reply_str = exec::handle_exec_op(&op).await;
        let reply: Value = serde_json::from_str(&reply_str)
            .unwrap_or_else(|e| panic!("failed to parse reply for safe command '{cmd}': {e}"));

        assert_eq!(reply["type"], MSG_TYPE_EXEC_OP_RESULT);
        assert_eq!(reply["success"], true, "safe command '{cmd}' must succeed");
        assert_eq!(
            reply["status"], "completed",
            "safe command '{cmd}' must report 'completed'"
        );
        assert_eq!(reply["exit_code"], 0);
        assert_eq!(reply["done"], true);
        if !expected_fragment.is_empty() {
            assert!(
                reply["output"]
                    .as_str()
                    .unwrap_or_default()
                    .contains(expected_fragment),
                "safe command '{cmd}' output must contain '{expected_fragment}'"
            );
        }
    }

    // 3. Stateful session integrity: session persists across denied commands
    let op_env = json!({
        "type": MSG_TYPE_EXEC_OP,
        "id": "req-env-set",
        "runtime": RUNTIME_TERMINAL,
        "command": "export AQVN_DENYLIST_TEST_KEY=session_still_intact",
        "session_id": session_id,
    });
    let rep_env_str = exec::handle_exec_op(&op_env).await;
    let rep_env: Value = serde_json::from_str(&rep_env_str).expect("parse env rep");
    assert_eq!(rep_env["success"], true);

    // Attempt dangerous command in same session
    let op_bad = json!({
        "type": MSG_TYPE_EXEC_OP,
        "id": "req-bad-in-session",
        "runtime": RUNTIME_TERMINAL,
        "command": "rm -rf /",
        "session_id": session_id,
    });
    let rep_bad_str = exec::handle_exec_op(&op_bad).await;
    let rep_bad: Value = serde_json::from_str(&rep_bad_str).expect("parse bad rep");
    assert_eq!(rep_bad["success"], false);
    assert_eq!(rep_bad["status"], "denied");

    // Verify session still holds previous environment variable
    let op_check = json!({
        "type": MSG_TYPE_EXEC_OP,
        "id": "req-env-check",
        "runtime": RUNTIME_TERMINAL,
        "command": "echo \"VAL=$AQVN_DENYLIST_TEST_KEY\"",
        "session_id": session_id,
    });
    let rep_check_str = exec::handle_exec_op(&op_check).await;
    let rep_check: Value = serde_json::from_str(&rep_check_str).expect("parse check rep");
    assert_eq!(rep_check["success"], true);
    assert!(
        rep_check["output"]
            .as_str()
            .unwrap_or_default()
            .contains("VAL=session_still_intact"),
        "session state must survive denied command attempt"
    );

    // 4. Streaming execution rejection
    let (chunk_tx, mut chunk_rx) = mpsc::channel::<String>(16);
    let op_stream_bad = json!({
        "type": MSG_TYPE_EXEC_OP,
        "id": "req-stream-bad",
        "runtime": RUNTIME_TERMINAL,
        "command": "mkfs /dev/sda",
        "session_id": session_id,
    });

    let rep_stream_str = exec::handle_exec_op_streaming(&op_stream_bad, Some(chunk_tx)).await;
    let rep_stream: Value = serde_json::from_str(&rep_stream_str).expect("parse stream rep");
    assert_eq!(rep_stream["success"], false);
    assert_eq!(rep_stream["status"], "denied");
    // Ensure no chunks were sent to client
    assert!(
        chunk_rx.try_recv().is_err(),
        "no chunks should stream for denied command"
    );

    exec::reset_session(session_id);
}
