//@ pragma UseQApplication
import QtQuick
import QtQuick.Layouts
import Qt5Compat.GraphicalEffects
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

PopupWindow {
    id: root
    visible: false
    color: "transparent"
    implicitWidth: 340
    implicitHeight: 330
    
    property double lastCloseTime: 0
    property QtObject popupSysData: null
    property QtObject popupCfg: null
    property var anchorItem: null

    anchor { 
        item: anchorItem
        edges: Edges.Right | Edges.Top
        gravity: Edges.Right | Edges.Bottom 
    }

    MouseArea {
        anchors.fill: parent
        onClicked: {
            lastCloseTime = Date.now()
            root.visible = false
        }
    }

    Rectangle {
        id: powerRect
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.topMargin: 0
        anchors.leftMargin: 10    

        width: 280
        height: 270
        color: popupCfg?.bgColor ?? "#121217"

        MouseArea { 
            anchors.fill: parent 
        }

        Process {
            id: powerActionProc
            running: false
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
                    text: "SYSTEM POWER"
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

            // --- CPU MONITOR CARD ---
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 4

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 6

                    Text {
                        text: "󰘚"
                        color: popupCfg?.accentColor ?? "#C16FB8"
                        font.pixelSize: 16
                        font.family: "AurulentSansMNerdFontPropo"
                    }

                    Text {
                        text: "CPU LOAD"
                        color: popupCfg?.textColor ?? "#A3A0AF"
                        font.pixelSize: 10
                        font.bold: true
                        font.family: "monospace"
                    }

                    Item { Layout.fillWidth: true }

                    Text {
                        text: (popupSysData?.cpuPercent ?? 0) + "%"
                        color: popupCfg?.accentColor ?? "#C16FB8"
                        font.pixelSize: 12
                        font.bold: true
                        font.family: "monospace"
                    }
                }

                // 18-block discrete LED bar
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 3

                    Repeater {
                        model: 18
                        Rectangle {
                            Layout.fillWidth: true
                            height: 6
                            color: {
                                let pct = popupSysData?.cpuPercent ?? 0
                                let active = Math.round((pct / 100.0) * 18)
                                return index < active ? (popupCfg?.accentColor ?? "#C16FB8") : (popupCfg?.inactiveColor ?? "#5e5e65")
                            }
                        }
                    }
                }
            }

            // --- RAM MONITOR CARD ---
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 4

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 6

                    Text {
                        text: "󰍛"
                        color: popupCfg?.accentColor ?? "#C16FB8"
                        font.pixelSize: 16
                        font.family: "AurulentSansMNerdFontPropo"
                    }

                    Text {
                        text: "MEMORY"
                        color: popupCfg?.textColor ?? "#A3A0AF"
                        font.pixelSize: 10
                        font.bold: true
                        font.family: "monospace"
                    }

                    Item { Layout.fillWidth: true }

                    Text {
                        text: Math.round(((popupSysData?.ramBlocks ?? 0) / 24) * 100) + "%"
                        color: popupCfg?.accentColor ?? "#C16FB8"
                        font.pixelSize: 12
                        font.bold: true
                        font.family: "monospace"
                    }
                }

                // 18-block discrete LED bar
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 3

                    Repeater {
                        model: 18
                        Rectangle {
                            Layout.fillWidth: true
                            height: 6
                            color: {
                                let pct = ((popupSysData?.ramBlocks ?? 0) / 24) * 100
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

            // --- POWER ACTION SQUARES ---
            RowLayout {
                Layout.fillWidth: true
                spacing: 6

                Repeater {
                    model: [
                        { icon: "󰤄", label: "SLEEP", cmd: "loginctl suspend" },
                        { icon: "󰑐", label: "REBOOT", cmd: "loginctl reboot" },
                        { icon: "󰐥", label: "SHUTDOWN", cmd: "loginctl poweroff" }
                    ]

                    delegate: Rectangle {
                        Layout.fillWidth: true
                        height: 42
                        color: btnMouse.containsMouse ? "#281e30" : "transparent"
                        border.width: 1
                        border.color: btnMouse.containsMouse ? (popupCfg?.accentColor ?? "#C16FB8") : (popupCfg?.sepLightColor ?? "#505050")

                        ColumnLayout {
                            anchors.centerIn: parent
                            spacing: 2

                            Text {
                                text: modelData.icon
                                font.pixelSize: 15
                                color: btnMouse.containsMouse ? (popupCfg?.accentColor ?? "#C16FB8") : (popupCfg?.textColor ?? "#A3A0AF")
                                font.family: "AurulentSansMNerdFontPropo"
                                Layout.alignment: Qt.AlignHCenter
                            }

                            Text {
                                text: modelData.label
                                font.pixelSize: 8
                                font.bold: true
                                font.family: "monospace"
                                color: btnMouse.containsMouse ? (popupCfg?.accentColor ?? "#C16FB8") : (popupCfg?.inactiveColor ?? "#5e5e65")
                                Layout.alignment: Qt.AlignHCenter
                            }
                        }

                        MouseArea {
                            id: btnMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                powerActionProc.command = ["sh", "-c", modelData.cmd]
                                powerActionProc.running = true
                                root.visible = false
                            }
                        }
                    }
                }
            }
        }
    }

    // Shadow
    DropShadow {
        source: powerRect
        anchors.fill: powerRect
        horizontalOffset: 15
        verticalOffset: 15
        radius: 15
        samples: 16
        color: "#A6000000"
        transparentBorder: true
    }
}
