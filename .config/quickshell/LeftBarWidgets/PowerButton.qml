import QtQuick
import QtQuick.Layouts
import Quickshell

Item {
    property color accentColor: "#7a362d"
    property color existingColor: "#b5893f"
    property string fontOrnaments: "Bodoni Ornaments ITC"
    property var powerPopup: null

    Layout.alignment: Qt.AlignHCenter
    Layout.preferredWidth: 46
    Layout.preferredHeight: 36

    // Victorian Rosette Logo: ')' (U+0029) inactive, '*' (U+002A) hover
    Text {
        id: logoText
        anchors.topMargin: 4
        anchors.centerIn: parent
        text: archMouseArea.containsMouse ? "*" : ")"
        font.family: fontOrnaments
        font.pixelSize: 31
        color: archMouseArea.containsMouse ? accentColor : existingColor
        scale: archMouseArea.pressed ? 0.92 : (archMouseArea.containsMouse ? 1.08 : 1.0)

        Behavior on color { ColorAnimation { duration: 150 } }
        Behavior on scale { NumberAnimation { duration: 150; easing.type: Easing.OutQuad } }
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
