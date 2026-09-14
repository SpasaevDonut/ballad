import QtQuick
import QtQuick.Layouts
import Quickshell

Item {
    id: root

    property color existingColor: "#939399"
    property int barWidth: 46
    property var calPopup: null

    SystemClock {
        id: clock
        precision: SystemClock.Seconds
    }

    Layout.preferredWidth: barWidth
    Layout.preferredHeight: 40
    Layout.alignment: Qt.AlignHCenter
    Layout.topMargin: 4
    Layout.bottomMargin: 0

    Text {
        id: clockText
        text: Qt.formatDateTime(clock.date, "hh\nmm")
        anchors.centerIn: parent
        color: root.existingColor
        font.pixelSize: 14
        font.bold: true
        font.family: "AurulentSansMNerdFontPropo"
        horizontalAlignment: Text.AlignHCenter
    }

    MouseArea {
        anchors.fill: parent
        cursorShape: Qt.PointingHandCursor
        onClicked: {
            leftBar.togglePopup(calPopup)
        }
    }
}
