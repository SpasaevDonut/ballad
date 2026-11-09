//! Niri IPC client — replaces every `niri msg …` subprocess in the QML
//! shell with two persistent JSON-lines connections on `$NIRI_SOCKET`:
//! an event-stream reader thread and a request/action writer thread.
//!
//! Protocol verified against niri 26.04 (see `niri-ipc-26.4.0`).

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::sync::mpsc;
use std::time::Duration;

use serde::Deserialize;
use serde_json::{Value, json};
use smithay_client_toolkit::reexports::calloop;

// ---------------------------------------------------------------------------
// Domain types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Workspace {
    pub id: u64,
    #[serde(default)]
    pub idx: u32,
    pub name: Option<String>,
    #[serde(default)]
    pub is_focused: bool,
    #[serde(default)]
    pub is_urgent: bool,
    #[serde(default)]
    pub active_window_id: Option<u64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Window {
    pub id: u64,
    pub title: Option<String>,
    pub app_id: Option<String>,
    pub workspace_id: Option<u64>,
    #[serde(default)]
    pub is_focused: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Layouts {
    pub names: Vec<String>,
    pub current_idx: usize,
}

#[derive(Debug, Default)]
pub struct State {
    pub workspaces: Vec<Workspace>,
    pub active_workspace: u64,
    pub windows: Vec<Window>,
    pub layouts: Layouts,
    pub connected: bool,
}

// ---------------------------------------------------------------------------
// Events (niri → shell)
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum Event {
    Workspaces(Vec<Workspace>),
    WorkspaceActivated(u64),
    Windows(Vec<Window>),
    WindowOpened(Window),
    WindowClosed(u64),
    WindowFocusChanged(Option<u64>),
    Layouts(Layouts),
    LayoutSwitched(usize),
    Disconnected,
}

#[derive(Deserialize)]
enum RawEvent {
    WorkspacesChanged { workspaces: Vec<Workspace> },
    WorkspaceActivated { id: u64 },
    WindowsChanged { windows: Vec<Window> },
    WindowOpenedOrChanged { window: Window },
    WindowClosed { id: u64 },
    WindowFocusChanged { id: Option<u64> },
    KeyboardLayoutsChanged { keyboard_layouts: Layouts },
    KeyboardLayoutSwitched { idx: usize },
    #[serde(other)]
    Other,
}

// ---------------------------------------------------------------------------
// Actions (shell → niri)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum WsRef {
    Id(u64),
    Index(u8),
    Name(String),
}

#[derive(Debug, Clone)]
pub enum Action {
    FocusWorkspace(WsRef),
    FocusWindow(u64),
    CloseWindow(u64),
    FullscreenWindow(u64),
    ToggleFloating(u64),
    SetUrgent(u64),
    MoveToWorkspace { window: u64, ws: WsRef },
}

fn ref_json(r: &WsRef) -> Value {
    match r {
        WsRef::Id(id) => json!({"Id": id}),
        WsRef::Index(i) => json!({"Index": i}),
        WsRef::Name(n) => json!({"Name": n}),
    }
}

fn action_json(a: &Action) -> String {
    let v = match a {
        Action::FocusWorkspace(r) => json!({"Action": {"FocusWorkspace": {"reference": ref_json(r)}}}),
        Action::FocusWindow(id) => json!({"Action": {"FocusWindow": {"id": id}}}),
        Action::CloseWindow(id) => json!({"Action": {"CloseWindow": {"id": id}}}),
        Action::FullscreenWindow(id) => json!({"Action": {"FullscreenWindow": {"id": id}}}),
        Action::ToggleFloating(id) => json!({"Action": {"ToggleWindowFloating": {"id": id}}}),
        Action::SetUrgent(id) => json!({"Action": {"SetWindowUrgent": {"id": id}}}),
        // `focus` is a required field in niri 26.x (no serde default).
        Action::MoveToWorkspace { window, ws } => json!({"Action": {"MoveWindowToWorkspace": {
            "window_id": window, "reference": ref_json(ws), "focus": true,
        }}}),
    };
    v.to_string()
}

