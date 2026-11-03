//! Host system telemetry protocol handler for Outbound Host Gateway Hub (`_a0_connector`).
//!
//! Handles `connector_telemetry_op` and returns `connector_telemetry_op_result` with
//! CPU core count, CPU usage percentage, total RAM, free RAM, available RAM, and disk free space.

use crate::connector::protocol::{
    attach_id_and_op_id, extract_id_and_op_id, ConnectorTelemetryOpResult, HostSystemMetrics,
    MSG_TYPE_TELEMETRY_OP_RESULT,
};
use serde_json::Value;
use std::fs;
use std::sync::RwLock;
use std::time::Duration;

static TEST_SYSTEM_METRICS: RwLock<Option<HostSystemMetrics>> = RwLock::new(None);
static LAST_CPU_SAMPLE: RwLock<Option<(u64, u64)>> = RwLock::new(None);

/// Overrides system metrics collection for testing or mocked scenarios.
pub fn set_test_system_metrics(metrics: Option<HostSystemMetrics>) {
    if let Ok(mut lock) = TEST_SYSTEM_METRICS.write() {
        *lock = metrics;
    }
}

/// Reads Linux `/proc/stat` to sample cumulative CPU ticks (total_ticks, idle_ticks).
#[cfg(target_os = "linux")]
fn sample_linux_cpu_ticks() -> Option<(u64, u64)> {
    let stat_content = fs::read_to_string("/proc/stat").ok()?;
    for line in stat_content.lines() {
        if line.starts_with("cpu ") {
            let mut parts = line.split_whitespace().skip(1);
            let user: u64 = parts.next()?.parse().ok()?;
            let nice: u64 = parts.next()?.parse().ok()?;
            let system: u64 = parts.next()?.parse().ok()?;
            let idle: u64 = parts.next()?.parse().ok()?;
            let iowait: u64 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
            let irq: u64 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
            let softirq: u64 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
            let steal: u64 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);

            let idle_ticks = idle.saturating_add(iowait);
            let active_ticks = user
                .saturating_add(nice)
                .saturating_add(system)
                .saturating_add(irq)
                .saturating_add(softirq)
                .saturating_add(steal);
            let total_ticks = idle_ticks.saturating_add(active_ticks);
            return Some((total_ticks, idle_ticks));
        }
    }
    None
}

/// Computes the CPU usage percentage from ticks delta or baseline.
pub fn compute_cpu_usage_percent() -> f64 {
    #[cfg(target_os = "linux")]
    {
        if let Some((cur_total, cur_idle)) = sample_linux_cpu_ticks() {
            let mut lock = match LAST_CPU_SAMPLE.write() {
                Ok(l) => l,
                Err(_) => return 0.0,
            };

            let (prev_total, prev_idle) = match *lock {
                Some(prev) => prev,
                None => {
                    // Initial sample: sleep brief duration (15ms) to establish delta
                    std::thread::sleep(Duration::from_millis(15));
                    let next = sample_linux_cpu_ticks().unwrap_or((cur_total, cur_idle));
                    *lock = Some(next);
                    (cur_total, cur_idle)
                }
            };

            *lock = Some((cur_total, cur_idle));

            let total_diff = cur_total.saturating_sub(prev_total);
            let idle_diff = cur_idle.saturating_sub(prev_idle);

            if total_diff > 0 {
                let active_diff = total_diff.saturating_sub(idle_diff);
                let pct = (active_diff as f64 / total_diff as f64) * 100.0;
                return (pct.clamp(0.0, 100.0) * 100.0).round() / 100.0;
            }
        }
    }

    0.0
}

/// Parses `/proc/meminfo` on Linux to extract memory in megabytes (total, free, available).
#[cfg(target_os = "linux")]
fn sample_linux_meminfo() -> (u64, u64, u64) {
    let mut total_mb = 0u64;
    let mut free_mb = 0u64;
    let mut available_mb = 0u64;

    if let Ok(mem_content) = fs::read_to_string("/proc/meminfo") {
        for line in mem_content.lines() {
            let mut parts = line.split_whitespace();
            let key = match parts.next() {
                Some(k) => k,
                None => continue,
            };
            let val: u64 = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0);
            match key {
                "MemTotal:" => total_mb = val / 1024,
                "MemFree:" => free_mb = val / 1024,
                "MemAvailable:" => available_mb = val / 1024,
                _ => {}
            }
        }
    }

    if available_mb == 0 {
        available_mb = free_mb;
    }

    (total_mb, free_mb, available_mb)
}

