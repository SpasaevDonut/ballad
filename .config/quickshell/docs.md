# Quickshell Docs v0.3.0

> A toolkit for building status bars, widgets, lockscreens, and other desktop components using QtQuick.

---

## Table of Contents

- [Installation & Setup](#installation--setup)
- [Introduction](#introduction)
- [QML Language](#qml-language)
- [FAQ](#faq)
- [Advanced Options](#advanced-options)
- [Type Definitions](#type-definitions)

---

## Installation & Setup

> NOTE: Quickshell is still in a somewhat early stage of development. There will be breaking changes before 1.0, however a migration guide will be provided.

### Prerequisites

You may want to install additional packages (names vary by distro):

- `qtsvg`: support for SVG image loading (bundled with most packages)
- `qtimageformats`: support for WEBP images and some less common ones
- `qtmultimedia`: support for playing videos, audio, etc
- `qt5compat`: extra visual effects, notably gaussian blur. MultiEffect is usually preferable.

### Nix

```nix
{
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    quickshell = {
      url = "git+https://git.outfoxxed.me/outfoxxed/quickshell";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
}
```

Package: `quickshell.packages.default`

### Arch

```bash
pacman -S quickshell
# or master branch:
yay -S quickshell-git
```

> WARNING: When using an AUR package, Quickshell may break whenever Qt is updated. Quickshell will attempt to warn you if it detects a breakage. If warned, please reinstall the package.

### Fedora

```bash
sudo dnf install quickshell
```

Or from COPR:

```bash
sudo dnf copr enable errornointernet/quickshell
sudo dnf install quickshell
# or
sudo dnf install quickshell-git
```

### Debian

```bash
sudo apt install quickshell
```

### OpenSUSE

```bash
sudo zypper install quickshell
```

### Manual Build

```bash
git clone https://git.outfoxxed.me/outfoxxed/quickshell.git
cd quickshell
ninja -C build
```

---

## Introduction

### Config Files

Quickshell searches the `quickshell` subfolder of every XDG standard config path for configs. Usually, this is `~/.config/quickshell`.

Each named subfolder containing a `shell.qml` file is considered to be a config. If the base `quickshell` folder contains a `shell.qml` file, subfolders will not be considered.

A specific configuration can be picked using the `--config` or `-c` argument to Quickshell.

Configs located at other paths outside XDG standard, including raw qml files, can be run with `--path` or `-p`.

> CAUTION: Many users use root imports, in the form `import "root:/path/to/module"`. These are an old Quickshell feature that will break the LSP and singletons. Keep that in mind if you decide to use them. A replacement without these issues is planned.

### Creating Windows

Quickshell has two main window types:

- `PanelWindow` for bars, widgets, and overlays
- `FloatingWindow` for standard desktop windows

**Simple centered bar:**

```qml
import Quickshell
import QtQuick

PanelWindow {
  anchors {
    top: true
    left: true
    right: true
  }
  implicitHeight: 30

  Text {
    anchors.centerIn: parent
    text: "hello world"
  }
}
```

### Running a Process

```qml
import Quickshell
import Quickshell.Io
import QtQuick

PanelWindow {
  anchors { top: true; left: true; right: true }
  implicitHeight: 30

  Text {
    id: clock
    anchors.centerIn: parent

    Process {
      command: ["date"]
      run

      onStdioFinished: {
        clock.text = collector.text
      }
    }
  }
}
```

### Running code at an interval

See `IntervalTimer` in the type definitions.

### Reusable components

Use `Component` or `Loader` for reusable components.

### Multiple files

Quickshell supports multi-file configurations. See the type definitions for `Component` and `Loader`.

### Singletons

See `Quickshell` singleton in the type definitions.

---

## QML Language

### Structure

```qml
import QtQuick 6.0
import "myjs.js" as MyJs

Item {
  id: root
  property int myProp: 5;
  width: 100
  height: width
  prop: { 5 }
  objProp: Object { ... }
  AnotherObject { ... }
  signal foo(bar: int)
  onSignal: console.log("received signal!")
  onWidthChanged: console.log(`width is now ${width}!`)
  onOtherSignal: {
    console.log("received other signal!");
    console.log(`5 * 2 is ${dub(5)}`);
  }
  Component.onCompleted: MyJs.myfunction()
  function dub(x: int): int { return x * 2 }
  component MyComponent: Object { ... }
}
```

### Imports

**Module import:**

```qml
import QtQuick 6.0
```

- `Module`: name of the module (e.g., `QtQuick`)
- `Major.Minor`: version of the module
- `Namespace`: optional namespace

**Relative import:**

```qml
import "<directory>" [as <Namespace>]
```

**Quickshell module import (new in v0.2.0):**

```qml
import qs.<path> [as <Namespace>]
```

- `path`: path relative to the folder `shell.qml` is in
- `qs`: can be used to import the shell root folder
- Dotted paths access nested subfolders, e.g. `qs.foo.bar`

### Comments

- Single-line: `/* ... */`
- Multi-line: `/* ... */`
- C++ comments (`//`) are allowed but be consistent within a file
- Copyright headers should use `/* ... */` with `SPDX-License-Identifier`

### Semicolons

Semicolons are permitted basically everywhere and recommended in functions and expressions.

### Types

While types can often be elided, use them where possible to catch problems early.

---

## FAQ

### What will breaking changes look like?

Quickshell will have breaking changes in future releases spanning APIs and best practices, but not language syntax or Qt-exposed APIs. Migration guides will be provided between release versions.

### Should I use a process per widget?

No. Using a process per widget uses significantly more memory than a single process.

### How do I make a rounded window?

```qml
PanelWindow {
  color: "transparent"
  Rectangle {
    anchors.fill: parent
    radius: 5
    color: "white"
  }
}
```

### How do I make a list of widgets?

- Short list (e.g., active music players): `Repeater` + `RowLayout`/`ColumnLayout`
- Long list (e.g., app launcher): `ListView`

### How do I run a program or script?

- Entire output as a single string: use `StdioCollector`
- Streaming data: use `SplitParser`

### How do I show widgets conditionally?

- Use `Item.visible` property, or
- Use a `Loader`

Use `Loader` if you want to unload a widget tree to save memory or speed up load times. You can also change a `Loader`'s component conditionally:

```qml
Loader {
  readonly property Component thing1: ...
  readonly property Component thing2: ...
  sourceComponent: condition ? thing1 : thing2
}
```

### How do I round an image?

```qml
ClippingWrapperRectangle {
  radius: 10
  IconImage {
    source: ...
    implicitSize: ...
  }
}
```

### How do I reference images and resources?

Paths passed to components like `Image` or `FileView` as strings are relative to Quickshell's working directory.

---

## Advanced Options

### Pragmas

Quickshell pragmas are comments in the form `//@ ...`.

#### Internal

```qml
//@ pragma Internal
```

Prevents the QML file from being exposed outside its module.

#### If/Endif

```qml
//@ if <condition>
//@ endif
```

C-preprocessor-like blocks whose content disappears if the condition is false. Condition is evaluated as a JavaScript expression.

Available functions:

- `hasVersion(major, minor, features): bool` — checks for a given Quickshell version and unreleased features
- `hasQtVersion(major, minor): bool` — checks for a given Qt version
- `env(name): string` — returns an environment variable value
- `isEnvSet(name): bool` — returns if an environment variable is set

#### Instance Pragmas

Affect properties of the whole Quickshell instance. Place at the top of the root `shell.qml` file.

##### Environment

```qml
//@ pragma Env VAR = VAL
//@ pragma DefaultEnv VAR = VAL
```

Set Qt/Quickshell environment variables. Useful for changing Qt scaling, theming, etc.

##### UseQApplication

```qml
//@ pragma UseQApplication
```

Switch from `QGuiApplication` to `QApplication`, enabling QtWidgets controls (needed for things like `qqc2-desktop-style`).

##### NativeTextRendering

```qml
//@ pragma UseQApplication
```

Uses the system text rendering backend over Qt's internal scene backend. Equivalent to setting `Text.renderType` to `Text.NativeRendering` for all text objects.

##### IgnoreSystemSettings

```qml
//@ pragma UseQApplication
```

Entirely ignore system settings relating to application style. Usually should not be set.

##### RespectSystemStyle

```qml
//@ pragma RespectSystemStyle
```

Prevent Quickshell from setting the QtQuick Controls style to Fusion (which avoids inconsistencies between systems).

##### DropExpensiveFonts

```qml
//@ pragma DropExpensiveFonts
```

Exclude all `woff` and `woff2` fonts from Qt's font fallback, as font loading can be expensive and cause stutters.

---

## Type Definitions

See the [Type Definitions](https://quickshell.org/docs/v0.3.0/types) page for full type reference.

Key types include:

- `Quickshell` — singleton providing `cachePath()`, `dataPath()`, `iconPath()`, `shellRoot`, `screens`, etc.
- `PanelWindow` — main window type for bars, widgets, overlays
- `FloatingWindow` — standard desktop windows
- `Process` — run shell commands
- `StdioCollector` — collect process stdio output
- `SplitParser` — stream process output
- `IntervalTimer` — run code at intervals
- `Loader` — load components dynamically
- `ClippingWrapperRectangle` — round images
- `Repeater` — repeat items
- `ListView` — scrollable lists
- And many more QtQuick types.

---

## Links

- [Usage Guide](https://quickshell.org/docs/v0.3.0/guide)
- [Type Definitions](https://quickshell.org/docs/v0.3.0/types)
- [Official Examples](https://github.com/Quickshell-Mirror/quickshell-examples)
- [GitHub Repo](https://git.outfoxxed.me/outfoxxed/quickshell)
- [Matrix Space / Discord](https://quickshell.org/)
