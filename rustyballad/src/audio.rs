//! Audio state via `wpctl` (PipeWire's CLI), replacing the in-process
//! PipeWire client the QML shell links through Quickshell.
//!
//! Polling is a short-lived `wpctl get-volume` (~1 ms); all blocking calls
//! run on worker threads, never on the Wayland event loop.

use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AudioState {
    /// Linear volume, 0.0..=1.0 (wpctl reports the same scale as the QML
    /// `audio.volume`).
    pub volume: f32,
    pub muted: bool,
}

impl Default for AudioState {
    fn default() -> Self {
        Self {
            volume: 0.0,
            muted: false,
        }
    }
}

/// Parses `wpctl get-volume` output: `Volume: 0.65` / `Volume: 0.65 [MUTED]`.
pub fn parse_get_volume(out: &str) -> Option<AudioState> {
    let rest = out.trim().strip_prefix("Volume:")?;
    let mut it = rest.split_whitespace();
    let volume: f32 = it.next()?.parse().ok()?;
    let muted = rest.contains("[MUTED]");
    Some(AudioState {
        volume: volume.clamp(0.0, 1.0),
        muted,
    })
}

pub fn get() -> Option<AudioState> {
    let out = Command::new("wpctl")
        .args(["get-volume", "@DEFAULT_AUDIO_SINK@"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    parse_get_volume(&String::from_utf8_lossy(&out.stdout))
}

fn set_volume_arg(arg: &str) -> bool {
    Command::new("wpctl")
        .args(["set-volume", "@DEFAULT_AUDIO_SINK@", arg])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Absolute volume set (clamped to 0..=1, like the QML slider).
pub fn set_volume(v: f32) -> bool {
    set_volume_arg(&format!("{:.2}", v.clamp(0.0, 1.0)))
}

/// Relative step, also clamping at 1.0 (`-l 1.0`), mirroring the QML wheel
/// and slider semantics.
pub fn change_volume(delta: f32) -> bool {
    let sign = if delta >= 0.0 { "+" } else { "-" };
    set_volume_arg(&format!("{sign}{:.2}", delta.abs()))
}

pub fn toggle_mute() -> bool {
    Command::new("wpctl")
        .args(["set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Nerd Font icon selection, identical to `VolumeWidget.qml`.
pub fn icon(state: AudioState) -> &'static str {
    if state.muted {
        "󰝟"
    } else if state.volume > 0.5 {
        "󰕾"
    } else if state.volume > 0.0 {
        "󰖀"
    } else {
        "󰕿"
    }
}

// ---------------------------------------------------------------------------
// Mixer streams — `pactl list sink-inputs` (the QML used a native PipeWire
// node list; pactl gives the same app/volume/mute data in one subprocess).
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct Stream {
    pub id: u32,
    pub name: String,
    pub volume: f32,
    pub muted: bool,
}

/// Extracts the first `NN%` after a `Volume:` line.
fn volume_percent(line: &str) -> Option<f32> {
    let idx = line.find('%')?;
    let before = &line[..idx];
    let num: String = before
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_digit())
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    num.parse::<f32>().ok().map(|v| (v / 100.0).clamp(0.0, 1.0))
}

/// Parses `pactl list sink-inputs` text into playback streams.
pub fn parse_streams(text: &str) -> Vec<Stream> {
    let mut out = Vec::new();
    let mut cur: Option<Stream> = None;
    for line in text.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("Sink Input #") {
            if let Some(s) = cur.take() {
                out.push(s);
            }
            cur = Some(Stream {
                id: rest.trim().parse().unwrap_or(0),
                name: String::new(),
                volume: 0.0,
                muted: false,
            });
            continue;
        }
        let Some(s) = cur.as_mut() else { continue };
        if let Some(rest) = trimmed.strip_prefix("Volume:") {
            if let Some(v) = volume_percent(rest) {
                s.volume = v;
            }
        } else if let Some(rest) = trimmed.strip_prefix("Mute:") {
            s.muted = rest.trim().eq_ignore_ascii_case("yes");
        } else if let Some(rest) = trimmed.strip_prefix("application.name = ") {
            s.name = rest.trim().trim_matches('"').to_string();
        } else if s.name.is_empty() {
            if let Some(rest) = trimmed.strip_prefix("media.name = ") {
                s.name = rest.trim().trim_matches('"').to_string();
            }
        }
    }
    if let Some(s) = cur {
        out.push(s);
    }
    out.retain(|s| !s.name.is_empty());
    out
}

pub fn streams() -> Vec<Stream> {
    let Ok(out) = Command::new("pactl")
        .args(["list", "sink-inputs"])
        .output()
    else {
        return Vec::new();
    };
    parse_streams(&String::from_utf8_lossy(&out.stdout))
}

pub fn set_stream_volume(id: u32, v: f32) -> bool {
    // `id` is a Pulse sink-input index, not a PipeWire node id.
    Command::new("pactl")
        .args([
            "set-sink-input-volume",
            &id.to_string(),
            &format!("{}%", (v.clamp(0.0, 1.0) * 100.0).round() as u32),
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn set_stream_mute(id: u32, muted: bool) -> bool {
    Command::new("pactl")
        .args([
            "set-sink-input-mute",
            &id.to_string(),
            if muted { "1" } else { "0" },
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_volume() {
        let s = parse_get_volume("Volume: 0.65\n").unwrap();
        assert!((s.volume - 0.65).abs() < 1e-6);
        assert!(!s.muted);
    }

    #[test]
    fn parses_muted_volume() {
        let s = parse_get_volume("Volume: 0.40 [MUTED]").unwrap();
        assert!((s.volume - 0.40).abs() < 1e-6);
        assert!(s.muted);
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_get_volume("").is_none());
        assert!(parse_get_volume("wpctl: not found").is_none());
        assert!(parse_get_volume("Volume: loud").is_none());
    }

    #[test]
    fn clamps_out_of_range() {
        assert_eq!(parse_get_volume("Volume: 3.0").unwrap().volume, 1.0);
    }

    #[test]
    fn icons_match_qml() {
        assert_eq!(icon(AudioState { volume: 0.0, muted: false }), "󰕿");
        assert_eq!(icon(AudioState { volume: 0.3, muted: false }), "󰖀");
        assert_eq!(icon(AudioState { volume: 0.9, muted: false }), "󰕾");
        assert_eq!(icon(AudioState { volume: 0.9, muted: true }), "󰝟");
    }

    #[test]
    fn parses_mixer_streams() {
        let text = "Sink Input #42\n\tProperties:\n\t\tmedia.name = \"playback\"\n\t\tapplication.name = \"Firefox\"\n\tVolume: front-left: 32768 /  50% / -18.06 dB,   front-right: 32768 /  50% / -18.06 dB\n\tMute: no\n\nSink Input #43\n\tProperties:\n\t\tapplication.name = \"mpv\"\n\tVolume: front-left: 65536 / 100% / 0.00 dB\n\tMute: yes\n";
        let s = parse_streams(text);
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].id, 42);
        assert_eq!(s[0].name, "Firefox");
        assert!((s[0].volume - 0.5).abs() < 1e-6);
        assert!(!s[0].muted);
        assert_eq!(s[1].name, "mpv");
        assert!(s[1].muted);
        assert!((s[1].volume - 1.0).abs() < 1e-6);
    }

    #[test]
    fn empty_mixer() {
        assert!(parse_streams("").is_empty());
    }
}

#[cfg(test)]
mod live_tests {
    #[test]
    #[ignore]
    fn live_streams_are_parsed() {
        let s = super::streams();
        eprintln!("{s:#?}");
        assert!(!s.is_empty());
    }
}
