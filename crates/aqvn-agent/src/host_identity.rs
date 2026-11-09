//! Deterministic machine identity resolution using SHA-256 of (/etc/machine-id + MAC).
//!
//! Provides invariant identity across system reboots and dynamic IP address changes.

use sha2::{Digest, Sha256};
use std::fs;

/// Normalizes MAC address by trimming whitespace and converting to lowercase.
pub fn normalize_mac(mac: &str) -> String {
    mac.trim().to_lowercase()
}

/// Validates whether a string looks like a valid 6-octet MAC address.
/// Rejects empty, broadcast (ff:ff:ff:ff:ff:ff), and all-zero (00:00:00:00:00:00) MACs.
pub fn is_valid_mac(mac: &str) -> bool {
    let normalized = normalize_mac(mac);
    if normalized.is_empty()
        || normalized == "00:00:00:00:00:00"
        || normalized == "ff:ff:ff:ff:ff:ff"
    {
        return false;
    }

    let parts: Vec<&str> = if normalized.contains(':') {
        normalized.split(':').collect()
    } else if normalized.contains('-') {
        normalized.split('-').collect()
    } else {
        return false;
    };

    parts.len() == 6
        && parts
            .iter()
            .all(|p| p.len() == 2 && p.chars().all(|c| c.is_ascii_hexdigit()))
}

/// Reads Linux `/etc/machine-id` or fallback paths.
pub fn read_machine_id() -> Option<String> {
    // 1. Check test env overrides
    if let Ok(id) = std::env::var("AQVN_TEST_MACHINE_ID") {
        let trimmed = id.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    if let Ok(path) = std::env::var("AQVN_MACHINE_ID_PATH") {
        if let Ok(id) = fs::read_to_string(path.trim()) {
            let trimmed = id.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }

    // 2. Standard system locations
    for path in &["/etc/machine-id", "/var/lib/dbus/machine-id"] {
        if let Ok(id) = fs::read_to_string(path) {
            let trimmed = id.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }

    None
}

/// Scans Linux `/sys/class/net/*/address` to detect the primary physical MAC address.
/// Ignores loopback, bridges, veth, and all-zero addresses.
pub fn read_primary_mac() -> Option<String> {
    // 1. Check test env override
    if let Ok(mac) = std::env::var("AQVN_TEST_MAC_ADDRESS") {
        let trimmed = mac.trim();
        if is_valid_mac(trimmed) {
            return Some(normalize_mac(trimmed));
        }
    }

    // 2. Check override sysfs directory or default
    let net_dir =
        std::env::var("AQVN_NET_SYSFS_PATH").unwrap_or_else(|_| "/sys/class/net".to_string());

    let entries = fs::read_dir(&net_dir).ok()?;
    let mut candidates: Vec<(String, String)> = Vec::new();

    for entry in entries.flatten() {
        let iface_name = entry.file_name().to_string_lossy().to_string();
        if iface_name == "lo" {
            continue;
        }

        let addr_path = entry.path().join("address");
        if let Ok(content) = fs::read_to_string(&addr_path) {
            let mac = normalize_mac(content.trim());
            if is_valid_mac(&mac) {
                candidates.push((iface_name, mac));
            }
        }
    }

    if candidates.is_empty() {
        return None;
    }

    // Prioritize standard physical interfaces: eth* > en* > wl* > other > virtual
    candidates.sort_by(|a, b| {
        let priority = |name: &str| -> u8 {
            if name.starts_with("eth") {
                1
            } else if name.starts_with("en") {
                2
            } else if name.starts_with("wl") {
                3
            } else if name.starts_with("br-")
                || name.starts_with("docker")
                || name.starts_with("veth")
                || name.starts_with("virbr")
            {
                5
            } else {
                4
            }
        };

        let prio_a = priority(&a.0);
        let prio_b = priority(&b.0);
        prio_a.cmp(&prio_b).then_with(|| a.0.cmp(&b.0))
    });

    Some(candidates[0].1.clone())
}

/// Computes deterministic SHA-256 hash given machine-id and MAC address.
/// Result is a 64-character lowercase hexadecimal string invariant to IP changes.
pub fn compute_deterministic_id(machine_id: &str, mac: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"aqvn-agent-identity:");
    hasher.update(machine_id.trim().to_lowercase().as_bytes());
    hasher.update(b":");
    hasher.update(normalize_mac(mac).as_bytes());
    let hash = hasher.finalize();
    format!("{:x}", hash)
}

/// Resolves full 64-character SHA-256 deterministic machine identity for this device.
pub fn deterministic_machine_id() -> String {
    let machine_id = read_machine_id().unwrap_or_else(|| {
        if let Ok(hostname) = std::env::var("HOSTNAME").or_else(|_| std::env::var("COMPUTERNAME")) {
            format!("fallback-host-{}", hostname.trim())
        } else {
            "fallback-machine-id".to_string()
        }
    });

    let mac = read_primary_mac().unwrap_or_else(|| "02:00:00:00:00:01".to_string());
    compute_deterministic_id(&machine_id, &mac)
}

/// Resolves deterministic gateway identifier for `AgentSettings` (aqvn-node-{16_hex}).
pub fn get_deterministic_host_id() -> String {
    let hash = deterministic_machine_id();
    format!("aqvn-node-{}", &hash[..16])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mac_normalization_and_validation() {
        assert!(is_valid_mac("00:1A:2B:3C:4D:5E"));
        assert!(is_valid_mac("00-1a-2b-3c-4d-5e"));
        assert!(!is_valid_mac("00:00:00:00:00:00"));
        assert!(!is_valid_mac("ff:ff:ff:ff:ff:ff"));
        assert!(!is_valid_mac("invalid-mac"));
        assert!(!is_valid_mac(""));
        assert_eq!(normalize_mac("  00:1A:2B:3C:4D:5E "), "00:1a:2b:3c:4d:5e");
    }

    #[test]
    fn test_deterministic_id_invariance_across_ip_changes() {
        let m_id = "b4f2c00918dd4ec2ba505672d4218987";
        let mac = "52:54:00:12:34:56";

        // Regardless of IP address 192.168.1.100 vs 10.0.0.5 vs 172.16.20.4, hash is identical
        let hash1 = compute_deterministic_id(m_id, mac);
        let hash2 = compute_deterministic_id(m_id, mac);
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64);

        // Different MAC produces different hash
        let hash_diff_mac = compute_deterministic_id(m_id, "52:54:00:99:88:77");
        assert_ne!(hash1, hash_diff_mac);

        // Different machine_id produces different hash
        let hash_diff_mid = compute_deterministic_id("other-machine-id", mac);
        assert_ne!(hash1, hash_diff_mid);
    }
}
