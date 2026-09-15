import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import Quickshell
import Quickshell.Services.SystemTray
import Quickshell.Wayland
import Quickshell.Widgets

ColumnLayout {
    id: root

    property color badgeColor: "#3a2a20"
    property color badgeHoverColor: "#7a362d"
    property color borderColor: "#b5893f"
    property color borderHoverColor: "#d5a86d"
    property color shadowColor: "#353a2a20"

    spacing: 6
    Layout.alignment: Qt.AlignHCenter
    Layout.topMargin: 4
    Layout.bottomMargin: 4
    visible: trayRepeater.count > 0

    Repeater {
        id: trayRepeater
        model: SystemTray.items
        delegate: Item {
            width: 28
            height: 28
            Layout.alignment: Qt.AlignHCenter

            // Warm engraved drop shadow
            Rectangle {
                anchors.centerIn: parent
                anchors.verticalCenterOffset: 1
                width: 26
                height: 26
                radius: 13
                color: root.shadowColor
            }

            // Victorian Medallion Plate (provides high-contrast dark background for white/light icons)
            Rectangle {
                id: badgeBg
                anchors.centerIn: parent
                width: 26
                height: 26
                radius: 13
                color: trayMouse.containsMouse ? root.badgeHoverColor : root.badgeColor
                border.width: 1
                border.color: trayMouse.containsMouse ? root.borderHoverColor : root.borderColor
                scale: trayMouse.pressed ? 0.93 : (trayMouse.containsMouse ? 1.08 : 1.0)

                Behavior on color { ColorAnimation { duration: 150 } }
                Behavior on border.color { ColorAnimation { duration: 150 } }
                Behavior on scale { NumberAnimation { duration: 150; easing.type: Easing.OutQuad } }

                IconImage {
                    anchors.centerIn: parent
                    source: modelData.icon
                    implicitSize: 16
                }
            }

            MouseArea {
                id: trayMouse
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                acceptedButtons: Qt.LeftButton | Qt.RightButton | Qt.MiddleButton

                onClicked: (mouse) => {
                    if (mouse.button === Qt.LeftButton) {
                        if (typeof modelData.activate === "function") modelData.activate()
                    } else if (mouse.button === Qt.MiddleButton) {
                        if (typeof modelData.secondaryActivate === "function") modelData.secondaryActivate()
                    } else if (mouse.button === Qt.RightButton) {
                        menuAnchor.open()
                    }
                }

                onWheel: (wheel) => {
                    if (wheel.angleDelta.y !== 0 && typeof modelData.scroll === "function") {
                        modelData.scroll(wheel.angleDelta.y, "vertical")
                    }
                }
            }

            QsMenuAnchor {
                id: menuAnchor
                menu: modelData.menu
                anchor {
                    item: trayMouse
                    edges: Edges.Right | Edges.Bottom
                    gravity: Edges.Right | Edges.Top
                }
            }
        }
    }
}
