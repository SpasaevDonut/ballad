import QtQuick
import QtQuick.Layouts
import Qt5Compat.GraphicalEffects
import Quickshell

Item {
    property color accentColor: "#C16FB8"
    property color existingColor: "#939399"
    property var powerPopup: null

    Layout.alignment: Qt.AlignHCenter
    Layout.preferredWidth: 46
    Layout.preferredHeight: 36

    Image {
        id: defyImage
        source: "../defy.svg"
        sourceSize.width: 26
        sourceSize.height: 26
        anchors.centerIn: parent
    }

    ColorOverlay {
        source: defyImage
        anchors.fill: defyImage
        color: archMouseArea.containsMouse ? accentColor : existingColor
        Behavior on color { ColorAnimation { duration: 150 } }
    }

    MouseArea {
        id: archMouseArea
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: {
            leftBar.togglePopup(powerPopup)
        }
    }
}
