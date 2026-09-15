//@ pragma UseQApplication
import QtQuick
import QtQuick.Layouts
import QtQuick.Effects
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

PanelWindow {
    id: desktopWidgets
    
    property QtObject sysData: null
    property QtObject cfg: null
    readonly property color tickActiveColor: "#a17228"
    readonly property color tickActiveBorder: "#734e1b"
    readonly property color tickInactiveColor: Qt.rgba(0.23, 0.16, 0.13, 0.14)
    
    anchors {
        top: true
        bottom: true
        left: true
        right: true
    }
    
    exclusiveZone: 0
    WlrLayershell.layer: WlrLayer.Background 
    
    color: "transparent"

    Item {
        anchors.fill: parent

        Column {
            anchors.left: parent.left
            anchors.leftMargin: cfg.barWidth + 120
            anchors.verticalCenter: parent.verticalCenter
            spacing: 12

            Column {
                id: clockColumn
                spacing: 4
                anchors.horizontalCenter: parent.horizontalCenter

                SystemClock {
                    id: desktopClock
                    precision: SystemClock.Seconds
                }

                Item {
                    width: timeText.implicitWidth
                    height: 88
                    anchors.horizontalCenter: parent.horizontalCenter

                    Text {
                        id: timeText
                        y: -16
                        anchors.horizontalCenter: parent.horizontalCenter
                        text: Qt.formatDateTime(desktopClock.date, "hh:mm")
                        color: cfg?.fg ?? "#3a2a20"
                        font.pixelSize: 76
                        font.bold: true
                        font.family: cfg?.fontTitle ?? "Cinzel"
                        horizontalAlignment: Text.AlignHCenter
                    }

                    MultiEffect {
                        source: timeText
                        anchors.fill: timeText
                        shadowEnabled: true
                        shadowColor: "#80d5a86d"
                        shadowHorizontalOffset: 1
                        shadowVerticalOffset: 1
                        shadowBlur: 0.1
                    }
                }

                Text {
                    id: dateText
                    text: Qt.formatDateTime(desktopClock.date, "dddd, d MMMM")
                    color: cfg?.red ?? "#7a362d"
                    font.pixelSize: 26
                    font.family: cfg?.fontAccent ?? "Berkshire Swash"
                    anchors.horizontalCenter: parent.horizontalCenter
                }
            }

            Text {
                text: "t"
                font.family: cfg?.fontBodoni ?? "Bodoni Ornaments ITC"
                font.pixelSize: 22
                color: cfg?.gold ?? "#b5893f"
                opacity: 0.85
                anchors.horizontalCenter: parent.horizontalCenter
            }

            GridLayout {
                columns: 2
                columnSpacing: 44
                rowSpacing: 20
                anchors.horizontalCenter: parent.horizontalCenter

                // 1. CPU (Left)
                RowLayout {
                    spacing: 12
                    Rectangle {
                        width: 36; height: 36; radius: 18
                        color: cfg?.fg ?? "#3a2a20"
                        border.color: cfg?.gold ?? "#b5893f"; border.width: 1.5
                        Text { anchors.centerIn: parent; text: "󰘚"; color: cfg?.goldBright ?? "#d5a86d"; font.family: cfg?.fontIcons ?? "AurulentSansMNerdFontPropo"; font.pixelSize: 18 }
                    }
                    ColumnLayout {
                        spacing: 3
                        RowLayout {
                            spacing: 8
                            Text { text: "PROCESSOR"; color: cfg?.red ?? "#7a362d"; font.family: cfg?.fontTitle ?? "Cinzel"; font.pixelSize: 11; font.bold: true }
                            Text { text: (sysData ? sysData.cpuPercent : 0) + "%"; color: cfg?.fg ?? "#3a2a20"; font.family: cfg?.fontWidget ?? "EB Garamond"; font.pixelSize: 14; font.bold: true }
                        }
                        Row {
                            spacing: 3
                            Repeater {
                                model: 24
                                Rectangle {
                                    width: 4; height: 9; radius: 1
                                    color: index < Math.round((sysData ? sysData.cpuPercent : 0) / 100 * 24) ? desktopWidgets.tickActiveColor : desktopWidgets.tickInactiveColor
                                    border.color: index < Math.round((sysData ? sysData.cpuPercent : 0) / 100 * 24) ? desktopWidgets.tickActiveBorder : "transparent"
                                    border.width: 0.5
                                }
                            }
                        }
                    }
                }

                // 2. NETWORK (Right)
                RowLayout {
                    spacing: 12
                    Rectangle {
                        width: 36; height: 36; radius: 18
                        color: cfg?.fg ?? "#3a2a20"
                        border.color: cfg?.gold ?? "#b5893f"; border.width: 1.5
                        Text { anchors.centerIn: parent; text: "󰛳"; color: cfg?.goldBright ?? "#d5a86d"; font.family: cfg?.fontIcons ?? "AurulentSansMNerdFontPropo"; font.pixelSize: 18 }
                    }
                    ColumnLayout {
                        spacing: 3
                        RowLayout {
                            spacing: 6
                            Text { text: "NETWORK"; color: cfg?.red ?? "#7a362d"; font.family: cfg?.fontTitle ?? "Cinzel"; font.pixelSize: 11; font.bold: true }
                            RowLayout {
                                spacing: 4
                                Text { text: "↓"; color: cfg?.red ?? "#7a362d"; font.pixelSize: 11; font.bold: true }
                                Text { text: sysData?.downloadStr ?? "0B"; color: cfg?.fg ?? "#3a2a20"; font.family: cfg?.fontWidget ?? "EB Garamond"; font.pixelSize: 13; font.bold: true }
                                Item { width: 2 }
                                Text { text: "↑"; color: cfg?.red ?? "#7a362d"; font.pixelSize: 11; font.bold: true }
                                Text { text: sysData?.uploadStr ?? "0B"; color: cfg?.fg ?? "#3a2a20"; font.family: cfg?.fontWidget ?? "EB Garamond"; font.pixelSize: 13; font.bold: true }
                            }
                        }
                        Row {
                            spacing: 3
                            Repeater {
                                model: 24
                                Rectangle {
                                    width: 4; height: 9; radius: 1
                                    color: index < Math.round((sysData ? Math.max(sysData.dlRatio, sysData.ulRatio) : 0) * 24) ? desktopWidgets.tickActiveColor : desktopWidgets.tickInactiveColor
                                    border.color: index < Math.round((sysData ? Math.max(sysData.dlRatio, sysData.ulRatio) : 0) * 24) ? desktopWidgets.tickActiveBorder : "transparent"
                                    border.width: 0.5
                                }
                            }
                        }
                    }
                }

                // 3. RAM (Left)
                RowLayout {
                    spacing: 12
                    Rectangle {
                        width: 36; height: 36; radius: 18
                        color: cfg?.fg ?? "#3a2a20"
                        border.color: cfg?.gold ?? "#b5893f"; border.width: 1.5
                        Text { anchors.centerIn: parent; text: "󰍛"; color: cfg?.goldBright ?? "#d5a86d"; font.family: cfg?.fontIcons ?? "AurulentSansMNerdFontPropo"; font.pixelSize: 18 }
                    }
                    ColumnLayout {
                        spacing: 3
                        RowLayout {
                            spacing: 8
                            Text { text: "MEMORY"; color: cfg?.red ?? "#7a362d"; font.family: cfg?.fontTitle ?? "Cinzel"; font.pixelSize: 11; font.bold: true }
                            Text { text: (sysData ? Math.round((sysData.ramBlocks / 24) * 100) : 0) + "%"; color: cfg?.fg ?? "#3a2a20"; font.family: cfg?.fontWidget ?? "EB Garamond"; font.pixelSize: 14; font.bold: true }
                        }
                        Row {
                            spacing: 3
                            Repeater {
                                model: 24
                                Rectangle {
                                    width: 4; height: 9; radius: 1
                                    color: index < (sysData ? sysData.ramBlocks : 0) ? desktopWidgets.tickActiveColor : desktopWidgets.tickInactiveColor
                                    border.color: index < (sysData ? sysData.ramBlocks : 0) ? desktopWidgets.tickActiveBorder : "transparent"
                                    border.width: 0.5
                                }
                            }
                        }
                    }
                }

                // 4. DISK I/O (Right)
                RowLayout {
                    spacing: 12
                    Rectangle {
                        width: 36; height: 36; radius: 18
                        color: cfg?.fg ?? "#3a2a20"
                        border.color: cfg?.gold ?? "#b5893f"; border.width: 1.5
                        Text { anchors.centerIn: parent; text: "󰆼"; color: cfg?.goldBright ?? "#d5a86d"; font.family: cfg?.fontIcons ?? "AurulentSansMNerdFontPropo"; font.pixelSize: 18 }
                    }
                    ColumnLayout {
                        spacing: 3
                        RowLayout {
                            spacing: 6
                            Text { text: "DISK I/O"; color: cfg?.red ?? "#7a362d"; font.family: cfg?.fontTitle ?? "Cinzel"; font.pixelSize: 11; font.bold: true }
                            RowLayout {
                                spacing: 4
                                Text { text: "R"; color: cfg?.red ?? "#7a362d"; font.family: cfg?.fontTitle ?? "Cinzel"; font.pixelSize: 10; font.bold: true }
                                Text { text: sysData?.diskReadStr ?? "0B"; color: cfg?.fg ?? "#3a2a20"; font.family: cfg?.fontWidget ?? "EB Garamond"; font.pixelSize: 13; font.bold: true }
                                Item { width: 2 }
                                Text { text: "W"; color: cfg?.red ?? "#7a362d"; font.family: cfg?.fontTitle ?? "Cinzel"; font.pixelSize: 10; font.bold: true }
                                Text { text: sysData?.diskWriteStr ?? "0B"; color: cfg?.fg ?? "#3a2a20"; font.family: cfg?.fontWidget ?? "EB Garamond"; font.pixelSize: 13; font.bold: true }
                            }
                        }
                        Row {
                            spacing: 3
                            Repeater {
                                model: 24
                                Rectangle {
                                    width: 4; height: 9; radius: 1
                                    color: index < Math.round((sysData ? Math.max(sysData.diskReadRatio, sysData.diskWriteRatio) : 0) * 24) ? desktopWidgets.tickActiveColor : desktopWidgets.tickInactiveColor
                                    border.color: index < Math.round((sysData ? Math.max(sysData.diskReadRatio, sysData.diskWriteRatio) : 0) * 24) ? desktopWidgets.tickActiveBorder : "transparent"
                                    border.width: 0.5
                                }
                            }
                        }
                    }
                }

                // 5. STORAGE (Left)
                RowLayout {
                    spacing: 12
                    Rectangle {
                        width: 36; height: 36; radius: 18
                        color: cfg?.fg ?? "#3a2a20"
                        border.color: cfg?.gold ?? "#b5893f"; border.width: 1.5
                        Text { anchors.centerIn: parent; text: "󰋊"; color: cfg?.goldBright ?? "#d5a86d"; font.family: cfg?.fontIcons ?? "AurulentSansMNerdFontPropo"; font.pixelSize: 18 }
                    }
                    ColumnLayout {
                        spacing: 3
                        RowLayout {
                            spacing: 8
                            Text { text: "STORAGE"; color: cfg?.red ?? "#7a362d"; font.family: cfg?.fontTitle ?? "Cinzel"; font.pixelSize: 11; font.bold: true }
                            Text { text: (sysData ? sysData.storagePercent : 0) + "%"; color: cfg?.fg ?? "#3a2a20"; font.family: cfg?.fontWidget ?? "EB Garamond"; font.pixelSize: 14; font.bold: true }
                        }
                        Row {
                            spacing: 3
                            Repeater {
                                model: 24
                                Rectangle {
                                    width: 4; height: 9; radius: 1
                                    color: index < Math.round((sysData ? sysData.storagePercent : 0) / 100 * 24) ? desktopWidgets.tickActiveColor : desktopWidgets.tickInactiveColor
                                    border.color: index < Math.round((sysData ? sysData.storagePercent : 0) / 100 * 24) ? desktopWidgets.tickActiveBorder : "transparent"
                                    border.width: 0.5
                                }
                            }
                        }
                    }
                }

                // 6. THERMAL (Right)
                RowLayout {
                    spacing: 12
                    Rectangle {
                        width: 36; height: 36; radius: 18
                        color: cfg?.fg ?? "#3a2a20"
                        border.color: cfg?.gold ?? "#b5893f"; border.width: 1.5
                        Text { anchors.centerIn: parent; text: "󰔏"; color: cfg?.goldBright ?? "#d5a86d"; font.family: cfg?.fontIcons ?? "AurulentSansMNerdFontPropo"; font.pixelSize: 18 }
                    }
                    ColumnLayout {
                        spacing: 3
                        RowLayout {
                            spacing: 8
                            Text { text: "THERMAL"; color: cfg?.red ?? "#7a362d"; font.family: cfg?.fontTitle ?? "Cinzel"; font.pixelSize: 11; font.bold: true }
                            Text { text: (sysData ? sysData.temp1 : 0) + " °C"; color: cfg?.fg ?? "#3a2a20"; font.family: cfg?.fontWidget ?? "EB Garamond"; font.pixelSize: 14; font.bold: true }
                        }
                        Row {
                            spacing: 3
                            Repeater {
                                model: 24
                                Rectangle {
                                    width: 4; height: 9; radius: 1
                                    color: index < Math.round((sysData ? sysData.temp1 : 0) / 100 * 24) ? desktopWidgets.tickActiveColor : desktopWidgets.tickInactiveColor
                                    border.color: index < Math.round((sysData ? sysData.temp1 : 0) / 100 * 24) ? desktopWidgets.tickActiveBorder : "transparent"
                                    border.width: 0.5
                                }
                            }
                        }
                    }
                }

                // 7. SYS TEMP (Left)
                RowLayout {
                    spacing: 12
                    Rectangle {
                        width: 36; height: 36; radius: 18
                        color: cfg?.fg ?? "#3a2a20"
                        border.color: cfg?.gold ?? "#b5893f"; border.width: 1.5
                        Text { anchors.centerIn: parent; text: "󰔏"; color: cfg?.goldBright ?? "#d5a86d"; font.family: cfg?.fontIcons ?? "AurulentSansMNerdFontPropo"; font.pixelSize: 18 }
                    }
                    ColumnLayout {
                        spacing: 3
                        RowLayout {
                            spacing: 8
                            Text { text: "SYS TEMP"; color: cfg?.red ?? "#7a362d"; font.family: cfg?.fontTitle ?? "Cinzel"; font.pixelSize: 11; font.bold: true }
                            Text { text: (sysData ? sysData.temp2 : 0) + " °C"; color: cfg?.fg ?? "#3a2a20"; font.family: cfg?.fontWidget ?? "EB Garamond"; font.pixelSize: 14; font.bold: true }
                        }
                        Row {
                            spacing: 3
                            Repeater {
                                model: 24
                                Rectangle {
                                    width: 4; height: 9; radius: 1
                                    color: index < Math.round((sysData ? sysData.temp2 : 0) / 100 * 24) ? desktopWidgets.tickActiveColor : desktopWidgets.tickInactiveColor
                                    border.color: index < Math.round((sysData ? sysData.temp2 : 0) / 100 * 24) ? desktopWidgets.tickActiveBorder : "transparent"
                                    border.width: 0.5
                                }
                            }
                        }
                    }
                }

                // 8. COOLING (Right)
                RowLayout {
                    spacing: 12
                    Rectangle {
                        width: 36; height: 36; radius: 18
                        color: cfg?.fg ?? "#3a2a20"
                        border.color: cfg?.gold ?? "#b5893f"; border.width: 1.5
                        Text { anchors.centerIn: parent; text: "󰈐"; color: cfg?.goldBright ?? "#d5a86d"; font.family: cfg?.fontIcons ?? "AurulentSansMNerdFontPropo"; font.pixelSize: 18 }
                    }
                    ColumnLayout {
                        spacing: 3
                        RowLayout {
                            spacing: 8
                            Text { text: "COOLING"; color: cfg?.red ?? "#7a362d"; font.family: cfg?.fontTitle ?? "Cinzel"; font.pixelSize: 11; font.bold: true }
                            Text { text: (sysData ? Math.round(sysData.fan1Ratio * 100) : 0) + "%"; color: cfg?.fg ?? "#3a2a20"; font.family: cfg?.fontWidget ?? "EB Garamond"; font.pixelSize: 14; font.bold: true }
                        }
                        Row {
                            spacing: 3
                            Repeater {
                                model: 24
                                Rectangle {
                                    width: 4; height: 9; radius: 1
                                    color: index < Math.round((sysData ? sysData.fan1Ratio : 0) * 24) ? desktopWidgets.tickActiveColor : desktopWidgets.tickInactiveColor
                                    border.color: index < Math.round((sysData ? sysData.fan1Ratio : 0) * 24) ? desktopWidgets.tickActiveBorder : "transparent"
                                    border.width: 0.5
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