/// Fallback memory info extraction on non-Linux Unix platforms using sysconf.
#[cfg(not(target_os = "linux"))]
fn sample_mem_fallback() -> (u64, u64, u64) {
    #[cfg(unix)]
    unsafe {
        let pages = libc::sysconf(libc::_SC_PHYS_PAGES);
        let page_size = libc::sysconf(libc::_SC_PAGE_SIZE);
        if pages > 0 && page_size > 0 {
            let total_bytes = (pages as u64).saturating_mul(page_size as u64);
            let total_mb = total_bytes / (1024 * 1024);
            return (total_mb, total_mb / 2, total_mb / 2);
        }
    }
    (0, 0, 0)
}

/// Computes the disk free space in gigabytes using `statvfs` on Unix platforms.
pub fn compute_disk_free_gb(target_path: &str) -> f64 {
    #[cfg(unix)]
    {
        use std::ffi::CString;
        use std::mem::MaybeUninit;

        let path_to_probe = if target_path.is_empty() {
            "/"
        } else {
            target_path
        };
        let c_path = match CString::new(path_to_probe) {
            Ok(p) => p,
            Err(_) => return 0.0,
        };

        unsafe {
            let mut stat = MaybeUninit::<libc::statvfs>::uninit();
            if libc::statvfs(c_path.as_ptr(), stat.as_mut_ptr()) == 0 {
                let stat = stat.assume_init();
                #[allow(clippy::unnecessary_cast)]
                let free_bytes = (stat.f_bavail as u64).saturating_mul(stat.f_frsize as u64);
                let free_gb = (free_bytes as f64) / (1024.0 * 1024.0 * 1024.0);
                return (free_gb * 100.0).round() / 100.0;
            }
        }
    }

    #[cfg(not(unix))]
    {
        let _ = target_path;
    }

    0.0
}

/// Gathers the live system metrics from the host operating system.
pub fn collect_system_metrics() -> HostSystemMetrics {
    if let Ok(lock) = TEST_SYSTEM_METRICS.read() {
        if let Some(ref mock) = *lock {
            return mock.clone();
        }
    }

    let cpu_count = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);

    let cpu_usage_percent = compute_cpu_usage_percent();

    #[cfg(target_os = "linux")]
    let (total_ram_mb, free_ram_mb, available_ram_mb) = sample_linux_meminfo();

    #[cfg(not(target_os = "linux"))]
    let (total_ram_mb, free_ram_mb, available_ram_mb) = sample_mem_fallback();

    let disk_free_gb = compute_disk_free_gb("/");

    HostSystemMetrics {
        cpu_count,
        cpu_usage_percent,
        total_ram_mb,
        available_ram_mb,
        free_ram_mb,
        disk_free_gb,
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
    }
}

/// Dispatches an incoming `connector_telemetry_op` envelope.
pub async fn handle_telemetry_op(envelope: &Value) -> String {
    let (id_str, op_id_str) = extract_id_and_op_id(envelope, "tel-op-req");
    let id = id_str.as_str();
    let op_id = op_id_str.as_str();

    let metrics = collect_system_metrics();

    let action = envelope
        .get("action")
        .or_else(|| envelope.get("operation"))
        .or_else(|| envelope.get("cmd"))
        .or_else(|| envelope.get("params").and_then(|p| p.get("action")))
        .and_then(Value::as_str)
        .map(String::from);

    let resp = ConnectorTelemetryOpResult {
        msg_type: MSG_TYPE_TELEMETRY_OP_RESULT.to_string(),
        op: MSG_TYPE_TELEMETRY_OP_RESULT.to_string(),
        id: id.to_string(),
        op_id: op_id.to_string(),
        success: true,
        status: "ok".to_string(),
        action,
        cpu_count: metrics.cpu_count,
        cpu_usage_percent: metrics.cpu_usage_percent,
        total_ram: metrics.total_ram_mb,
        free_ram: metrics.free_ram_mb,
        total_ram_mb: metrics.total_ram_mb,
        free_ram_mb: metrics.free_ram_mb,
        available_ram_mb: metrics.available_ram_mb,
        disk_free_gb: metrics.disk_free_gb,
        os: metrics.os.clone(),
        arch: metrics.arch.clone(),
        metrics: Some(metrics),
        error: None,
    };

    let raw_reply = serde_json::to_string(&resp).unwrap_or_else(|_| "{}".to_string());
    attach_id_and_op_id(raw_reply, id, op_id)
}
