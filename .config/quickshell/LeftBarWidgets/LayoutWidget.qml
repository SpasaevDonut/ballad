import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

Item {
    id: root

    property color textColor: "#b5893f"
    property string fontTitle: "Cinzel"
    property int barWidth: 46

    Layout.preferredWidth: barWidth
    Layout.preferredHeight: 18
    Layout.alignment: Qt.AlignHCenter
    Layout.topMargin: 3
    Layout.bottomMargin: 3

    property var layouts: []
    property int currentIndex: 0

    function updateLayouts(names, idx) {
        layouts = names
        currentIndex = idx
    }

    function switchLayout(idx) {
        currentIndex = idx
    }

    Text {
        anchors.centerIn: parent
        text: root.layouts.length > 0 ? (root.layouts[root.currentIndex].includes("Russian") ? "RU" : "EN") : "--"
        color: root.textColor
        font.pixelSize: 11
        font.bold: true
        font.family: root.fontTitle
    }

    Process {
        id: layoutProcess
        command: ["niri", "msg", "-j", "keyboard-layouts"]
        running: true
        stdout: SplitParser {
            onRead: data => {
                try {
                    let obj = JSON.parse(data)
                    root.layouts = obj.names
                    root.currentIndex = obj.current_idx
                    layoutProcess.running = false
                } catch(e) { console.warn("layout parse:", e) }
            }
        }
    }
}
