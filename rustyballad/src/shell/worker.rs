//! Background I/O worker.
//!
//! Every blocking subprocess (`wpctl`, `nmcli`, `pacman`, `pactl`, `loginctl`,
//! profile switches) runs on this single thread. The Wayland event loop only
//! ever sees non-blocking channel messages, which keeps input latency and
//! idle CPU low.

use std::sync::mpsc::{self, Receiver};
use std::thread;

use smithay_client_toolkit::reexports::calloop::channel::Sender as LoopSender;

use crate::audio::{self, AudioState, Stream};
use crate::net::{self, WifiNetwork};
use crate::power::{self, SessionAction};
use crate::updates::{self, Update};

#[derive(Debug, Clone)]
pub enum Request {
    PollAudio,
    PollWifiActive,
    PollWifiRadio,
    PollUpdates,
    PollPower,
    ScanWifi { rescan: bool },
    ChangeVolume(f32),
    SetVolume(f32),
    ToggleMute,
    Session(SessionAction),
    ApplyProfile(u8),
    Connect { ssid: String, password: Option<String> },
    Disconnect { iface: String, ssid: String },
    SetRadio(bool),
    SpawnUpgrade,
    PollMixer,
    SetStreamVolume { id: u32, volume: f32 },
    SetStreamMute { id: u32, muted: bool },
    LaunchMixer,
}

#[derive(Debug, Clone)]
pub enum Response {
    Audio(AudioState),
    WifiActive(Option<(String, u8)>),
    Radio(bool),
    Updates(Vec<Update>),
    Power(u8),
    Scan(Vec<WifiNetwork>),
    /// Result of a connect attempt (QML uses the process exit status).
    ConnectResult { ssid: String, result: Result<(), String> },
    Mixer(Vec<Stream>),
}

/// Spawns the worker thread; returns the request sender.
pub fn start(tx: LoopSender<Response>) -> mpsc::Sender<Request> {
    let (req_tx, req_rx) = mpsc::channel::<Request>();
    thread::Builder::new()
        .name("rb-io".into())
        .spawn(move || run(req_rx, tx))
        .expect("spawn io worker");
    req_tx
}

fn run(rx: Receiver<Request>, tx: LoopSender<Response>) {
    while let Ok(req) = rx.recv() {
        let resp = match req {
            Request::PollAudio => Some(Response::Audio(audio::get().unwrap_or_default())),
            Request::PollWifiActive => Some(Response::WifiActive(net::active())),
            Request::PollWifiRadio => Some(Response::Radio(net::radio_enabled().unwrap_or(true))),
            Request::PollUpdates => {
                let cmd = crate::config::Config::default().pacman_update_cmd;
                spawn(&tx, move || {
                    Some(Response::Updates(updates::check(&cmd)))
                });
                None
            }
            Request::PollPower => Some(Response::Power(power::detect_profile())),
            Request::ScanWifi { rescan } => {
                // `nmcli --rescan yes` can take 10+ s on some drivers; keep it
                // off the shared worker thread so audio/updates still flow.
                spawn(&tx, move || Some(Response::Scan(net::scan(rescan))));
                None
            }
            Request::ChangeVolume(d) => {
                audio::change_volume(d);
                Some(Response::Audio(audio::get().unwrap_or_default()))
            }
            Request::SetVolume(v) => {
                audio::set_volume(v);
                Some(Response::Audio(audio::get().unwrap_or_default()))
            }
            Request::ToggleMute => {
                audio::toggle_mute();
                Some(Response::Audio(audio::get().unwrap_or_default()))
            }
            Request::Session(action) => {
                action.run();
                None
            }
            Request::ApplyProfile(mode) => {
                spawn(&tx, move || {
                    power::apply_profile(mode);
                    Some(Response::Power(power::detect_profile()))
                });
                None
            }
            Request::Connect { ssid, password } => {
                let results = tx.clone();
                spawn(&tx, move || {
                    let result = net::connect(&ssid, password.as_deref());
                    let _ = results.send(Response::ConnectResult {
                        ssid: ssid.clone(),
                        result,
                    });
                    Some(Response::Scan(net::scan(true)))
                });
                None
            }
            Request::Disconnect { iface, ssid } => {
                spawn(&tx, move || {
                    net::disconnect(&iface, &ssid);
                    Some(Response::Scan(net::scan(true)))
                });
                None
            }
            Request::SetRadio(on) => {
                net::set_radio(on);
                Some(Response::Radio(on))
            }
            Request::SpawnUpgrade => {
                let cfg = crate::config::Config::default();
                updates::spawn_upgrade(&cfg.terminal_cmd, &cfg.upgrade_cmd);
                None
            }
            Request::PollMixer => Some(Response::Mixer(audio::streams())),
            Request::SetStreamVolume { id, volume } => {
                audio::set_stream_volume(id, volume);
                Some(Response::Mixer(audio::streams()))
            }
            Request::SetStreamMute { id, muted } => {
                audio::set_stream_mute(id, muted);
                Some(Response::Mixer(audio::streams()))
            }
            Request::LaunchMixer => {
                let cfg = crate::config::Config::default();
                let _ = std::process::Command::new(&cfg.mixer_cmd).spawn();
                None
            }
        };
        if let Some(resp) = resp {
            if tx.send(resp).is_err() {
                break; // shell exited
            }
        }
    }
}

/// Runs one slow request on its own thread, delivering the response to the
/// event loop when done. Keeps scanning/connecting from blocking polls.
fn spawn<F>(tx: &LoopSender<Response>, f: F)
where
    F: FnOnce() -> Option<Response> + Send + 'static,
{
    let tx = tx.clone();
    let _ = thread::Builder::new()
        .name("rb-io-task".into())
        .spawn(move || {
            if let Some(resp) = f() {
                let _ = tx.send(resp);
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<Request>();
        assert_send::<Response>();
    }
}
