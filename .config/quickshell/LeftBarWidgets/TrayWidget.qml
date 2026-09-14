import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import Quickshell
import Quickshell.Services.SystemTray
import Quickshell.Wayland
import Quickshell.Widgets

ColumnLayout {
    id: root

    spacing: 6
    Layout.alignment: Qt.AlignHCenter
    Layout.topMargin: 6
    Layout.bottomMargin: 6
    visible: trayRepeater.count > 0

    Repeater {
        id: trayRepeater
        model: SystemTray.items
        delegate: Item {
            width: 24
            height: 24

            IconImage {
                anchors.centerIn: parent
                source: modelData.icon
                implicitSize: 18
            }

            MouseArea {
                id: trayMouse
                anchors.fill: parent
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
