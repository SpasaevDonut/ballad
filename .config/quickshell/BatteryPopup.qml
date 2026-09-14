//@ pragma UseQApplication
import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import Qt5Compat.GraphicalEffects
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

PopupWindow {
    id: root
    visible: false
    color: "transparent"
    implicitWidth: 340
    implicitHeight: 320

    property double lastCloseTime: 0
    property QtObject popupSysData: null
    property QtObject popupCfg: null
    property var anchorItem: null

    property int currentMode: 1 // 0: Powersave, 1: Balanced, 2: Performance
    property bool isChanging: false
    property string statusMessage: ""
    property bool statusIsError: false

    function applyMode(idx) {
        if (idx < 0 || idx > 2) return
        currentMode = idx
        isChanging = true
        statusIsError = false

        var label = (idx === 0) ? "Powersave" : (idx === 2 ? "Performance" : "Balanced")
        statusMessage = "Applying " + label + " mode..."

        var cmd = ""
        if (idx === 0) {
            cmd = "sudo -n tlp bat 2>/dev/null; sudo -n tlp power-saver 2>/dev/null; powerprofilesctl set power-saver 2>/dev/null; echo low-power | sudo -n tee /sys/firmware/acpi/platform_profile 2>/dev/null || echo quiet | sudo -n tee /sys/firmware/acpi/platform_profile 2>/dev/null || echo power-saver | sudo -n tee /sys/firmware/acpi/platform_profile 2>/dev/null; true"
        } else if (idx === 1) {
            cmd = "sudo -n tlp auto 2>/dev/null; sudo -n tlp balanced 2>/dev/null; powerprofilesctl set balanced 2>/dev/null; echo balanced | sudo -n tee /sys/firmware/acpi/platform_profile 2>/dev/null; true"
        } else {
            cmd = "sudo -n tlp ac 2>/dev/null; sudo -n tlp performance 2>/dev/null; powerprofilesctl set performance 2>/dev/null; echo performance | sudo -n tee /sys/firmware/acpi/platform_profile 2>/dev/null; true"
        }

        setModeProc.command = ["sh", "-c", cmd]
        setModeProc.running = true
    }

    function refreshMode() {
        getModeProc.running = true
    }

    anchor {
        item: anchorItem
        edges: Edges.Right
        gravity: Edges.Right
    }

    MouseArea {
        anchors.fill: parent
        onClicked: {
            lastCloseTime = Date.now()
            root.visible = false
        }
    }

    Rectangle {
        id: popupRect
        anchors.verticalCenter: parent.verticalCenter
        anchors.left: parent.left
        anchors.leftMargin: 10

        width: 280
        height: 260
        color: popupCfg?.bgColor ?? "#121217"

        MouseArea {
            anchors.fill: parent
        }

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 15
            spacing: 8

            // --- HEADER ---
            RowLayout {
                Layout.fillWidth: true

                Item { Layout.fillWidth: true } // Spacer

                Text {
                    text: "POWER PROFILE"
                    color: popupCfg?.existingColor ?? "#939399"
                    font.pixelSize: 14
                    font.bold: true
                    font.family: "AurulentSansMNerdFontPropo"
                }

                Item { Layout.fillWidth: true } // Spacer

                Text {
                    text: "✖"
                    color: popupCfg?.inactiveColor ?? "#5e5e65"
                    font.pixelSize: 14
                    font.bold: true

                    MouseArea {
                        anchors.fill: parent
                        anchors.margins: -10
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            lastCloseTime = Date.now()
                            root.visible = false
                        }
                    }
                }
            }

            // Accent separator line
            Rectangle {
                Layout.fillWidth: true
                height: 2
                color: popupCfg?.accentColor ?? "#C16FB8"
            }

            // --- BATTERY STATUS CARD ---
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 6

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 6

                    Text {
                        text: (popupSysData?.batteryStatus ?? "") === "Charging" ? "⚡" : "󰁹"
                        color: popupCfg?.accentColor ?? "#C16FB8"
                        font.pixelSize: 16
                        font.family: "AurulentSansMNerdFontPropo"
                    }

                    Text {
                        text: "BATTERY"
                        color: popupCfg?.textColor ?? "#A3A0AF"
                        font.pixelSize: 11
                        font.bold: true
                        font.family: "monospace"
                    }

                    Item { Layout.fillWidth: true }

                    Text {
                        text: Math.round(popupSysData?.batteryPercent ?? 0) + "%"
                        color: popupCfg?.accentColor ?? "#C16FB8"
                        font.pixelSize: 13
                        font.bold: true
                        font.family: "monospace"
                    }

                    Text {
                        text: "• " + (popupSysData?.batteryStatus ?? "UNKNOWN").toUpperCase()
                        color: popupCfg?.inactiveColor ?? "#5e5e65"
                        font.pixelSize: 10
                        font.family: "monospace"
                    }
                }

                // Full-width discrete LED charge bar
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 3

                    Repeater {
                        model: 18
                        Rectangle {
                            Layout.fillWidth: true
                            height: 6
                            color: {
                                let pct = popupSysData?.batteryPercent ?? 0
                                let active = Math.round((pct / 100.0) * 18)
                                return index < active ? (popupCfg?.accentColor ?? "#C16FB8") : (popupCfg?.inactiveColor ?? "#5e5e65")
                            }
                        }
                    }
                }
            }

            // Separator
            Rectangle {
                Layout.fillWidth: true
                height: 1
                color: popupCfg?.sepColor ?? "#2F2F38"
            }

            // --- SLIDER SECTION (LOW TO HIGH) ---
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 4

                RowLayout {
                    Layout.fillWidth: true
                    Text {
                        text: "LOW (POWERSAVE)"
                        color: root.currentMode === 0 ? (popupCfg?.accentColor ?? "#C16FB8") : (popupCfg?.inactiveColor ?? "#5e5e65")
                        font.pixelSize: 9
                        font.bold: root.currentMode === 0
                        font.family: "monospace"
                    }
                    Item { Layout.fillWidth: true }
                    Text {
                        text: "HIGH (PERF)"
                        color: root.currentMode === 2 ? (popupCfg?.accentColor ?? "#C16FB8") : (popupCfg?.inactiveColor ?? "#5e5e65")
                        font.pixelSize: 9
                        font.bold: root.currentMode === 2
                        font.family: "monospace"
                    }
                }

                // 3-Step Slider
                Slider {
                    id: modeSlider
                    Layout.fillWidth: true
                    from: 0
                    to: 2
                    stepSize: 1
                    snapMode: Slider.SnapAlways
                    value: root.currentMode

                    onMoved: {
                        root.applyMode(Math.round(value))
                    }

                    background: Rectangle {
                        x: modeSlider.leftPadding
                        y: modeSlider.topPadding + modeSlider.availableHeight / 2 - height / 2
                        implicitWidth: 200
                        implicitHeight: 6
                        width: modeSlider.availableWidth
                        height: implicitHeight
                        color: "#16161f"
                        border.color: popupCfg?.sepLightColor ?? "#505050"
                        border.width: 1

                        // Step Ticks
                        RowLayout {
                            anchors.fill: parent
                            spacing: 0

                            Rectangle {
                                width: 2
                                height: 6
                                color: modeSlider.value >= 0 ? (popupCfg?.accentColor ?? "#C16FB8") : (popupCfg?.inactiveColor ?? "#5e5e65")
                            }
                            Item { Layout.fillWidth: true }
                            Rectangle {
                                width: 2
                                height: 6
                                color: modeSlider.value >= 1 ? (popupCfg?.accentColor ?? "#C16FB8") : (popupCfg?.inactiveColor ?? "#5e5e65")
                            }
                            Item { Layout.fillWidth: true }
                            Rectangle {
                                width: 2
                                height: 6
                                color: modeSlider.value >= 2 ? (popupCfg?.accentColor ?? "#C16FB8") : (popupCfg?.inactiveColor ?? "#5e5e65")
                            }
                        }
                    }

                    handle: Rectangle {
                        x: modeSlider.leftPadding + modeSlider.visualPosition * (modeSlider.availableWidth - width)
                        y: modeSlider.topPadding + modeSlider.availableHeight / 2 - height / 2
                        implicitWidth: 16
                        implicitHeight: 18
                        color: popupCfg?.accentColor ?? "#C16FB8"
                        border.color: "#ffffff"
                        border.width: 1
                    }
                }
            }

            // --- 3 MODE BUTTONS ---
            RowLayout {
                Layout.fillWidth: true
                spacing: 4

                // Mode 0: Powersave
                Rectangle {
                    Layout.fillWidth: true
                    height: 26
                    color: root.currentMode === 0 ? "#281e30" : (b0Mouse.containsMouse ? "#1c1c24" : "transparent")
                    border.width: 1
                    border.color: root.currentMode === 0 ? (popupCfg?.accentColor ?? "#C16FB8") : (popupCfg?.sepLightColor ?? "#505050")

                    Text {
                        anchors.centerIn: parent
                        text: "POWERSAVE"
                        color: root.currentMode === 0 ? (popupCfg?.accentColor ?? "#C16FB8") : (popupCfg?.textColor ?? "#A3A0AF")
                        font.pixelSize: 9
                        font.bold: true
                        font.family: "monospace"
                    }

                    MouseArea {
                        id: b0Mouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: root.applyMode(0)
                    }
                }

                // Mode 1: Balanced
                Rectangle {
                    Layout.fillWidth: true
                    height: 26
                    color: root.currentMode === 1 ? "#281e30" : (b1Mouse.containsMouse ? "#1c1c24" : "transparent")
                    border.width: 1
                    border.color: root.currentMode === 1 ? (popupCfg?.accentColor ?? "#C16FB8") : (popupCfg?.sepLightColor ?? "#505050")

                    Text {
                        anchors.centerIn: parent
                        text: "BALANCED"
                        color: root.currentMode === 1 ? (popupCfg?.accentColor ?? "#C16FB8") : (popupCfg?.textColor ?? "#A3A0AF")
                        font.pixelSize: 9
                        font.bold: true
                        font.family: "monospace"
                    }

                    MouseArea {
                        id: b1Mouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: root.applyMode(1)
                    }
                }

                // Mode 2: Performance
                Rectangle {
                    Layout.fillWidth: true
                    height: 26
                    color: root.currentMode === 2 ? "#281e30" : (b2Mouse.containsMouse ? "#1c1c24" : "transparent")
                    border.width: 1
                    border.color: root.currentMode === 2 ? (popupCfg?.accentColor ?? "#C16FB8") : (popupCfg?.sepLightColor ?? "#505050")

                    Text {
                        anchors.centerIn: parent
                        text: "PERFORMANCE"
                        color: root.currentMode === 2 ? (popupCfg?.accentColor ?? "#C16FB8") : (popupCfg?.textColor ?? "#A3A0AF")
                        font.pixelSize: 9
                        font.bold: true
                        font.family: "monospace"
                    }

                    MouseArea {
                        id: b2Mouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: root.applyMode(2)
                    }
                }
            }

            // --- STATUS / FOOTER INFO ---
            Text {
                Layout.alignment: Qt.AlignHCenter
                text: {
                    if (root.statusMessage) return root.statusMessage
                    if (root.currentMode === 0) return "MODE: POWERSAVE (TLP BAT)"
                    if (root.currentMode === 2) return "MODE: PERFORMANCE (TLP AC)"
                    return "MODE: BALANCED (TLP AUTO)"
                }
                color: root.statusIsError ? "#ff6b81" : (popupCfg?.inactiveColor ?? "#5e5e65")
                font.pixelSize: 9
                font.family: "monospace"
            }
        }
    }

    // Shadow
    DropShadow {
        source: popupRect
        anchors.fill: popupRect
        horizontalOffset: 15
        verticalOffset: 15
        radius: 15
        samples: 16
        color: "#A6000000"
        transparentBorder: true
    }

    // Process to read current power mode
    Process {
        id: getModeProc
        command: ["sh", "-c", `
            # 1. Check laptop ACPI platform_profile (hardware LED state)
            if [ -f /sys/firmware/acpi/platform_profile ]; then
                pp=$(cat /sys/firmware/acpi/platform_profile 2>/dev/null | tr '[:upper:]' '[:lower:]')
                if [ "$pp" = "low-power" ] || [ "$pp" = "quiet" ] || [ "$pp" = "power-saver" ]; then echo "0"; exit 0; fi
                if [ "$pp" = "performance" ]; then echo "2"; exit 0; fi
                if [ "$pp" = "balanced" ]; then echo "1"; exit 0; fi
            fi

            # 2. Check TLP manual_mode and last_mode
            if [ -f /run/tlp/manual_mode ]; then
                mm=$(cat /run/tlp/manual_mode 2>/dev/null | tr '[:upper:]' '[:lower:]')
                if [ "$mm" = "battery" ] || [ "$mm" = "bat" ]; then echo "0"; exit 0; fi
                if [ "$mm" = "ac" ]; then echo "2"; exit 0; fi
            fi
            if [ -f /run/tlp/last_mode ]; then
                lm=$(cat /run/tlp/last_mode 2>/dev/null | tr '[:upper:]' '[:lower:]')
                if [ "$lm" = "battery" ] || [ "$lm" = "bat" ]; then echo "0"; exit 0; fi
                if [ "$lm" = "ac" ]; then echo "2"; exit 0; fi
            fi

            # 3. Check tlp-stat mode
            if command -v tlp-stat >/dev/null 2>&1; then
                ms=$(sudo -n tlp-stat -s 2>/dev/null || tlp-stat -s 2>/dev/null)
                if echo "$ms" | grep -i "Mode" | grep -qi "battery"; then echo "0"; exit 0; fi
                if echo "$ms" | grep -i "Mode" | grep -qi "ac"; then echo "2"; exit 0; fi
            fi

            # 4. Check CPU energy performance preference
            if [ -f /sys/devices/system/cpu/cpu0/cpufreq/energy_performance_preference ]; then
                epp=$(cat /sys/devices/system/cpu/cpu0/cpufreq/energy_performance_preference 2>/dev/null | tr '[:upper:]' '[:lower:]')
                if [ "$epp" = "power" ] || [ "$epp" = "power-saver" ] || [ "$epp" = "balance_power" ]; then echo "0"; exit 0; fi
                if [ "$epp" = "performance" ]; then echo "2"; exit 0; fi
                if [ "$epp" = "balance_performance" ] || [ "$epp" = "default" ]; then echo "1"; exit 0; fi
            fi

            # 5. Check CPU scaling governor
            if [ -f /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor ]; then
                gov=$(cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor 2>/dev/null | tr '[:upper:]' '[:lower:]')
                if [ "$gov" = "powersave" ]; then echo "0"; exit 0; fi
                if [ "$gov" = "performance" ]; then echo "2"; exit 0; fi
            fi

            # 6. Check powerprofilesctl as fallback
            if command -v powerprofilesctl >/dev/null 2>&1; then
                m=$(powerprofilesctl get 2>/dev/null)
                if [ "$m" = "power-saver" ]; then echo "0"; exit 0; fi
                if [ "$m" = "performance" ]; then echo "2"; exit 0; fi
            fi

            echo "1"
        `]
        stdout: SplitParser {
            onRead: data => {
                let val = parseInt(data.trim())
                if (!isNaN(val) && val >= 0 && val <= 2) {
                    root.currentMode = val
                }
            }
        }
    }

    Timer {
        id: refreshTimer
        interval: 300
        repeat: false
        onTriggered: root.refreshMode()
    }

    // Process to apply power mode
    Process {
        id: setModeProc
        property string errorOutput: ""

        stderr: SplitParser {
            onRead: data => {
                setModeProc.errorOutput += data + " "
            }
        }

        onExited: (code) => {
            root.isChanging = false
            if (code !== 0) {
                root.statusIsError = true
                root.statusMessage = setModeProc.errorOutput.trim() || "Failed to set mode"
            } else {
                root.statusIsError = false
                root.statusMessage = ""
            }
            setModeProc.errorOutput = ""
            refreshTimer.start()
        }
    }

    onVisibleChanged: {
        if (visible) {
            root.statusMessage = ""
            root.statusIsError = false
            root.refreshMode()
        }
    }
}
