//! Wayland layer-shell runtime: bar + desktop surfaces, niri IPC, pointer
//! input, and a calloop event loop with dirty-flag redraw coalescing.
//! The process sleeps in `dispatch()` until a Wayland event, niri event or
//! timer fires — there is no frame loop.

pub mod bar;
pub mod desktop;
pub mod popup;
pub mod surface;
pub mod worker;

use std::error::Error;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use chrono::Local;
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState, Region},
    output::{OutputHandler, OutputState},
    reexports::{
        calloop::{self, EventLoop, channel, timer::TimeoutAction},
        calloop_wayland_source::WaylandSource,
        client::{
            Connection, QueueHandle,
            globals::registry_queue_init,
            protocol::{
                wl_output::{Transform, WlOutput},
                wl_keyboard::WlKeyboard,
                wl_pointer::WlPointer,
                wl_seat::WlSeat,
                wl_surface::WlSurface,
            },
        },
    },
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        Capability, SeatHandler, SeatState,
        keyboard::{KeyEvent, KeyboardHandler, Modifiers},
        pointer::{
            CursorIcon, PointerEvent, PointerEventKind, PointerHandler, ThemeSpec, ThemedPointer,
        },
    },
    shell::{
        WaylandSurface,
        wlr_layer::{
            Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
            LayerSurfaceConfigure,
        },
    },
    shm::{Shm, ShmHandler},
    {
        delegate_compositor, delegate_keyboard, delegate_layer, delegate_output,
        delegate_pointer, delegate_registry, delegate_seat, delegate_shm,
    },
};

use crate::config::Config;
use crate::niri::{self, Action as NiriAction, Event as NiriEvent};
use crate::sysinfo::{Sampler, Snapshot};
use crate::theme::Theme;
use crate::tray;
use crate::ui::text::Text;
use popup::{Hit, Kind};
use surface::Surface;
use worker::Request as IoRequest;

const BTN_LEFT: u32 = 0x110;
const BTN_RIGHT: u32 = 0x111;
const BTN_MIDDLE: u32 = 0x112;

/// Slider being dragged in the mixer popup.
#[derive(Debug, Clone, Copy)]
enum DragTarget {
    Master,
    Stream(u32),
    PowerSlider,
}

pub struct Shell {
    registry_state: RegistryState,
    output_state: OutputState,
    seat_state: SeatState,
    shm: Shm,
    compositor: CompositorState,

    bar: Surface,
    desktop: Surface,
    overlay: Surface,

    text: Text,
    theme: Theme,
    sampler: Sampler,
    snapshot: Snapshot,
    bar_data: bar::BarData,
    minute: String,

    niri: niri::State,
    niri_actions: mpsc::Sender<NiriAction>,

    io: mpsc::Sender<IoRequest>,
    tray: Vec<tray::TrayItem>,
    tray_cmds: mpsc::Sender<tray::Command>,
    tray_hover: Option<usize>,
    popup: popup::PopupState,
    popup_dirty: bool,
    drag: Option<DragTarget>,
    drag_last_send: Option<Instant>,
    /// Empty wl_region used to make the closed overlay click-through.
    empty_region: smithay_client_toolkit::compositor::Region,
    /// Whether the overlay has been mapped once with its transparent buffer.
    overlay_initialized: bool,

    pointer: Option<ThemedPointer>,
    cursor_icon: Option<CursorIcon>,
    pointer_on_bar: bool,
    pointer_on_overlay: bool,
    pointer_pos: (f64, f64),
    keyboard: Option<WlKeyboard>,

    dirty_bar: bool,
    dirty_desktop: bool,
    exit: bool,
}

impl Shell {
    fn draw_bar(&mut self) {
        let Self { bar, text, theme, snapshot, niri, bar_data, tray, tray_hover, .. } = self;
        let now = Local::now();
        bar.redraw(|pm| {
            bar::draw(pm, theme, text, snapshot, niri, bar_data, tray, *tray_hover, &now)
        });
    }

    fn draw_desktop(&mut self) {
        let Self { desktop, text, theme, snapshot, .. } = self;
        let now = Local::now();
        desktop.redraw(|pm| desktop::draw(pm, theme, text, snapshot, &now));
    }

    fn on_niri_event(&mut self, ev: NiriEvent) {
        self.niri.connected = true;
        match ev {
            NiriEvent::Workspaces(workspaces) => {
                if let Some(focused) = workspaces.iter().find(|w| w.is_focused) {
                    self.niri.active_workspace = focused.id;
                }
                self.niri.workspaces = workspaces;
            }
            NiriEvent::WorkspaceActivated(id) => {
                self.niri.active_workspace = id;
                for w in &mut self.niri.workspaces {
                    w.is_focused = w.id == id;
                }
            }
            NiriEvent::Windows(windows) => self.niri.windows = windows,
            NiriEvent::WindowOpened(window) => {
                self.niri.windows.retain(|w| w.id != window.id);
                self.niri.windows.push(window);
            }
            NiriEvent::WindowClosed(id) => self.niri.windows.retain(|w| w.id != id),
            NiriEvent::WindowFocusChanged(id) => {
                for w in &mut self.niri.windows {
                    w.is_focused = Some(w.id) == id;
                }
            }
            NiriEvent::Layouts(layouts) => self.niri.layouts = layouts,
            NiriEvent::LayoutSwitched(idx) => self.niri.layouts.current_idx = idx,
            NiriEvent::Disconnected => self.niri.connected = false,
        }
        self.dirty_bar = true;
    }

