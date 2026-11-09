//! Wi-Fi via NetworkManager's `nmcli`, mirroring `SysData.qml` and
//! `WifiPopup.qml` (same commands, same cadence, same terse-output parsing).

use std::collections::HashSet;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WifiNetwork {
    pub ssid: String,
    pub signal: u8,
    pub security: String,
    pub in_use: bool,
    pub is_saved: bool,
}

impl WifiNetwork {
    pub fn secured(&self) -> bool {
        !self.security.is_empty() && self.security != "--"
    }

    /// `WifiPopup.qml` icon ladder.
    pub fn icon(&self) -> &'static str {
        if self.in_use || self.signal >= 75 {
            "󰤨"
        } else if self.signal >= 50 {
            "󰤥"
        } else if self.signal >= 25 {
            "󰤢"
        } else {
            "󰤟"
        }
    }

    /// Subtitle: `Saved` / security string / `Open`.
    pub fn subtitle(&self) -> String {
        if self.is_saved {
            "Saved".into()
        } else if self.secured() {
            self.security.clone()
        } else {
            "Open".into()
        }
    }
}

/// Splits nmcli terse output, honouring `\:` and `\\` escapes.
fn split_terse(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.peek() {
                Some(':') | Some('\\') => cur.push(chars.next().unwrap()),
                _ => cur.push('\\'),
            },
            ':' => out.push(std::mem::take(&mut cur)),
            _ => cur.push(c),
        }
    }
    out.push(cur);
    out
}

/// Parses `nmcli -t -f IN-USE,SIGNAL,SECURITY,SSID dev wifi list` output.
/// Applies the QML dedupe (prefer in-use / strongest) and sort order.
pub fn parse_scan(output: &str, saved: &HashSet<String>) -> Vec<WifiNetwork> {
    let mut nets: Vec<WifiNetwork> = Vec::new();
    for line in output.lines() {
        let line = line.trim_end_matches('\r');
        if line.is_empty() {
            continue;
        }
        let fields = split_terse(line);
        if fields.len() < 4 {
            continue;
        }
        let in_use = fields[0].trim() == "*";
        let signal: u8 = fields[1].trim().parse().unwrap_or(0);
        let security = fields[2].trim().to_string();
        let ssid = fields[3..].join(":").trim().to_string();
        if ssid.is_empty() || ssid == "--" {
            continue;
        }
        let net = WifiNetwork {
            is_saved: saved.contains(&ssid),
            ssid,
            signal: signal.min(100),
            security,
            in_use,
        };
        match nets.iter_mut().find(|n| n.ssid == net.ssid) {
            Some(existing) => {
                if net.in_use || net.signal > existing.signal {
                    let was_in_use = existing.in_use;
                    *existing = net;
                    existing.in_use |= was_in_use;
                } else {
                    existing.in_use |= net.in_use;
                }
            }
            None => nets.push(net),
        }
    }
    nets.sort_by(|a, b| {
        b.in_use
            .cmp(&a.in_use)
            .then(b.is_saved.cmp(&a.is_saved))
            .then(b.signal.cmp(&a.signal))
    });
    nets
}

/// Parses `nmcli -t -f NAME,TYPE connection show`, returning Wi-Fi profile
/// names (the QML `savedConnections` map).
pub fn parse_saved(output: &str) -> HashSet<String> {
    let mut saved = HashSet::new();
    for line in output.lines() {
        let fields = split_terse(line);
        if fields.len() < 2 {
            continue;
        }
        let ty = fields[1].trim();
        if ty == "802-11-wireless" || ty == "wifi" {
            saved.insert(fields[0].trim().to_string());
        }
    }
    saved
}

