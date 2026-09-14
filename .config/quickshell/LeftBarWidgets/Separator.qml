import QtQuick
import QtQuick.Layouts

Column {
    property color sepColor: "#2F2F38"
    property color sepLightColor: "#505050"
    property int barWidth: 46

    Layout.alignment: Qt.AlignHCenter
    Layout.topMargin: 4
    Layout.bottomMargin: 4
    spacing: 0

    Rectangle {
        width: barWidth - 14
        height: 2
        color: sepColor
    }
    Rectangle {
        width: barWidth - 14
        height: 1
        color: sepLightColor
    }
}