    fn handle_bar_click(&mut self, button: u32) {
        let (x, y) = (self.pointer_pos.0 as f32, self.pointer_pos.1 as f32);
        let height = self.bar.height as f32;

        if let Some(widget) = bar::widget_hit(height, y, self.tray.len(), self.bar_data.updates > 0) {
            match widget {
                bar::WidgetHit::Power => {
                    if button == BTN_LEFT {
                        self.toggle_popup(Kind::Power);
                    }
                }
                bar::WidgetHit::Network => {
                    if button == BTN_LEFT {
                        self.toggle_popup(Kind::Wifi);
                    } else if button == BTN_RIGHT {
                        self.open_popup(Kind::Wifi);
                        self.popup.wifi.scanning = true;
                        let _ = self.io.send(IoRequest::ScanWifi { rescan: true });
                        self.popup_dirty = true;
                    }
                }
                bar::WidgetHit::Volume => {
                    if button == BTN_RIGHT {
                        self.toggle_popup(Kind::Mixer);
                    } else if button == BTN_LEFT {
                        let _ = self.io.send(IoRequest::ToggleMute);
                    }
                }
                bar::WidgetHit::Battery => {
                    if button == BTN_LEFT {
                        self.toggle_popup(Kind::Battery);
                    }
                }
                bar::WidgetHit::Updates => {
                    if button == BTN_LEFT {
                        self.toggle_popup(Kind::Updates);
                    }
                }
                bar::WidgetHit::Tray(i) => {
                    // QML TrayWidget: left activates, middle secondary,
                    // right opens the DBusMenu, wheel scrolls.
                    if button == BTN_LEFT {
                        let _ = self.tray_cmds.send(tray::Command::Activate(i));
                    } else if button == BTN_MIDDLE {
                        let _ = self.tray_cmds.send(tray::Command::Secondary(i));
                    } else if button == BTN_RIGHT {
                        self.popup.pointer = (x, y);
                        let _ = self.tray_cmds.send(tray::Command::OpenMenu(i));
                    }
                }
                bar::WidgetHit::Clock => {
                    if button == BTN_LEFT {
                        self.toggle_popup(Kind::Calendar);
                    }
                }
            }
            return;
        }

        let slots = niri::slots(&self.niri.workspaces, self.niri.active_workspace);
        let groups = niri::groups(&self.niri.windows, self.niri.active_workspace);
        let geom = bar::geometry(height, &groups, self.tray.len(), self.bar_data.updates > 0);
        match bar::hit_test(&geom, x, y) {
            Some(bar::Hit::Workspace(i)) if button == BTN_LEFT => {
                let _ = self
                    .niri_actions
                    .send(NiriAction::FocusWorkspace(niri::slot_ref(&slots[i], i)));
            }
            Some(hit @ (bar::Hit::Window(_) | bar::Hit::WindowDot { .. })) => {
                let Some(id) = bar::resolve_window(hit, &groups) else { return };
                if button == BTN_LEFT {
                    let _ = self.niri_actions.send(NiriAction::FocusWindow(id));
                } else if button == BTN_RIGHT {
                    self.popup.pointer = (x, y);
                    self.open_popup(Kind::WindowMenu);
                    self.popup.menu.window = id;
                    self.popup.menu.scroll = 0;
                    self.popup_dirty = true;
                }
            }
            _ => {}
        }
    }

    // -----------------------------------------------------------------------
    // Popup lifecycle
    // -----------------------------------------------------------------------

    fn open_popup(&mut self, kind: Kind) {
        if self.popup.kind == Some(kind) {
            self.close_popup();
            return;
        }
        self.popup.kind = Some(kind);
        self.popup.hover = None;
        self.popup.hover_row = None;
        self.drag = None;
        // The overlay is already mapped (see `hide_overlay`); restore pointer
        // input and keyboard focus, then redraw the card.
        self.overlay
            .set_click_through(&self.empty_region, false);
        self.overlay
            .set_keyboard(KeyboardInteractivity::OnDemand);
        match kind {
            Kind::Wifi => {
                self.popup.wifi.scanning = true;
                let _ = self.io.send(IoRequest::PollWifiRadio);
                let _ = self.io.send(IoRequest::ScanWifi { rescan: true });
            }
            Kind::Battery => {
                let _ = self.io.send(IoRequest::PollPower);
            }
            Kind::Updates => {
                self.popup.updates.checking = true;
                let _ = self.io.send(IoRequest::PollUpdates);
            }
            Kind::Mixer => {
                let _ = self.io.send(IoRequest::PollMixer);
            }
            _ => {}
        }
        self.popup_dirty = true;
    }

    /// Hides the popup while keeping the surface mapped. A null-buffer unmap
    /// makes niri remap the surface on the Background layer, so instead the
    /// overlay is cleared to transparent and made click-through.
    fn hide_overlay(&mut self) {
        self.overlay
            .set_click_through(&self.empty_region, true);
        self.overlay
            .set_keyboard(KeyboardInteractivity::None);
        self.overlay.clear_transparent();
    }

    fn close_popup(&mut self) {
        if self.popup.kind.is_none() {
            return;
        }
        self.popup.kind = None;
        self.popup.hover = None;
        self.popup.hover_row = None;
        self.drag = None;
        self.hide_overlay();
    }

    fn toggle_popup(&mut self, kind: Kind) {
        if self.popup.kind == Some(kind) {
            self.close_popup();
        } else {
            self.open_popup(kind);
        }
    }

    /// Tracks hovered tray medallion so the bar can redraw its hover plate.
    fn update_bar_hover(&mut self) {
        let hit = bar::widget_hit(
            self.bar.height as f32,
            self.pointer_pos.1 as f32,
            self.tray.len(),
            self.bar_data.updates > 0,
        );
        let tray = match hit {
            Some(bar::WidgetHit::Tray(i)) => Some(i),
            _ => None,
        };
        if hit != self.bar_data.hover || tray != self.tray_hover {
            self.bar_data.hover = hit;
            self.tray_hover = tray;
            self.dirty_bar = true;
        }
    }

