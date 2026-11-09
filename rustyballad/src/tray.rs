//! System tray (StatusNotifierItem + DBusMenu) over D-Bus.
//!
//! Runs on its own thread: it registers as a StatusNotifierHost, optionally
//! takes ownership of the `org.kde.StatusNotifierWatcher` name when no desktop
//! environment provides one, polls the watcher's item list (1 s), reads item
//! metadata, decodes icons and parses DBusMenu layouts on demand. The Wayland
//! event loop only receives ready-to-draw icon pixmaps and menu rows.
//!
//! Memory notes: icons are decoded once and downscaled to the 16×16 bar size;
//! menu structures are fetched only when the user right-clicks.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;

use smithay_client_toolkit::reexports::calloop::channel::Sender as LoopSender;
use tiny_skia::Pixmap;
use zbus::blocking::{Connection, Proxy};
use zbus::fdo::RequestNameFlags;
use zbus::message::Header;
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Value};

const WATCHER: &str = "org.kde.StatusNotifierWatcher";
const WATCHER_PATH: &str = "/StatusNotifierWatcher";
const WATCHER_IFACE: &str = "org.kde.StatusNotifierWatcher";
const SNI_IFACE: &str = "org.kde.StatusNotifierItem";
const MENU_IFACE: &str = "com.canonical.dbusmenu";
const ICON_SIZE: u32 = 16;

#[derive(Clone)]
pub struct TrayItem {
    pub title: String,
    /// 16×16 premultiplied RGBA icon, ready to blit.
    pub icon: Option<Arc<Pixmap>>,
    /// Whether the item exposes a DBusMenu.
    pub has_menu: bool,
}

#[derive(Debug, Clone)]
pub struct MenuEntry {
    pub id: i32,
    pub label: String,
    pub enabled: bool,
    pub separator: bool,
    pub checked: bool,
    pub submenu: bool,
    pub depth: u8,
}

#[derive(Debug)]
pub enum Command {
    Activate(usize),
    Secondary(usize),
    Scroll(usize, i32),
    OpenMenu(usize),
    MenuEvent(usize, i32),
    Refresh,
}

pub enum Event {
    Items(Vec<TrayItem>),
    Menu { tray_index: usize, entries: Vec<MenuEntry> },
}

pub fn start(tx: LoopSender<Event>) -> mpsc::Sender<Command> {
    let (cmd_tx, cmd_rx) = mpsc::channel::<Command>();
    thread::Builder::new()
        .name("rb-tray".into())
        .spawn(move || {
            if let Err(e) = run(cmd_rx, tx) {
                eprintln!("rustyballad: tray thread stopped: {e}");
            }
        })
        .expect("spawn tray thread");
    cmd_tx
}

// ---------------------------------------------------------------------------
// Watcher interface (only exported when we win the name)
// ---------------------------------------------------------------------------

struct WatcherIface {
    items: Arc<Mutex<Vec<String>>>,
    hosts: Arc<AtomicU32>,
}

#[zbus::interface(name = "org.kde.StatusNotifierWatcher")]
impl WatcherIface {
    fn register_status_notifier_item(&self, service: &str, #[zbus(header)] hdr: Header<'_>) {
        // Per spec, a leading '/' means the item lives on the caller's unique
        // name; otherwise the argument is already a well-known bus name.
        let key = if service.starts_with('/') {
            match hdr.sender() {
                Some(sender) => format!("{sender}{service}"),
                None => return,
            }
        } else {
            service.to_string()
        };
        let mut items = self.items.lock().unwrap();
        if !items.contains(&key) {
            items.push(key);
        }
    }

    fn register_status_notifier_host(&self, _service: &str) {
        self.hosts.fetch_add(1, Ordering::Relaxed);
    }

    #[zbus(property)]
    fn registered_status_notifier_items(&self) -> Vec<String> {
        self.items.lock().unwrap().clone()
    }

    #[zbus(property)]
    fn is_status_notifier_host_registered(&self) -> bool {
        self.hosts.load(Ordering::Relaxed) > 0
    }

