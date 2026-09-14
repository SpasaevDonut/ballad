# AGENTS.md — Quickshell Desktop Configuration (for Niri Compositor)

This document is intended for AI agents and developers working on this codebase. It describes the architecture, dependencies, code conventions, core subsystems, and guidelines for making changes.

---

## 1. Project Overview

The project is a custom Wayland shell / status bar and desktop widget set developed in **[Quickshell](https://quickshell.outfoxxed.me/)** (a Qt/QML-based framework) and optimized for the **[Niri](https://github.com/YaLTeR/niri)** tiling Wayland compositor.

### Key Features
* **Left Vertical Bar (`LeftBar.qml`)**:
  * Power button with quick access to session actions (`PowerButton.qml` -> `PowerPopup.qml`).
  * Niri workspace switcher/indicator + list of open windows with grouping and context window management menu (`WorkspacePanel.qml` -> `WindowMenu.qml`).
  * Network speed monitor with interactive Wi-Fi manager popup (`NetworkWidget.qml` -> `WifiPopup.qml`).
  * PipeWire volume control and mixer popup (`VolumeWidget.qml` -> `MixerPopup.qml`).
  * Battery charge level and power profile manager popup (`BatteryWidget.qml` -> `BatteryPopup.qml`).
  * Pacman package update monitor and upgrade popup (`UpdatesWidget.qml` -> `UpdatesPopup.qml`).
  * System tray with DBus / StatusNotifierItem support (`TrayWidget.qml`).
  * Current keyboard layout indicator (`LayoutWidget.qml`, Niri IPC events).
  * Vertical clock with calendar popup (`ClockWidget.qml` -> `CalendarPopup.qml`).
* **Desktop Widgets (`DesktopWidgets.qml`)**:
  * Wayland background layer (`WlrLayer.Background`).
  * Large digital clock and date display.
  * Discrete block/LED system monitors: CPU, RAM, Network I/O, Disk I/O, Disk Usage %, Temperature sensors (Temp1, Temp2), Fan speed monitors.
* **Centralized Metric Aggregator (`SysData.qml`)**:
  * Periodic polling of system utilities and `/proc`, `/sys` files.
  * Centralized state distribution to the status bar and desktop widgets.
  * Pacman available updates detection and listing (`pacman -Qu`).
* **Configuration (`Config.qml`)**:
  * Color palette, dimensions, interface/sensor names, terminal and mixer commands.

---

## 2. File Structure and Architecture

```
~/.config/quickshell/
├── shell.qml                     # Root entry point (ShellRoot, pragmas and component initialization)
├── Config.qml                    # Central configuration: theme, colors, network/disk interfaces, tools
├── SysData.qml                   # System metric aggregator (CPU, RAM, Disk, Temp, Fans, Network, Battery, Updates)
├── LeftBar.qml                   # Main status bar window (PanelWindow, WlrLayershell)
├── DesktopWidgets.qml            # Desktop background widgets (PanelWindow, WlrLayer.Background)
├── BatteryPopup.qml              # Battery and power profiles popup (TLP / powerprofilesctl)
├── CalendarPopup.qml             # Calendar popup window
├── PowerPopup.qml                # Power management popup (Suspend, Reboot, Poweroff)
├── UpdatesPopup.qml              # Pacman available updates and upgrade popup
├── WifiPopup.qml                 # Wi-Fi network management popup (nmcli)
├── WindowMenu.qml                # Niri window management context menu (close, fullscreen, move to workspace)
├── LeftBarWidgets/               # Left status bar components
│   ├── BatteryWidget.qml         # Battery widget (UPower)
│   ├── ClockWidget.qml           # Clock widget
│   ├── LayoutWidget.qml          # Niri keyboard layout indicator
│   ├── MixerPopup.qml            # PipeWire audio mixer popup
│   ├── NetworkWidget.qml         # Network speed monitor widget
│   ├── PollingData.qml           # Helper component for periodic CLI command execution
│   ├── PowerButton.qml           # Power button widget
│   ├── Separator.qml             # Vertical separator
│   ├── TrayWidget.qml            # System tray widget (Quickshell.Services.SystemTray)
│   ├── UpdatesWidget.qml         # Pacman update indicator widget
│   ├── VolumeWidget.qml          # Audio volume widget (Quickshell.Services.Pipewire)
│   └── WorkspacePanel.qml        # Workspaces and window list (Niri IPC)
├── nucolors                      # Hex color reference file
└── *.svg                         # SVG icons (cpu, defy, down, fan, storage, thermometer, traffic, transmission, up)
```

---

## 3. Tech Stack and Dependencies

### 3.1. QML & Quickshell Modules
* `QtQuick`, `QtQuick.Layouts`, `QtQuick.Controls`, `Qt5Compat.GraphicalEffects`
* `Quickshell`
* `Quickshell.Io` (`Process`, `SplitParser`)
* `Quickshell.Wayland` (`PanelWindow`, `PopupWindow`, `WlrLayershell`, `WlrLayer`, `QsMenuAnchor`)
* `Quickshell.Services.Pipewire` (`Pipewire`, `PwObjectTracker`)
* `Quickshell.Services.UPower` (`UPower`, `UPowerDeviceState`)
* `Quickshell.Services.SystemTray` (`SystemTray`)
* `Quickshell.Widgets` (`IconImage`)

### 3.2. External CLI Tools
* **Niri**: `niri msg -j workspaces`, `niri msg -j event-stream`, `niri msg -j windows`, `niri msg -j keyboard-layouts`, `niri msg action ...`
* **Network**: `ip monitor link`, `/proc/net/dev`, `nmcli` (Wi-Fi network management, status, and scanning)
* **Disk & Resources**: `/proc/diskstats`, `top`, `free`, `df`, `lm_sensors` (`sensors`)
* **Session**: `loginctl` (`poweroff`, `reboot`, `suspend`)
* **Audio**: `PipeWire`, `pwvucontrol` (external mixer)
* **Terminal**: `kitty` (or configured in `Config.qml`)

### 3.3. Fonts and Typography
* Default fonts: `AurulentSansMNerdFontPropo` and `monospace`.
* Glyphs: Nerd Font icons and standard Unicode symbols.

---

## 4. Design and Style Conventions

1. **Color Scheme**:
   All colors should originate from `Config.qml` (or passed via `popupCfg` / `cfg`):
   * `bgColor` (`#121217`): Main dark panel and popup background.
   * `popupBgColor` (`#202029`): Context menu background.
   * `accentColor` (`#C16FB8`): Primary accent color (magenta / pink).
   * `textColor` (`#A3A0AF`): Primary light text.
   * `inactiveColor` (`#5e5e65`): Inactive elements, grey meter blocks.
   * `existingColor` (`#939399`): Secondary elements.
   * `sepColor` / `sepLightColor` (`#2F2F38` / `#505050`): Separators.

2. **LED/Block Meter Aesthetics**:
   * Use `Repeater` with small `Rectangle` elements for discrete load meters (CPU, RAM, Temp, Fans, Storage, Battery, Network).
   * Discrete block filling instead of solid continuous progress bars.
   * Vertical orientation for left bar widgets: compact (default width 46px), vertical text formatting where appropriate (e.g. `appName.split('').join('\n')`).

3. **Popups (`PopupWindow`)**:
   * Use `grabFocus: true` or parent `MouseArea` to dismiss popups on outside click.
   * Debounce rapid clicks: check `Date.now() - popup.lastCloseTime < 150`.
   * Drop shadow using `Qt5Compat.GraphicalEffects.DropShadow` with 15px offset and `#A6000000` color.

---

## 5. IPC and Process Conventions (`Quickshell.Io.Process`)

1. **Process Cleanup**:
   * For long-running monitoring processes (`niri event-stream` or `ip monitor`), always declare `Component.onDestruction: running = false` to prevent zombie processes.
   * For periodic polling commands, use `PollingData.qml` or timer-triggered restart (`process.running = false; process.running = true;`).

2. **Safe Output Parsing (SplitParser & JSON)**:
   * Always wrap `JSON.parse` in `try { ... } catch (e) { ... }` inside `onRead` handlers to prevent QML runtime crashes on partial data:
     ```qml
     stdout: SplitParser {
         onRead: data => {
             try {
                 let obj = JSON.parse(data);
                 // handle data
             } catch(e) {
                 console.warn("Parse error:", e);
             }
         }
     }
     ```

3. **Locale & Decimal Formatting**:
   * Handle decimal separators (dot vs comma) when reading CLI output (e.g. `top` or `free`): apply `.replace(',', '.')` before `parseFloat()`.

---

## 6. Niri Compositor Integration

* **Events**: `WorkspacePanel.qml` listens to `niri msg -j event-stream`.
  * `KeyboardLayoutsChanged` / `KeyboardLayoutSwitched` -> forwarded to `LayoutWidget.qml`.
  * `WorkspacesChanged` / `WorkspaceActivated` -> updates workspace array (fixed 10 slots).
  * `WindowOpened` / `WindowClosed` / `WindowFocusChanged` / `WorkspaceActiveWindowChanged` -> triggers window query (`niri msg -j windows`).
* **Actions**:
  * Workspace switch: `niri msg action focus-workspace <name|id>`
  * Focus window: `niri msg action focus-window --id <id>`
  * Window management via `WindowMenu.qml`: `close-window`, `fullscreen-window`, `toggle-window-floating`, `set-window-urgent`, `move-window-to-workspace`.

---

## 7. Extension and Modification Guide

### Adding a new widget to the left bar (`LeftBar.qml`)
1. Create the widget file in `LeftBarWidgets/YourWidget.qml`.
2. Accept required color properties (`accentColor`, `textColor`, etc.) and `barWidth`.
3. Include the widget in `LeftBar.qml` within the `ColumnLayout` with a `Separator {}`.
4. If the widget requires new system metrics, add collection to `SysData.qml` and pass via `sysData`.

### Adding a new metric to `SysData.qml`
1. Add property in `SysData.qml`: `property <type> myMetric: <default>`.
2. Add a `PollingData {}` instance with `interval: ...` and shell command.
3. Parse the value in `onDataReady: data => { ... }` and assign to the property.
4. Add configuration variables to `Config.qml` if needed.

---

## 8. Running and Debugging

* Run Quickshell with current configuration:
  ```bash
  quickshell -p shell.qml
  ```
  or from root configuration directory:
  ```bash
  quickshell
  ```
* For debugging output, use `console.log(...)` in QML and check the quickshell process stdout/stderr.
