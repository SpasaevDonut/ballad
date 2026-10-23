# Ballad of Twilight Blossoms

> An elegant Victorian-Celtic rice with **Niri** as window manager and shell powered by **Quickshell**, inspired by antique manuscripts, pocket watch engraving, and some omarchy gothic rice.

Aesthetic inspired by *s.a.t.8 (Ballad of Twilight Blossoms)* by *nin (lion)*: warm parchment paper, deep walnut umber ink, antique brass, sunset gold, and rich velvet wine.

---

## Color Palette

| Token | Hex | Role in Interface |
| :--- | :--- | :--- |
| `bg0` | `#f1e5ce` | Light warm parchment (window background, terminal paper, base bar) |
| `bg1` | `#e8d2b4` | Dense parchment (popups, cards, sidebars, Niri overview backdrop) |
| `bg2` | `#d5c4a1` | Parchment borders, dividers, inactive gauge tick marks |
| `fg` | `#3a2a20` | Dark walnut umber (primary text, icons, borders) |
| `fg-muted` | `#7d6a5c` | Muted stone umber for secondary labels and hints |
| `gold` | `#b5893f` | Antique brass & sunset gold (borders, focus indicators, active gauge ticks) |
| `goldBright` | `#d5a86d` | Bright gold (highlights, active workspace cartouche) |
| `red` | `#7a362d` | Velvet wine (selections, hover states, accent headers, power button) |
| `ghost` | `#a29081` | Faded antique sepia (ghost text for shell autosuggestions) |

---

## Software

| Software | Piece | What it is |
| :--- | :--- | :--- |
| **Window Manager** | `niri` | Wayland compositor |
| **Status Bar & UI** | `quickshell` | Status bar, popups, and desktop telemetry widgets (Qt6 / QML) |
| **Application Launcher** | `rofi` (`rofi-wayland`) | App launcher |
| **Terminal Emulator** | `foot` | Wayland terminal |
| **System Fetch** | `fastfetch` | Fastfetch |
| **Wallpaper** | `swaybg` | Wallpaper daemon |
| **Audio Server** | `pipewire` + `wireplumber` | Audio engine managed via `wpctl` |
| **Network Backend** | `NetworkManager` (`nmcli`) | Wi-Fi manager |
| **File Manager** | `pcmanfm` | file manager |
| **GTK Theming** | `adw-gtk3` + `gtk.css` | Theme for GTK 3 and GTK 4 applications |
| **Qt Theming** | `qt6ct` | Styling and palette config for Qt6 apps |
| **Dotfiles Manager** | `GNU Stow` | Symlink farm manager for dotfiles deployment |

---

## Dotfile Specifics & Widget Mechanics

### 1. Quickshell Architecture & Wayland Layer Shell
The user interface is built on **Quickshell** (Qt 6 / QML) and structured across two independent Wayland Layer Shell planes:
* **`LeftBar.qml`**: Attached to `WlrLayer.Top` along the left screen edge with an exclusive screen reserve (`exclusiveZone: 46`).
* **`DesktopWidgets.qml`**: Placed on `WlrLayer.Background` (under all windows) offset to the right of the status bar.

### 2. Left Status Bar (`LeftBar.qml`)
* **Engraved Terminations**: The bar is capped at the top by an arched filigree pediment (glyph `7` from *Victorian Free Ornaments*) and at the bottom by a tail vignette (glyph `A`).
* **Section Dividers**: Ornate Victorian curly brackets from *Bodoni Ornaments ITC* (glyph `\u00B8` / PUA `U+F0B8`) rendered in antique gold.
* **Pocket Watch Workspaces (`WorkspacePanel.qml`)**:
  * Workspaces are represented by monolithic Roman numerals `Ⅰ` through `Ⅴ` (Unicode range U+2160–U+2169) rendered in *Cinzel Decorative*.
  * The active workspace is framed by a glowing `goldBright` cartouche; workspaces containing open windows feature a dark umber imprint with a tiny wine dot underneath; empty workspaces remain muted parchment. Open window markers appear as petite golden diamonds.
* **Power Button (`PowerButton.qml`)**:
  * In its idle state, it displays an 8-petal brass rosette.
  * On hover, the rosette blossoms into a lush 12-petal flower of deep velvet wine with a smooth scale-up animation.
* **System Tray Cameos (`TrayWidget.qml`)**:
  * Tray icons are encased in circular walnut-umber medallions (`#3a2a20`) with thin brass borders. This guarantees that white system tray icons retain maximum contrast and clarity on a light parchment screen.
  * On hover, the medallion smoothly shifts into velvet wine while the border glows golden.

