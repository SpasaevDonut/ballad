import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

ColumnLayout {
    id: root

    property color accentColor: "#b5893f"
    property color inactiveColor: "#d5c4a1"
    property color existingColor: "#7a362d"
    property color textColor: "#3a2a20"
    property color sepColor: "#d5c4a1"
    property color sepLightColor: "#f1e5ce"
    property string fontRoman: "Cinzel Decorative"
    property string fontTitle: "Cinzel"
    property string fontWidget: "EB Garamond"
    property int barWidth: 46
    property var winMenu: null

    readonly property var romanNumerals: ["Ⅰ", "Ⅱ", "Ⅲ", "Ⅳ", "Ⅴ", "Ⅵ", "Ⅶ", "Ⅷ", "Ⅸ", "Ⅹ"]

    spacing: 0
    Layout.alignment: Qt.AlignHCenter

    property var workspaces: []
    property int activeWorkspaceId: -1

    signal keyboardLayoutsChanged(var names, int currentIdx)
    signal keyboardLayoutSwitched(int idx)

    function updateWorkspaces(wArray) {
        wArray.sort((a, b) => {
            let labelA = a.name ? a.name : a.id.toString()
            let labelB = b.name ? b.name : b.id.toString()
            let numA = parseInt(labelA)
            let numB = parseInt(labelB)
            if (!isNaN(numA) && !isNaN(numB)) return numA - numB
            return labelA > labelB ? 1 : -1
        })

        let arr = []
        for (let i = 0; i < 10; i++) {
            if (i < wArray.length) {
                let w = wArray[i]
                w.exists = true
                if (w.is_focused) activeWorkspaceId = w.id
                arr.push(w)
            } else {
                arr.push({ name: (i + 1).toString(), is_focused: false, exists: false, id: -1 })
            }
        }
        workspaces = arr
    }

    Process {
        command: ["niri", "msg", "-j", "workspaces"]
        running: true
        stdout: SplitParser {
            onRead: data => {
                try {
                    let w = JSON.parse(data)
                    if (Array.isArray(w)) root.updateWorkspaces(w)
                } catch(e) { console.warn("workspaces parse:", e) }
            }
        }
    }

    Process {
        command: ["niri", "msg", "-j", "event-stream"]
        running: true
        stdout: SplitParser {
            onRead: data => {
                try {
                    let event = JSON.parse(data)

                    if (event.KeyboardLayoutsChanged) {
                        root.keyboardLayoutsChanged(event.KeyboardLayoutsChanged.keyboard_layouts.names, event.KeyboardLayoutsChanged.keyboard_layouts.current_idx)
                    } else if (event.KeyboardLayoutSwitched) {
                        root.keyboardLayoutSwitched(event.KeyboardLayoutSwitched.idx)
                    }

                    if (event.WorkspacesChanged) {
                        root.updateWorkspaces(event.WorkspacesChanged.workspaces)
                    } else if (event.WorkspaceActivated) {
                        root.activeWorkspaceId = event.WorkspaceActivated.id
                        let w = root.workspaces
                        for (let i = 0; i < w.length; i++) {
                            if (w[i].id !== -1) w[i].is_focused = (w[i].id === event.WorkspaceActivated.id)
                        }
                        root.workspaces = Array.from(w)
                    }

                    if (event.WindowOpened || event.WindowClosed || event.WindowFocusChanged || event.WorkspaceActivated || event.WorkspaceActiveWindowChanged) {
                        windowsProcess.running = false
                        windowsProcess.running = true
                    }
                } catch(e) { console.warn("event-stream parse:", e) }
            }
        }
        Component.onDestruction: running = false
    }

    Process {
        id: clickProc
        running: false
    }

    // --- WORKSPACES (Pocket Watch Dial Roman Numerals) ---
    GridLayout {
        columns: 2
        rowSpacing: 4
        columnSpacing: 4
        Layout.alignment: Qt.AlignHCenter
        Layout.topMargin: 4
        Layout.bottomMargin: 4

        Repeater {
            model: root.workspaces
            Item {
                width: 17
                height: 22

                Rectangle {
                    anchors.fill: parent
                    radius: 3
                    color: modelData.is_focused ? "#30d5a86d" : (modelData.exists ? "#15b5893f" : "transparent")
                    border.width: 1
                    border.color: modelData.is_focused ? root.accentColor : (modelData.exists ? "#80d5c4a1" : "transparent")
                    Behavior on border.color { ColorAnimation { duration: 150 } }
                    Behavior on color { ColorAnimation { duration: 150 } }
                }

                Text {
                    anchors.centerIn: parent
                    text: root.romanNumerals[index] ?? (index + 1).toString()
                    font.family: root.fontRoman
                    font.pixelSize: 11
                    font.bold: modelData.is_focused
                    color: {
                        if (modelData.is_focused) return "#b5893f"
                        if (modelData.exists) return root.textColor
                        return root.inactiveColor
                    }
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                    Behavior on color { ColorAnimation { duration: 150 } }
                }

                // Window presence indicator dot
                Rectangle {
                    visible: modelData.exists && !modelData.is_focused
                    width: 3
                    height: 3
                    radius: 1.5
                    color: root.existingColor
                    anchors.bottom: parent.bottom
                    anchors.bottomMargin: 2
                    anchors.horizontalCenter: parent.horizontalCenter
                }

                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.PointingHandCursor
                    hoverEnabled: true
                    onClicked: {
                        clickProc.command = ["niri", "msg", "action", "focus-workspace", modelData.name ? modelData.name : modelData.id.toString()]
                        clickProc.running = true
                    }
                }
            }
        }
    }

    Separator {
        sepColor: root.sepColor
        sepLightColor: root.sepLightColor
        barWidth: root.barWidth
        visible: windowsLayout.count > 0
    }

    // --- OPEN WINDOWS ---
    ListView {
        id: windowsLayout
        Layout.alignment: Qt.AlignHCenter
        Layout.preferredWidth: root.barWidth
        Layout.fillHeight: true
        spacing: 8
        clip: true
        boundsBehavior: Flickable.StopAtBounds

        property var allWindows: []

        property var groupedWindowsArray: {
            let activeWs = root.activeWorkspaceId
            let filtered = allWindows.filter(w => w.workspace_id === activeWs)
            filtered.sort((a, b) => a.id - b.id)

            let groups = []
            for (let i = 0; i < filtered.length; i++) {
                let w = filtered[i]
                let name = w.app_id ? w.app_id : (w.title ? w.title : "APP")
                let parts = name.split('.')
                let shortName = parts[parts.length - 1].toUpperCase()

                if (shortName.includes("KITTY") || shortName.includes("ALACRITTY") || shortName.includes("FOOT")) shortName = "TERM"
                else if (shortName.includes("FIREFOX")) shortName = "FFOX"
                else if (shortName.includes("CHROME") || shortName.includes("BRAVE")) shortName = "WEB"
                else if (shortName.includes("TELEGRAM")) shortName = "TG"
                else if (shortName.includes("DISCORD")) shortName = "DSCD"
                else if (shortName.includes("DOLPHIN") || shortName.includes("NEMO")) shortName = "FILE"
                else if (shortName.includes("CODE")) shortName = "CODE"
                else shortName = shortName.replace(/[-_]/g, '').substring(0, 5)

                let existingGroup = groups.find(g => g.appNameRaw === shortName)
                if (existingGroup) {
                    existingGroup.windows.push(w)
                    if (w.is_focused) existingGroup.isFocused = true
                } else {
                    groups.push({
                        appNameRaw: shortName,
                        appName: shortName.split('').join('\n'),
                        windows: [w],
                        isFocused: w.is_focused
                    })
                }
            }
            return groups
        }

        model: groupedWindowsArray

        Process {
            id: windowsProcess
            command: ["niri", "msg", "-j", "windows"]
            running: true
            stdout: SplitParser {
                onRead: data => {
                    try {
                        let wins = JSON.parse(data)
                        windowsLayout.allWindows = wins
                    } catch(e) { console.warn("windows parse:", e) }
                }
            }
        }

        Process {
            id: winClickProc
            running: false
        }

        delegate: Item {
            width: ListView.view.width
            height: contentRow.height + 16

            property bool isFocused: modelData.isFocused
            property string appName: modelData.appName
            property color itemColor: isFocused ? root.accentColor : root.textColor

            Row {
                id: contentRow
                anchors.centerIn: parent
                spacing: 6
                height: Math.max(appNameText.implicitHeight, dotsColumn.implicitHeight)

                Item {
                    width: 4
                    height: appNameText.implicitHeight + 4
                    anchors.verticalCenter: parent.verticalCenter

                    Rectangle { width: parent.width; height: 1; color: itemColor; anchors.left: parent.left; anchors.top: parent.top; Behavior on color { ColorAnimation { duration: 150 } } }
                    Rectangle { width: 3; height: parent.height; color: itemColor; anchors.left: parent.left; anchors.top: parent.top; Behavior on color { ColorAnimation { duration: 150 } } }
                    Rectangle { width: parent.width; height: 1; color: itemColor; anchors.left: parent.left; anchors.bottom: parent.bottom; Behavior on color { ColorAnimation { duration: 150 } } }
                }

                Text {
                    id: appNameText
                    text: appName
                    color: itemColor
                    font.pixelSize: 10
                    font.bold: true
                    font.family: root.fontTitle
                    horizontalAlignment: Text.AlignHCenter
                    lineHeight: 0.95
                    anchors.verticalCenter: parent.verticalCenter
                    Behavior on color { ColorAnimation { duration: 150 } }
                }

                Column {
                    id: dotsColumn
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: 5

                    Repeater {
                        model: modelData.windows
                        delegate: Rectangle {
                            width: 4
                            height: 4
                            rotation: 45
                            color: modelData.is_focused ? root.accentColor : root.existingColor
                            Behavior on color { ColorAnimation { duration: 150 } }

                            MouseArea {
                                anchors.fill: parent
                                anchors.margins: -4
                                cursorShape: Qt.PointingHandCursor
                                acceptedButtons: Qt.LeftButton | Qt.RightButton
                                onClicked: (mouse) => {
                                    if (mouse.button === Qt.RightButton) {
                                        var pos = parent.mapToItem(null, parent.width, 0)
                                        root.winMenu.targetX = pos.x
                                        root.winMenu.targetY = pos.y
                                        root.winMenu.targetWindowId = modelData.id
                                        root.winMenu.workspaces = root.workspaces
                                        root.winMenu.menuWidth = 240
                                        root.winMenu.menuHeight = 420
                                        root.winMenu.visible = true
                                        return
                                    }
                                    winClickProc.command = ["niri", "msg", "action", "focus-window", "--id", modelData.id.toString()]
                                    winClickProc.running = true
                                }
                            }
                        }
                    }
                }
            }

            MouseArea {
                anchors.fill: parent
                z: -1
                cursorShape: Qt.PointingHandCursor
                acceptedButtons: Qt.LeftButton | Qt.RightButton
                onClicked: (mouse) => {
                    if (mouse.button === Qt.RightButton) {
                        let targetId = modelData.windows[0].id
                        for (let i = 0; i < modelData.windows.length; i++) {
                            if (modelData.windows[i].is_focused) { targetId = modelData.windows[i].id; break }
                        }
                        var pos = parent.mapToItem(null, parent.width, 0)
                        root.winMenu.targetX = pos.x
                        root.winMenu.targetY = pos.y
                        root.winMenu.targetWindowId = targetId
                        root.winMenu.workspaces = root.workspaces
                        root.winMenu.menuWidth = 240
                        root.winMenu.menuHeight = 420
                        root.winMenu.visible = true
                        return
                    }
                    let targetId = modelData.windows[0].id
                    for (let i = 0; i < modelData.windows.length; i++) {
                        if (modelData.windows[i].is_focused) { targetId = modelData.windows[i].id; break }
                    }
                    winClickProc.command = ["niri", "msg", "action", "focus-window", "--id", targetId.toString()]
                    winClickProc.running = true
                }
            }
        }
    }

}