    /// Updates the pointer cursor shape (hand over interactive elements).
    fn update_cursor(&mut self, conn: &Connection) {
        let icon = if self.pointer_on_bar {
            let h = self.bar.height as f32;
            let (x, y) = (self.pointer_pos.0 as f32, self.pointer_pos.1 as f32);
            let widget = bar::widget_hit(h, y, self.tray.len(), self.bar_data.updates > 0);
            let slots = niri::slots(&self.niri.workspaces, self.niri.active_workspace);
            let groups = niri::groups(&self.niri.windows, self.niri.active_workspace);
            let geom = bar::geometry(h, &groups, self.tray.len(), self.bar_data.updates > 0);
            let _ = slots;
            if widget.is_some() || bar::hit_test(&geom, x, y).is_some() {
                CursorIcon::Pointer
            } else {
                CursorIcon::Default
            }
        } else if self.pointer_on_overlay && self.popup.kind.is_some() {
            let geom = self.popup_geometry();
            let hit = popup::hit(
                &self.popup,
                &geom,
                self.pointer_pos.0 as f32,
                self.pointer_pos.1 as f32,
                self.visible_workspace_count(),
            );
            let card = geom.card(
                self.popup.kind.unwrap(),
                self.popup.pointer,
                self.visible_workspace_count(),
                &self.popup.tray.entries,
            );
            let inside = self.pointer_pos.0 as f32 >= card.x()
                && self.pointer_pos.0 as f32 <= card.x() + card.width()
                && self.pointer_pos.1 as f32 >= card.y()
                && self.pointer_pos.1 as f32 <= card.y() + card.height();
            if inside && hit != Hit::Outside {
                CursorIcon::Pointer
            } else {
                CursorIcon::Default
            }
        } else {
            CursorIcon::Default
        };
        if self.cursor_icon != Some(icon) {
            self.cursor_icon = Some(icon);
            if let Some(pointer) = &self.pointer {
                let _ = pointer.set_cursor(conn, icon);
            }
        }
    }

    fn on_tray_event(&mut self, ev: tray::Event) {
        match ev {
            tray::Event::Items(items) => {
                if self.tray.len() != items.len() {
                    self.dirty_bar = true;
                }
                self.tray = items;
                self.dirty_bar = true;
            }
            tray::Event::Menu {
                tray_index,
                entries,
            } => {
                self.popup.tray.tray_index = tray_index;
                self.popup.tray.entries = entries;
                self.popup.tray.scroll = 0;
                self.open_popup(Kind::TrayMenu);
                self.popup_dirty = true;
            }
        }
    }

    fn popup_geometry(&self) -> popup::Geometry {
        popup::Geometry::new(
            self.overlay.width as f32,
            self.overlay.height as f32,
            self.tray.len(),
            self.bar_data.updates > 0,
        )
    }

    fn visible_workspace_count(&self) -> usize {
        self.niri.workspaces.len()
    }

    fn handle_popup_press(&mut self, button: u32) {
        let Some(kind) = self.popup.kind else {
            return;
        };
        let (x, y) = (self.pointer_pos.0 as f32, self.pointer_pos.1 as f32);
        let geom = self.popup_geometry();
        let n_ws = self.visible_workspace_count();
        let hit = popup::hit(&self.popup, &geom, x, y, n_ws);
        let card = geom.card(kind, self.popup.pointer, n_ws, &self.popup.tray.entries);
        let inside_card = x >= card.x()
            && x <= card.x() + card.width()
            && y >= card.y()
            && y <= card.y() + card.height();
        if button == BTN_RIGHT {
            // QML cards only respond to left clicks; right-click outside closes.
            if hit == Hit::Outside && !inside_card {
                self.close_popup();
            }
            return;
        }
        match hit {
            Hit::Outside => {
                // A click on the card's own empty space must not dismiss it;
                // only clicks outside the card do (QML card MouseArea swallows).
                if !inside_card {
                    self.close_popup();
                }
            }
            Hit::PowerButton(i) => {
                if let Some(action) = crate::power::SessionAction::all().get(i as usize) {
                    let _ = self.io.send(IoRequest::Session(*action));
                }
                self.close_popup();
            }
            Hit::PowerProfile(i) => {
                self.apply_profile(i);
            }
            Hit::PowerSlider { x } => {
                self.drag = Some(DragTarget::PowerSlider);
                self.drag_to(x);
            }
            Hit::WifiToggle => {
                let on = !self.popup.wifi.enabled;
                let _ = self.io.send(IoRequest::SetRadio(on));
                self.popup_dirty = true;
            }
            Hit::WifiScan => {
                self.popup.wifi.scanning = true;
                let _ = self.io.send(IoRequest::ScanWifi { rescan: true });
                self.popup_dirty = true;
            }
            Hit::WifiEnable => {
                let _ = self.io.send(IoRequest::SetRadio(true));
                self.popup_dirty = true;
            }
            Hit::WifiDisconnect => {
                if let Some(net) = self
                    .popup
                    .wifi
                    .networks
                    .iter()
                    .find(|n| n.in_use)
                    .cloned()
                {
                    let _ = self.io.send(IoRequest::Disconnect {
                        iface: "wlan0".into(),
                        ssid: net.ssid,
                    });
                }
            }
            Hit::WifiRow(i) => {
                let Some(net) = self.popup.wifi.networks.get(i).cloned() else {
                    return;
                };
                if net.in_use {
                    return;
                }
                if net.secured() && !net.is_saved {
                    if net.secured() && self.popup.wifi.password.is_empty() {
                        self.popup.wifi.status = "Enter password".into();
                        self.popup.wifi.error = true;
                    }
                    self.popup.wifi.expanded = Some(net.ssid);
                    self.popup.wifi.password.clear();
                } else {
                    self.popup.wifi.expanded = None;
                    self.popup.wifi.password.clear();
                    self.start_wifi_connect(net.ssid, None);
                }
                self.popup_dirty = true;
            }
            Hit::WifiConnect => {
                if let Some(ssid) = self.popup.wifi.expanded.clone() {
                    let pwd = self.popup.wifi.password.clone();
                    self.start_wifi_connect(ssid, Some(pwd));
                }
            }
            Hit::WifiCancel => {
                self.popup.wifi.expanded = None;
                self.popup.wifi.password.clear();
                self.popup.wifi.status.clear();
                self.popup_dirty = true;
            }
            Hit::WifiReveal => {
                self.popup.wifi.show_password = !self.popup.wifi.show_password;
                self.popup_dirty = true;
            }
            Hit::UpdatesRefresh => {
                self.popup.updates.checking = true;
                let _ = self.io.send(IoRequest::PollUpdates);
                self.popup_dirty = true;
            }
            Hit::UpdatesUpgrade => {
                let _ = self.io.send(IoRequest::SpawnUpgrade);
                self.close_popup();
            }
            Hit::MenuAction(i) => {
                let id = self.popup.menu.window;
                let action = match i {
                    0 => NiriAction::CloseWindow(id),
                    1 => NiriAction::FullscreenWindow(id),
                    2 => NiriAction::ToggleFloating(id),
                    _ => NiriAction::SetUrgent(id),
                };
                let _ = self.niri_actions.send(action);
                self.close_popup();
            }
            Hit::MenuWorkspace(i) => {
                let mut workspaces = self.niri.workspaces.clone();
                workspaces.sort_by_key(|w| w.idx);
                if let Some(ws) = workspaces.get(i) {
                    let ws_ref = match &ws.name {
                        Some(name) => niri::WsRef::Name(name.clone()),
                        None => niri::WsRef::Id(ws.id),
                    };
                    let _ = self.niri_actions.send(NiriAction::MoveToWorkspace {
                        window: self.popup.menu.window,
                        ws: ws_ref,
                    });
                }
                self.close_popup();
            }
            Hit::TrayMenuRow(i) => {
                let Some(entry) = self.popup.tray.entries.get(i) else {
                    return;
                };
                if entry.separator || !entry.enabled {
                    return;
                }
                let id = entry.id;
                let tray_index = self.popup.tray.tray_index;
                let _ = self.tray_cmds.send(tray::Command::MenuEvent(tray_index, id));
                self.close_popup();
            }
            Hit::MixerMasterMute => {
                let _ = self.io.send(IoRequest::ToggleMute);
            }
            Hit::MixerMasterSeek { x } => {
                self.drag = Some(DragTarget::Master);
                self.drag_to(x);
            }
            Hit::MixerOpenExternal => {
                let _ = self.io.send(IoRequest::LaunchMixer);
                self.close_popup();
            }
            Hit::MixerMute(row) => {
                if let Some(s) = self.popup.mixer.streams.get(row) {
                    let _ = self.io.send(IoRequest::SetStreamMute {
                        id: s.id,
                        muted: !s.muted,
                    });
                }
            }
            Hit::MixerSeek { row, x } => {
                if let Some(s) = self.popup.mixer.streams.get(row) {
                    self.drag = Some(DragTarget::Stream(s.id));
                    self.drag_to(x);
                }
            }
            Hit::WheelUp | Hit::WheelDown => {}
        }
    }

