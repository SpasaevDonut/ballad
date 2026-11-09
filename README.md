# Ballad of Twilight Blossoms

> An elegant Victorian-Celtic rice with **Niri** as window manager and shell powered by **Rust**, inspired by antique manuscripts, pocket watch engraving, and some omarchy gothic rice.

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

## Required Fonts
1. **[Cinzel](https://fonts.google.com/specimen/Cinzel)** — Headers, labels, status text, and popup action buttons.
2. **[Cinzel Decorative](https://fonts.google.com/specimen/Cinzel+Decorative)** — Roman numeral glyphs on the pocket watch workspace dial.
3. **[EB Garamond](https://fonts.google.com/specimen/EB+Garamond)** — Primary text, calendar numerals, telemetry values, package lists (complete Cyrillic support).
4. **[Berkshire Swash](https://fonts.google.com/specimen/Berkshire+Swash)** — Calligraphic Victorian script for the desktop date display.
5. **[JetBrainsMono Nerd Font](https://www.nerdfonts.com/)** — Terminal font and Nerd Font system icons.
6. **Bodoni Ornaments ITC** — Ornamental bracket dividers (`\u00B8`), fleur-de-lis (`t`), and editorial flourishes.
7. **Victorian Free Ornaments** — Arched pediment (`7`), tail vignette (`A`), corner filigree (`z`), and power button rosettes (`)`, `*`).
---
## Installation with Stow
Deploy configuration files to `$HOME` using `stow`:
```bash
cd ~/.dotfiles
stow .
```