// ---------------------------------------------------------------------------
// Threads
// ---------------------------------------------------------------------------

fn socket_path() -> Option<String> {
    std::env::var("NIRI_SOCKET").ok()
}

/// Spawns the two IPC threads; returns the action sender. Events arrive on
/// `event_tx` (a calloop channel drained by the Wayland event loop).
pub fn start(event_tx: calloop::channel::Sender<Event>) -> mpsc::Sender<Action> {
    let (tx, rx) = mpsc::channel::<Action>();
    let _ = std::thread::Builder::new()
        .name("niri-events".into())
        .stack_size(256 * 1024)
        .spawn(move || event_loop(event_tx));
    let _ = std::thread::Builder::new()
        .name("niri-actions".into())
        .stack_size(256 * 1024)
        .spawn(move || action_loop(rx));
    tx
}

fn event_loop(tx: calloop::channel::Sender<Event>) {
    loop {
        let Some(path) = socket_path() else {
            // Not running under niri: stay quiet, the bar still renders.
            return;
        };
        if let Ok(mut stream) = UnixStream::connect(&path) {
            if stream.write_all(b"\"EventStream\"\n").is_ok() {
                let mut reader = BufReader::new(stream);
                let mut line = String::new();
                // Skip the {"Ok":"Handled"} handshake line, then stream events.
                let mut handshake = false;
                loop {
                    line.clear();
                    match reader.read_line(&mut line) {
                        Ok(0) | Err(_) => break,
                        Ok(_) => {
                            if !handshake {
                                handshake = true;
                                continue;
                            }
                            match serde_json::from_str::<RawEvent>(&line) {
                                Ok(raw) => {
                                    let ev = match raw {
                                        RawEvent::WorkspacesChanged { workspaces } => Event::Workspaces(workspaces),
                                        RawEvent::WorkspaceActivated { id } => Event::WorkspaceActivated(id),
                                        RawEvent::WindowsChanged { windows } => Event::Windows(windows),
                                        RawEvent::WindowOpenedOrChanged { window } => Event::WindowOpened(window),
                                        RawEvent::WindowClosed { id } => Event::WindowClosed(id),
                                        RawEvent::WindowFocusChanged { id } => Event::WindowFocusChanged(id),
                                        RawEvent::KeyboardLayoutsChanged { keyboard_layouts } => Event::Layouts(keyboard_layouts),
                                        RawEvent::KeyboardLayoutSwitched { idx } => Event::LayoutSwitched(idx),
                                        RawEvent::Other => continue,
                                    };
                                    if tx.send(ev).is_err() {
                                        return; // shell exited
                                    }
                                }
                                Err(e) => {
                                    if std::env::var_os("RB_DEBUG").is_some() {
                                        eprintln!("rustyballad: niri event parse failed: {e}: {line}");
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        if tx.send(Event::Disconnected).is_err() {
            return;
        }
        std::thread::sleep(Duration::from_secs(2));
    }
}

fn action_loop(rx: mpsc::Receiver<Action>) {
    let mut stream: Option<UnixStream> = None;
    while let Ok(action) = rx.recv() {
        let payload = action_json(&action);
        let mut ok = false;
        for _attempt in 0..2 {
            if stream.is_none() {
                let Some(path) = socket_path() else { return };
                stream = UnixStream::connect(&path).ok();
            }
            let Some(s) = stream.as_mut() else { break };
            if s.write_all(payload.as_bytes()).is_err() || s.write_all(b"\n").is_err() {
                stream = None;
                continue;
            }
            let mut reader = BufReader::new(s);
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(n) if n > 0 => {
                    ok = true;
                    // Responses are informational only; niri state arrives
                    // through the event stream.
                    if line.contains("\"Err\"") {
                        eprintln!("rustyballad: niri action rejected: {}", line.trim());
                    }
                }
                _ => {
                    stream = None;
                    continue;
                }
            }
            break;
        }
        if !ok {
            eprintln!("rustyballad: niri action failed to send");
        }
    }
}

// ---------------------------------------------------------------------------
// Workspace slots & window groups (QML WorkspacePanel logic)
// ---------------------------------------------------------------------------

/// One of the ten fixed dial slots.
#[derive(Debug, Clone)]
pub struct Slot {
    pub ws: Option<Workspace>,
    pub exists: bool,
    pub focused: bool,
}

fn ws_label(ws: &Workspace) -> String {
    ws.name.clone().unwrap_or_else(|| ws.id.to_string())
}

/// QML `updateWorkspaces`: numeric-aware sort, then first ten slots.
pub fn slots(workspaces: &[Workspace], active_workspace: u64) -> Vec<Slot> {
    let mut sorted: Vec<Workspace> = workspaces.to_vec();
    sorted.sort_by(|a, b| {
        let (la, lb) = (ws_label(a), ws_label(b));
        match (la.parse::<i64>(), lb.parse::<i64>()) {
            (Ok(na), Ok(nb)) => na.cmp(&nb),
            _ => la.cmp(&lb),
        }
    });
    (0..10)
        .map(|i| {
            if let Some(ws) = sorted.get(i) {
                Slot {
                    focused: ws.is_focused || ws.id == active_workspace,
                    exists: true,
                    ws: Some(ws.clone()),
                }
            } else {
                Slot { ws: None, exists: false, focused: false }
            }
        })
        .collect()
}

/// Reference used when clicking slot `i` (0-based).
pub fn slot_ref(slot: &Slot, i: usize) -> WsRef {
    match &slot.ws {
        Some(ws) => match &ws.name {
            Some(name) => WsRef::Name(name.clone()),
            None => WsRef::Id(ws.id),
        },
        None => WsRef::Index((i + 1) as u8),
    }
}

#[derive(Debug, Clone)]
pub struct Group {
    pub name: String,
    pub windows: Vec<Window>,
    pub focused: bool,
}

impl Group {
    /// QML: focused window if any, else the first (lowest id).
    pub fn target_window(&self) -> Option<u64> {
        self.windows
            .iter()
            .find(|w| w.is_focused)
            .or_else(|| self.windows.first())
            .map(|w| w.id)
    }
}

/// QML short-name mapping (order matters, verbatim rules).
pub fn short_name(app_id: Option<&str>, title: Option<&str>) -> String {
    let name = app_id.or(title).unwrap_or("APP");
    let last = name.rsplit('.').next().unwrap_or(name);
    let upper = last.to_uppercase();
    if upper.contains("KITTY") || upper.contains("ALACRITTY") || upper.contains("FOOT") {
        "TERM".into()
    } else if upper.contains("FIREFOX") {
        "FFOX".into()
    } else if upper.contains("CHROME") || upper.contains("BRAVE") {
        "WEB".into()
    } else if upper.contains("TELEGRAM") {
        "TG".into()
    } else if upper.contains("DISCORD") {
        "DSCD".into()
    } else if upper.contains("DOLPHIN") || upper.contains("NEMO") {
        "FILE".into()
    } else if upper.contains("CODE") {
        "CODE".into()
    } else {
        upper.replace(['-', '_'], "").chars().take(5).collect()
    }
}

/// Groups windows of the active workspace by shortened app name.
pub fn groups(windows: &[Window], active_workspace: u64) -> Vec<Group> {
    let mut filtered: Vec<Window> = windows
        .iter()
        .filter(|w| w.workspace_id == Some(active_workspace))
        .cloned()
        .collect();
    filtered.sort_by_key(|w| w.id);

    let mut groups: Vec<Group> = Vec::new();
    for w in filtered {
        let name = short_name(w.app_id.as_deref(), w.title.as_deref());
        match groups.iter_mut().find(|g| g.name == name) {
            Some(g) => {
                g.focused |= w.is_focused;
                g.windows.push(w);
            }
            None => groups.push(Group {
                name,
                focused: w.is_focused,
                windows: vec![w],
            }),
        }
    }
    groups
}

/// Keyboard layout badge text (QML LayoutWidget).
pub fn layout_badge(layouts: &Layouts) -> String {
    match layouts.names.get(layouts.current_idx) {
        Some(name) if name.contains("Russian") => "RU".into(),
        Some(_) => "EN".into(),
        None => "--".into(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn ws(id: u64, name: Option<&str>, focused: bool) -> Workspace {
        Workspace {
            id,
            idx: id as u32,
            name: name.map(String::from),
            is_focused: focused,
            ..Default::default()
        }
    }

    fn win(id: u64, app: &str, ws: u64, focused: bool) -> Window {
        Window {
            id,
            app_id: Some(app.into()),
            title: None,
            workspace_id: Some(ws),
            is_focused: focused,
        }
    }

    #[test]
    fn short_names_match_qml() {
        assert_eq!(short_name(Some("foot"), None), "TERM");
        assert_eq!(short_name(Some("kitty"), None), "TERM");
        assert_eq!(short_name(Some("firefox"), None), "FFOX");
        assert_eq!(short_name(Some("org.telegram.desktop"), None), "DESKT");
        assert_eq!(short_name(Some("com.brave.Browser"), None), "BROWS");
        assert_eq!(short_name(Some("discord"), None), "DSCD");
        assert_eq!(short_name(Some("dolphin"), None), "FILE");
        assert_eq!(short_name(Some("code"), None), "CODE");
        assert_eq!(short_name(Some("niri"), None), "NIRI");
        assert_eq!(short_name(Some("very-long-app_name"), None), "VERYL");
        assert_eq!(short_name(None, None), "APP");
    }

    #[test]
    fn slots_sort_and_fill() {
        let workspaces = vec![ws(3, None, false), ws(1, None, false), ws(2, Some("10"), false)];
        let s = slots(&workspaces, 1);
        assert_eq!(s.len(), 10);
        // Labels "1", "3", "10" sort numerically: 1, 3, 10.
        assert_eq!(s[0].ws.as_ref().unwrap().id, 1);
        assert_eq!(s[1].ws.as_ref().unwrap().id, 3);
        assert_eq!(s[2].ws.as_ref().unwrap().id, 2);
        assert!(!s[3].exists);
        assert!(s[0].focused);
    }

    #[test]
    fn slot_refs() {
        let named = Slot { ws: Some(ws(5, Some("web"), false)), exists: true, focused: false };
        assert!(matches!(slot_ref(&named, 0), WsRef::Name(n) if n == "web"));
        let unnamed = Slot { ws: Some(ws(7, None, false)), exists: true, focused: false };
        assert!(matches!(slot_ref(&unnamed, 0), WsRef::Id(7)));
        let empty = Slot { ws: None, exists: false, focused: false };
        assert!(matches!(slot_ref(&empty, 3), WsRef::Index(4)));
    }

    #[test]
    fn groups_by_short_name() {
        let windows = vec![
            win(2, "foot", 1, false),
            win(1, "firefox", 1, false),
            win(3, "foot", 1, true),
            win(4, "niri", 2, false), // other workspace: excluded
        ];
        let g = groups(&windows, 1);
        assert_eq!(g.len(), 2);
        assert_eq!(g[0].name, "FFOX");
        assert_eq!(g[1].name, "TERM");
        assert_eq!(g[1].windows.len(), 2);
        assert!(g[1].focused);
        assert_eq!(g[1].target_window(), Some(3));
        assert_eq!(g[0].target_window(), Some(1));
    }

    #[test]
    fn badges() {
        let l = Layouts { names: vec!["English (US)".into(), "Russian".into()], current_idx: 1 };
        assert_eq!(layout_badge(&l), "RU");
        let l2 = Layouts { names: vec!["English (US)".into()], current_idx: 0 };
        assert_eq!(layout_badge(&l2), "EN");
        assert_eq!(layout_badge(&Layouts::default()), "--");
    }

    #[test]
    fn action_json_shapes() {
        assert_eq!(
            action_json(&Action::FocusWindow(42)),
            r#"{"Action":{"FocusWindow":{"id":42}}}"#
        );
        let mv = action_json(&Action::MoveToWorkspace { window: 3, ws: WsRef::Id(1) });
        assert!(mv.contains(r#""window_id":3"#));
        assert!(mv.contains(r#""focus":true"#));
        assert!(mv.contains(r#""reference":{"Id":1}"#));
    }
}
