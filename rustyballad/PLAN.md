# RustyBallad — Rewrite Plan

**Goal:** rewrite the `ballad` Quickshell (QML/Qt) shell as a compiled binary that is
feature-equivalent, with a hard ceiling of **< 100 MiB RAM** (target: ≤ 60 MiB idle RSS),
and lower idle CPU than the QML original.

**Baseline (measured on this machine, 2026-11-09):**

| Process | VmSize | VmRSS |
|---|---|---|
| `quickshell` (current ballad, running) | 1 166 812 kB | **294 556 kB (~288 MiB)** |

The QML tree is unchanged and stays in `quickshell/` as the reference implementation.
The Rust crate lives at the repository root (`Cargo.toml`, `src/`).

---

## 1. What exists today (audit of `quickshell/`, 22 files, ~210 kB QML)

### Surfaces
| Surface | QML | Layer-shell | Contents |
|---|---|---|---|
| Left bar | `LeftBar.qml` | top layer, anchored left/top/bottom, exclusive zone 50 px, 65 px wide incl. 15 px gradient shadow | PowerButton, workspace dial (10 roman-numeral slots) + per-workspace window groups, network, volume, battery, updates, tray, keyboard layout, vertical clock |
| Desktop widgets | `DesktopWidgets.qml` | **background** layer, full screen, exclusive 0 | Big clock (Cinzel 76 px) + date, 2×4 grid of LED-block monitors: CPU, NETWORK, MEMORY, DISK I/O, STORAGE, THERMAL, SYS TEMP, COOLING |
| Popups (7) | `PowerPopup`, `WifiPopup`, `BatteryPopup`, `UpdatesPopup`, `CalendarPopup`, `WindowMenu`, `MixerPopup` | full-screen Top-layer surface (`Layer::Overlay` is misplaced by niri 26.04) | session power, Wi-Fi manager, power profiles, pacman updates, calendar, window context menu, audio mixer |

### Data flows (QML → replacement)
| Metric | QML source | Rust replacement |
|---|---|---|
| CPU % | spawns `sh -c "top -bn1 \| awk ..."` every 2 s | read `/proc/stat` twice, delta math, **no subprocess** |
| RAM % | spawns `sh -c "free \| awk ..."` every 2 s | read `/proc/meminfo` (`MemTotal`, `MemAvailable`) |
| Net speed | spawns `sh -c "awk ... /proc/net/dev; cat operstate"` every 2 s + long-running `ip monitor link` | read `/proc/net/dev` + `/sys/class/net/<iface>/operstate`; deltas divided by **measured** elapsed time |
| Wi-Fi SSID/signal/radio | `nmcli` polls every 5–6 s | `nmcli` subprocess (unchanged protocol), same cadence |
| Disk I/O | `sh -c "awk ... /proc/diskstats"` every 2 s | read `/proc/diskstats` directly (sectors × 512) |
| Storage % | `sh -c "df / \| awk ..."` every 60 s | `statvfs("/")` via `rustix` |
| Temperatures | `sh -c "sensors \| grep -E 'Tctl\|edge' ..."` every 2 s | scan `/sys/class/hwmon/*/temp*_label` for `Tctl`/`edge` |
| Fans | `sh -c "sensors \| grep -i fanN ..."` every 3 s | `/sys/class/hwmon/*/fanN_input` |
| Battery | UPower over D-Bus (Qt service) | `/sys/class/power_supply/*` (`type=Battery`: `capacity`, `status`) |
| Workspaces/windows/layouts | long-running `niri msg -j event-stream` + repeated `niri msg -j windows` spawns | **one persistent JSON-lines client on `$NIRI_SOCKET`**; requests + event-stream on two connections, zero `niri` process spawns |
| Volume/mixer | Quickshell PipeWire service (in-process PipeWire client) | `wpctl status`/`get-volume` poll (2 s) + `wpctl set-volume/set-mute` on interaction |
| Updates | `pacman -Qu` every 40 min | identical subprocess + interval |
| Power profiles | `sh -c` script probing sysfs/TLP/powerprofilesctl | same probes, sysfs parts read natively, TLP/powerprofilesctl via subprocess |
| Session power | `loginctl suspend/reboot/poweroff` | identical |
| Tray | Quickshell SystemTray (StatusNotifierItem + DBusMenu over D-Bus) | own SNI watcher/host + item client over D-Bus (stage 7) |

