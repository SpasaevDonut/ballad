# AGENTS.md — rustyballad

Compiled Rust rewrite of the **ballad** Quickshell shell for [niri](https://github.com/YaLTeR/niri).
The reference QML is the live Quickshell config at `~/.config/quickshell`
(symlinked from `.config/quickshell` in the dotfiles repo); a development copy
also lives in `quickshell/` here. It is the source of truth for behaviour,
layout and styling. `PLAN.md` holds the milestone log and measurements.

## Current status

- Feature-complete for normal single-output use: left bar, desktop widgets, all
  seven popups, WindowMenu, system tray (SNI + DBusMenu), niri IPC, hover states
  and cursor shapes.
- **57 unit tests pass, 5 live/debug tests are `#[ignore]`d.**
- Memory (1920×1080, tray + audio + browser): **45–64 MB RSS / 24–35 MB PSS**,
  8 threads, ~0.6 % CPU. Hard ceiling is 100 MB; target ≤ 60 MB idle. The upper
  RSS range comes from warm `wl_shm` slot pools; keep an eye on it.
- Release binary is ~5.2 MB. `~/.local/bin/rustyballad` is the installed copy.

## Build, test, run

```sh
cargo build --release
cargo test                       # unit suite
cargo test -- --ignored          # live D-Bus tests (needs a tray item / audio)
install -Dm755 target/release/rustyballad ~/.local/bin/rustyballad
```

Diagnostics / debug hooks:

```sh
rustyballad --probe [interval_ms]     # one system-metrics sample
RB_POPUP=power|wifi|battery|updates|calendar|mixer|tray rustyballad
```

`RB_POPUP` opens the popup at startup (used for headless screenshots). Start the
shell from niri with `spawn-at-startup "rustyballad"`; there are no systemd/OpenRC
units by design.

### Nested niri testbed

The reference test environment is niri 26.04 on Wayland, 1920×1080, scale 1.

```sh
# terminal 1
WAYLAND_DISPLAY=wayland-1 niri -c /tmp/opencode/niri-nested/config.kdl
# it creates wayland-2 + /run/user/1000/niri.wayland-2.<pid>.sock

# terminal 2
WAYLAND_DISPLAY=wayland-2 NIRI_SOCKET=<nested.sock> \
  RB_POPUP=power ./target/release/rustyballad
```

Screenshot the nested output with `NIRI_SOCKET=<nested.sock> niri msg action
screenshot-screen` (saved to `~/Pictures/Screenshots/`). Use `niri msg -j layers`
when debugging stacking/layer issues. There is no working input-injection tool on
the reference machine; interactive click/hover tests need the user.

## Source layout

```
src/
  main.rs            CLI (--probe/--help), RAYON_NUM_THREADS=1 cap, starts shell
  config.rs          compile-time Config defaults mirroring quickshell/Config.qml
  theme.rs           palette, font families, bar dimensions (mirrors Config.qml)
  sysinfo.rs         /proc + /sys sampling: CPU, RAM, net, disk, temp, fan, battery
  niri.rs            niri IPC (2 persistent JSON-lines connections), State, actions
  audio.rs           wpctl/pactl + pactl sink-input parsing, stream volume/mute
  net.rs             nmcli scan/connect/disconnect/radio + saved profiles
  updates.rs         pacman -Qu parser + update check/spawn upgrade
  power.rs           power-profiles-daemon/TLP detection + session actions
  tray.rs            SNI host/items, ARGB32 icons (box-filtered to 16 px),
                     dead-name filtering, DBusMenu (own thread)
  ui/
    draw.rs          tiny-skia helpers: rects, borders, circles, gradients, LED bars
    text.rs          cosmic-text wrapper: layout/measure/draw into shm slots
  shell/
    mod.rs           SCTK setup, event loop, input, popups, hover/cursor, timers
    surface.rs       layer surfaces + persistent shm slots + direct PixmapMut draw
    bar.rs           LeftBar equivalent: anchors, geometry, hit-tests, painting
    desktop.rs       DesktopWidgets equivalent
    popup.rs         all popup cards, hit-testing, geometry, corner ornaments
    worker.rs        single background I/O thread + detached tasks for slow calls
```

Threads: main Wayland/event loop, `niri-events`, `niri-actions`, `rb-io`,
`rb-tray`, plus zbus/blocking threads and short-lived `rb-io-task` threads for
scans/connects/update checks. Never block the event loop; add work as a
`worker::Request`/`Response` pair instead.

## Architecture rules

1. **Rendering is CPU-only.** Everything is drawn into `wl_shm` slots with
   tiny-skia (`Format::Abgr8888`, RGBA byte order). No EGL/GPU/wgpu.
2. **Draw and hit-test share geometry.** Anchor/geometry constants live in
   `bar.rs` and `popup.rs`; hit functions must use exactly the drawn coordinates.
   Offset bugs have happened repeatedly (battery buttons 56 px, power actions
   66 px). Add a regression test for every new interactive area.
3. **Persistent slots.** `Surface` keeps a `Vec<Slot>` and reuses whichever the
   compositor released (steady state 2 slots/surface). Do **not** call
   `SlotPool::create_buffer` per frame — it allocates a new slot every time and
   was pushing RSS to 91 MB.
4. **Never unmap the overlay.** niri 26.04 composites the Overlay layer at the
   Background level in practice, and a layer surface unmapped via a null-buffer
   commit is **downgraded to Background when remapped**. The popup surface is
   therefore permanently mapped on the **Top** layer: closed = transparent
   buffer + empty `wl_input_region` + keyboard `None`. Opening is just a redraw.
5. **Popups:** one overlay surface, one popup at a time (no debounce currently).
   `card_frame`/`card_chrome` provide the parchment card, border, rotating corner
   ornaments and divider. Click-outside closes; card dead space does not. The
   wheel scrolls popup lists (Updates/Wi-Fi/Mixer/WindowMenu), clamped to the
   page size. Wayland's positive vertical axis means scroll **down**; Qt/QML
   `angleDelta` is positive **up** — invert when mapping (the bar volume and
   tray scroll use the same convention).
6. **Fonts:** cosmic-text 0.19 + swash 0.2 is required. It supports variable-font
   weight axes (real bold for Cinzel/EB Garamond) and CFF fonts (Bodoni
   Ornaments). `Cinzel Decorative` has no U+2160 — Roman numerals use EB Garamond.
   Ornaments: Victorian Free Ornaments for `z`/`7`/`A`, Bodoni for the power
   rosette/separators/dividers.
7. **Cursor:** SCTK `ThemedPointer` (`wp_cursor_shape_v1`, XCursor fallback).
   `Shell::update_cursor` shows a hand over widgets, workspaces/window rows, tray
   and popup controls. Reload it on every pointer enter.
8. **Hover:** `BarData::hover` drives QML hover states (power `)`→`*` red,
   updates red, clock gold, tray 1.08×). Popup hover uses `PopupState::hover`.
9. **Memory:** avoid retaining pixmaps/fonts/images; tray icons are 16×16
   `Arc<Pixmap>`; menu trees are fetched only on right-click. Keep
   `RAYON_NUM_THREADS=1`.

## QML fidelity notes

- Palette/theme comes from `quickshell/Config.qml` (parchment/rust/gold), **not**
  from `quickshell/AGENTS.md` (its palette is stale).
- Window group names follow QML shortening (`TERM`, `FFOX`, `WEB`, `TG`, `DSCD`,
  `FILE`, `CODE`; split on `.`, take last segment).
- Popup positions: Power 8 px below the top edge, Calendar bottom-anchored
  10 px above the screen, Wi-Fi centered on the network widget, Battery on
  battery, Mixer on volume, Updates on updates, WindowMenu at the pointer.
- Bar bottom stack heights: net 108, volume 92, battery 92, updates 32 (+12 when
  the count is visible), tray 8 + 28·N + 6·(N−1), layout 24, clock 40, separators
  18, plus 16 px bottom margin.

## What remains for full QML parity

Ordered roughly by user-visible impact. Files in parentheses.

1. **Animations / transitions** (none implemented)
   - QML uses 150 ms colour/scale `Behavior`s, press-scale 0.92/0.93 on power and
     tray, and continuously rotating icons while checking updates/scanning Wi-Fi.
   - Needs a small frame timer (only while an animation is active) plus damage
     redraws; keep it off at idle. (`bar.rs`, `popup.rs`, `shell/mod.rs`)
2. **Text-input fidelity**
   - QML `TextInput`s auto-focus and show an I-beam cursor; ours need a click to
     focus (niri on-demand focus) and show hand/default cursor.
   - Add `CursorIcon::Text` over password/filter fields, focus the overlay when a
     popup opens (possibly `KeyboardInteractivity::Exclusive`), and draw a caret.
     (`popup.rs`, `shell/mod.rs`)
3. **Native PipeWire mixer**
   - QML enumerates PipeWire nodes directly; we parse `pactl list sink-inputs`
     and add a master section. Streams that exist only in PipeWire are missed.
   - QML also supports wheel-to-change per-row volume; ours only drags. Replace
     the `pactl` layer with `pw-dump`/PipeWire bindings, keep the same UI.
     (`audio.rs`, `worker.rs`, `popup.rs`)
4. **Tray submenus**
   - DBusMenu trees are flattened with indentation; QML opens nested fly-out
     menus (and AboutToShow per submenu). Add submenu state/popup or nested card
     rendering, plus check/radio submenu handling. (`tray.rs`, `popup.rs`)
5. **Shadow fidelity**
   - QML `DropShadow` is a Gaussian blur (radius 14, offset 12); ours approximates
     with alpha rings. Consider a real blur/mask pass or more ring samples.
     (`popup.rs`)
6. **WindowMenu placement**
   - QML anchors the menu to the window item with `QsMenuAnchor`; we place it at
     the pointer + 10. Match the anchor if pixel-exact placement is required.
     (`popup.rs`, `shell/mod.rs`)
7. **Reopen debounce**
   - QML guards 150 ms after close; ours removed the debounce so fast clicks are
     not swallowed. Decide whether to restore a small guard for parity.
8. **Desktop widget audit**
   - Verify every QML widget (fans, storage, disk I/O, temps, network, RAM, CPU)
     matches values/labels/timers and the ornate divider positions. (`desktop.rs`)
9. **Buffer scale / output transform**
   - We never call `wl_surface.set_buffer_scale`; on fractional scaling the shell
     will be blurry/have wrong logical sizes. Handle output scale/transform.
     (`surface.rs`, `shell/mod.rs`)
10. **Tray details**
    - `IconThemePath`, attention/overlay icons, tooltips, and dynamic icon theme
      changes are partially handled. Native menu positioning/animation too.
      (`tray.rs`, `popup.rs`)
11. **Runtime configuration**
    - `Config.qml` values are compiled into `config.rs`/`theme.rs`. A runtime
      `~/.config/rustyballad/config.toml` is not implemented.
12. **Test coverage**
    - Unit tests cover parsers and popup geometry; there are no UI/integration
      tests beyond the ignored live D-Bus ones. Keep adding geometry↔hit-test
      regressions when touching popups.

## Measurement commands

```sh
pid=$(pgrep -n rustyballad)
grep -E 'VmRSS|Threads' /proc/$pid/status
grep -E '^(Rss|Pss):' /proc/$pid/smaps_rollup
a=$(awk '{print $14+$15}' /proc/$pid/stat); sleep 10; b=$(awk '{print $14+$15}' /proc/$pid/stat); echo $((b-a))
```

```sh
# Confirm the popup layer is not downgraded to Background
niri msg -j layers | python3 -c \
  "import json,sys; [print(l['namespace'],l['layer']) for l in json.load(sys.stdin)]"
```

## Known pitfalls

- Reintroducing per-frame `SlotPool::create_buffer` → RSS blow-up.
- Unmapping/remapping the overlay → niri moves it to Background; popup disappears
  behind windows while still eating input.
- Hard-coded popup hit Y values drifting from the draw code (always test).
- Using the non-Propo Nerd Font family changes icon metrics; QML uses
  `AurulentSansM Nerd Font Propo`.
- `Cinzel Decorative` has no Roman-numeral glyphs; fallback differs per engine.
- cosmic-text < 0.19 cannot render CFF fonts or variable weights; do not downgrade.
- Live tests in `tray.rs`/`audio.rs` assume the user session has a tray item /
  audio stream; keep them `#[ignore]`d.
- Never trust the SNI watcher list blindly: dead unique names linger after an
  item reconnects; check `NameHasOwner` before rendering.
- Wheel sign: wl_pointer positive axis = down, Qt angleDelta positive = up.
- Multiple rustyballad instances can fight over the watcher/tray; kill old ones.
