//! Power profiles and session actions.
//!
//! Profile *detection* is ported from the QML `getModeProc` shell script, but
//! every sysfs probe is a direct file read; only `tlp-stat` /
//! `powerprofilesctl` remain subprocesses. Applying a profile runs the same
//! multi-tool command the QML used (`tlp`, `powerprofilesctl`, ACPI
//! `platform_profile`), because those need privileged writes.

use std::fs;
use std::path::Path;
use std::process::Command;

fn read_trimmed(path: &str) -> Option<String> {
    fs::read_to_string(Path::new(path))
        .ok()
        .map(|s| s.trim().to_lowercase())
}

/// 0 = powersave, 1 = balanced, 2 = performance.
pub fn detect_profile() -> u8 {
    // 1. ACPI platform profile (hardware LED state).
    if let Some(pp) = read_trimmed("/sys/firmware/acpi/platform_profile") {
        match pp.as_str() {
            "low-power" | "quiet" | "power-saver" => return 0,
            "performance" => return 2,
            "balanced" => return 1,
            _ => {}
        }
    }
    // 2. TLP manual/last mode.
    for path in ["/run/tlp/manual_mode", "/run/tlp/last_mode"] {
        if let Some(m) = read_trimmed(path) {
            match m.as_str() {
                "battery" | "bat" => return 0,
                "ac" => return 2,
                _ => {}
            }
        }
    }
    // 3. tlp-stat (needs sudo -n, fall back to unprivileged run).
    if Command::new("sh")
        .args(["-c", "command -v tlp-stat"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
    {
        let out = Command::new("sh")
            .args(["-c", "sudo -n tlp-stat -s 2>/dev/null || tlp-stat -s 2>/dev/null"])
            .output()
            .ok();
        if let Some(out) = out {
            let text = String::from_utf8_lossy(&out.stdout).to_lowercase();
            for line in text.lines() {
                if line.contains("mode") {
                    if line.contains("battery") {
                        return 0;
                    }
                    if line.contains("ac") {
                        return 2;
                    }
                }
            }
        }
    }
    // 4. CPU EPP.
    if let Some(epp) =
        read_trimmed("/sys/devices/system/cpu/cpu0/cpufreq/energy_performance_preference")
    {
        match epp.as_str() {
            "power" | "power-saver" | "balance_power" => return 0,
            "performance" => return 2,
            "balance_performance" | "default" => return 1,
            _ => {}
        }
    }
    // 5. CPU governor.
    if let Some(gov) = read_trimmed("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor") {
        match gov.as_str() {
            "powersave" => return 0,
            "performance" => return 2,
            _ => {}
        }
    }
    // 6. powerprofilesctl fallback.
    if Command::new("sh")
        .args(["-c", "command -v powerprofilesctl"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
    {
        if let Ok(out) = Command::new("powerprofilesctl").arg("get").output() {
            match String::from_utf8_lossy(&out.stdout).trim() {
                "power-saver" => return 0,
                "performance" => return 2,
                _ => {}
            }
        }
    }
    1
}

pub fn apply_profile(mode: u8) -> bool {
    let cmd = match mode {
        0 => "sudo -n tlp bat 2>/dev/null; sudo -n tlp power-saver 2>/dev/null; powerprofilesctl set power-saver 2>/dev/null; echo low-power | sudo -n tee /sys/firmware/acpi/platform_profile 2>/dev/null || echo quiet | sudo -n tee /sys/firmware/acpi/platform_profile 2>/dev/null || echo power-saver | sudo -n tee /sys/firmware/acpi/platform_profile 2>/dev/null; true",
        2 => "sudo -n tlp ac 2>/dev/null; sudo -n tlp performance 2>/dev/null; powerprofilesctl set performance 2>/dev/null; echo performance | sudo -n tee /sys/firmware/acpi/platform_profile 2>/dev/null; true",
        _ => "sudo -n tlp auto 2>/dev/null; sudo -n tlp balanced 2>/dev/null; powerprofilesctl set balanced 2>/dev/null; echo balanced | sudo -n tee /sys/firmware/acpi/platform_profile 2>/dev/null; true",
    };
    Command::new("sh")
        .args(["-c", cmd])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionAction {
    Suspend,
    Reboot,
    PowerOff,
}

impl SessionAction {
    pub fn run(self) -> bool {
        let arg = match self {
            SessionAction::Suspend => "suspend",
            SessionAction::Reboot => "reboot",
            SessionAction::PowerOff => "poweroff",
        };
        Command::new("loginctl")
            .arg(arg)
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    pub fn icon(self) -> &'static str {
        match self {
            SessionAction::Suspend => "󰤄",
            SessionAction::Reboot => "󰑐",
            SessionAction::PowerOff => "󰐥",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            SessionAction::Suspend => "SLEEP",
            SessionAction::Reboot => "REBOOT",
            SessionAction::PowerOff => "SHUTDOWN",
        }
    }

    pub fn all() -> [SessionAction; 3] {
        [
            SessionAction::Suspend,
            SessionAction::Reboot,
            SessionAction::PowerOff,
        ]
    }
}

pub fn profile_name(mode: u8) -> &'static str {
    match mode {
        0 => "Powersave",
        2 => "Performance",
        _ => "Balanced",
    }
}

pub fn profile_footer(mode: u8) -> &'static str {
    match mode {
        0 => "MODE: POWERSAVE (TLP BAT)",
        2 => "MODE: PERFORMANCE (TLP AC)",
        _ => "MODE: BALANCED (TLP AUTO)",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert_eq!(profile_name(0), "Powersave");
        assert_eq!(profile_name(1), "Balanced");
        assert_eq!(profile_name(2), "Performance");
        assert!(profile_footer(2).contains("PERFORMANCE"));
    }

    #[test]
    fn session_actions_cover_qml_set() {
        let a = SessionAction::all();
        assert_eq!(a[0].label(), "SLEEP");
        assert_eq!(a[1].label(), "REBOOT");
        assert_eq!(a[2].label(), "SHUTDOWN");
        assert_eq!(a[2].icon(), "󰐥");
    }

    #[test]
    fn detection_is_in_range() {
        // On any machine this must be one of the three valid modes.
        assert!(detect_profile() <= 2);
    }
}