---

## 2. Technology decisions

**Language: Rust** (repo name, rustc 1.97.1 + mold/sccache already configured).

| Choice | Selection | Why |
|---|---|---|
| Wayland protocol | `smithay-client-toolkit` (SCTK) + `wayland-client` | mature layer-shell + wl_seat/wl_pointer/wl_keyboard + wl_shm handling, pure Rust, no GTK/Qt |
| Rendering | **CPU rasterization into `wl_shm` ARGB8888 buffers** via `tiny-skia` | no GPU driver, no EGL/Vulkan loader, no pipeline caches; GPU stacks alone can eat the whole 100 MiB budget |
| Text | `cosmic-text` 0.19 (shaping, font discovery through `fontdb`, swash 0.2) | variable-font weight axes (Cinzel/EB Garamond real bold) and CFF ornament fonts (Bodoni Ornaments); shared `SwashCache`, rasterized on demand |
| Async/threads | **none** — single Wayland event loop + one poller thread per blocking source (niri socket, D-Bus), `calloop` for timers, channels for results | no tokio runtime, no per-task overhead |
| D-Bus (tray only) | `zbus` (blocking API) or `rustbus` fallback | only subsystem that truly needs D-Bus; everything else uses files or CLI |
| Config | compile-time defaults in `src/config.rs` / `src/theme.rs` (replicating `Config.qml`) | parsed once at startup; no runtime TOML yet |
| Build profile | `lto = "fat"`, `codegen-units = 1`, `panic = "abort"`, `strip = "symbols"`, `opt-level = 2` | smaller binary → fewer mapped pages |

### Why not …
- **GTK4 / gtk4-layer-shell:** base GTK4 + GL renderer typically exceeds the budget by itself.
- **wgpu:** driver + allocator + shader caches make <100 MiB fragile; a bar redraws small rects — software wins.
- **QML in C++ (rewriting against Qt directly):** still carries Qt scenegraph/QML engine RAM; misses the point.

### Memory budget (RSS, idle, all surfaces mapped)
| Component | Budget |
|---|---|
| Binary + Rust runtime + deps (mapped pages) | ~12 MiB |
| cosmic-text fontdb + glyph cache (bounded) | ~10 MiB |
| Bar buffer 65×2160×4 B ×2 (double buffer) | ~1.1 MiB |
| Background surface 1920×1080×4 B ×2 | ~16.6 MiB (only redrawn region touched; single-buffer + damage if needed) |
| Popup surface + catcher (allocated **only while open**) | ~8 MiB peak |
| Poller threads + channels + niri client | ~4 MiB |
| D-Bus/tray (stage 7) | ~8 MiB |
| **Total (steady state, no popup)** | **≈ 45–55 MiB** |

**Rules that keep it there:**
1. Popup buffers are freed on unmap; popups do not exist as hidden surfaces.
2. Glyph cache capped; no pre-rasterized atlases of whole fonts.
3. Redraw only on state change (dirty flags), not per frame; the 2 s tick repaints the changed strip only.
4. No long-lived subprocesses (the QML version keeps `ip monitor` + `niri event-stream` alive).
5. All polling output parsed with borrowed slices; no unbounded `String` growth, no log buffers.
6. Clock: one `calloop` timer per visible time surface (bar clock = minutes, desktop clock = minutes, calendar seconds only while open).

---

## 3. Crate layout

