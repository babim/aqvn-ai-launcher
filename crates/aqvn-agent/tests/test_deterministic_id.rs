//! Integration tests for VAL-AGENT-ID-001:
//! Deterministic Machine Identity, Systemd Watchdog (sd_notify), and Service Install Privilege Detection.

use aqvn_agent::host_identity::compute_deterministic_id;
use aqvn_agent::watchdog::{notify_ready, notify_stopping, notify_watchdog, watchdog_interval};
use aqvn_agent::{generate_systemd_unit, parse_cli_args, resolve_is_user_unit, CliAction};
use std::env;
use std::os::unix::net::UnixDatagram;
use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;

static TEST_MUTEX: Mutex<()> = Mutex::new(());

#[test]
fn test_deterministic_id() {
    let _lock = TEST_MUTEX.lock().unwrap();

    let machine_id = "4f8a3d21b9c7e0123456789abcdef012";
    let mac = "00:1A:2B:3C:4D:5E";

    // 1. SHA-256 hash invariant across reboots and calls
    let hash_boot_1 = compute_deterministic_id(machine_id, mac);
    let hash_boot_2 = compute_deterministic_id(machine_id, mac);
    assert_eq!(
        hash_boot_1, hash_boot_2,
        "Device identity must be deterministic and invariant across reboots"
    );
    assert_eq!(
        hash_boot_1.len(),
        64,
        "SHA-256 digest must be 64 hexadecimal characters"
    );

    // 2. Invariance across simulated dynamic IP changes
    let ip_initial = "10.0.4.15";
    let ip_after_dhcp_renew = "192.168.100.88";
    let ip_roaming_vpn = "172.24.1.200";

    // Simulating system environment changes with IP shifts
    env::set_var("AQVN_CURRENT_IP", ip_initial);
    let hash_ip_1 = compute_deterministic_id(machine_id, mac);

    env::set_var("AQVN_CURRENT_IP", ip_after_dhcp_renew);
    let hash_ip_2 = compute_deterministic_id(machine_id, mac);

    env::set_var("AQVN_CURRENT_IP", ip_roaming_vpn);
    let hash_ip_3 = compute_deterministic_id(machine_id, mac);

    assert_eq!(
        hash_ip_1, hash_ip_2,
        "Identity must not change when IP address changes"
    );
    assert_eq!(
        hash_ip_2, hash_ip_3,
        "Identity must remain invariant across network switches"
    );

    env::remove_var("AQVN_CURRENT_IP");

    // 3. Sensitivity: different MAC or machine-id yields different hash
    let hash_diff_mac = compute_deterministic_id(machine_id, "00:1A:2B:3C:4D:5F");
    assert_ne!(
        hash_boot_1, hash_diff_mac,
        "Different MAC address must produce distinct machine ID"
    );

    let hash_diff_machine = compute_deterministic_id("other_machine_id_string", mac);
    assert_ne!(
        hash_boot_1, hash_diff_machine,
        "Different /etc/machine-id must produce distinct machine ID"
    );

    // 4. Host node format
    let node_id = format!("aqvn-node-{}", &hash_boot_1[..16]);
    assert!(node_id.starts_with("aqvn-node-"));
    assert_eq!(node_id.len(), 26);
}

#[test]
fn test_deterministic_id_systemd_watchdog_sd_notify() {
    let _lock = TEST_MUTEX.lock().unwrap();

    let temp_dir = env::temp_dir().join(format!("test_watchdog_val_001_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let sock_path = temp_dir.join("systemd_notify.sock");
    let listener = UnixDatagram::bind(&sock_path).expect("bind mock systemd notify socket");
    listener.set_nonblocking(true).unwrap();

    env::set_var("NOTIFY_SOCKET", &sock_path);
    env::set_var("WATCHDOG_USEC", "20000000"); // 20s watchdog

    // 1. Check interval calculation: half of WATCHDOG_USEC (10s)
    let interval = watchdog_interval().expect("watchdog interval present");
    assert_eq!(interval, Duration::from_secs(10));

    // 2. Check READY=1 notification
    assert!(notify_ready(), "notify_ready must succeed");
    let mut buf = [0u8; 64];
    let (n1, _) = listener.recv_from(&mut buf).expect("read READY datagram");
    assert_eq!(String::from_utf8_lossy(&buf[..n1]), "READY=1\n");

    // 3. Check WATCHDOG=1 periodic keepalive
    assert!(notify_watchdog(), "notify_watchdog must succeed");
    let (n2, _) = listener
        .recv_from(&mut buf)
        .expect("read WATCHDOG datagram");
    assert_eq!(String::from_utf8_lossy(&buf[..n2]), "WATCHDOG=1\n");

    // 4. Check STOPPING=1 notification
    assert!(notify_stopping(), "notify_stopping must succeed");
    let (n3, _) = listener
        .recv_from(&mut buf)
        .expect("read STOPPING datagram");
    assert_eq!(String::from_utf8_lossy(&buf[..n3]), "STOPPING=1\n");

    env::remove_var("NOTIFY_SOCKET");
    env::remove_var("WATCHDOG_USEC");
    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_deterministic_id_service_install_auto_privilege() {
    let _lock = TEST_MUTEX.lock().unwrap();

    // 1. Service unit generation includes WatchdogSec=30
    let bin = Path::new("/opt/aqvn/aqvn-agent");
    let ws = Path::new("/var/aqvn/workspace");
    let unit = generate_systemd_unit(bin, ws, None, false);
    assert!(
        unit.contains("WatchdogSec=30"),
        "Generated unit must include WatchdogSec=30"
    );
    assert!(unit.contains("WantedBy=multi-user.target"));

    // 2. Auto-detection: UID 0 installs as system service unit
    env::set_var("AQVN_FORCE_EUID", "0");
    let is_user_root = resolve_is_user_unit(None);
    assert!(
        !is_user_root,
        "Root privilege (UID 0) must auto-detect system unit"
    );

    // 3. Auto-detection: Non-root UID installs as user service unit
    env::set_var("AQVN_FORCE_EUID", "1001");
    let is_user_non_root = resolve_is_user_unit(None);
    assert!(
        is_user_non_root,
        "Non-root privilege (UID 1001) must auto-detect user unit"
    );

    // 4. CLI command: aqvn-agent service install
    let parsed = parse_cli_args(&["service".to_string(), "install".to_string()])
        .expect("service install parsing succeeds");
    assert_eq!(parsed.action, CliAction::Install);

    // 5. CLI command: aqvn-agent service install --system
    let parsed_sys = parse_cli_args(&[
        "service".to_string(),
        "install".to_string(),
        "--system".to_string(),
    ])
    .expect("service install --system succeeds");
    assert_eq!(parsed_sys.action, CliAction::Install);
    assert_eq!(parsed_sys.options.user_unit, Some(false));

    // 6. CLI command: aqvn-agent service install --user
    let parsed_usr = parse_cli_args(&[
        "service".to_string(),
        "install".to_string(),
        "--user".to_string(),
    ])
    .expect("service install --user succeeds");
    assert_eq!(parsed_usr.action, CliAction::Install);
    assert_eq!(parsed_usr.options.user_unit, Some(true));

    env::remove_var("AQVN_FORCE_EUID");
}
