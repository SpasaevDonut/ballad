import QtQuick
import QtQuick.Layouts

Item {
    id: root

    property color sepColor: "#d5c4a1"
    property color sepLightColor: "#f1e5ce"
    property color ornamentColor: "#b5893f"
    property string fontOrnaments: "Bodoni Ornaments ITC"
    property int barWidth: 46

    property string text: "\u00B7"
    property int fontSize: 14
    property int yOffset: -5

    Layout.alignment: Qt.AlignHCenter
    Layout.preferredWidth: barWidth
    Layout.preferredHeight: 14
    Layout.topMargin: 2
    Layout.bottomMargin: 2

    // Visually centered Bodoni divider rule (compensated for font baseline asymmetry)
    Text {
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.verticalCenter: parent.verticalCenter
        anchors.verticalCenterOffset: root.yOffset
        text: root.text
        font.family: root.fontOrnaments
        font.pixelSize: root.fontSize
        color: root.ornamentColor
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
    }
}