```
Cargo.toml
src/
  main.rs            # startup, event loop, probe mode (--probe prints sysinfo)
  config.rs          # Config.qml defaults + TOML overrides
  theme.rs           # colors, fonts, dimensions (parsed once)
  sysinfo.rs         # /proc + /sys samplers (pure parsers, unit-tested)   [M1]
  niri.rs            # $NIRI_SOCKET JSON client: workspaces/windows/layouts + event-stream [M3]
  audio.rs           # wpctl wrappers [M4]
  net.rs             # nmcli wrappers (scan/connect/radio/disconnect) [M4]
  updates.rs         # pacman -Qu parser + checker [M4]
  power.rs           # loginctl + power-profile detection/apply [M4]
  wayland/
    mod.rs           # SCTK setup, seat/pointer/keyboard dispatch
    surface.rs       # shm buffer pool, damage, commit
    bar.rs           # LeftBar equivalent (layout + input)
    desktop.rs       # DesktopWidgets equivalent
    popup.rs         # popup + click-catcher management (one at a time, like closeAllPopupsExcept)
  ui/
    draw.rs          # tiny-skia helpers: LED block meters, borders, vignettes, shadow
    text.rs          # cosmic-text wrapper + glyph cache
    widgets.rs       # per-widget paint/hit-test code
    popups/          # power, wifi, battery, updates, calendar, window menu, mixer
  tray.rs            # SNI host + DBusMenu (own thread; M6)
```

## 4. Milestones (each verified before moving on)

| # | Deliverable | Verification |
|---|---|---|
| **M1** | `sysinfo` engine + config + `--probe` CLI | `cargo test` green; probe prints live CPU/RAM/net/disk/temp/fan/battery |
| **M2** | Wayland core: bar + desktop surfaces, software renderer, text, clock, static LED meters | runs on live niri; RSS measured via `smaps_rollup` |
| **M3** | niri IPC (workspaces, window groups, layout indicator, window focus/actions, WindowMenu) | click through workspaces/windows; compare with `niri msg` |
| **M4** | live widgets + popups: power, wifi, battery+profiles, updates, calendar, mixer | every command path exercised; popup open/close, debounce, Esc |
| **M5** | RAM/CPU optimization pass: dirty-region redraw, buffer trimming, cache caps | idle RSS ≤ 60 MiB target, <100 MiB worst case incl. popup; 10 min soak log |
| **M6** | tray (SNI + DBusMenu) | icon renders, activate/menu work with real apps |
| **M7** | packaging: release profile, `README.md` install/run docs, niri `spawn-at-startup`, migration notes (no systemd/OpenRC service — user session program) | final measurements table vs Quickshell baseline |

### Measured progress

| Milestone | State | RSS / PSS | Threads | Evidence |
|---|---|---|---|---|
| Quickshell baseline | running | 294 556 kB / — | — | `/proc/<pid>/status` |
| M2 | done | 45 500 / 33 788 kB (1080p desktop) | 2 | bar + desktop widgets screenshot |
| M3 | done | 37 988 / 28 267 kB (nested, with live niri IPC) | 4 | workspace slots + TERM window group + EN badge; 17/17 tests |
| M4 (popup closed) | done | 39 028 / 30 806 kB | 5 | live niri nested, full poll stack running |
| M4 (Wi-Fi popup open) | done | 42 500 / 32 490 kB | 5 | overlay mapped, full network list rendered |
| M4 (all 7 popups) | done | ≤ 43 MB | 5 | power/wifi/battery/updates/calendar/mixer(screenshots) + WindowMenu tests |
| M6 (tray idle, 2 items) | done | 41 080 / 30 883 kB | 9 | Telegram + Steam medallions in the bar; nested niri screenshot |
| M6 (tray menu open) | done | 45 724 / 35 427 kB | 9 | live DBusMenu (Open Telegram / Disable notifications / Quit Telegram) rendered beside the medallion |
| M7 | done | 40 972 / 30 792 kB (nested idle) | 9 | `README.md`: build, fonts/deps, niri `spawn-at-startup`, migration; no init service |
| real 1920×1080, tray idle | done | 44 364 / 23 840 kB | 8 | permanent transparent overlay, direct shm slots, cosmic-text 0.19 |
| real 1920×1080, popup open | done | 68 620 / 44 600 kB | 9 | power popup above a tiled browser window |

