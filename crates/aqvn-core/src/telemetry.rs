use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HostSystemMetrics {
    pub cpu_count: usize,
    pub cpu_usage_percent: f64,
    pub total_ram_mb: u64,
    pub available_ram_mb: u64,
    pub free_ram_mb: u64,
    pub disk_free_gb: f64,
    pub os: String,
    pub arch: String,
}
