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

        width: 270
        height: 400
        color: popupCfg.bgColor

        MouseArea { 
            anchors.fill: parent 
        }

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 15
            spacing: 6

            Text {
                id: popupTime
                text: Qt.formatDateTime(clock.date, "hh:mm:ss")
                color: popupCfg.accentColor
                font.pixelSize: 36
                font.bold: true
                font.family: "monospace"
                Layout.alignment: Qt.AlignHCenter
            }
            
            Text {
                id: popupDate
                text: Qt.formatDateTime(clock.date, "yyyy-MM-dd")
                color: popupCfg.textColor
                font.pixelSize: 13
                font.family: "monospace"
                Layout.alignment: Qt.AlignHCenter
            }

            RowLayout {
                Layout.alignment: Qt.AlignHCenter
                spacing: 6
                
                Text { 
                    text: "SYS_TIME"
                    font.pixelSize: 12
                    color: popupCfg.inactiveColor
                    font.family: "monospace" 
                }
                
                Text {
                    id: popupRegion
                    color: popupCfg.accentColor
                    font.pixelSize: 12
                    font.bold: true
                    font.family: "monospace"
                    
                    Component.onCompleted: { 
                        try { 
                            text = Intl.DateTimeFormat().resolvedOptions().timeZone 
                        } catch(e) { 
                            text = "LOCAL" 
                        } 
                    }
                }
            }

            Rectangle { 
                Layout.fillWidth: true
                height: 2
                color: popupCfg.accentColor
                Layout.topMargin: 10
                Layout.bottomMargin: 10 
            }

            GridLayout {
                columns: 7
                Layout.alignment: Qt.AlignHCenter
                rowSpacing: 8
                columnSpacing: 10
                
                Repeater { 
                    model: ["MO", "TU", "WE", "TH", "FR", "SA", "SU"]
                    Text { 
                        text: modelData
                        color: (index > 4) ? popupCfg.accentColor : popupCfg.inactiveColor
                        font.pixelSize: 13
                        font.bold: true
                        horizontalAlignment: Text.AlignHCenter
                        Layout.fillWidth: true 
                    } 
                }
                
                Repeater {
                    model: root.leadingBlanks
                    Item { 
                        Layout.preferredWidth: 24
                        Layout.preferredHeight: 24 
                    } 
                }
                
                Repeater {
                    model: root.daysInMonth
                    Rectangle {
                        Layout.preferredWidth: 24
                        Layout.preferredHeight: 24
                        color: (index + 1 === root.today) ? popupCfg.accentColor : "transparent"
                        border.color: (index + 1 === root.today) ? "transparent" : popupCfg.inactiveColor
                        border.width: 1

                        Text {
                            anchors.centerIn: parent
                            text: index + 1
                            color: (index + 1 === root.today) ? popupCfg.bgColor : popupCfg.textColor
                            font.pixelSize: 13
                            font.family: "monospace"
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
        horizontalOffset: 15
        verticalOffset: 15
        radius: 15
        samples: 16
        color: "#A6000000"
        transparentBorder: true
    }

    onVisibleChanged: {
        if (visible) {
            calRect.visible = true
            calRect.forceActiveFocus()
        }
    }
}
