# Ballad of Twilight Blossoms

> An elegant Victorian-Celtic rice with **Niri** as window manager and shell powered by **Quickshell**, inspired by antique manuscripts, pocket watch engraving, and some omarchy gothic rice.

Aesthetic inspired by *s.a.t.8 (Ballad of Twilight Blossoms)* by *nin (lion)*: warm parchment paper, deep walnut umber ink, antique brass, sunset gold, and rich velvet wine.

---
<img width="1920" height="1080" alt="image" src="https://github.com/user-attachments/assets/154666db-6209-45c9-9eae-a1432caed8e8" />
<img width="365" height="318" alt="image" src="https://github.com/user-attachments/assets/d8ff0bd9-d097-4b47-8ef2-b66150261d2e" />
<img width="356" height="329" alt="image" src="https://github.com/user-attachments/assets/463a8b6a-d76f-4647-9429-c09904079562" />
<img width="380" height="451" alt="image" src="https://github.com/user-attachments/assets/dd3e392f-eb69-4ba6-a26b-de2006d42163" />
<img width="380" height="451" alt="image" src="https://github.com/user-attachments/assets/005c905b-b738-4f85-ab75-1dc00da16f76" />

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
* **Engraved Terminations**: The bar is capped at the top by an filigree pediment (glyph `7` from *Victorian Free Ornaments*) and at the bottom by a tail vignette (glyph `A`).
* **Section Dividers**: Victorian brackets from *Bodoni Ornaments ITC* (glyph `\u00B8` / PUA `U+F0B8`).
* **Pocket Watch Workspaces (`WorkspacePanel.qml`)**:
  * Workspaces are roman numerals (Unicode range U+2160–U+2169) rendered in *Cinzel Decorative*.
* **System Tray Cameos (`TrayWidget.qml`)**:
  * Tray icons are encased in circular walnut-umber medallions (`#3a2a20`) with thin brass borders. This guarantees that white system tray icons retain maximum contrast and clarity on a light parchment screen.
  * On hover, the medallion smoothly shifts into velvet wine while the border glows golden.

### 3. Contextual Popups
Calendar, Wi-Fi, Audio Mixer, Power, and Pacman update popups:
* **Filigree Corner Brackets**: Every popup card features ornate corner flourishes (glyph `z` from *Victorian Free Ornaments*), symmetrically rotated at 0°, 90°, 180°, and 270°.
* **Wi-Fi Popup (`WifiPopup.qml`)**: Communicates directly with `nmcli` for network discovery and connection state. Includes a parchment password field with a toggleable eye icon and *Cinzel* action buttons.
* **Audio Mixer (`MixerPopup.qml`)**: Volume sliders with brass tick divisions controlling streams through `wpctl` (default pink Qt sliders are completely removed).
* **Updates Popup (`UpdatesPopup.qml`)**: Monitors pending Pacman packages, listing versions (*EB Garamond*, old version in umber ➔ new in gold) with a one-click terminal upgrade button.
* **Calendar (`CalendarPopup.qml`)**: Arabic numerals for dates and time for effortless readability, weekdays in *Cinzel*, dates in *EB Garamond*, with the current day highlighted in gold.

### 4. Desktop Telemetry Widgets (`DesktopWidgets.qml`)
* **Engraved Clock**: 76px time display in *Cinzel* with a drop-shadow intaglio effect rendered via the `MultiEffect` shader (QtQuick.Effects).
* **Calligraphic Date**: Script date rendered in velvet wine *Berkshire Swash*.
* **System Instrument Gauges**: Telemetry meters (CPU, GPU, RAM, VRAM, disks, temperatures, network throughput) feature 24-step brass gauge tick marks with etched depth cues and circular cameo icons.

### 5. Foot Terminal & Emergency Fastfetch
* **Calibrated Light Theme (`foot.ini`)**: All 16 ANSI colors are balanced to ensure WCAG AA contrast (> 5:1) against `#f1e5ce`.
* **Subtle Ghost Autosuggestions**: ANSI color `bright0` is mapped to pale antique sepia (`#a29081`), providing a soft, unobtrusive preview for `zsh-autosuggestions` that never blends into typed text.
* **Emergency Fastfetch button**:
  * Spawning terminal via `Super+Shift+F23`(Copilot button) executes `fastfetch --raw ~/.config/fastfetch/logo.sixel`.

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
