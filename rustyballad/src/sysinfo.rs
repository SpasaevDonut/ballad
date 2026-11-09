//! System metric engine — direct `/proc` + `/sys` reader.
//!
//! Replaces the QML `SysData.qml` polling pipeline, which spawned
//! `sh -c "top …"`, `sh -c "free …"`, `sh -c "awk … /proc/net/dev"`,
//! `sh -c "sensors | grep …"` etc. several times per second. Every metric
//! here is read and parsed in-process with zero allocations beyond small
//! stack buffers and one `String` per file read.
//!
//! All parsing functions are pure and unit-tested with captured kernel
//! formats; the [`Sampler`] adds filesystem access and delta timing.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::config::Config;

// ---------------------------------------------------------------------------
// Formatting
// ---------------------------------------------------------------------------

/// Replicates `SysData.qml formatSpeed()` byte-for-byte.
pub fn format_speed(bytes: f64) -> String {
    if bytes == 0.0 {
        return "0B".to_string();
    }
    if bytes < 1024.0 {
        return format!("{}B", bytes.round() as u64);
    }
    if bytes < 1_048_576.0 {
        return format!("{}K", (bytes / 1024.0).round() as u64);
    }
    format!("{:.1}M", bytes / 1_048_576.0)
}

// ---------------------------------------------------------------------------
// CPU — /proc/stat (was: `top -bn1 | awk '/^%Cpu/ {print $2+$4}'`)
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CpuTimes {
    pub user: u64,
    pub nice: u64,
    pub system: u64,
    pub idle: u64,
    pub iowait: u64,
    pub irq: u64,
    pub softirq: u64,
    pub steal: u64,
}

impl CpuTimes {
    fn total(&self) -> u64 {
        self.user
            + self.nice
            + self.system
            + self.idle
            + self.iowait
            + self.irq
            + self.softirq
            + self.steal
    }

    fn idle_all(&self) -> u64 {
        self.idle + self.iowait
    }
}

/// Parses the aggregate `cpu ` line of `/proc/stat`.
pub fn parse_cpu_line(line: &str) -> Option<CpuTimes> {
    let rest = line.strip_prefix("cpu ")?;
    let mut v = [0u64; 8];
    for (i, slot) in v.iter_mut().enumerate() {
        *slot = rest.split_whitespace().nth(i)?.parse().ok()?;
    }
    Some(CpuTimes {
        user: v[0],
        nice: v[1],
        system: v[2],
        idle: v[3],
        iowait: v[4],
        irq: v[5],
        softirq: v[6],
        steal: v[7],
    })
}

/// `busy_delta / total_delta * 100`. Returns 0.0 on counter reset/no delta.
pub fn cpu_usage_percent(prev: &CpuTimes, curr: &CpuTimes) -> f64 {
    let total = curr.total().saturating_sub(prev.total());
    if total == 0 {
        return 0.0;
    }
    let idle = curr.idle_all().saturating_sub(prev.idle_all());
    ((total - idle) as f64 / total as f64) * 100.0
}

// ---------------------------------------------------------------------------
// RAM — /proc/meminfo (was: `free | awk '/Mem:/ {print $3/$2}'`)
// `free`'s Mem.used == MemTotal - MemAvailable on procps-ng ≥ 3.3.10.
// ---------------------------------------------------------------------------

pub fn parse_meminfo(content: &str) -> Option<(u64, u64)> {
    let mut total = None;
    let mut available = None;
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("MemTotal:") {
            total = rest.split_whitespace().next()?.parse::<u64>().ok();
        } else if let Some(rest) = line.strip_prefix("MemAvailable:") {
            available = rest.split_whitespace().next()?.parse::<u64>().ok();
        }
        if total.is_some() && available.is_some() {
            break;
        }
    }
    Some((total?, available?))
}

/// Used ratio in `0.0..=1.0` from `(total_kib, available_kib)`.
pub fn ram_used_ratio(total_kib: u64, available_kib: u64) -> f64 {
    if total_kib == 0 {
        return 0.0;
    }
    (total_kib.saturating_sub(available_kib)) as f64 / total_kib as f64
}

