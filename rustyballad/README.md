# rustyballad

A compiled, low-RAM rewrite of the **ballad** Quickshell desktop shell for
[niri](https://github.com/YaLTeR/niri). One small Rust binary replaces the QML
runtime and renders everything on the CPU with `tiny-skia` + `cosmic-text` —
no Qt, no GPU, no frame loop.

Measured on the reference machine (nested niri, 929×1072 output):

| | Quickshell | rustyballad |
|---|---|---|
| idle RSS | 294.6 MB | **45.1 MB** (1920×1080, warm; 24.2 MB PSS) |
| with a popup/menu open | — | **68.6 MB** |
| idle CPU | — | ~0.6 % of one core |
| release binary | — | 4.9 MB |

## Features

- Left bar: power button, 10-slot workspace dial, per-workspace window groups,
  network/volume/battery/update indicators, system tray, keyboard-layout badge,
  vertical clock.
- Desktop widgets: clock, date, CPU/RAM/storage/temps/fans/net/disk meters.
- Seven popups: power, Wi-Fi, battery/profiles, updates, calendar, window menu,
  mixer. One mapped overlay surface, keyboard-aware, `Esc`/outside to close.
- niri IPC: live workspaces/windows, focus, move-to-workspace, close,
  fullscreen, floating, urgent.
- System tray: StatusNotifierItem hosts/items, ARGB32 icon decoding, DBusMenu
  with click, secondary-click and scroll support.
- QML hover behaviour and cursor: power glyph swaps `)` → `*` and reddens,
  updates/clock change colour, tray medallions grow, and the pointer becomes a
  hand over interactive widgets. The cursor uses `wp_cursor_shape_v1` when the
  compositor provides it (XCursor theme fallback).

## Requirements

- Rust 1.85+ (edition 2024) and `cargo`.
- A Wayland compositor with `wlr-layer-shell` — niri is the target.
- Optional runtime tools, depending on which widgets you use:
  - `wireplumber` (`wpctl`) and/or `pulseaudio-utils` (`pactl`) — volume/mixer
  - `networkmanager` (`nmcli`) — Wi-Fi list/connect/radio
  - `power-profiles-daemon` (`powerprofilesctl`) or TLP — power profiles
  - `elogind`/`systemd-logind` (`loginctl`) — session actions
  - `pacman` — update count and upgrade action
  - `foot` and `pwvucontrol` — terminal / external mixer defaults
- Fonts for faithful rendering: `Cinzel`, `Cinzel Decorative`, `EB Garamond`,
  `Berkshire Swash`, `Bodoni Ornaments ITC`, `Victorian Free Ornaments`, and a
  Nerd Font (`AurulentSansM Nerd Font`). Missing fonts fall back through fontdb,
  but glyphs/ornaments will differ.

## Build and install

```sh
cargo build --release
install -Dm755 target/release/rustyballad ~/.local/bin/rustyballad
```

The release profile uses fat LTO, one codegen unit, stripped symbols and
`panic = "abort"` (see `Cargo.toml`).

Run `cargo test` for the unit suite (54 tests; two `#[ignore]`d live D-Bus
tests can be run with `cargo test -- --ignored` when a tray item is present).

## Starting it

rustyballad is an ordinary graphical user program, not a daemon. Start it from
your compositor session so it inherits `WAYLAND_DISPLAY` and
`XDG_RUNTIME_DIR`.

In `~/.config/niri/config.kdl`:

```kdl
spawn-at-startup "rustyballad"
// or, if ~/.local/bin is not on the session PATH:
// spawn-at-startup "/home/you/.local/bin/rustyballad"
```

Or start it manually from a terminal inside niri:

```sh
rustyballad
```

There are deliberately **no systemd units and no OpenRC init scripts**. An
init service has no reliable ordering relative to the graphical session, and
running a user bar as root is the wrong model. If you want restart-on-crash,
supervise it from the session (for example a small `supervise-daemon` wrapper
spawned by niri) rather than from `/etc/init.d`.

### Migrating from the Quickshell shell

1. Remove the old `quickshell`/`qs` spawn line from your niri config.
2. Add `spawn-at-startup "rustyballad"` instead.
3. Restart niri (or run `rustyballad` once manually).

## Configuration

Colors, fonts, interfaces and polling intervals are compile-time defaults in
`src/config.rs` and `src/theme.rs`, mirroring `quickshell/Config.qml`. Edit and
rebuild to change them. (A runtime `config.toml` is not implemented yet.)

## Diagnostics

```sh
rustyballad --probe            # print one system-metrics sample
rustyballad --probe 500        # …every 500 ms
rustyballad --help
```

For headless testing/screenshots, `RB_POPUP` opens a popup at startup:

```sh
RB_POPUP=power|wifi|battery|updates|calendar|mixer|tray rustyballad
```

## Design notes

- Everything is drawn into `wl_shm` buffers with tiny-skia; text is shaped by
  cosmic-text with one shared `FontSystem`/`SwashCache` and per-draw buffers.
- The popup system is a single full-screen Top-layer surface that stays mapped:
  niri 26.04 misplaces Overlay-layer surfaces behind windows and downgrades any
  layer surface that is unmapped and remapped to Background. When closed, the
  overlay draws a transparent buffer, uses an empty input region (click-through)
  and no keyboard focus. Opening is just a redraw — no unmap/remap cycle.
- Drawing goes directly into reusable `wl_shm` slots (`Format::Abgr8888`); there
  is no intermediate full-screen pixmap and no per-frame slot allocation.
- One background thread handles all blocking subprocess I/O (`wpctl`, `nmcli`,
  `pacman`, `pactl`, `loginctl`); slow calls run detached so scans cannot starve
  polling. The tray runs on its own thread and polls D-Bus.
- There is no animation timer: the event loop sleeps in `dispatch()` until a
  Wayland event, niri event or timer fires.

See `PLAN.md` for the milestone log and measurement details.

## License

MIT.