    fn apply_profile(&mut self, i: u8) {
        self.popup.power.mode = i;
        self.popup.power.error = false;
        self.popup.power.status = format!("Applying {} mode...", crate::power::profile_name(i));
        let _ = self.io.send(IoRequest::ApplyProfile(i));
        self.popup_dirty = true;
    }

    /// Applies a slider position while dragging, throttling subprocess calls
    /// while keeping the drawn slider in sync with the pointer.
    fn drag_to(&mut self, x: f32) {
        let geom = self.popup_geometry();
        let card = geom.card(
            Kind::Mixer,
            self.popup.pointer,
            self.visible_workspace_count(),
            &[],
        );
        let now = Instant::now();
        let send = self
            .drag_last_send
            .is_none_or(|t| now.duration_since(t) >= Duration::from_millis(35));
        match self.drag {
            Some(DragTarget::Master) => {
                let track_x = card.x() + 16.0;
                let track_w = card.width() - 32.0;
                let v = ((x - track_x) / track_w).clamp(0.0, 1.0);
                self.bar_data.audio.volume = v;
                if send {
                    self.drag_last_send = Some(now);
                    let _ = self.io.send(IoRequest::SetVolume(v));
                }
            }
            Some(DragTarget::PowerSlider) => {
                let track_x = card.x() + 16.0;
                let track_w = card.width() - 32.0 - 16.0;
                let i = (((x - track_x) / track_w * 2.0).round() as i32).clamp(0, 2) as u8;
                if i != self.popup.power.mode {
                    self.apply_profile(i);
                }
            }
            Some(DragTarget::Stream(id)) => {
                let track_x = card.x() + 16.0;
                let track_w = card.width() - 32.0 - 80.0;
                let v = ((x - track_x) / track_w).clamp(0.0, 1.0);
                if let Some(s) = self.popup.mixer.streams.iter_mut().find(|s| s.id == id) {
                    s.volume = v;
                }
                if send {
                    self.drag_last_send = Some(now);
                    let _ = self.io.send(IoRequest::SetStreamVolume { id, volume: v });
                }
            }
            None => {}
        }
        self.popup_dirty = true;
    }

    fn start_wifi_connect(&mut self, ssid: String, password: Option<String>) {
        self.popup.wifi.connecting = true;
        self.popup.wifi.expanded = None;
        self.popup.wifi.password.clear();
        self.popup.wifi.connecting_ssid = ssid.clone();
        self.popup.wifi.status = format!("Connecting to {ssid}...");
        self.popup.wifi.error = false;
        let _ = self.io.send(IoRequest::Connect { ssid, password });
        self.popup_dirty = true;
    }

