import QtQuick
import QtQuick.Layouts
import Quickshell

Item {
    id: root

    property color accentColor: "#C16FB8"
    property color inactiveColor: "#5e5e65"
    property color existingColor: "#939399"
    property color textColor: "#A3A0AF"
    property int barWidth: 46
    property int updatesCount: 0
    property bool isChecking: false
    property var updatesPopup: null

    Layout.alignment: Qt.AlignHCenter
    Layout.preferredWidth: barWidth
    Layout.preferredHeight: contentCol.implicitHeight
    Layout.topMargin: 4
    Layout.bottomMargin: 4

    ColumnLayout {
        id: contentCol
        anchors.centerIn: parent
        width: root.barWidth
        spacing: 2

        // Icon container strictly centered on the bar axis
        Item {
            Layout.preferredWidth: root.barWidth
            Layout.preferredHeight: 20
            Layout.alignment: Qt.AlignHCenter

            Text {
                id: iconText
                anchors.centerIn: parent
                text: "󰮯"
                color: mouseArea.containsMouse ? root.accentColor : (root.updatesCount > 0 ? root.accentColor : root.inactiveColor)
                font.pixelSize: 18
                font.family: "AurulentSansMNerdFontPropo"

                Behavior on color { ColorAnimation { duration: 150 } }

                RotationAnimation on rotation {
                    running: root.isChecking
                    from: 0
                    to: 360
                    loops: Animation.Infinite
                    duration: 1000
                }
            }
        }

        // Count display (shown when updates > 0)
        Text {
            visible: root.updatesCount > 0
            text: root.updatesCount > 999 ? "999+" : root.updatesCount.toString()
            color: mouseArea.containsMouse ? root.accentColor : root.textColor
            font.pixelSize: 11
            font.bold: true
            font.family: "monospace"
            horizontalAlignment: Text.AlignHCenter
            Layout.preferredWidth: root.barWidth
            Layout.alignment: Qt.AlignHCenter

            Behavior on color { ColorAnimation { duration: 150 } }
        }
    }

    MouseArea {
        id: mouseArea
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: {
            leftBar.togglePopup(updatesPopup)
        }
    }
}