Idle CPU after M4: 2–3 clock ticks per 5 s (~0.4–0.6 % of one core), including
2 s audio poll, 5 s Wi-Fi poll, 6 s radio poll and 40 min update check.
With the tray (1 s item-list poll, 5 s metadata refresh) idle CPU is
5 ticks / 10 s (~0.5 % of one core). 54 unit tests pass (+1 ignored live test).

### M6 tray implementation

- `src/tray.rs` runs on its own thread. It tries to own
  `org.kde.StatusNotifierWatcher` (fallback for bare sessions) but normally
  registers as `org.kde.StatusNotifierHost-<pid>` with the desktop's watcher
  (Quickshell in the reference setup).
- Item list is polled every second; titles/menu paths/icons are re-read every
  fifth tick, so idle D-Bus traffic stays low.
- SNI `IconPixmap (iiay)` big-endian ARGB32 is converted to premultiplied RGBA
  and downscaled to 16×16 once per icon (kept as `Arc<Pixmap>`); no icon theme
  dependency when a pixmap is supplied, PNG theme fallback otherwise.
- Right-click fetches the DBusMenu `GetLayout` tree and flattens it to rows
  (labels, enabled/visible, separators, check marks, submenu arrows); clicks
  send `com.canonical.dbusmenu.Event(id, "clicked", variant, 0)`.
- Bar: 26 px circular medallions (shadow + gold border, hover = red/brighter
  gold) between the update icon and the keyboard-layout badge; the block and
  its trailing separator collapse when the tray is empty, exactly like the QML
  `visible: trayRepeater.count > 0`.

## 5. Popup model on niri (design note)

The QML popups rely on exclusive keyboard grabs; **niri does not support
`keyboard_interactivity = exclusive` for layer surfaces**. Therefore:
- All popups share **one full-screen Top-layer surface** (exclusive zone 0).
  `Layer::Overlay` is not usable on niri 26.04 in practice: overlay surfaces are
  composited at the Background level (verified against a tiled window), and a
  surface that is unmapped via a null commit and remapped is **downgraded to
  Background**, which made every reopen invisible. Top is reliably above windows.
- The overlay is therefore **mapped permanently**. Closed state = fully
  transparent buffer + an empty `wl_input_region` (click-through) + keyboard
  interactivity None. Opening only restores input/keyboard and redraws; there is
  no unmap/remap cycle, no reopen configure handshake, and no hidden pixmap.
- Rendering goes **directly into wl_shm slots** with `Format::Abgr8888` (RGBA
  byte order). Two slots per surface are reused (`Slot::canvas`); SCTK's
  `create_buffer` would allocate a fresh slot every frame and push RSS to 91 MB.
- The overlay itself is the click-catcher (no separate surface): clicks outside the
  card dismiss; clicks inside the card's dead space do nothing (QML card MouseArea).
- `keyboard_interactivity = on_demand` while open; `Esc` closes. Wi-Fi password and
  Updates filter consume Esc/Enter/Backspace/text before the close path.
- One popup at a time (150 ms reopen debounce). The tray DBusMenu is also a
  popup (`Kind::TrayMenu`): a 220 px card anchored beside the medallion, rows
  dispatched as `com.canonical.dbusmenu.Event` on click, closed by Esc/outside.
- The overlay's origin sits at the bar's exclusive-zone edge, so anchored cards are
  drawn at overlay x=10 (screen x≈60) and the full-screen popups at x=10; Wi-Fi 280×420,
  Updates 300×430, Battery 280×260, Power 280×270, Calendar 280×410, Mixer 280×420,
  WindowMenu 190 wide at the pointer; 12/12 drop shadow, corner `z` ornaments,
  `t` Bodoni dividers.