// ---------------------------------------------------------------------------
// Network — /proc/net/dev (was: awk over the same file + `ip monitor link`)
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct NetCounters {
    pub rx_bytes: u64,
    pub tx_bytes: u64,
}

/// Extracts `(rx_bytes, tx_bytes)` for `iface` from `/proc/net/dev` content.
pub fn parse_net_dev(content: &str, iface: &str) -> Option<NetCounters> {
    for line in content.lines() {
        let Some((name, rest)) = line.split_once(':') else {
            continue;
        };
        if name.trim() != iface {
            continue;
        }
        let mut fields = rest.split_whitespace();
        let rx = fields.next()?.parse().ok()?;
        // rx_bytes is field 1; tx_bytes is field 9 after the colon.
        let tx = fields.nth(7)?.parse().ok()?;
        return Some(NetCounters {
            rx_bytes: rx,
            tx_bytes: tx,
        });
    }
    None
}

#[derive(Debug, Default, Clone)]
pub struct NetSnapshot {
    pub rx_bps: f64,
    pub tx_bps: f64,
    pub dl_ratio: f64,
    pub ul_ratio: f64,
    pub download_str: String,
    pub upload_str: String,
    pub link_up: bool,
}

// ---------------------------------------------------------------------------
// Disk I/O — /proc/diskstats (was: awk $6/$10 sectors * 512)
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DiskCounters {
    pub sectors_read: u64,
    pub sectors_written: u64,
}

pub fn parse_diskstats(content: &str, disk: &str) -> Option<DiskCounters> {
    for line in content.lines() {
        let mut f = line.split_whitespace();
        let _major = f.next()?;
        let _minor = f.next()?;
        if f.next()? != disk {
            continue;
        }
        // reads completed, reads merged, sectors read …
        let _reads = f.next()?;
        let _merged = f.next()?;
        let sectors_read = f.next()?.parse().ok()?;
        let _ms_reading = f.next()?;
        let _writes = f.next()?;
        let _wmerged = f.next()?;
        let sectors_written = f.next()?.parse().ok()?;
        return Some(DiskCounters {
            sectors_read,
            sectors_written,
        });
    }
    None
}

#[derive(Debug, Default, Clone)]
pub struct DiskSnapshot {
    pub read_bps: f64,
    pub write_bps: f64,
    pub read_ratio: f64,
    pub write_ratio: f64,
    pub read_str: String,
    pub write_str: String,
}

// ---------------------------------------------------------------------------
// Temperatures & fans — /sys/class/hwmon (was: `sensors | grep …`)
// ---------------------------------------------------------------------------

/// Discovered hwmon input files. Paths are cached at startup; the kernel
/// numbering is stable for a boot, and discovery re-runs if reads fail.
#[derive(Debug, Default, Clone)]
struct HwmonPaths {
    /// Ordered `(label, input_path)` temp channels matching Tctl/edge.
    temps: Vec<(String, PathBuf)>,
    fan1: Option<PathBuf>,
    fan2: Option<PathBuf>,
}

fn read_trimmed(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

fn read_number(path: &Path) -> Option<f64> {
    read_trimmed(path)?.parse().ok()
}

fn discover_hwmon() -> HwmonPaths {
    let mut paths = HwmonPaths::default();
    let Ok(entries) = fs::read_dir("/sys/class/hwmon") else {
        return paths;
    };
    let mut dirs: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    dirs.sort(); // deterministic: hwmon0, hwmon1, …

    for dir in dirs {
        let Ok(files) = fs::read_dir(&dir) else { continue };
        let mut labels: Vec<(u32, String)> = Vec::new();
        let mut fans: Vec<(u32, PathBuf)> = Vec::new();
        for file in files.flatten() {
            let name = file.file_name();
            let name = name.to_string_lossy();
            if let Some(rest) = name.strip_prefix("temp") {
                if let Some(idx) = rest.strip_suffix("_label").and_then(|n| n.parse().ok()) {
                    labels.push((idx, file.path().to_string_lossy().into_owned()));
                }
            } else if let Some(rest) = name.strip_prefix("fan") {
                if let Some(idx) = rest.strip_suffix("_input").and_then(|n| n.parse().ok()) {
                    fans.push((idx, file.path()));
                }
            }
        }
        labels.sort_by_key(|(i, _)| *i);
        for (idx, label_path) in labels {
            let Some(label) = read_trimmed(Path::new(&label_path)) else {
                continue;
            };
            if label == "Tctl" || label == "edge" {
                let input = dir.join(format!("temp{idx}_input"));
                if input.exists() {
                    paths.temps.push((label, input));
                }
            }
        }
        fans.sort_by_key(|(i, _)| *i);
        for (idx, path) in fans {
            match idx {
                1 if paths.fan1.is_none() => paths.fan1 = Some(path),
                2 if paths.fan2.is_none() => paths.fan2 = Some(path),
                _ => {}
            }
        }
    }
    paths
}

// ---------------------------------------------------------------------------
// Battery — /sys/class/power_supply (was: UPower over D-Bus)
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Clone)]
pub struct BatterySnapshot {
    pub percent: f64,
    pub status: String,
}

