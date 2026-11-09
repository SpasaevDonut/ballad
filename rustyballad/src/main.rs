//! rustyballad — compiled rewrite of the ballad Quickshell shell.
//!
//! M1: system data engine + `--probe` diagnostics. The Wayland layer-shell
//! surfaces arrive with M2 (see PLAN.md).

mod audio;
mod config;
mod net;
mod niri;
mod power;
mod shell;
mod sysinfo;
mod theme;
mod tray;
mod ui;
mod updates;

use std::time::Duration;

use config::Config;
use sysinfo::Sampler;

fn main() {
    // cosmic-text shapes with rayon; a 16-thread global pool is pure overhead
    // for a status bar. Cap it before the first shaping call. SAFETY: this
    // runs before any threads exist.
    unsafe { std::env::set_var("RAYON_NUM_THREADS", "1") };

    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("--probe") => {
            let interval_ms: u64 = args
                .next()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1000);
            probe(interval_ms);
        }
        Some("-h" | "--help") => {
            println!("rustyballad {}\n", env!("CARGO_PKG_VERSION"));
            println!("USAGE:");
            println!("  rustyballad                         run the Wayland layer-shell (bar + desktop widgets)");
            println!("  rustyballad --probe [interval_ms]   sample system metrics once and print them");
        }
        Some(other) => {
            eprintln!("unknown argument: {other} (try --help)");
            std::process::exit(2);
        }
        None => {
            if let Err(e) = shell::run() {
                eprintln!("rustyballad: {e}");
                std::process::exit(1);
            }
        }
    }
}

fn probe(interval_ms: u64) {
    let mut sampler = Sampler::new(Config::default());
    // First sample primes the CPU/net/disk delta trackers.
    let _ = sampler.sample();
    std::thread::sleep(Duration::from_millis(interval_ms));
    let s = sampler.sample();

    println!("CPU       : {}%", s.cpu_percent);
    println!(
        "RAM       : {:.1}% ({} / 24 blocks)",
        s.ram_ratio * 100.0,
        s.ram_blocks
    );
    println!(
        "NET       : link={} down={} up={} (dl_ratio={:.3} ul_ratio={:.3})",
        if s.net.link_up { "up" } else { "down" },
        s.net.download_str,
        s.net.upload_str,
        s.net.dl_ratio,
        s.net.ul_ratio
    );
    println!(
        "DISK I/O  : read={} write={} (r_ratio={:.3} w_ratio={:.3})",
        s.disk.read_str, s.disk.write_str, s.disk.read_ratio, s.disk.write_ratio
    );
    println!(
        "TEMPS     : temp1={} temp2={}",
        fmt_temp(s.temp1),
        fmt_temp(s.temp2)
    );
    println!(
        "FANS      : fan1={} fan2={}",
        fmt_fan(s.fan1),
        fmt_fan(s.fan2)
    );
    println!(
        "BATTERY   : {:.0}% ({})",
        s.battery.percent, s.battery.status
    );
}

fn fmt_temp(t: Option<i32>) -> String {
    t.map_or_else(|| "n/a".into(), |v| format!("{v} °C"))
}

fn fmt_fan(f: Option<(f64, f64)>) -> String {
    f.map_or_else(|| "n/a".into(), |(rpm, ratio)| format!("{rpm:.0} rpm ({:.0}%)", ratio * 100.0))
}
