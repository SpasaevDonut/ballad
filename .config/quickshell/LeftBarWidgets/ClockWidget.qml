import QtQuick
import QtQuick.Layouts
import Quickshell

Item {
    id: root

    property color existingColor: "#3a2a20"
    property color accentColor: "#b5893f"
    property string fontTitle: "Cinzel"
    property int barWidth: 46
    property var calPopup: null

    SystemClock {
        id: clock
        precision: SystemClock.Seconds
    }

    Layout.preferredWidth: barWidth
    Layout.preferredHeight: 38
    Layout.alignment: Qt.AlignHCenter
    Layout.topMargin: 2
    Layout.bottomMargin: 0

    Text {
        id: clockText
        text: Qt.formatDateTime(clock.date, "hh\nmm")
        anchors.centerIn: parent
        color: mouseArea.containsMouse ? root.accentColor : root.existingColor
        font.pixelSize: 13
        font.bold: true
        font.family: root.fontTitle
        lineHeight: 0.92
        horizontalAlignment: Text.AlignHCenter
        Behavior on color { ColorAnimation { duration: 150 } }
    }

    MouseArea {
        id: mouseArea
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: {
            leftBar.togglePopup(calPopup)
        }
    }
}