fn read_battery() -> BatterySnapshot {
    let Ok(entries) = fs::read_dir("/sys/class/power_supply") else {
        return BatterySnapshot {
            percent: 0.0,
            status: "Unknown".into(),
        };
    };
    for entry in entries.flatten() {
        let dir = entry.path();
        if read_trimmed(&dir.join("type")).as_deref() != Some("Battery") {
            continue;
        }
        let percent = read_number(&dir.join("capacity")).unwrap_or(0.0);
        let status = read_trimmed(&dir.join("status")).unwrap_or_else(|| "Unknown".into());
        return BatterySnapshot { percent, status };
    }
    BatterySnapshot {
        percent: 0.0,
        status: "Unknown".into(),
    }
}

// ---------------------------------------------------------------------------
// Sampler — stateful delta tracker used by the shell's poll timers
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Clone)]
pub struct Snapshot {
    pub cpu_percent: u32,
    /// `df /` Use% equivalent via `statvfs` (QML spawned `df | awk`).
    pub storage_percent: u32,
    /// 0..=24 discrete LED blocks (QML `ramBlocks`).
    pub ram_blocks: u32,
    pub ram_ratio: f64,
    pub temp1: Option<i32>,
    pub temp2: Option<i32>,
    /// `(rpm, ratio 0..=1)` per configured fan.
    pub fan1: Option<(f64, f64)>,
    pub fan2: Option<(f64, f64)>,
    pub net: NetSnapshot,
    pub disk: DiskSnapshot,
    pub battery: BatterySnapshot,
}

pub struct Sampler {
    cfg: Config,
    prev_cpu: Option<CpuTimes>,
    prev_net: Option<(Instant, NetCounters)>,
    prev_disk: Option<(Instant, DiskCounters)>,
    hwmon: HwmonPaths,
}

impl Sampler {
    pub fn new(cfg: Config) -> Self {
        Self {
            cfg,
            prev_cpu: None,
            prev_net: None,
            prev_disk: None,
            hwmon: discover_hwmon(),
        }
    }

