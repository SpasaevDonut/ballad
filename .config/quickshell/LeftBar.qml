//@ pragma UseQApplication
import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import "./LeftBarWidgets"

PanelWindow {
    id: leftBar

    property QtObject sysData: null
    property QtObject cfg: null

    anchors {
        top: true
        bottom: true
        left: true
    }

    implicitWidth: (cfg?.barWidth ?? 46) + 15
    color: "transparent"
    exclusiveZone: cfg?.barWidth ?? 46

    Rectangle {
        id: mainBar
        width: cfg?.barWidth ?? 46
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        anchors.left: parent.left
        color: cfg?.bgColor ?? "#121217"

        ColumnLayout {
            anchors.fill: parent
            anchors.topMargin: 8
            anchors.bottomMargin: 8
            spacing: 0

            PowerButton {
                id: archLogoContainer
                accentColor: cfg?.accentColor ?? "#C16FB8"
                existingColor: cfg?.existingColor ?? "#939399"
                powerPopup: powerPopup
            }

            Separator {
                sepColor: cfg?.sepColor ?? "#2F2F38"
                sepLightColor: cfg?.sepLightColor ?? "#505050"
                barWidth: cfg?.barWidth ?? 46
            }

            WorkspacePanel {
                id: workspacePanel
                accentColor: cfg?.accentColor ?? "#C16FB8"
                inactiveColor: cfg?.inactiveColor ?? "#5e5e65"
                existingColor: cfg?.existingColor ?? "#939399"
                textColor: cfg?.textColor ?? "#A3A0AF"
                sepColor: cfg?.sepColor ?? "#2F2F38"
                sepLightColor: cfg?.sepLightColor ?? "#505050"
                barWidth: cfg?.barWidth ?? 46
		winMenu: winMenu
            }

            Separator {
                sepColor: cfg?.sepColor ?? "#2F2F38"
                sepLightColor: cfg?.sepLightColor ?? "#505050"
                barWidth: cfg?.barWidth ?? 46
            }

            NetworkWidget {
                id: netWidgetContainer
                accentColor: cfg?.accentColor ?? "#C16FB8"
                inactiveColor: cfg?.inactiveColor ?? "#5e5e65"
                textColor: cfg?.textColor ?? "#A3A0AF"
                barWidth: cfg?.barWidth ?? 46
                netIcon: sysData?.netIcon ?? "󰤭"
                isConnected: sysData?.isConnected ?? false
                wifiSsid: sysData?.wifiSsid ?? ""
                wifiSignal: sysData?.wifiSignal ?? 0
                wifiEnabled: sysData?.wifiEnabled ?? true
                dlRatio: sysData?.dlRatio ?? 0
                ulRatio: sysData?.ulRatio ?? 0
                downloadStr: sysData?.downloadStr ?? "0B"
                uploadStr: sysData?.uploadStr ?? "0B"
                wifiPopup: wifiPopup
            }

            Separator {
                sepColor: cfg?.sepColor ?? "#2F2F38"
                sepLightColor: cfg?.sepLightColor ?? "#505050"
                barWidth: cfg?.barWidth ?? 46
            }

            VolumeWidget {
                id: volumeWidgetContainer
                accentColor: cfg?.accentColor ?? "#C16FB8"
                inactiveColor: cfg?.inactiveColor ?? "#5e5e65"
                textColor: cfg?.textColor ?? "#A3A0AF"
                barWidth: cfg?.barWidth ?? 46
                mixerPopup: mixerPopup
            }

            Separator {
                sepColor: cfg?.sepColor ?? "#2F2F38"
                sepLightColor: cfg?.sepLightColor ?? "#505050"
                barWidth: cfg?.barWidth ?? 46
            }

            BatteryWidget {
                id: batteryWidgetContainer
                accentColor: cfg?.accentColor ?? "#C16FB8"
                inactiveColor: cfg?.inactiveColor ?? "#5e5e65"
                textColor: cfg?.textColor ?? "#A3A0AF"
                batteryPercent: sysData?.batteryPercent ?? 0
                batteryStatus: sysData?.batteryStatus ?? "Unknown"
                batteryPopup: batteryPopup
            }

            Separator {
                sepColor: cfg?.sepColor ?? "#2F2F38"
                sepLightColor: cfg?.sepLightColor ?? "#505050"
                barWidth: cfg?.barWidth ?? 46
            }

            UpdatesWidget {
                id: updatesWidgetContainer
                accentColor: cfg?.accentColor ?? "#C16FB8"
                inactiveColor: cfg?.inactiveColor ?? "#5e5e65"
                existingColor: cfg?.existingColor ?? "#939399"
                textColor: cfg?.textColor ?? "#A3A0AF"
                barWidth: cfg?.barWidth ?? 46
                updatesCount: sysData?.pacmanUpdatesCount ?? 0
                isChecking: sysData?.isCheckingUpdates ?? false
                updatesPopup: updatesPopup
            }

            Separator {
                sepColor: cfg?.sepColor ?? "#2F2F38"
                sepLightColor: cfg?.sepLightColor ?? "#505050"
                barWidth: cfg?.barWidth ?? 46
            }

            TrayWidget {
                id: trayLayout
            }

            Separator {
                sepColor: cfg?.sepColor ?? "#2F2F38"
                sepLightColor: cfg?.sepLightColor ?? "#505050"
                barWidth: cfg?.barWidth ?? 46
                visible: trayLayout.visible
            }

            LayoutWidget {
                id: layoutWidget
                textColor: cfg?.textColor ?? "#A3A0AF"
                barWidth: cfg?.barWidth ?? 46

                Component.onCompleted: {
                    workspacePanel.keyboardLayoutsChanged.connect(layoutWidget.updateLayouts)
                    workspacePanel.keyboardLayoutSwitched.connect(layoutWidget.switchLayout)
                }
            }

            Separator {
                sepColor: cfg?.sepColor ?? "#2F2F38"
                sepLightColor: cfg?.sepLightColor ?? "#505050"
                barWidth: cfg?.barWidth ?? 46
            }

            ClockWidget {
                id: clockContainer
                existingColor: cfg?.existingColor ?? "#939399"
                barWidth: cfg?.barWidth ?? 46
                calPopup: calPopup
            }
        }
    }

    // Shadow
    Rectangle {
        width: 15
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        anchors.left: mainBar.right

        gradient: Gradient {
            orientation: Gradient.Horizontal
            GradientStop { position: 0.0; color: "#A6000000" }
            GradientStop { position: 1.0; color: "transparent" }
        }
    }

    function closeAllPopupsExcept(exceptPopup) {
        if (powerPopup && powerPopup !== exceptPopup && powerPopup.visible) powerPopup.visible = false
        if (wifiPopup && wifiPopup !== exceptPopup && wifiPopup.visible) wifiPopup.visible = false
        if (batteryPopup && batteryPopup !== exceptPopup && batteryPopup.visible) batteryPopup.visible = false
        if (calPopup && calPopup !== exceptPopup && calPopup.visible) calPopup.visible = false
        if (mixerPopup && mixerPopup !== exceptPopup && mixerPopup.visible) mixerPopup.visible = false
        if (winMenu && winMenu !== exceptPopup && winMenu.visible) winMenu.visible = false
        if (updatesPopup && updatesPopup !== exceptPopup && updatesPopup.visible) updatesPopup.visible = false
    }

    function togglePopup(popup) {
        if (!popup) return
        if (popup.visible) {
            popup.visible = false
        } else {
            closeAllPopupsExcept(popup)
            popup.visible = true
        }
    }

    PowerPopup {
        id: powerPopup
        popupSysData: sysData
        popupCfg: cfg
        anchorItem: archLogoContainer
        onVisibleChanged: {
            if (visible) leftBar.closeAllPopupsExcept(powerPopup)
        }
    }

    WifiPopup {
        id: wifiPopup
        popupSysData: sysData
        popupCfg: cfg
        anchorItem: netWidgetContainer
        onVisibleChanged: {
            if (visible) leftBar.closeAllPopupsExcept(wifiPopup)
        }
    }

    BatteryPopup {
        id: batteryPopup
        popupSysData: sysData
        popupCfg: cfg
        anchorItem: batteryWidgetContainer
        onVisibleChanged: {
            if (visible) leftBar.closeAllPopupsExcept(batteryPopup)
        }
    }

    UpdatesPopup {
        id: updatesPopup
        popupSysData: sysData
        popupCfg: cfg
        anchorItem: updatesWidgetContainer
        onVisibleChanged: {
            if (visible) leftBar.closeAllPopupsExcept(updatesPopup)
        }
    }

    CalendarPopup {
        id: calPopup
        popupSysData: sysData
        popupCfg: cfg
        anchorItem: clockContainer
        onVisibleChanged: {
            if (visible) leftBar.closeAllPopupsExcept(calPopup)
        }
    }

    WindowMenu {
        id: winMenu
        popupCfg: cfg
        parentWin: leftBar
        onVisibleChanged: {
            if (visible) leftBar.closeAllPopupsExcept(winMenu)
        }
    }

    MixerPopup {
        id: mixerPopup
        popupCfg: cfg
        anchorItem: volumeWidgetContainer
        onVisibleChanged: {
            if (visible) leftBar.closeAllPopupsExcept(mixerPopup)
        }
    }
}
