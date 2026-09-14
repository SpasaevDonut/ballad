import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Services.Pipewire

ColumnLayout {
    id: root

    property color accentColor: "#C16FB8"
    property color inactiveColor: "#5e5e65"
    property color textColor: "#A3A0AF"
    property int barWidth: 46
    property var mixerPopup: null

    Layout.alignment: Qt.AlignHCenter
    Layout.preferredWidth: barWidth
    Layout.topMargin: 6
    Layout.bottomMargin: 6
    spacing: 4

    property real volume: Pipewire.defaultAudioSink?.audio?.volume ?? 0
    property bool isMuted: Pipewire.defaultAudioSink?.audio?.muted ?? false

    PwObjectTracker {
        objects: Pipewire.defaultAudioSink ? [Pipewire.defaultAudioSink] : []
    }

    // Audio icon container (strictly centered on the bar axis)
    Item {
        Layout.preferredWidth: root.barWidth
        Layout.preferredHeight: 20
        Layout.alignment: Qt.AlignHCenter

        Text {
            anchors.centerIn: parent
            text: root.isMuted ? "󰝟" : (root.volume > 0.5 ? "󰕾" : (root.volume > 0 ? "󰖀" : "󰕿"))
            color: root.isMuted ? root.inactiveColor : root.accentColor
            font.pixelSize: 18
            font.family: "AurulentSansMNerdFontPropo"
        }
    }

    // Volume LED block meter (8 blocks, height: 38px)
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
                        if (root.isMuted) return root.inactiveColor
                        return index >= 8 - Math.round(root.volume * 8) ? root.accentColor : root.inactiveColor
                    }
                }
            }
        }

        MouseArea {
            anchors.fill: parent
            anchors.margins: -4
            cursorShape: Qt.PointingHandCursor
            acceptedButtons: Qt.LeftButton | Qt.RightButton

            onClicked: (mouse) => {
                if (mouse.button === Qt.RightButton) {
                    leftBar.togglePopup(root.mixerPopup)
                } else if (Pipewire.defaultAudioSink?.audio) {
                    Pipewire.defaultAudioSink.audio.muted = !Pipewire.defaultAudioSink.audio.muted
                }
            }

            onWheel: (wheel) => {
                if (Pipewire.defaultAudioSink?.audio) {
                    var step = 0.05
                    var current = Pipewire.defaultAudioSink.audio.volume
                    if (wheel.angleDelta.y > 0) {
                        Pipewire.defaultAudioSink.audio.volume = Math.min(1.0, current + step)
                    } else {
                        Pipewire.defaultAudioSink.audio.volume = Math.max(0.0, current - step)
                    }
                }
            }
        }
    }

    // Volume percentage text
    Text {
        text: Math.round(root.volume * 100) + "%"
        color: root.isMuted ? root.inactiveColor : root.textColor
        font.pixelSize: 13
        font.bold: true
        font.family: "AurulentSansMNerdFontPropo"
        horizontalAlignment: Text.AlignHCenter
        Layout.preferredWidth: root.barWidth
        Layout.alignment: Qt.AlignHCenter
    }
}