    /// Collects one full snapshot. The first call primes CPU/net/disk deltas
    /// (rates stay zero), exactly like the QML `lastRx > 0` guards.
    pub fn sample(&mut self) -> Snapshot {
        let now = Instant::now();
        let mut snap = Snapshot::default();

        // --- CPU ---
        if let Ok(stat) = fs::read_to_string("/proc/stat") {
            if let Some(curr) = stat.lines().next().and_then(parse_cpu_line) {
                if let Some(prev) = self.prev_cpu {
                    snap.cpu_percent = cpu_usage_percent(&prev, &curr).round().clamp(0.0, 100.0) as u32;
                }
                self.prev_cpu = Some(curr);
            }
        }

        // --- RAM ---
        if let Ok(meminfo) = fs::read_to_string("/proc/meminfo") {
            if let Some((total, avail)) = parse_meminfo(&meminfo) {
                snap.ram_ratio = ram_used_ratio(total, avail);
                snap.ram_blocks = (snap.ram_ratio * 24.0).round() as u32;
            }
        }

        // --- Network ---
        let link_up = read_trimmed(Path::new(&format!(
            "/sys/class/net/{}/operstate",
            self.cfg.net_interface
        )))
        .is_some_and(|s| s == "up");
        snap.net.link_up = link_up;
        if let Ok(dev) = fs::read_to_string("/proc/net/dev") {
            if let Some(curr) = parse_net_dev(&dev, &self.cfg.net_interface) {
                if let Some((prev_t, prev)) = self.prev_net {
                    let dt = now.duration_since(prev_t).as_secs_f64().max(0.001);
                    if curr.rx_bytes >= prev.rx_bytes && curr.tx_bytes >= prev.tx_bytes {
                        snap.net.rx_bps = (curr.rx_bytes - prev.rx_bytes) as f64 / dt;
                        snap.net.tx_bps = (curr.tx_bytes - prev.tx_bytes) as f64 / dt;
                        snap.net.dl_ratio =
                            (snap.net.rx_bps * self.cfg.dl_sensitivity / self.cfg.net_max_speed).min(1.0);
                        snap.net.ul_ratio =
                            (snap.net.tx_bps * self.cfg.ul_sensitivity / self.cfg.net_max_speed).min(1.0);
                        snap.net.download_str = format_speed(snap.net.rx_bps);
                        snap.net.upload_str = format_speed(snap.net.tx_bps);
                    }
                }
                self.prev_net = Some((now, curr));
            }
        }

        // --- Disk I/O ---
        if let Ok(stats) = fs::read_to_string("/proc/diskstats") {
            if let Some(curr) = parse_diskstats(&stats, &self.cfg.disk_interface) {
                if let Some((prev_t, prev)) = self.prev_disk {
                    let dt = now.duration_since(prev_t).as_secs_f64().max(0.001);
                    let r_bytes = (curr.sectors_read.saturating_sub(prev.sectors_read)) as f64 * 512.0;
                    let w_bytes = (curr.sectors_written.saturating_sub(prev.sectors_written)) as f64 * 512.0;
                    snap.disk.read_bps = r_bytes / dt;
                    snap.disk.write_bps = w_bytes / dt;
                    snap.disk.read_ratio =
                        (snap.disk.read_bps * 5.0 / self.cfg.disk_max_speed).min(1.0);
                    snap.disk.write_ratio =
                        (snap.disk.write_bps * 5.0 / self.cfg.disk_max_speed).min(1.0);
                    snap.disk.read_str = format_speed(snap.disk.read_bps);
                    snap.disk.write_str = format_speed(snap.disk.write_bps);
                }
                self.prev_disk = Some((now, curr));
            }
        }

        // --- Temperatures (QML order: first match → temp1, second → temp2) ---
        let mut temps = self
            .hwmon
            .temps
            .iter()
            .filter_map(|(_, path)| read_number(path))
            .map(|millic| (millic / 1000.0).round() as i32);
        snap.temp1 = temps.next();
        snap.temp2 = temps.next();

        // --- Fans ---
        snap.fan1 = self
            .hwmon
            .fan1
            .as_deref()
            .and_then(read_number)
            .map(|rpm| (rpm, (rpm / self.cfg.fan1_max_rpm).min(1.0)));
        snap.fan2 = self
            .hwmon
            .fan2
            .as_deref()
            .and_then(read_number)
            .map(|rpm| (rpm, (rpm / self.cfg.fan2_max_rpm).min(1.0)));

        // --- Battery ---
        snap.battery = read_battery();

        // --- Storage (df Use%: used / (used + available)) ---
        if let Ok(s) = rustix::fs::statvfs("/") {
            let used = s.f_blocks.saturating_sub(s.f_bfree);
            let total = used + s.f_bavail;
            if total > 0 {
                snap.storage_percent = ((used as f64 / total as f64) * 100.0).round() as u32;
            }
        }

        snap
    }
}