    fn handle_popup_motion(&mut self) {
        if self.popup.kind.is_none() {
            return;
        }
        let (x, y) = (self.pointer_pos.0 as f32, self.pointer_pos.1 as f32);
        let geom = self.popup_geometry();
        let n_ws = self.visible_workspace_count();
        let hit = popup::hit(&self.popup, &geom, x, y, n_ws);
        let hover = if hit == Hit::Outside { None } else { Some(hit) };
        let row = match hit {
            Hit::WifiRow(i) => Some(i),
            _ => None,
        };
        if self.drag.is_some() {
            self.drag_to(x);
        }
        if hover != self.popup.hover || row != self.popup.hover_row {
            self.popup.hover = hover;
            self.popup.hover_row = row;
            self.popup_dirty = true;
        }
    }

    fn handle_popup_key(&mut self, event: &KeyEvent) {
        let Some(kind) = self.popup.kind else { return };
        let keysym: u32 = event.keysym.into();
        const ESC: u32 = 0xff1b;
        const ENTER: u32 = 0xff0d;
        const KP_ENTER: u32 = 0xff8d;
        const BACKSPACE: u32 = 0xff08;

        if keysym == ESC {
            match kind {
                Kind::Wifi if self.popup.wifi.expanded.is_some() => {
                    self.popup.wifi.expanded = None;
                    self.popup.wifi.password.clear();
                    self.popup.wifi.status.clear();
                    self.popup_dirty = true;
                }
                Kind::Updates if !self.popup.updates.filter.is_empty() => {
                    self.popup.updates.filter.clear();
                    self.popup.updates.filter_focused = true;
                    self.popup.updates.scroll = 0;
                    self.popup_dirty = true;
                }
                _ => self.close_popup(),
            }
            return;
        }
        if keysym == ENTER || keysym == KP_ENTER {
            match kind {
                Kind::Wifi if self.popup.wifi.expanded.is_some() => {
                    if let Some(ssid) = self.popup.wifi.expanded.clone() {
                        let pwd = self.popup.wifi.password.clone();
                        self.start_wifi_connect(ssid, Some(pwd));
                    }
                }
                Kind::Updates => {
                    self.popup.updates.filter_focused = false;
                    self.popup_dirty = true;
                }
                _ => {}
            }
            return;
        }
        if keysym == BACKSPACE {
            match kind {
                Kind::Wifi if self.popup.wifi.expanded.is_some() => {
                    self.popup.wifi.password.pop();
                    self.popup_dirty = true;
                }
                Kind::Updates if self.popup.updates.filter_focused => {
                    self.popup.updates.filter.pop();
                    self.popup.updates.scroll = 0;
                    self.popup_dirty = true;
                }
                _ => {}
            }
            return;
        }
        if let Some(text) = &event.utf8 {
            if text.chars().all(|c| !c.is_control()) {
                match kind {
                    Kind::Wifi if self.popup.wifi.expanded.is_some() => {
                        self.popup.wifi.password.push_str(text);
                        self.popup_dirty = true;
                    }
                    Kind::Updates => {
                        self.popup.updates.filter_focused = true;
                        self.popup.updates.filter.push_str(text);
                        self.popup.updates.scroll = 0;
                        self.popup_dirty = true;
                    }
                    _ => {}
                }
            }
        }
    }

    fn draw_popup(&mut self) {
        if self.popup.kind.is_none() {
            return;
        }
        let Self {
            overlay,
            text,
            theme,
            snapshot,
            niri,
            popup,
            bar_data,
            tray,
            ..
        } = self;
        let tray_len = tray.len();
        let now = Local::now();
        overlay.redraw(|pm| {
            popup::draw(
                pm,
                theme,
                text,
                popup,
                snapshot,
                niri,
                bar_data.audio,
                tray_len,
                bar_data.updates > 0,
                &now,
            )
        });
    }

    fn on_io_response(&mut self, resp: worker::Response) {
        match resp {
            worker::Response::Audio(a) => {
                if a != self.bar_data.audio {
                    self.bar_data.audio = a;
                    self.dirty_bar = true;
                }
                if self.popup.kind == Some(Kind::Mixer) {
                    self.popup_dirty = true;
                }
            }
            worker::Response::WifiActive(active) => {
                let (connected, signal) = match &active {
                    Some((_, signal)) => (true, *signal),
                    None => (false, 0),
                };
                if connected != self.bar_data.wifi_connected
                    || signal != self.bar_data.wifi_signal
                {
                    self.bar_data.wifi_connected = connected;
                    self.bar_data.wifi_signal = signal;
                    self.dirty_bar = true;
                }
            }
            worker::Response::Radio(on) => {
                if self.bar_data.wifi_enabled != on {
                    self.bar_data.wifi_enabled = on;
                    self.dirty_bar = true;
                }
                if self.popup.wifi.enabled != on {
                    self.popup.wifi.enabled = on;
                    if !on {
                        self.popup.wifi.networks.clear();
                    }
                    self.popup_dirty = true;
                }
            }
            worker::Response::Updates(list) => {
                let count = list.len();
                self.popup.updates.list = list;
                self.popup.updates.checking = false;
                self.popup.updates.last_check = Local::now().format("%H:%M").to_string();
                if self.bar_data.updates != count {
                    self.bar_data.updates = count;
                    self.dirty_bar = true;
                }
                self.popup_dirty = true;
            }
            worker::Response::Power(mode) => {
                self.popup.power.mode = mode;
                self.popup.power.status.clear();
                self.popup.power.error = false;
                self.popup_dirty = true;
            }
            worker::Response::Scan(nets) => {
                self.popup.wifi.scanning = false;
                self.popup.wifi.networks = nets;
                if self.popup.wifi.connecting {
                    self.popup.wifi.connecting = false;
                    let ssid = self.popup.wifi.connecting_ssid.clone();
                    if self.popup.wifi.networks.iter().any(|n| n.in_use) {
                        self.popup.wifi.status = format!("Connected to {ssid}");
                        self.popup.wifi.error = false;
                    } else {
                        self.popup.wifi.status = format!("Failed to connect to {ssid}");
                        self.popup.wifi.error = true;
                    }
                }
                self.popup_dirty = true;
            }
            worker::Response::ConnectResult { ssid, result } => {
                self.popup.wifi.connecting = false;
                self.popup.wifi.connecting_ssid.clear();
                match result {
                    Ok(()) => {
                        self.popup.wifi.status = format!("Connected to {ssid}");
                        self.popup.wifi.error = false;
                        self.popup.wifi.expanded = None;
                        self.popup.wifi.password.clear();
                    }
                    Err(msg) => {
                        self.popup.wifi.status = format!("Failed: {msg}");
                        self.popup.wifi.error = true;
                    }
                }
                self.popup_dirty = true;
            }
            worker::Response::Mixer(streams) => {
                self.popup.mixer.streams = streams;
                self.popup_dirty = true;
            }
        }
    }
}

