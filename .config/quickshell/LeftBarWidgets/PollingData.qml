import QtQuick
import Quickshell.Io

Item {
    id: pollingControl
    property var command: []
    property int interval: 2000
    property bool triggeredOnStart: true

    signal dataReady(string data)

    Process {
        id: process
        command: pollingControl.command
        stdout: SplitParser {
            onRead: data => pollingControl.dataReady(data)
        }
    }

    Timer {
        interval: pollingControl.interval
        running: pollingControl.command.length > 0 && pollingControl.command[0] !== undefined
        repeat: true
        triggeredOnStart: pollingControl.triggeredOnStart
        onTriggered: {
            if (pollingControl.command.length > 0) {
                process.running = false
                process.running = true
            }
        }
    }

    Component.onDestruction: process.running = false
}
