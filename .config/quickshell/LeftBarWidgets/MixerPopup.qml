import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import Qt5Compat.GraphicalEffects
import Quickshell
import Quickshell.Wayland
import Quickshell.Services.Pipewire

PopupWindow {
    id: root

    property QtObject popupCfg: null
    property var anchorItem: null
    property double lastCloseTime: 0

    visible: false
    color: "transparent"
    implicitWidth: 340
    implicitHeight: 480

    property var sinks: Pipewire.ready ? Pipewire.nodes.values.filter(n => {
        if (!n.audio || !n.isStream) return false
        var lower = n.name.toLowerCase()
        var descLower = (n.description ?? "").toLowerCase()
        return !lower.includes("pulse") && !descLower.includes("pulse")
    }) : []

    anchor {
        item: anchorItem
        edges: Edges.Right
        gravity: Edges.Right
    }

    Rectangle {
        id: mixerRect
        anchors.verticalCenter: parent.verticalCenter
        anchors.left: parent.left
        anchors.leftMargin: 10

        width: 280
        height: 420
        color: popupCfg.bgColor

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 15
            spacing: 8

            Text {
		    text: "Mixer"
                color: popupCfg.textColor
                font.pixelSize: 13
                font.bold: true
                Layout.alignment: Qt.AlignHCenter
            }

            Rectangle {
                Layout.fillWidth: true
                height: 2
                color: popupCfg.accentColor
            }

            ScrollView {
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true

                ColumnLayout {
                    width: parent.width
                    spacing: 6

                    Repeater {
                        model: root.sinks

                        delegate: Item {
                            width: parent?.width ?? 260
                            height: 60
                            required property var modelData

                            PwObjectTracker {
                                objects: [modelData]
                            }

                            MouseArea {
                                anchors.fill: parent
                                acceptedButtons: Qt.NoButtons
                                onWheel: (wheel) => {
                                    if (!modelData.ready || !modelData.audio) return
                                    var step = 0.05
                                    var vol = modelData.audio.volume
                                    if (wheel.angleDelta.y > 0) {
                                        modelData.audio.volume = Math.min(1.0, vol + step)
                                    } else {
                                        modelData.audio.volume = Math.max(0.0, vol - step)
                                    }
                                }
                            }

                            ColumnLayout {
                                anchors.fill: parent
                                anchors.margins: 8
                                spacing: 4

                                RowLayout {
                                    Layout.fillWidth: true
                                    spacing: 6

                                    Text {
                                        text: modelData.ready ? (modelData.properties?.["application.name"] || modelData.description || modelData.name || "Stream") : "..."
                                        color: popupCfg.textColor
                                        font.pixelSize: 12
                                        font.bold: true
                                        elide: Text.ElideRight
                                        Layout.fillWidth: true
                                    }

                                    Text {
                                        text: modelData.isStream ? "APP" : (modelData.isSink ? "SINK" : "SRC")
                                        color: popupCfg.inactiveColor
                                        font.pixelSize: 9
                                        font.bold: true
                                    }
                                }

                                RowLayout {
                                    Layout.fillWidth: true
                                    spacing: 6

                                    Slider {
                                        id: volSlider
                                        from: 0
                                        to: 1
                                        stepSize: 0.01
                                        Layout.fillWidth: true
                                        enabled: modelData.ready

                                        value: modelData.ready && modelData.audio ? modelData.audio.volume : 0

                                        onMoved: {
                                            if (modelData.ready && modelData.audio) modelData.audio.volume = value
                                        }
                                    }

                                    Text {
                                        text: modelData.ready && modelData.audio ? Math.round(modelData.audio.volume * 100) + "%" : "0%"
                                        color: popupCfg.textColor
                                        font.pixelSize: 11
                                        font.family: "AurulentSansMNerdFontPropo"
                                        horizontalAlignment: Text.AlignRight
                                        Layout.preferredWidth: 36
                                    }

                                    Rectangle {
                                        width: 24
                                        height: 24
                                        radius: 4
                                        color: muteMouse.containsMouse ? popupCfg.inactiveColor : "transparent"
                                        border.color: modelData.ready && modelData.audio && modelData.audio.muted ? popupCfg.accentColor : popupCfg.inactiveColor
                                        border.width: 1

                                        Text {
                                            anchors.centerIn: parent
                                            text: modelData.ready && modelData.audio && modelData.audio.muted ? "󰝟" : "󰕾"
                                            color: modelData.ready && modelData.audio && modelData.audio.muted ? popupCfg.accentColor : popupCfg.textColor
                                            font.pixelSize: 12
                                        }

                                        MouseArea {
                                            id: muteMouse
                                            anchors.fill: parent
                                            hoverEnabled: true
                                            cursorShape: Qt.PointingHandCursor
                                            onClicked: {
                                                if (modelData.ready && modelData.audio) modelData.audio.muted = !modelData.audio.muted
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Shadow
    DropShadow {
        source: mixerRect
        anchors.fill: mixerRect
        horizontalOffset: 15
        verticalOffset: 15
        radius: 15
        samples: 16
        color: "#A6000000"
        transparentBorder: true
    }
}