impl CompositorHandler for Shell {
    fn scale_factor_changed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _surface: &WlSurface, _factor: i32) {}
    fn transform_changed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _surface: &WlSurface, _transform: Transform) {}
    fn frame(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _surface: &WlSurface, _time: u32) {}
    fn surface_enter(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _surface: &WlSurface, _output: &WlOutput) {}
    fn surface_leave(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _surface: &WlSurface, _output: &WlOutput) {}
}

impl OutputHandler for Shell {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }
    fn new_output(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: WlOutput) {}
    fn update_output(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: WlOutput) {}
    fn output_destroyed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: WlOutput) {}
}

impl LayerShellHandler for Shell {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _surface: &LayerSurface) {
        self.exit = true;
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        surface: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        if surface.wl_surface() == self.bar.layer.wl_surface() {
            if self.bar.apply_configure(configure.new_size) {
                self.dirty_bar = true;
            }
        } else if surface.wl_surface() == self.desktop.layer.wl_surface() {
            if self.desktop.apply_configure(configure.new_size) {
                self.dirty_desktop = true;
            }
        } else if surface.wl_surface() == self.overlay.layer.wl_surface() {
            if self.overlay.apply_configure(configure.new_size) {
                self.popup_dirty = true;
            }
            if !self.overlay_initialized {
                // First configure: map the overlay transparent so it never
                // needs to be unmapped/remapped again.
                self.overlay_initialized = true;
                if self.popup.kind.is_none() {
                    self.hide_overlay();
                }
            }
        }
    }
}

impl ShmHandler for Shell {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl SeatHandler for Shell {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }
    fn new_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: WlSeat) {}
    fn new_capability(&mut self, _conn: &Connection, qh: &QueueHandle<Self>, seat: WlSeat, capability: Capability) {
        if capability == Capability::Pointer && self.pointer.is_none() {
            let cursor_surface = self.compositor.create_surface(qh);
            self.pointer = self
                .seat_state
                .get_pointer_with_theme(
                    qh,
                    &seat,
                    self.shm.wl_shm(),
                    cursor_surface,
                    ThemeSpec::default(),
                )
                .ok();
            self.cursor_icon = None;
        }
        if capability == Capability::Keyboard && self.keyboard.is_none() {
            self.keyboard = self.seat_state.get_keyboard(qh, &seat, None).ok();
        }
    }
    fn remove_capability(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: WlSeat, capability: Capability) {
        if capability == Capability::Pointer {
            self.pointer = None;
        }
        if capability == Capability::Keyboard {
            self.keyboard = None;
        }
    }
    fn remove_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: WlSeat) {}
}

