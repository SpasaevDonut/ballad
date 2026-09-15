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
        color: cfg?.bg0 ?? "#f1e5ce"

        // Inner double-line bookbinding / watch rim border
        Rectangle {
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            anchors.right: parent.right
            anchors.rightMargin: 1
            width: 1
            color: cfg?.gold ?? "#b5893f"
            opacity: 0.45
        }
        Rectangle {
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            anchors.right: parent.right
            width: 1
            color: cfg?.bg2 ?? "#d5c4a1"
            opacity: 0.8
        }

        // Top Vignette
        Rectangle {
            anchors.top: parent.top
            anchors.left: parent.left
            anchors.right: parent.right
            height: 70
            gradient: Gradient {
                GradientStop { position: 0.0; color: "#284c2d26" }
                GradientStop { position: 1.0; color: "transparent" }
            }
        }

        // Bottom Vignette
        Rectangle {
            anchors.bottom: parent.bottom
            anchors.left: parent.left
            anchors.right: parent.right
            height: 70
            gradient: Gradient {
                GradientStop { position: 0.0; color: "transparent" }
                GradientStop { position: 1.0; color: "#284c2d26" }
            }
        }

        // Top Victorian Pediment
        Text {
            anchors.top: parent.top
	    anchors.topMargin: 2
            anchors.horizontalCenter: parent.horizontalCenter
            text: "7"
            font.family: cfg?.fontVictorian ?? "Victorian Free Ornaments"
            font.pixelSize: 15
            color: cfg?.gold ?? "#b5893f"
            opacity: 0.75
            z: 2
        }

        // Bottom Victorian Tailpiece
        Text {
            anchors.bottom: parent.bottom
            anchors.bottomMargin: 2
            anchors.horizontalCenter: parent.horizontalCenter
            text: "A"
            font.family: cfg?.fontVictorian ?? "Victorian Free Ornaments"
            font.pixelSize: 13
            color: cfg?.gold ?? "#b5893f"
            opacity: 0.75
            z: 2
        }

        ColumnLayout {
            anchors.fill: parent
            anchors.topMargin: 20
            anchors.bottomMargin: 16
            spacing: 0

            PowerButton {
                id: archLogoContainer
                accentColor: cfg?.red ?? "#7a362d"
                existingColor: cfg?.gold ?? "#b5893f"
                powerPopup: powerPopup
            }

            Separator {
                sepColor: cfg?.sepColor ?? "#d5c4a1"
                sepLightColor: cfg?.sepLightColor ?? "#f1e5ce"
                barWidth: cfg?.barWidth ?? 46
            }

            WorkspacePanel {
                id: workspacePanel
                accentColor: cfg?.accentColor ?? "#b5893f"
                inactiveColor: cfg?.inactiveColor ?? "#d5c4a1"
                existingColor: cfg?.existingColor ?? "#7a362d"
                textColor: cfg?.textColor ?? "#3a2a20"
                sepColor: cfg?.sepColor ?? "#d5c4a1"
                sepLightColor: cfg?.sepLightColor ?? "#f1e5ce"
                barWidth: cfg?.barWidth ?? 46
		winMenu: winMenu
            }

            Separator {
                sepColor: cfg?.sepColor ?? "#d5c4a1"
                sepLightColor: cfg?.sepLightColor ?? "#f1e5ce"
                barWidth: cfg?.barWidth ?? 46
            }

            NetworkWidget {
                id: netWidgetContainer
                accentColor: cfg?.accentColor ?? "#b5893f"
                inactiveColor: cfg?.inactiveColor ?? "#d5c4a1"
                textColor: cfg?.textColor ?? "#3a2a20"
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
                sepColor: cfg?.sepColor ?? "#d5c4a1"
                sepLightColor: cfg?.sepLightColor ?? "#f1e5ce"
                barWidth: cfg?.barWidth ?? 46
            }

            VolumeWidget {
                id: volumeWidgetContainer
                accentColor: cfg?.accentColor ?? "#b5893f"
                inactiveColor: cfg?.inactiveColor ?? "#d5c4a1"
                textColor: cfg?.textColor ?? "#3a2a20"
                barWidth: cfg?.barWidth ?? 46
                mixerPopup: mixerPopup
            }

            Separator {
                sepColor: cfg?.sepColor ?? "#d5c4a1"
                sepLightColor: cfg?.sepLightColor ?? "#f1e5ce"
                barWidth: cfg?.barWidth ?? 46
            }

            BatteryWidget {
                id: batteryWidgetContainer
                accentColor: cfg?.accentColor ?? "#b5893f"
                inactiveColor: cfg?.inactiveColor ?? "#d5c4a1"
                textColor: cfg?.textColor ?? "#3a2a20"
                batteryPercent: sysData?.batteryPercent ?? 0
                batteryStatus: sysData?.batteryStatus ?? "Unknown"
                batteryPopup: batteryPopup
            }

            Separator {
                sepColor: cfg?.sepColor ?? "#d5c4a1"
                sepLightColor: cfg?.sepLightColor ?? "#f1e5ce"
                barWidth: cfg?.barWidth ?? 46
            }

            UpdatesWidget {
                id: updatesWidgetContainer
                accentColor: cfg?.accentColor ?? "#b5893f"
                hoverColor: cfg?.red ?? "#7a362d"
                inactiveColor: cfg?.inactiveColor ?? "#d5c4a1"
                existingColor: cfg?.existingColor ?? "#7a362d"
                textColor: cfg?.textColor ?? "#3a2a20"
                barWidth: cfg?.barWidth ?? 46
                updatesCount: sysData?.pacmanUpdatesCount ?? 0
                isChecking: sysData?.isCheckingUpdates ?? false
                updatesPopup: updatesPopup
            }

            Separator {
                sepColor: cfg?.sepColor ?? "#d5c4a1"
                sepLightColor: cfg?.sepLightColor ?? "#f1e5ce"
                barWidth: cfg?.barWidth ?? 46
            }

            TrayWidget {
                id: trayLayout
                badgeColor: cfg?.fg ?? "#3a2a20"
                badgeHoverColor: cfg?.red ?? "#7a362d"
                borderColor: cfg?.gold ?? "#b5893f"
                borderHoverColor: cfg?.goldBright ?? "#d5a86d"
            }

            Separator {
                sepColor: cfg?.sepColor ?? "#d5c4a1"
                sepLightColor: cfg?.sepLightColor ?? "#f1e5ce"
                barWidth: cfg?.barWidth ?? 46
                visible: trayLayout.visible
            }

            LayoutWidget {
                id: layoutWidget
                textColor: cfg?.textColor ?? "#3a2a20"
                barWidth: cfg?.barWidth ?? 46

                Component.onCompleted: {
                    workspacePanel.keyboardLayoutsChanged.connect(layoutWidget.updateLayouts)
                    workspacePanel.keyboardLayoutSwitched.connect(layoutWidget.switchLayout)
                }
            }

            Separator {
                sepColor: cfg?.sepColor ?? "#d5c4a1"
                sepLightColor: cfg?.sepLightColor ?? "#f1e5ce"
                barWidth: cfg?.barWidth ?? 46
            }

            ClockWidget {
                id: clockContainer
                existingColor: cfg?.textColor ?? "#3a2a20"
                accentColor: cfg?.gold ?? "#b5893f"
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
            GradientStop { position: 0.0; color: "#353a2a20" }
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
