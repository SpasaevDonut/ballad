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
        color: popupCfg?.popupBgColor ?? popupCfg?.bg0 ?? "#f1e5ce"
        border.width: 1
        border.color: popupCfg?.bg2 ?? "#d5c4a1"

        // Victorian Corner Filigree
        Item {
            anchors.left: parent.left
            anchors.top: parent.top
            anchors.margins: 4
            width: 16; height: 16
            rotation: 270

            Text {
                anchors.right: parent.right
                anchors.top: parent.top
                text: "z"
                font.family: popupCfg?.fontVictorian ?? "Victorian Free Ornaments"
                font.pixelSize: 16
                color: popupCfg?.gold ?? "#b5893f"
                opacity: 0.65
            }
        }
        Item {
            anchors.right: parent.right
            anchors.top: parent.top
            anchors.margins: 4
            width: 16; height: 16

            Text {
                anchors.right: parent.right
                anchors.top: parent.top
                text: "z"
                font.family: popupCfg?.fontVictorian ?? "Victorian Free Ornaments"
                font.pixelSize: 16
                color: popupCfg?.gold ?? "#b5893f"
                opacity: 0.65
            }
        }
        Item {
            anchors.left: parent.left
            anchors.bottom: parent.bottom
            anchors.margins: 4
            width: 16; height: 16
            rotation: 180

            Text {
                anchors.right: parent.right
                anchors.top: parent.top
                text: "z"
                font.family: popupCfg?.fontVictorian ?? "Victorian Free Ornaments"
                font.pixelSize: 16
                color: popupCfg?.gold ?? "#b5893f"
                opacity: 0.65
            }
        }
        Item {
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            anchors.margins: 4
            width: 16; height: 16
            rotation: 90

            Text {
                anchors.right: parent.right
                anchors.top: parent.top
                text: "z"
                font.family: popupCfg?.fontVictorian ?? "Victorian Free Ornaments"
                font.pixelSize: 16
                color: popupCfg?.gold ?? "#b5893f"
                opacity: 0.65
            }
        }

        MouseArea { 
            anchors.fill: parent 
        }

        Process {
            id: powerActionProc
            running: false
        }

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 16
            spacing: 8

            // --- HEADER ---
            RowLayout {
                Layout.fillWidth: true

                Item { Layout.fillWidth: true } // Spacer

                Text { 
                    text: "SYSTEM POWER"
                    color: popupCfg?.fg ?? "#3a2a20"
                    font.pixelSize: 13
                    font.bold: true
                    font.family: popupCfg?.fontTitle ?? "Cinzel"
                }

                Item { Layout.fillWidth: true } // Spacer
            }

            // Filigree divider
            RowLayout {
                Layout.fillWidth: true
		spacing: 6

                Rectangle { Layout.fillWidth: true; height: 1; color: popupCfg?.bg2 ?? "#d5c4a1"; opacity: 0.7 }
                Text {
                    text: "t"
                    font.family: popupCfg?.fontBodoni ?? "Bodoni Ornaments ITC"
                    font.pixelSize: 14
                    color: popupCfg?.gold ?? "#b5893f"
                }
                Rectangle { Layout.fillWidth: true; height: 1; color: popupCfg?.bg2 ?? "#d5c4a1"; opacity: 0.7 }
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
                        color: popupCfg?.gold ?? "#b5893f"
                        font.pixelSize: 15
                        font.family: "AurulentSansMNerdFontPropo"
                    }

                    Text {
                        text: "CPU LOAD"
                        color: popupCfg?.fg ?? "#3a2a20"
                        font.pixelSize: 10
                        font.bold: true
                        font.family: popupCfg?.fontTitle ?? "Cinzel"
                    }

                    Item { Layout.fillWidth: true }

                    Text {
                        text: (popupSysData?.cpuPercent ?? 0) + "%"
                        color: popupCfg?.gold ?? "#b5893f"
                        font.pixelSize: 12
                        font.bold: true
                        font.family: popupCfg?.fontWidget ?? "EB Garamond"
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
                            height: 5
                            radius: 1
                            color: {
                                let pct = popupSysData?.cpuPercent ?? 0
                                let active = Math.round((pct / 100.0) * 18)
                                return index < active ? (popupCfg?.gold ?? "#b5893f") : (popupCfg?.bg2 ?? "#d5c4a1")
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
                        color: popupCfg?.gold ?? "#b5893f"
                        font.pixelSize: 15
                        font.family: "AurulentSansMNerdFontPropo"
                    }

                    Text {
                        text: "MEMORY"
                        color: popupCfg?.fg ?? "#3a2a20"
                        font.pixelSize: 10
                        font.bold: true
                        font.family: popupCfg?.fontTitle ?? "Cinzel"
                    }

                    Item { Layout.fillWidth: true }

                    Text {
                        text: Math.round(((popupSysData?.ramBlocks ?? 0) / 24) * 100) + "%"
                        color: popupCfg?.gold ?? "#b5893f"
                        font.pixelSize: 12
                        font.bold: true
                        font.family: popupCfg?.fontWidget ?? "EB Garamond"
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
                            height: 5
                            radius: 1
                            color: {
                                let pct = ((popupSysData?.ramBlocks ?? 0) / 24) * 100
                                let active = Math.round((pct / 100.0) * 18)
                                return index < active ? (popupCfg?.gold ?? "#b5893f") : (popupCfg?.bg2 ?? "#d5c4a1")
                            }
                        }
                    }
                }
            }

            // Separator
            Rectangle {
                Layout.fillWidth: true
                height: 1
                color: popupCfg?.bg2 ?? "#d5c4a1"
                opacity: 0.7
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
                        radius: 2
                        color: btnMouse.containsMouse ? "#20b5893f" : "transparent"
                        border.width: 1
                        border.color: btnMouse.containsMouse ? (popupCfg?.goldBright ?? "#d5a86d") : (popupCfg?.bg2 ?? "#d5c4a1")

                        ColumnLayout {
                            anchors.centerIn: parent
                            spacing: 2

                            Text {
                                text: modelData.icon
                                font.pixelSize: 15
                                color: btnMouse.containsMouse ? (popupCfg?.red ?? "#7a362d") : (popupCfg?.fg ?? "#3a2a20")
                                font.family: "AurulentSansMNerdFontPropo"
                                Layout.alignment: Qt.AlignHCenter
                            }

                            Text {
                                text: modelData.label
                                font.pixelSize: 9
                                font.bold: true
                                font.family: popupCfg?.fontTitle ?? "Cinzel"
                                color: btnMouse.containsMouse ? (popupCfg?.red ?? "#7a362d") : (popupCfg?.fg ?? "#3a2a20")
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
        horizontalOffset: 12
        verticalOffset: 12
        radius: 14
        samples: 16
        color: "#353a2a20"
        transparentBorder: true
    }
}