impl PointerHandler for Shell {
    fn pointer_frame(&mut self, conn: &Connection, _qh: &QueueHandle<Self>, _pointer: &WlPointer, events: &[PointerEvent]) {
        for ev in events {
            match &ev.kind {
                PointerEventKind::Enter { .. } => {
                    self.pointer_on_bar = ev.surface == *self.bar.layer.wl_surface();
                    self.pointer_on_overlay = ev.surface == *self.overlay.layer.wl_surface();
                    self.pointer_pos = ev.position;
                    self.cursor_icon = None;
                    if self.pointer_on_bar {
                        self.update_bar_hover();
                    }
                    self.update_cursor(conn);
                }
                PointerEventKind::Leave { .. } => {
                    if ev.surface == *self.bar.layer.wl_surface() {
                        self.pointer_on_bar = false;
                        let had_hover = self.bar_data.hover.take().is_some();
                        let had_tray = self.tray_hover.take().is_some();
                        if had_hover || had_tray {
                            self.dirty_bar = true;
                        }
                    }
                    if ev.surface == *self.overlay.layer.wl_surface() {
                        self.pointer_on_overlay = false;
                    }
                }
                PointerEventKind::Motion { .. } => {
                    self.pointer_pos = ev.position;
                    if self.pointer_on_overlay {
                        self.handle_popup_motion();
                    } else if self.pointer_on_bar {
                        self.update_bar_hover();
                    }
                    self.update_cursor(conn);
                }
                PointerEventKind::Press { button, .. } => {
                    if self.pointer_on_overlay {
                        self.handle_popup_press(*button);
                    } else if self.pointer_on_bar {
                        self.handle_bar_click(*button);
                    }
                }
                PointerEventKind::Release { .. } => {
                    self.drag = None;
                    self.drag_last_send = None;
                }
                PointerEventKind::Axis { vertical, .. } => {
                    // Wayland: positive axis = scroll down; Qt/QML angleDelta
                    // (and therefore the QML behaviour) is positive upwards.
                    let up = vertical.discrete < 0
                        || (vertical.discrete == 0 && vertical.absolute < 0.0);
                    let down = vertical.discrete > 0
                        || (vertical.discrete == 0 && vertical.absolute > 0.0);
                    if !(up || down) {
                        continue;
                    }
                    let step: i32 = if up { -1 } else { 1 };

                    // Wheel over a popup card scrolls its list.
                    if self.pointer_on_overlay {
                        if let Some(kind) = self.popup.kind {
                            let geom = self.popup_geometry();
                            let card = geom.card(
                                kind,
                                self.popup.pointer,
                                self.visible_workspace_count(),
                                &self.popup.tray.entries,
                            );
                            let (px, py) = (
                                self.pointer_pos.0 as f32,
                                self.pointer_pos.1 as f32,
                            );
                            let inside = px >= card.x()
                                && px <= card.x() + card.width()
                                && py >= card.y()
                                && py <= card.y() + card.height();
                            if inside {
                                match kind {
                                    Kind::Updates => {
                                        let n = self.popup.updates.filtered().len();
                                        let max = n.saturating_sub(6);
                                        self.popup.updates.scroll = (self.popup.updates.scroll
                                            as i32
                                            + step)
                                            .clamp(0, max as i32)
                                            as usize;
                                    }
                                    Kind::Wifi => {
                                        let n = self.popup.wifi.networks.len();
                                        let max = n.saturating_sub(8);
                                        self.popup.wifi.scroll = (self.popup.wifi.scroll as i32
                                            + step)
                                            .clamp(0, max as i32)
                                            as usize;
                                    }
                                    Kind::Mixer => {
                                        let n = self.popup.mixer.streams.len();
                                        let max = n.saturating_sub(4);
                                        self.popup.mixer.scroll = (self.popup.mixer.scroll as i32
                                            + step)
                                            .clamp(0, max as i32)
                                            as usize;
                                    }
                                    Kind::WindowMenu => {
                                        let n = self.visible_workspace_count();
                                        let max = n.saturating_sub(6);
                                        self.popup.menu.scroll = (self.popup.menu.scroll as i32
                                            + step)
                                            .clamp(0, max as i32)
                                            as usize;
                                    }
                                    _ => {}
                                }
                                self.popup_dirty = true;
                            }
                        }
                        continue;
                    }

                    if !self.pointer_on_bar {
                        continue;
                    }
                    let h = self.bar.height as f32;
                    match bar::widget_hit(h, ev.position.1 as f32, self.tray.len(), self.bar_data.updates > 0) {
                        Some(bar::WidgetHit::Volume) => {
                            let _ = self.io.send(IoRequest::ChangeVolume(if up {
                                0.05
                            } else {
                                -0.05
                            }));
                        }
                        Some(bar::WidgetHit::Tray(i)) => {
                            // QML passes the raw wheel delta; one notch = ±120.
                            let _ = self
                                .tray_cmds
                                .send(tray::Command::Scroll(i, if up { 120 } else { -120 }));
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

impl KeyboardHandler for Shell {
    fn enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &WlKeyboard,
        _surface: &WlSurface,
        _serial: u32,
        _raw: &[u32],
        _keysyms: &[smithay_client_toolkit::seat::keyboard::Keysym],
    ) {
    }

    fn leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &WlKeyboard,
        _surface: &WlSurface,
        _serial: u32,
    ) {
    }

    fn press_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &WlKeyboard,
        _serial: u32,
        event: KeyEvent,
    ) {
        if self.popup.kind.is_some() {
            self.handle_popup_key(&event);
        }
    }

    fn release_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &WlKeyboard,
        _serial: u32,
        _event: KeyEvent,
    ) {
    }

    fn update_modifiers(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &WlKeyboard,
        _serial: u32,
        _modifiers: Modifiers,
        _layout: u32,
    ) {
    }
}

impl ProvidesRegistryState for Shell {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState];
}

delegate_compositor!(Shell);
delegate_output!(Shell);
delegate_layer!(Shell);
delegate_shm!(Shell);
delegate_seat!(Shell);
delegate_pointer!(Shell);
delegate_keyboard!(Shell);
delegate_registry!(Shell);

pub fn run() -> Result<(), Box<dyn Error>> {
    let conn = Connection::connect_to_env()?;
    let (globals, event_queue) = registry_queue_init::<Shell>(&conn)?;
    let qh = event_queue.handle();

    let compositor = CompositorState::bind(&globals, &qh)?;
    let layer_shell = LayerShell::bind(&globals, &qh)?;
    let shm = Shm::bind(&globals, &qh)?;
    let output_state = OutputState::new(&globals, &qh);
    let seat_state = SeatState::new(&globals, &qh);

    let theme = Theme::default();
    let cfg = Config::default();
    let mut sampler = Sampler::new(cfg.clone());
    let snapshot = sampler.sample(); // primes delta trackers

    let bar = Surface::new(
        &compositor,
        &layer_shell,
        &shm,
        &qh,
        "rustyballad-bar",
        Layer::Top,
        (theme.bar_total_width(), 0),
        Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT,
        theme.bar_width as i32,
    );
    let desktop = Surface::new(
        &compositor,
        &layer_shell,
        &shm,
        &qh,
        "rustyballad-desktop",
        Layer::Background,
        (0, 0),
        Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT,
        0,
    );
    let overlay = Surface::new(
        &compositor,
        &layer_shell,
        &shm,
        &qh,
        "rustyballad-popup",
        // niri 26.04 places overlay-layer surfaces behind normal windows in
        // practice; Top is reliably above windows while still below the bar.
        Layer::Top,
        (0, 0),
        Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT,
        0,
    );

    // Niri IPC: action sender + event channel.
    let (niri_tx, niri_rx) = channel::channel::<NiriEvent>();
    let niri_actions = niri::start(niri_tx);

    // Background I/O worker: blocking subprocesses off the event loop.
    let (io_tx, io_rx) = channel::channel::<worker::Response>();
    let io = worker::start(io_tx);

    // System tray: own thread, D-Bus StatusNotifierHost + DBusMenu.
    let (tray_tx, tray_rx) = channel::channel::<tray::Event>();
    let tray_cmds = tray::start(tray_tx);

    // Empty input region: makes the closed popup overlay click-through.
    let empty_region = Region::new(&compositor)?;

    let mut shell = Shell {
        registry_state: RegistryState::new(&globals),
        output_state,
        seat_state,
        shm,
        compositor,
        bar,
        desktop,
        overlay,
        text: Text::new(),
        theme,
        sampler,
        snapshot,
        bar_data: bar::BarData::default(),
        minute: String::new(),
        niri: niri::State::default(),
        niri_actions,
        io,
        tray: Vec::new(),
        tray_cmds,
        tray_hover: None,
        popup: popup::PopupState::default(),
        popup_dirty: false,
        drag: None,
        drag_last_send: None,
        empty_region,
        overlay_initialized: false,
        pointer: None,
        cursor_icon: None,
        pointer_on_bar: false,
        pointer_on_overlay: false,
        pointer_pos: (0.0, 0.0),
        keyboard: None,
        dirty_bar: true,
        dirty_desktop: true,
        exit: false,
    };

    let mut event_loop: EventLoop<Shell> = EventLoop::try_new()?;
    let loop_handle = event_loop.handle();
    WaylandSource::new(conn, event_queue).insert(loop_handle.clone())?;

    loop_handle.insert_source(niri_rx, |ev, _, shell| {
        if let channel::Event::Msg(niri_ev) = ev {
            shell.on_niri_event(niri_ev);
        }
    })?;

    loop_handle.insert_source(io_rx, |ev, _, shell| {
        if let channel::Event::Msg(resp) = ev {
            shell.on_io_response(resp);
        }
    })?;

    loop_handle.insert_source(tray_rx, |ev, _, shell| {
        if let channel::Event::Msg(tray_ev) = ev {
            shell.on_tray_event(tray_ev);
        }
    })?;

    // `RB_POPUP=tray` screenshot hook: open the first medallion's menu once
    // the bar has been configured (surfaces report their real size then).
    if std::env::var("RB_POPUP").as_deref() == Ok("tray") {
        loop_handle.insert_source(
            calloop::timer::Timer::from_duration(Duration::from_millis(1500)),
            |_, _, shell| {
                if !shell.tray.is_empty() && shell.popup.kind.is_none() {
                    let a = bar::anchors(
                        shell.bar.height as f32,
                        shell.tray.len(),
                        shell.bar_data.updates > 0,
                    );
                    shell.popup.pointer = (5.0, a.tray_top + 4.0 + 14.0);
                    let _ = shell.tray_cmds.send(tray::Command::OpenMenu(0));
                }
                TimeoutAction::Drop
            },
        )?;
    }

    // Volume poll (QML Pipewire is push-based; 2 s polling suffices here).
    loop_handle.insert_source(calloop::timer::Timer::from_duration(Duration::from_secs(2)), |_, _, shell| {
        let _ = shell.io.send(IoRequest::PollAudio);
        if shell.popup.kind == Some(Kind::Mixer) {
            let _ = shell.io.send(IoRequest::PollMixer);
        }
        TimeoutAction::ToDuration(Duration::from_secs(2))
    })?;

    // Wi-Fi connected SSID/signal (QML 5 s) and radio state (QML 6 s).
    loop_handle.insert_source(calloop::timer::Timer::from_duration(Duration::from_secs(5)), |_, _, shell| {
        let _ = shell.io.send(IoRequest::PollWifiActive);
        TimeoutAction::ToDuration(Duration::from_secs(5))
    })?;
    loop_handle.insert_source(calloop::timer::Timer::from_duration(Duration::from_secs(6)), |_, _, shell| {
        let _ = shell.io.send(IoRequest::PollWifiRadio);
        TimeoutAction::ToDuration(Duration::from_secs(6))
    })?;

    // Pacman update check (QML 40 min).
    let updates_interval = Duration::from_millis(cfg.updates_check_interval_ms);
    loop_handle.insert_source(calloop::timer::Timer::from_duration(updates_interval), |_, _, shell| {
        let _ = shell.io.send(IoRequest::PollUpdates);
        TimeoutAction::ToDuration(Duration::from_millis(2_400_000))
    })?;

    // Metric tick (QML SysData timers: 2 s for cpu/ram/net/disk/temps).
    loop_handle.insert_source(calloop::timer::Timer::from_duration(Duration::from_secs(2)), |_, _, shell| {
        shell.snapshot = shell.sampler.sample();
        shell.dirty_bar = true;
        shell.dirty_desktop = true;
        TimeoutAction::ToDuration(Duration::from_secs(2))
    })?;

    // Clock tick: redraw only when the displayed minute changes.
    loop_handle.insert_source(calloop::timer::Timer::from_duration(Duration::from_secs(1)), |_, _, shell| {
        let minute = Local::now().format("%H:%M").to_string();
        if minute != shell.minute {
            shell.minute = minute;
            shell.dirty_bar = true;
            shell.dirty_desktop = true;
        }
        TimeoutAction::ToDuration(Duration::from_secs(1))
    })?;

    // Prime the pacman count immediately (QML checks on startup as well).
    let _ = shell.io.send(IoRequest::PollUpdates);

    // Debug helper: open a popup at startup (`RB_POPUP=wifi|power|battery|
    // updates|calendar|mixer`) for screenshots and headless verification.
    if let Ok(name) = std::env::var("RB_POPUP") {
        let kind = match name.as_str() {
            "power" => Some(Kind::Power),
            "wifi" => Some(Kind::Wifi),
            "battery" => Some(Kind::Battery),
            "updates" => Some(Kind::Updates),
            "calendar" => Some(Kind::Calendar),
            "mixer" => Some(Kind::Mixer),
            _ => None,
        };
        if let Some(kind) = kind {
            shell.open_popup(kind);
        }
    }

    loop {
        event_loop.dispatch(None, &mut shell)?;
        if shell.exit {
            break;
        }
        // Coalesce all state changes from this dispatch into one redraw.
        if shell.dirty_bar {
            shell.dirty_bar = false;
            shell.draw_bar();
        }
        if shell.dirty_desktop {
            shell.dirty_desktop = false;
            shell.draw_desktop();
        }
        if shell.popup_dirty {
            shell.popup_dirty = false;
            shell.draw_popup();
        }
    }
    Ok(())
}
