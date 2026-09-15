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
    implicitWidth: 330
    implicitHeight: 460
    
    property double lastCloseTime: 0
    property QtObject popupSysData: null
    property QtObject popupCfg: null
    property var anchorItem: null

    SystemClock {
        id: clock
        precision: SystemClock.Seconds
    }

    property int today: clock.date.getDate()
    property int daysInMonth: new Date(clock.date.getFullYear(), clock.date.getMonth() + 1, 0).getDate()
    property int leadingBlanks: {
        var firstDay = new Date(clock.date.getFullYear(), clock.date.getMonth(), 1).getDay()
        return firstDay === 0 ? 6 : firstDay - 1
    }

    anchor { 
        item: anchorItem
        edges: Edges.Right | Edges.Bottom
        gravity: Edges.Right | Edges.Top 
    }

    MouseArea {
        anchors.fill: parent
        onClicked: {
            lastCloseTime = Date.now()
            root.visible = false
        }
    }

    Rectangle {
        id: calRect
        anchors.bottom: parent.bottom
        anchors.left: parent.left
        anchors.bottomMargin: 10
        anchors.leftMargin: 10    

        width: 280
        height: 410
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

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 16
            spacing: 6

            Text {
                id: popupTime
                text: Qt.formatDateTime(clock.date, "hh:mm:ss")
                color: popupCfg?.fg ?? "#3a2a20"
                font.pixelSize: 32
                font.bold: true
                font.family: popupCfg?.fontTitle ?? "Cinzel"
                Layout.alignment: Qt.AlignHCenter
            }
            
            Text {
                id: popupDate
                text: Qt.formatDateTime(clock.date, "yyyy-MM-dd")
                color: popupCfg?.gold ?? "#b5893f"
                font.pixelSize: 14
                font.family: popupCfg?.fontWidget ?? "EB Garamond"
                Layout.alignment: Qt.AlignHCenter
            }

            RowLayout {
                Layout.alignment: Qt.AlignHCenter
                spacing: 6
                
                Text { 
                    text: "SYS_TIME"
                    font.pixelSize: 11
                    color: popupCfg?.redDim ?? "#4c2d26"
                    font.family: popupCfg?.fontTitle ?? "Cinzel" 
                }
                
                Text {
                    id: popupRegion
                    color: popupCfg?.fg ?? "#3a2a20"
                    font.pixelSize: 12
                    font.bold: true
                    font.family: popupCfg?.fontWidget ?? "EB Garamond"
                    
                    Component.onCompleted: { 
                        try { 
                            text = Intl.DateTimeFormat().resolvedOptions().timeZone 
                        } catch(e) { 
                            text = "LOCAL" 
                        } 
                    }
                }
            }

            // Filigree divider
            RowLayout {
                Layout.fillWidth: true
                Layout.topMargin: 4
                Layout.bottomMargin: 4
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

            GridLayout {
                columns: 7
                Layout.alignment: Qt.AlignHCenter
                rowSpacing: 6
                columnSpacing: 8
                
                Repeater { 
                    model: ["MO", "TU", "WE", "TH", "FR", "SA", "SU"]
                    Text { 
                        text: modelData
                        color: (index > 4) ? (popupCfg?.red ?? "#7a362d") : (popupCfg?.fg ?? "#3a2a20")
                        font.pixelSize: 12
                        font.bold: true
                        font.family: popupCfg?.fontTitle ?? "Cinzel"
                        horizontalAlignment: Text.AlignHCenter
                        Layout.fillWidth: true 
                    } 
                }
                
                Repeater {
                    model: root.leadingBlanks
                    Item { 
                        Layout.preferredWidth: 26
                        Layout.preferredHeight: 26 
                    } 
                }
                
                Repeater {
                    model: root.daysInMonth
                    Rectangle {
                        Layout.preferredWidth: 26
                        Layout.preferredHeight: 26
                        radius: 2
                        color: (index + 1 === root.today) ? (popupCfg?.goldBright ?? "#d5a86d") : "transparent"
                        border.color: (index + 1 === root.today) ? (popupCfg?.gold ?? "#b5893f") : (popupCfg?.bg2 ?? "#d5c4a1")
                        border.width: 1

                        Text {
                            anchors.centerIn: parent
                            text: index + 1
                            color: (index + 1 === root.today) ? (popupCfg?.fg ?? "#3a2a20") : (popupCfg?.fg ?? "#3a2a20")
                            font.pixelSize: 13
                            font.bold: (index + 1 === root.today)
                            font.family: popupCfg?.fontWidget ?? "EB Garamond"
                        }
                    }
                }
            }
            
            Item { 
                Layout.fillHeight: true 
            } 
        }
    }

    // Shadow
    DropShadow {
        source: calRect
        anchors.fill: calRect
        horizontalOffset: 12
        verticalOffset: 12
        radius: 14
        samples: 16
        color: "#353a2a20"
        transparentBorder: true
    }

    onVisibleChanged: {
        if (visible) {
            calRect.visible = true
            calRect.forceActiveFocus()
        }
    }
}
