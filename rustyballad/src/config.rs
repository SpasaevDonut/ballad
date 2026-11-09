//! Compile-time defaults replicating `quickshell/Config.qml`.
//!
//! TOML overrides (`~/.config/rustyballad/config.toml`) arrive with the M2
//! shell; every consumer already goes through this struct so the override
//! layer will not require call-site changes.

/// Default system-polling configuration (mirrors `Config.qml`).
// Fields are consumed by later milestones (Wayland shell, popups, timers).
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Config {
    // --- Network ---
    pub net_interface: String,
    /// 20 MiB/s, meter scaling ceiling (`netMaxSpeed`).
    pub net_max_speed: f64,
    pub dl_sensitivity: f64,
    pub ul_sensitivity: f64,

    // --- Disk ---
    pub disk_interface: String,
    /// 500 MiB/s (`diskMaxSpeed`).
    pub disk_max_speed: f64,

    // --- Fans ---
    pub fan1_name: String,
    pub fan2_name: String,
    pub fan1_max_rpm: f64,
    pub fan2_max_rpm: f64,

    // --- Commands ---
    pub terminal_cmd: String,
    pub mixer_cmd: String,

    // --- Updates ---
    /// 40 minutes (`updatesCheckInterval`).
    pub updates_check_interval_ms: u64,
    pub pacman_update_cmd: String,
    pub upgrade_cmd: String,

    // --- Dimensions ---
    pub bar_width: u32,

    // --- Polling cadence (QML timers, milliseconds) ---
    pub net_interval_ms: u64,
    pub disk_interval_ms: u64,
    pub cpu_interval_ms: u64,
    pub ram_interval_ms: u64,
    pub storage_interval_ms: u64,
    pub temp_interval_ms: u64,
    pub fan_interval_ms: u64,
    pub wifi_info_interval_ms: u64,
    pub wifi_radio_interval_ms: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            net_interface: "wlan0".into(),
            net_max_speed: 20_971_520.0,
            dl_sensitivity: 1.5,
            ul_sensitivity: 2.0,

            disk_interface: "nvme0n1".into(),
            disk_max_speed: 524_288_000.0,

            fan1_name: "fan1".into(),
            fan2_name: "fan2".into(),
            fan1_max_rpm: 5000.0,
            fan2_max_rpm: 5000.0,

            terminal_cmd: "foot".into(),
            mixer_cmd: "pwvucontrol".into(),

            updates_check_interval_ms: 2_400_000,
            pacman_update_cmd: "pacman -Qu".into(),
            upgrade_cmd: "sudo pacman -Syu".into(),

            bar_width: 50,

            net_interval_ms: 2000,
            disk_interval_ms: 2000,
            cpu_interval_ms: 2000,
            ram_interval_ms: 2000,
            storage_interval_ms: 60_000,
            temp_interval_ms: 2000,
            fan_interval_ms: 3000,
            wifi_info_interval_ms: 5000,
            wifi_radio_interval_ms: 6000,
        }
    }
}
