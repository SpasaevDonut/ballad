import QtQuick
import QtQuick.Layouts
import Quickshell

Item {
    id: root

    property color accentColor: "#C16FB8"
    property color inactiveColor: "#5e5e65"
    property color textColor: "#A3A0AF"
    property real batteryPercent: 0
    property string batteryStatus: "Unknown"
    property int barWidth: 46
    property var batteryPopup: null

    Layout.alignment: Qt.AlignHCenter
    Layout.preferredWidth: barWidth
    Layout.preferredHeight: contentCol.implicitHeight
    Layout.topMargin: 6
    Layout.bottomMargin: 6

    property real capacity: batteryPercent
    property string status: batteryStatus

    ColumnLayout {
        id: contentCol
        anchors.centerIn: parent
        width: root.barWidth
        spacing: 4

        // Battery icon container (strictly centered on the bar axis)
        Item {
            Layout.preferredWidth: root.barWidth
            Layout.preferredHeight: 20
            Layout.alignment: Qt.AlignHCenter

            Text {
                anchors.centerIn: parent
                text: root.status === "Charging" ? "⚡" : "󰁹"
                color: root.accentColor
                font.pixelSize: 18
                font.family: "AurulentSansMNerdFontPropo"
            }
        }

        // Battery LED block meter (8 blocks, height: 38px)
        Item {
            Layout.preferredWidth: root.barWidth
            Layout.preferredHeight: 38
            Layout.alignment: Qt.AlignHCenter

            Column {
                anchors.centerIn: parent
                spacing: 2

                Repeater {
                    model: 8
                    Rectangle {
                        width: 14
                        height: 3
                        color: {
                            let activeBlocks = Math.round((root.capacity / 100.0) * 8)
                            if (index >= 8 - activeBlocks) {
                                if (root.status === "Charging") return root.accentColor
                                if (root.capacity <= 20) return root.accentColor
                                return root.accentColor
                            }
                            return root.inactiveColor
                        }
                    }
                }
            }
        }

        // Battery percentage text
        Text {
            text: Math.round(root.capacity) + "%"
            color: root.status === "Charging" ? root.textColor : (root.capacity <= 20 ? root.accentColor : root.textColor)
            font.pixelSize: 13
            font.bold: true
            font.family: "AurulentSansMNerdFontPropo"
            horizontalAlignment: Text.AlignHCenter
            Layout.preferredWidth: root.barWidth
            Layout.alignment: Qt.AlignHCenter
        }
    }

    MouseArea {
        id: mouseArea
        anchors.fill: parent
        cursorShape: Qt.PointingHandCursor
        onClicked: {
            leftBar.togglePopup(batteryPopup)
        }
    }
}