fn run(args: &[&str]) -> Option<String> {
    let out = Command::new("nmcli").args(args).output().ok()?;
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// One popup scan: saved profiles first, then the network list.
pub fn scan(rescan: bool) -> Vec<WifiNetwork> {
    let saved = run(&["-t", "-f", "NAME,TYPE", "connection", "show"])
        .map(|s| parse_saved(&s))
        .unwrap_or_default();
    let rescan_flag = if rescan { "--rescan" } else { "--rescan" };
    let flag_val = if rescan { "yes" } else { "no" };
    // `nmcli -t -f IN-USE,SIGNAL,SECURITY,SSID dev wifi list --rescan yes|no`
    run(&[
        "-t",
        "-f",
        "IN-USE,SIGNAL,SECURITY,SSID",
        "dev",
        "wifi",
        "list",
        rescan_flag,
        flag_val,
    ])
    .map(|s| parse_scan(&s, &saved))
    .unwrap_or_default()
}

/// Active SSID + signal for the bar (`SysData.qml` poll).
pub fn active() -> Option<(String, u8)> {
    let out = run(&[
        "-t",
        "-f",
        "ACTIVE,SSID,SIGNAL",
        "dev",
        "wifi",
        "list",
        "--rescan",
        "no",
    ])?;
    for line in out.lines() {
        let f = split_terse(line);
        if f.len() >= 3 && f[0].trim() == "yes" {
            return Some((f[1].trim().to_string(), f[2].trim().parse().unwrap_or(0)));
        }
    }
    None
}

pub fn radio_enabled() -> Option<bool> {
    let out = run(&["radio", "wifi"])?;
    Some(out.trim().eq_ignore_ascii_case("enabled"))
}

pub fn set_radio(on: bool) -> bool {
    Command::new("nmcli")
        .args(["radio", "wifi", if on { "on" } else { "off" }])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Runs nmcli and returns stderr/stdout on failure.
fn run_capture(args: &[&str]) -> Result<(), String> {
    let out = Command::new("nmcli")
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        return Ok(());
    }
    let mut msg = String::from_utf8_lossy(&out.stderr).trim().to_string();
    if msg.is_empty() {
        msg = String::from_utf8_lossy(&out.stdout).trim().to_string();
    }
    if msg.is_empty() {
        msg = format!("nmcli exited with {}", out.status);
    }
    Err(msg)
}

/// Connect to an SSID; `password` is used only for new secured networks,
/// otherwise the saved profile is brought up (QML logic). Reports the same
/// success criterion as QML: the exit status of the nmcli command.
pub fn connect(ssid: &str, password: Option<&str>) -> Result<(), String> {
    match password {
        Some(pwd) => run_capture(&["dev", "wifi", "connect", ssid, "password", pwd]),
        None => match run_capture(&["con", "up", "id", ssid]) {
            Ok(()) => Ok(()),
            Err(first) => run_capture(&["dev", "wifi", "connect", ssid])
                .map_err(|second| format!("{first} / {second}")),
        },
    }
}

pub fn disconnect(iface: &str, ssid: &str) -> bool {
    let ok = Command::new("nmcli")
        .args(["dev", "disconnect", iface])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if ok {
        return true;
    }
    Command::new("nmcli")
        .args(["con", "down", "id", ssid])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn saved(items: &[&str]) -> HashSet<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn terse_split_handles_escapes() {
        assert_eq!(split_terse("a:b:c"), vec!["a", "b", "c"]);
        assert_eq!(split_terse(r"a\:b:c"), vec!["a:b", "c"]);
        assert_eq!(split_terse(r"a\\b:c"), vec![r"a\b", "c"]);
    }

    #[test]
    fn parses_scan_rows() {
        let out = "*:87:WPA2:HomeNet\n :40::OpenCafe\n :60:WPA3:Cafe\\:Guest\n";
        let nets = parse_scan(out, &saved(&["HomeNet"]));
        assert_eq!(nets.len(), 3);
        assert_eq!(nets[0].ssid, "HomeNet");
        assert!(nets[0].in_use && nets[0].is_saved);
        let cafe = nets.iter().find(|n| n.ssid == "Cafe:Guest").unwrap();
        assert_eq!(cafe.subtitle(), "WPA3");
        let open = nets.iter().find(|n| n.ssid == "OpenCafe").unwrap();
        assert!(!open.secured());
        assert_eq!(open.subtitle(), "Open");
    }

    #[test]
    fn dedupes_preferring_active_or_strongest() {
        let out = " :10:WPA2:Net\n :70:WPA2:Net\n*:30:WPA2:Other\n :90:WPA2:Other\n";
        let nets = parse_scan(out, &HashSet::new());
        let net = nets.iter().find(|n| n.ssid == "Net").unwrap();
        assert_eq!(net.signal, 70);
        let other = nets.iter().find(|n| n.ssid == "Other").unwrap();
        assert!(other.in_use);
        // Active always sorts first.
        assert_eq!(nets[0].ssid, "Other");
    }

    #[test]
    fn parses_saved_profiles() {
        let out = "HomeNet:802-11-wireless\nWired:802-3-ethernet\nOld\\:Name:wifi\n";
        let s = parse_saved(out);
        assert!(s.contains("HomeNet"));
        assert!(s.contains("Old:Name"));
        assert!(!s.contains("Wired"));
    }

    #[test]
    fn icons_match_qml() {
        let n = |signal: u8, in_use: bool| WifiNetwork {
            ssid: "x".into(),
            signal,
            security: "WPA2".into(),
            in_use,
            is_saved: false,
        };
        assert_eq!(n(20, false).icon(), "󰤟");
        assert_eq!(n(30, false).icon(), "󰤢");
        assert_eq!(n(60, false).icon(), "󰤥");
        assert_eq!(n(10, true).icon(), "󰤨");
    }

    #[test]
    fn empty_scan_is_empty() {
        assert!(parse_scan("", &HashSet::new()).is_empty());
        assert!(parse_scan(" :80:WPA2:--\n", &HashSet::new()).is_empty());
    }
}
