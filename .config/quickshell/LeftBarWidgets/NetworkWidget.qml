import QtQuick
import QtQuick.Layouts
import Quickshell

Item {
    id: root

    property color accentColor: "#b5893f"
    property color inactiveColor: "#d5c4a1"
    property color textColor: "#3a2a20"
    property string fontWidget: "EB Garamond"
    property int barWidth: 46
    property string netIcon: "󰤭"
    property bool isConnected: false
    property string wifiSsid: ""
    property int wifiSignal: 0
    property bool wifiEnabled: true
    property real dlRatio: 0
    property real ulRatio: 0
    property string downloadStr: "0B"
    property string uploadStr: "0B"
    property var wifiPopup: null

    Layout.preferredWidth: barWidth
    Layout.preferredHeight: 100
    Layout.alignment: Qt.AlignHCenter
    Layout.topMargin: 2
    Layout.bottomMargin: 6

    ColumnLayout {
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.verticalCenter: parent.verticalCenter
        width: root.barWidth
        spacing: 4

        // Fixed-width icon container (strictly centered on the bar axis)
        Item {
            Layout.preferredWidth: root.barWidth
            Layout.preferredHeight: 20
            Layout.alignment: Qt.AlignHCenter

            Text {
                id: iconText
                anchors.centerIn: parent
                text: root.netIcon
                color: !root.wifiEnabled ? root.inactiveColor : (root.isConnected ? root.accentColor : root.inactiveColor)
                font.pixelSize: 18
                font.family: "AurulentSansMNerdFontPropo"
            }
        }

        // Speed meters (LED block bars)
        RowLayout {
            spacing: 6
            Layout.alignment: Qt.AlignHCenter

            // Download bar (8 blocks)
            Column {
                spacing: 2
                Repeater {
                    model: 8
                    Rectangle {
                        width: 10
                        height: 3
                        color: index >= 8 - Math.round(root.dlRatio * 8) ? root.accentColor : root.inactiveColor
                    }
                }
            }

            // Upload bar (8 blocks)
            Column {
                spacing: 2
                Repeater {
                    model: 8
                    Rectangle {
                        width: 10
                        height: 3
                        color: index >= 8 - Math.round(root.ulRatio * 8) ? root.textColor : root.inactiveColor
                    }
                }
            }
        }

        // Speed text with fixed width
        ColumnLayout {
            Layout.preferredWidth: root.barWidth
            Layout.alignment: Qt.AlignHCenter
            spacing: 0

            Text {
                text: root.downloadStr
                color: root.accentColor
                font.pixelSize: 11
                font.family: root.fontWidget
                horizontalAlignment: Text.AlignHCenter
                Layout.preferredWidth: root.barWidth
                Layout.alignment: Qt.AlignHCenter
            }

            Text {
                text: root.uploadStr
                color: root.textColor
                font.pixelSize: 11
                font.family: root.fontWidget
                horizontalAlignment: Text.AlignHCenter
                Layout.preferredWidth: root.barWidth
                Layout.alignment: Qt.AlignHCenter
            }
        }
    }

    MouseArea {
        id: mouseArea
        anchors.fill: parent
        cursorShape: Qt.PointingHandCursor
        acceptedButtons: Qt.LeftButton | Qt.RightButton

        onClicked: (mouse) => {
            if (mouse.button === Qt.LeftButton) {
                leftBar.togglePopup(wifiPopup)
            } else if (mouse.button === Qt.RightButton) {
                leftBar.closeAllPopupsExcept(wifiPopup)
                if (wifiPopup) {
                    wifiPopup.visible = true
                    wifiPopup.triggerScan(true)
                }
            }
        }
    }
}