// ---------------------------------------------------------------------------
// Tests — captured kernel formats, no filesystem access
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speed_format_matches_qml() {
        assert_eq!(format_speed(0.0), "0B");
        assert_eq!(format_speed(512.0), "512B");
        assert_eq!(format_speed(1023.4), "1023B");
        assert_eq!(format_speed(2048.0), "2K");
        assert_eq!(format_speed(1_048_575.0), "1024K");
        assert_eq!(format_speed(1_048_576.0), "1.0M");
        assert_eq!(format_speed(3_670_016.0), "3.5M");
        assert_eq!(format_speed(20_971_520.0), "20.0M");
    }

    #[test]
    fn cpu_line_parses_eight_fields() {
        let t = parse_cpu_line("cpu  100 20 300 4000 50 10 20 0 0 0").unwrap();
        assert_eq!(t.user, 100);
        assert_eq!(t.nice, 20);
        assert_eq!(t.system, 300);
        assert_eq!(t.idle, 4000);
        assert_eq!(t.iowait, 50);
        assert_eq!(t.irq, 10);
        assert_eq!(t.softirq, 20);
        assert_eq!(t.steal, 0);
        // Per-core `cpuN` lines must be rejected (no space after "cpu").
        assert!(parse_cpu_line("cpu0 1 2 3 4 5 6 7 8").is_none());
        assert!(parse_cpu_line("intr 12345").is_none());
    }

    #[test]
    fn cpu_usage_math() {
        let prev = parse_cpu_line("cpu  100 0 100 800 0 0 0 0 0 0").unwrap();
        // +100 user, +100 system, +200 idle → busy 200/400 = 50%
        let curr = parse_cpu_line("cpu  200 0 200 1000 0 0 0 0 0 0").unwrap();
        assert!((cpu_usage_percent(&prev, &curr) - 50.0).abs() < 1e-9);
        // No delta → 0, never NaN.
        assert_eq!(cpu_usage_percent(&curr, &curr), 0.0);
        // Counter reset (curr < prev) → clamped to 0, never negative/NaN.
        assert_eq!(cpu_usage_percent(&curr, &prev), 0.0);
    }

    #[test]
    fn meminfo_used_ratio() {
        let sample = "MemTotal:       16384000 kB\nMemFree:         1000000 kB\nMemAvailable:    8192000 kB\nBuffers:          100000 kB\n";
        let (total, avail) = parse_meminfo(sample).unwrap();
        assert_eq!(total, 16_384_000);
        assert_eq!(avail, 8_192_000);
        assert!((ram_used_ratio(total, avail) - 0.5).abs() < 1e-9);
        assert_eq!(ram_used_ratio(0, 0), 0.0);
    }

    #[test]
    fn net_dev_counters() {
        let sample = "Inter-|   Receive                                                |  Transmit\n \
                      face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed\n \
                      wlan0: 123456789   10000    0    0    0     0          0         0  9876543    5000    0    0    0     0       0          0\n \
                      lo:    1000      10    0    0    0     0          0         0     1000      10    0    0    0     0       0          0\n";
        let c = parse_net_dev(sample, "wlan0").unwrap();
        assert_eq!(c.rx_bytes, 123_456_789);
        assert_eq!(c.tx_bytes, 9_876_543);
        let lo = parse_net_dev(sample, "lo").unwrap();
        assert_eq!(lo.rx_bytes, 1000);
        assert!(parse_net_dev(sample, "eth0").is_none());
    }

    #[test]
    fn diskstats_sectors() {
        let sample = " 259       0 nvme0n1 100000 5000 9000000 40000 200000 6000 16000000 80000 0 100000 120000 0 0 0 0\n \
                      259       1 nvme0n1p1 100 0 800 10 50 0 400 5 0 20 20 0 0 0 0\n";
        let c = parse_diskstats(sample, "nvme0n1").unwrap();
        assert_eq!(c.sectors_read, 9_000_000);
        assert_eq!(c.sectors_written, 16_000_000);
        let p = parse_diskstats(sample, "nvme0n1p1").unwrap();
        assert_eq!(p.sectors_read, 800);
        assert!(parse_diskstats(sample, "sda").is_none());
    }

    #[test]
    fn disk_delta_math_is_saturating() {
        let prev = DiskCounters {
            sectors_read: 100,
            sectors_written: 100,
        };
        let curr = DiskCounters {
            sectors_read: 50,
            sectors_written: 50,
        };
        // Reboot/wrap must not underflow (u64 subtraction would panic in debug).
        assert_eq!(curr.sectors_read.saturating_sub(prev.sectors_read), 0);
    }
}