### 3. Contextual Popups
Calendar, Wi-Fi, Audio Mixer, Power, and Pacman update popups share a unified card aesthetic:
* **Filigree Corner Brackets**: Every popup card features ornate corner flourishes (glyph `z` from *Victorian Free Ornaments*), symmetrically rotated at 0°, 90°, 180°, and 270°.
* **Wi-Fi Popup (`WifiPopup.qml`)**: Communicates directly with `nmcli` for network discovery and connection state. Includes a parchment password field with a toggleable eye icon and *Cinzel* action buttons.
* **Audio Mixer (`MixerPopup.qml`)**: Fully custom volume sliders with brass tick divisions controlling streams through `wpctl` (default pink Qt sliders are completely removed).
* **Updates Popup (`UpdatesPopup.qml`)**: Monitors pending Pacman packages, listing versions (*EB Garamond*, old version in umber ➔ new in gold) with a one-click terminal upgrade button.
* **Calendar (`CalendarPopup.qml`)**: Arabic numerals for dates and time for effortless readability, weekdays in *Cinzel*, dates in *EB Garamond*, with the current day highlighted in gold.

### 4. Desktop Telemetry Widgets (`DesktopWidgets.qml`)
* **Engraved Clock**: 76px time display in *Cinzel* with a drop-shadow intaglio effect rendered via the `MultiEffect` shader (QtQuick.Effects).
* **Calligraphic Date**: Script date rendered in velvet wine *Berkshire Swash*.
* **System Instrument Gauges**: Telemetry meters (CPU, GPU, RAM, VRAM, disks, temperatures, network throughput) feature 24-step brass gauge tick marks with etched depth cues and circular cameo icons.

### 5. Foot Terminal & Sixel Fastfetch
* **Calibrated Light Theme (`foot.ini`)**: All 16 ANSI colors are balanced to ensure WCAG AA contrast (> 5:1) against `#f1e5ce`.
* **Subtle Ghost Autosuggestions**: ANSI color `bright0` is mapped to pale antique sepia (`#a29081`), providing a soft, unobtrusive preview for `zsh-autosuggestions` that never blends into typed text.
* **Zero-Latency Fastfetch**:
  * Spawning terminal via `Super+Shift+F23` executes `fastfetch --raw ~/.config/fastfetch/logo.sixel`.
  * The `logo.sixel` binary is pre-rendered from the transparent S.A.T.8 chibi (`sat8drag.png`) using Sixel transparency codes (`?`), delivering instant 0ms launches without ImageMagick runtime lag or rectangular border artifacts.

### 6. Rofi Application Menu (`config.rasi`)
* The launcher uses a clean, rectangular flat design with zero outer borders and zero border radius.
* Prompt and active selection are styled in velvet wine `#7a362d` with high-contrast parchment text `#f1e5ce`.
* `element-text` and `element-icon` inherit transparency cleanly.

---

## Required Fonts
Ensure the following fonts are installed for correct rendering of typography, glyphs, and ornamental vignettes:
1. **[Cinzel](https://fonts.google.com/specimen/Cinzel)** — Headers, labels, status text, and popup action buttons.
2. **[Cinzel Decorative](https://fonts.google.com/specimen/Cinzel+Decorative)** — Roman numeral glyphs on the pocket watch workspace dial.
3. **[EB Garamond](https://fonts.google.com/specimen/EB+Garamond)** — Primary text, calendar numerals, telemetry values, package lists (complete Cyrillic support).
4. **[Berkshire Swash](https://fonts.google.com/specimen/Berkshire+Swash)** — Calligraphic Victorian script for the desktop date display.
5. **[JetBrainsMono Nerd Font](https://www.nerdfonts.com/)** — Terminal font and Nerd Font system icons.
6. **Bodoni Ornaments ITC** — Ornamental bracket dividers (`\u00B8`), fleur-de-lis (`t`), and editorial flourishes.
7. **Victorian Free Ornaments** — Arched pediment (`7`), tail vignette (`A`), corner filigree (`z`), and power button rosettes (`)`, `*`).
---
## Installation & Management with GNU Stow
Deploy configuration files to `$HOME` using `stow`:
```bash
cd ~/.dotfiles
stow .
```
*The `.stow-local-ignore` file automatically prevents linking of git metadata, backups, and documentation into `$HOME`.*