## 6. Fidelity notes / intentional changes

- Colors, fonts, glyphs (Nerd Font PUA codepoints), LED block counts (24 desktop / 8 bar / 18 popup),
  roman numerals Ⅰ–Ⅹ, window-name shortening rules (`TERM`, `FFOX`, `WEB`, `TG`, `DSCD`, `FILE`, `CODE`),
  vertical app labels, `RU`/`EN` layout logic — all preserved verbatim from QML.
- CPU% uses `busy/total` from `/proc/stat` instead of `top`'s `us+sy` (more accurate, no subprocess).
- Net/disk rates divide by measured elapsed time instead of assuming exactly 2 s.
- `mixerCmd = "pwvucontrol"` is dead config in QML; the Rust mixer popup gets an explicit
  "open external mixer" button so the config value is honored.
- Battery via sysfs loses UPower's time-to-empty estimate (QML never displayed it) — no UI change.

### Fidelity fixes (post-M7)

- Text stack upgraded to cosmic-text 0.19 + swash 0.2: variable fonts now use
  their weight axes (real Cinzel/EB Garamond bold instead of a fake smear) and
  CFF fonts render, so the Bodoni Ornaments power rosette, separators and popup
  dividers are the exact QML glyphs. Roman numerals use EB Garamond (Cinzel
  Decorative has no U+2160; its fallback is visually matched).
- Mixer: master mute button, click-and-drag master meter, per-app slider drag
  with 35 ms throttling; per-stream commands use `pactl set-sink-input-*` with
  the sink-input index (previously passed to `wpctl` as a node id, so it was a
  no-op). Streams are re-polled every 2 s while the popup is open.
- Wi-Fi: saved networks connect through `nmcli con up id` / `nmcli dev wifi
  connect`, and success is taken from the command exit status like QML (not from
  a follow-up scan). Failures show nmcli's stderr in the status strip. The
  expanded password row mirrors QML (label, 26 px field, full-width CONNECT +
  58 px CANCEL), all button labels are centred, and the scan icon sits 6 px
  left of the ON/OFF switch.
- Bar layout: the updates block grows by 12 px when a count is visible, exactly
  like QML's content-height ColumnLayout, so the separator under the count no
  longer crosses the digits. Popup geometry: power at the top edge, calendar
  bottom-anchored 10 px above the screen, corners rotated 270/0/180/90.
- Mixer: master meter ends before the %/mute column, and mute sits clear of the
  track.
- Tray: SNI icon pixmaps are box-filtered down to 16×16 (antialiased instead
  of nearest-neighbour), and watcher entries whose bus name has no owner are
  filtered out and pruned from our watcher list (materialgram left a dead
  `:1.x` entry that showed as an empty medallion with no menu).
- Hit-testing audit: battery profile buttons were hit-tested 56 px below where
  they draw and the 3-step slider had no target (now click + drag); the power
  action row was 66 px off. Both now share the drawn coordinates and have
  regression tests.
- Hover/cursor: QML hover states are mirrored (power `)`→`*` + red + 1.08x,
  updates red, clock gold, tray medallions 1.08x) and the pointer uses
  `wp_cursor_shape_v1` (SCTK ThemedPointer, XCursor fallback) to show a hand
  over widgets, workspaces/windows, tray and popup controls.

## 7. Risks

| Risk | Mitigation |
|---|---|
| Tray DBusMenu is the largest single feature | isolated behind `tray/` module, delivered last; shell is fully usable without it |
| Ornament fonts missing on some systems | fontdb fallback chain, same family names as QML |
| wl_shm full-screen background at 4K ≈ 66 MiB for two buffers | single-buffered background + partial damage; bar/popups stay double-buffered |
| `wpctl` poll cost vs native PipeWire | 2 s interval, ~1 ms per spawn; revisit only if profiling says otherwise |