    #[zbus(property)]
    fn protocol_version(&self) -> i32 {
        0
    }
}

// ---------------------------------------------------------------------------
// Tray engine
// ---------------------------------------------------------------------------

struct ItemRuntime {
    key: String,
    service: String,
    path: String,
    title: String,
    icon: Option<Arc<Pixmap>>,
    menu_path: Option<String>,
    item_is_menu: bool,
}

fn split_item(key: &str) -> Option<(String, String)> {
    if let Some((service, rest)) = key.split_once('/') {
        Some((service.to_string(), format!("/{rest}")))
    } else if key.starts_with(':') || key.contains('.') {
        Some((key.to_string(), "/StatusNotifierItem".to_string()))
    } else {
        None
    }
}

fn run(rx: mpsc::Receiver<Command>, tx: LoopSender<Event>) -> zbus::Result<()> {
    let conn = Connection::session()?;

    // Become the watcher when the desktop has none (Quickshell usually owns
    // it; the fallback keeps the tray working on a bare niri session).
    let items = Arc::new(Mutex::new(Vec::<String>::new()));
    let hosts = Arc::new(AtomicU32::new(0));
    let watcher_is_ours = conn
        .request_name_with_flags(WATCHER, RequestNameFlags::DoNotQueue.into())
        .is_ok();
    if watcher_is_ours {
        conn.object_server().at(
            WATCHER_PATH,
            WatcherIface {
                items: items.clone(),
                hosts: hosts.clone(),
            },
        )?;
    }

    let host_name = format!("org.kde.StatusNotifierHost-{}", std::process::id());
    let _ = conn.request_name(host_name.as_str());
    if let Ok(w) = Proxy::new(&conn, WATCHER, WATCHER_PATH, WATCHER_IFACE) {
        let _: zbus::Result<()> = w.call("RegisterStatusNotifierHost", &(host_name.as_str()));
    }

    let mut runtimes: Vec<ItemRuntime> = Vec::new();
    let mut last_sig: Vec<(String, String, usize)> = Vec::new();
    let _ = tx.send(Event::Items(Vec::new()));

    let mut tick: u32 = 0;
    loop {
        // Metadata (title/menu/icon) is re-read every 5 s; between those ticks
        // only the watcher's cheap item list is polled, avoiding round-trips
        // and redundant icon decoding while idle.
        refresh(
            &conn,
            watcher_is_ours,
            &items,
            &mut runtimes,
            &mut last_sig,
            &tx,
            tick % 5 == 0,
        );
        tick = tick.wrapping_add(1);

        match rx.recv_timeout(Duration::from_secs(1)) {
            Ok(Command::Refresh) => {}
            Ok(Command::Activate(i)) => {
                if let Some(item) = runtimes.get(i) {
                    with_item(&conn, item, |p| {
                        let _: zbus::Result<()> = p.call("Activate", &(0i32, 0i32));
                    });
                }
            }
            Ok(Command::Secondary(i)) => {
                if let Some(item) = runtimes.get(i) {
                    with_item(&conn, item, |p| {
                        let _: zbus::Result<()> = p.call("SecondaryActivate", &(0i32, 0i32));
                    });
                }
            }
            Ok(Command::Scroll(i, delta)) => {
                if let Some(item) = runtimes.get(i) {
                    with_item(&conn, item, |p| {
                        // SNI: Scroll(in i delta, in s orientation) — no x/y.
                        let _: zbus::Result<()> =
                            p.call("Scroll", &(delta.clamp(-1, 1) as i32, "Vertical"));
                    });
                }
            }
            Ok(Command::OpenMenu(i)) => {
                if let Some(item) = runtimes.get(i) {
                    if let Some(entries) = fetch_menu(&conn, item) {
                        let _ = tx.send(Event::Menu {
                            tray_index: i,
                            entries,
                        });
                    }
                }
            }
            Ok(Command::MenuEvent(i, id)) => {
                if let Some(item) = runtimes.get(i) {
                    if let Some(path) = &item.menu_path {
                        if let Ok(p) =
                            Proxy::new(&conn, item.service.as_str(), path.as_str(), MENU_IFACE)
                        {
                            let _: zbus::Result<bool> = p.call("AboutToShow", &(id));
                            // DBusMenu `Event` data argument is a variant.
                            let placeholder = Value::Value(Box::new(Value::I32(0)));
                            let _: zbus::Result<()> =
                                p.call("Event", &(id, "clicked", placeholder, 0u32));
                        }
                    } else {
                        with_item(&conn, item, |p| {
                            let _: zbus::Result<()> = p.call("ContextMenu", &(0i32, 0i32));
                        });
                    }
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
        }
    }
}

fn with_item(conn: &Connection, item: &ItemRuntime, f: impl FnOnce(&Proxy<'_>)) {
    if let Ok(p) = Proxy::new(conn, item.service.as_str(), item.path.as_str(), SNI_IFACE) {
        f(&p);
    }
}

fn watcher_items(conn: &Connection, ours: bool, shared: &Mutex<Vec<String>>) -> Vec<String> {
    if ours {
        return shared.lock().unwrap().clone();
    }
    if let Ok(w) = Proxy::new(conn, WATCHER, WATCHER_PATH, WATCHER_IFACE) {
        if let Ok(v) = w.get_property::<Vec<String>>("RegisteredStatusNotifierItems") {
            return v;
        }
    }
    Vec::new()
}

#[allow(clippy::too_many_arguments)]
/// A watcher can keep stale unique names after an item reconnects/crashes.
fn service_alive(conn: &Connection, service: &str) -> bool {
    let Ok(p) = Proxy::new(
        conn,
        "org.freedesktop.DBus",
        "/org/freedesktop/DBus",
        "org.freedesktop.DBus",
    ) else {
        return true;
    };
    p.call::<_, _, bool>("NameHasOwner", &(service,))
        .unwrap_or(true)
}

fn refresh(
    conn: &Connection,
    ours: bool,
    shared: &Mutex<Vec<String>>,
    runtimes: &mut Vec<ItemRuntime>,
    last_sig: &mut Vec<(String, String, usize)>,
    tx: &LoopSender<Event>,
    deep: bool,
) {
    let keys: Vec<String> = watcher_items(conn, ours, shared)
        .into_iter()
        .filter(|key| {
            split_item(key)
                .map(|(service, _)| service_alive(conn, &service))
                .unwrap_or(false)
        })
        .collect();

    // When we own the watcher, prune dead names so they never reappear.
    if ours {
        if let Ok(mut shared) = shared.lock() {
            shared.retain(|key| keys.contains(key));
        }
    }

    // Drop items that disappeared.
    runtimes.retain(|r| keys.contains(&r.key));

    let mut added = false;
    for key in &keys {
        if runtimes.iter().any(|r| &r.key == key) {
            continue;
        }
        let Some((service, path)) = split_item(key) else { continue };
        runtimes.push(ItemRuntime {
            key: key.clone(),
            service,
            path,
            title: String::new(),
            icon: None,
            menu_path: None,
            item_is_menu: false,
        });
        added = true;
    }
    if deep || added {
        for item in runtimes.iter_mut() {
            read_metadata(conn, item);
        }
    }

    let sig: Vec<(String, String, usize)> = runtimes
        .iter()
        .map(|r| {
            (
                r.key.clone(),
                r.title.clone(),
                r.icon
                    .as_ref()
                    .map(|p| Arc::as_ptr(p) as usize)
                    .unwrap_or(0),
            )
        })
        .collect();
    if sig != *last_sig {
        *last_sig = sig;
        let snapshot = runtimes
            .iter()
            .map(|r| TrayItem {
                title: r.title.clone(),
                icon: r.icon.clone(),
                has_menu: r.menu_path.is_some(),
            })
            .collect();
        let _ = tx.send(Event::Items(snapshot));
    }
}

fn read_metadata(conn: &Connection, item: &mut ItemRuntime) {
    let Ok(p) = Proxy::new(
        conn,
        item.service.as_str(),
        item.path.as_str(),
        SNI_IFACE,
    ) else {
        return;
    };
    if let Ok(t) = p.get_property::<String>("Title") {
        if !t.is_empty() {
            item.title = t;
        }
    }
    if item.title.is_empty() {
        if let Ok(t) = p.get_property::<String>("Id") {
            item.title = t;
        }
    }
    item.item_is_menu = p.get_property::<bool>("ItemIsMenu").unwrap_or(false);
    item.menu_path = p
        .get_property::<OwnedObjectPath>("Menu")
        .ok()
        .filter(|o| o.as_str() != "/")
        .map(|o| o.to_string());

    // Prefer a real pixmap (works regardless of icon theme availability).
    let mut icon = None;
    if let Ok(pixmaps) = p.get_property::<Vec<(i32, i32, Vec<u8>)>>("IconPixmap") {
        icon = pick_pixmap(&pixmaps);
    }
    if icon.is_none() {
        if let Ok(name) = p.get_property::<String>("IconName") {
            icon = load_theme_icon(&name);
        }
    }
    if icon.is_none() {
        if let Ok(pixmaps) = p.get_property::<Vec<(i32, i32, Vec<u8>)>>("AttentionIconPixmap") {
            icon = pick_pixmap(&pixmaps);
        }
    }
    if icon.is_some() {
        item.icon = icon;
    }
}

fn pick_pixmap(pixmaps: &[(i32, i32, Vec<u8>)]) -> Option<Arc<Pixmap>> {
    let best = pixmaps
        .iter()
        .filter(|(w, h, d)| *w > 0 && *h > 0 && d.len() >= (*w as usize * *h as usize * 4))
        .max_by_key(|(w, h, _)| (*w, *h))?;
    argb_to_pixmap(best.0, best.1, &best.2)
}

/// SNI `(iiay)` icon data is big-endian ARGB32; tiny-skia wants premultiplied
/// RGBA. It is also downscaled to the 16 px tray slot immediately.
fn argb_to_pixmap(w: i32, h: i32, data: &[u8]) -> Option<Arc<Pixmap>> {
    let (w, h) = (w as u32, h as u32);
    let mut src = Pixmap::new(w, h)?;
    {
        let out = src.data_mut();
        for i in 0..(w * h) as usize {
            let a = data[i * 4];
            let r = data[i * 4 + 1];
            let g = data[i * 4 + 2];
            let b = data[i * 4 + 3];
            let prem = |c: u8| ((c as u32 * a as u32 + 127) / 255) as u8;
            out[i * 4] = prem(r);
            out[i * 4 + 1] = prem(g);
            out[i * 4 + 2] = prem(b);
            out[i * 4 + 3] = a;
        }
    }
    Some(Arc::new(resize(src, ICON_SIZE)))
}

fn resize(src: Pixmap, size: u32) -> Pixmap {
    let (sw, sh) = (src.width(), src.height());
    if (sw, sh) == (size, size) {
        return src;
    }
    let mut dst = Pixmap::new(size, size).expect("icon pixmap");
    let sdata = src.data();
    let ddata = dst.data_mut();

    // Small sources are upscaled nearest-neighbour (SNI icons are normally
    // >= 22 px); larger ones are box-filtered so edges are antialiased.
    if sw <= size && sh <= size {
        for y in 0..size {
            for x in 0..size {
                let sx = (x as u64 * sw as u64 / size as u64).min(sw as u64 - 1) as usize;
                let sy = (y as u64 * sh as u64 / size as u64).min(sh as u64 - 1) as usize;
                let si = (sy * sw as usize + sx) * 4;
                let di = ((y * size + x) * 4) as usize;
                ddata[di..di + 4].copy_from_slice(&sdata[si..si + 4]);
            }
        }
        return dst;
    }

    for dy in 0..size {
        let sy0 = (dy * sh / size) as usize;
        let sy1 = (((dy + 1) * sh + size - 1) / size).max(sy0 as u32 + 1).min(sh) as usize;
        for dx in 0..size {
            let sx0 = (dx * sw / size) as usize;
            let sx1 = (((dx + 1) * sw + size - 1) / size).max(sx0 as u32 + 1).min(sw) as usize;
            let mut acc = [0u32; 4];
            let mut count = 0u32;
            for sy in sy0..sy1 {
                for sx in sx0..sx1 {
                    let si = (sy * sw as usize + sx) * 4;
                    for (c, a) in acc.iter_mut().enumerate() {
                        *a += sdata[si + c] as u32;
                    }
                    count += 1;
                }
            }
            let di = ((dy * size + dx) * 4) as usize;
            for c in 0..4 {
                ddata[di + c] = (acc[c] / count.max(1)) as u8;
            }
        }
    }
    dst
}

fn load_theme_icon(name: &str) -> Option<Arc<Pixmap>> {
    if name.is_empty() {
        return None;
    }
    if name.starts_with('/') {
        let bytes = std::fs::read(name).ok()?;
        return Pixmap::decode_png(&bytes).ok().map(|p| Arc::new(resize(p, ICON_SIZE)));
    }
    let home = std::env::var("HOME").unwrap_or_default();
    let mut roots = vec![
        format!("{home}/.local/share/icons"),
        format!("{home}/.icons"),
        "/usr/share/icons".to_string(),
        "/usr/local/share/icons".to_string(),
        "/usr/share/pixmaps".to_string(),
        "/usr/local/share/pixmaps".to_string(),
    ];
    let stems = if name.contains('.') {
        vec![name.to_string()]
    } else {
        vec![format!("{name}.png"), format!("{name}.svg"), name.to_string()]
    };
    let sizes = ["32x32", "24x24", "22x22", "16x16", "48x48", "64x64", "128x128"];
    for root in roots.drain(..) {
        for stem in &stems {
            if stem.ends_with(".svg") {
                continue; // tiny-skia only decodes PNG
            }
            let mut candidates = vec![format!("{root}/{stem}")];
            for size in sizes {
                candidates.push(format!("{root}/hicolor/{size}/apps/{stem}"));
                candidates.push(format!("{root}/Adwaita/{size}/apps/{stem}"));
            }
            for path in candidates {
                if let Ok(bytes) = std::fs::read(&path) {
                    if let Ok(p) = Pixmap::decode_png(&bytes) {
                        return Some(Arc::new(resize(p, ICON_SIZE)));
                    }
                }
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// DBusMenu
// ---------------------------------------------------------------------------

fn fetch_menu(conn: &Connection, item: &ItemRuntime) -> Option<Vec<MenuEntry>> {
    let path = item.menu_path.as_deref()?;
    let proxy = Proxy::new(conn, item.service.as_str(), path, MENU_IFACE).ok()?;
    let _: zbus::Result<bool> = proxy.call("AboutToShow", &(0i32,));
    let reply = proxy
        .call_method("GetLayout", &(0i32, -1i32, Vec::<String>::new()))
        .ok()?;
    let body = reply.body();
    // Reply is `u(ia{sv}av)`. Decode into concrete types (the layout is a
    // struct, not a variant, so `OwnedValue` cannot be used here).
    type Props = HashMap<String, OwnedValue>;
    let (_revision, (root_id, props, children)): (u32, (i32, Props, Vec<OwnedValue>)) =
        body.deserialize().ok()?;

    let mut out = Vec::new();
    let pairs: Vec<(String, Value<'static>)> = props
        .iter()
        .map(|(k, v)| (k.clone(), (**v).clone()))
        .collect();
    let kids: Vec<Value<'static>> = children.iter().map(|v| (**v).clone()).collect();
    push_entry(root_id, &pairs, &kids, 0, &mut out);
    Some(out)
}

/// Extracts the DBusMenu attributes we render.
struct Attrs {
    label: String,
    enabled: bool,
    visible: bool,
    separator: bool,
    checked: bool,
    submenu: bool,
}

fn attrs(pairs: &[(String, Value<'_>)]) -> Attrs {
    let mut a = Attrs {
        label: String::new(),
        enabled: true,
        visible: true,
        separator: false,
        checked: false,
        submenu: false,
    };
    for (key, v) in pairs {
        // `a{sv}` values arrive wrapped in a variant.
        let v = unwrap_variant(v);
        match (key.as_str(), v) {
            ("label", Value::Str(s)) => a.label = s.to_string(),
            ("enabled", Value::Bool(b)) => a.enabled = *b,
            ("visible", Value::Bool(b)) => a.visible = *b,
            ("type", Value::Str(s)) => a.separator = s.as_str() == "separator",
            ("toggle-state", Value::I32(i)) => a.checked = *i == 1,
            ("children-display", Value::Str(s)) => a.submenu = s.as_str() == "submenu",
            _ => {}
        }
    }
    // `_File` / `F__ile` mnemonic escaping.
    a.label = a
        .label
        .replace("__", "\u{0}")
        .replace('_', "")
        .replace('\u{0}', "_");
    a
}

fn unwrap_variant<'a>(v: &'a Value<'a>) -> &'a Value<'a> {
    match v {
        Value::Value(inner) => unwrap_variant(inner),
        _ => v,
    }
}

fn push_entry(
    id: i32,
    props: &[(String, Value<'_>)],
    children: &[Value<'_>],
    depth: u8,
    out: &mut Vec<MenuEntry>,
) {
    let a = attrs(props);
    if id != 0 && a.visible {
        out.push(MenuEntry {
            id,
            label: a.label,
            enabled: a.enabled,
            separator: a.separator,
            checked: a.checked,
            submenu: a.submenu,
            depth,
        });
    }
    for child in children {
        flatten_menu(child, depth.saturating_add(1), out);
    }
}

fn flatten_menu(value: &Value<'_>, depth: u8, out: &mut Vec<MenuEntry>) {
    let Value::Structure(st) = value else { return };
    let fields = st.fields();
    if fields.len() < 3 {
        return;
    }
    let id = int_of(&fields[0]);
    let pairs: Vec<(String, Value<'_>)> = match &fields[1] {
        Value::Dict(dict) => dict
            .iter()
            .filter_map(|(k, v)| match k {
                Value::Str(s) => Some((s.to_string(), v.clone())),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    };
    let children: Vec<Value<'_>> = match &fields[2] {
        Value::Array(arr) => arr.iter().cloned().collect(),
        _ => Vec::new(),
    };
    push_entry(id, &pairs, &children, depth, out);
}

fn int_of(v: &Value<'_>) -> i32 {
    match v {
        Value::I32(v) => *v,
        Value::U32(v) => *v as i32,
        Value::I64(v) => *v as i32,
        Value::U16(v) => *v as i32,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_service_and_path() {
        assert_eq!(
            split_item(":1.115/StatusNotifierItem"),
            Some((":1.115".into(), "/StatusNotifierItem".into()))
        );
        assert_eq!(
            split_item(":1.96/org/ayatana/NotificationItem/steam"),
            Some((
                ":1.96".into(),
                "/org/ayatana/NotificationItem/steam".into()
            ))
        );
        // A well-known bus name without a path is valid and means the default
        // `/StatusNotifierItem` object.
        assert_eq!(
            split_item("org.kde.StatusNotifierItem-1-1"),
            Some((
                "org.kde.StatusNotifierItem-1-1".into(),
                "/StatusNotifierItem".into()
            ))
        );
    }

    #[test]
    fn decodes_argb_premultiplied() {
        // 1×1 opaque red, ARGB byte order.
        let p = argb_to_pixmap(1, 1, &[255, 255, 0, 0]).unwrap();
        assert_eq!(p.width(), ICON_SIZE);
        assert_eq!(&p.data()[0..4], &[255, 0, 0, 255]);
    }

    #[test]
    fn transparent_stays_transparent() {
        let p = argb_to_pixmap(1, 1, &[0, 255, 0, 0]).unwrap();
        assert_eq!(&p.data()[0..4], &[0, 0, 0, 0]);
    }

    #[test]
    fn menu_label_mnemonics_are_stripped() {
        assert_eq!("_File".replace('_', ""), "File");
    }

    /// Live check against the running session bus (needs a tray item).
    #[test]
    #[ignore]
    fn live_steam_menu_parses() {
        let conn = Connection::session().unwrap();
        let item = ItemRuntime {
            key: ":1.96/org/ayatana/NotificationItem/steam".into(),
            service: ":1.96".into(),
            path: "/org/ayatana/NotificationItem/steam".into(),
            title: String::new(),
            icon: None,
            menu_path: Some("/org/ayatana/NotificationItem/steam/Menu".into()),
            item_is_menu: false,
        };
        let entries = fetch_menu(&conn, &item).expect("menu");
        eprintln!("{entries:#?}");
        assert!(entries.iter().any(|e| e.label == "Exit Steam"));
    }

    /// Sends a no-op DBusMenu event to the root node, validating the `isvu`
    /// argument marshalling used by real menu clicks.
    #[test]
    #[ignore]
    fn live_menu_event_root_is_accepted() {
        let conn = Connection::session().unwrap();
        let proxy = Proxy::new(
            &conn,
            ":1.96",
            "/org/ayatana/NotificationItem/steam/Menu",
            MENU_IFACE,
        )
        .unwrap();
        let data = Value::Value(Box::new(Value::I32(0)));
        let r: zbus::Result<()> = proxy.call("Event", &(0i32, "clicked", data, 0u32));
        assert!(r.is_ok(), "{r:?}");

        // Scroll takes `is` (delta, orientation) — zero is a no-op.
        let sni = Proxy::new(
            &conn,
            ":1.96",
            "/org/ayatana/NotificationItem/steam",
            SNI_IFACE,
        )
        .unwrap();
        let r: zbus::Result<()> = sni.call("Scroll", &(0i32, "Vertical"));
        assert!(r.is_ok(), "{r:?}");
    }
}

#[cfg(test)]
mod aa_tests {
    use super::*;

    #[test]
    fn downscale_box_filters_edges() {
        // 4x4 checkerboard (opaque red / transparent) -> 2x2 averages.
        let mut src = Pixmap::new(4, 4).unwrap();
        for y in 0..4 {
            for x in 0..4 {
                let i = ((y * 4 + x) * 4) as usize;
                if (x + y) % 2 == 0 {
                    src.data_mut()[i..i + 4].copy_from_slice(&[255, 0, 0, 255]);
                }
            }
        }
        let dst = resize(src, 2);
        let a = dst.data()[3];
        assert!((100..=160).contains(&a), "alpha {a} should be averaged");
    }
}

#[cfg(test)]
mod live_debug {
    use super::*;
    #[test]
    #[ignore]
    fn dump_current_items() {
        let conn = Connection::session().unwrap();
        let w = Proxy::new(&conn, WATCHER, WATCHER_PATH, WATCHER_IFACE).unwrap();
        let keys: Vec<String> = w.get_property("RegisteredStatusNotifierItems").unwrap();
        for key in keys {
            let Some((service, path)) = split_item(&key) else { continue };
            let mut item = ItemRuntime {
                key: key.clone(),
                service,
                path,
                title: String::new(),
                icon: None,
                menu_path: None,
                item_is_menu: false,
            };
            read_metadata(&conn, &mut item);
            println!(
                "{key}: title={:?} icon={:?} menu={:?}",
                item.title,
                item.icon.as_ref().map(|p| (p.width(), p.height())),
                item.menu_path
            );
            if item.menu_path.is_some() {
                println!("  menu entries: {:?}", fetch_menu(&conn, &item).map(|v| v.len()));
            }
        }
    }
}

#[cfg(test)]
mod owner_tests {
    use super::*;

    #[test]
    #[ignore]
    fn bogus_bus_name_has_no_owner() {
        let conn = Connection::session().unwrap();
        assert!(!service_alive(&conn, ":1.999999"));
        assert!(service_alive(&conn, "org.freedesktop.DBus"));
    }
}
