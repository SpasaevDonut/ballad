//@ pragma UseQApplication
//@ pragma Env QT_SCALE_FACTOR=1
//@ pragma Env QT_WAYLAND_FORCE_DPI=96
//@ pragma Env QT_AUTO_SCREEN_SCALE_FACTOR=0
import QtQuick
import Quickshell

ShellRoot {
    Config { id: cfg }
    SysData { id: sysData }
    LeftBar { sysData: sysData; cfg: cfg }
    DesktopWidgets { sysData: sysData; cfg: cfg }
}
