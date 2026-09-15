//@ pragma UseQApplication
import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import Qt5Compat.GraphicalEffects
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

PanelWindow {
    id: root
    visible: false
    color: "transparent"

    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.keyboardFocus: visible ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None
    WlrLayershell.namespace: "quickshell:updates-popup"

    anchors {
        top: true
        bottom: true
        left: true
        right: true
    }

    exclusiveZone: 0

    property double lastCloseTime: 0
    property QtObject popupSysData: null
    property QtObject popupCfg: null
    property var anchorItem: null
    property string searchQuery: ""

    // Theme colors
    readonly property color bgColor: popupCfg ? (popupCfg.popupBgColor ?? popupCfg.bg0) : "#f1e5ce"
    readonly property color accentColor: popupCfg ? (popupCfg.gold ?? popupCfg.accentColor) : "#b5893f"
    readonly property color textColor: popupCfg ? (popupCfg.fg ?? popupCfg.textColor) : "#3a2a20"
    readonly property color inactiveColor: popupCfg ? (popupCfg.bg2 ?? popupCfg.inactiveColor) : "#d5c4a1"
    readonly property color existingColor: popupCfg ? (popupCfg.red ?? popupCfg.existingColor) : "#7a362d"
    readonly property color sepColor: popupCfg ? (popupCfg.bg2 ?? popupCfg.sepColor) : "#d5c4a1"
    readonly property color sepLightColor: popupCfg ? popupCfg.sepLightColor : "#505050"

    readonly property int updatesCount: popupSysData ? popupSysData.pacmanUpdatesCount : 0
    readonly property bool isChecking: popupSysData ? popupSysData.isCheckingUpdates : false
    readonly property string checkTime: popupSysData ? popupSysData.lastUpdatesCheckTime : ""

    readonly property var filteredList: {
        var list = (popupSysData && popupSysData.pacmanUpdatesList) ? popupSysData.pacmanUpdatesList : []
        if (!searchQuery || !searchQuery.trim()) return list
        var q = searchQuery.trim().toLowerCase()
        return list.filter(function(item) {
            return item && item.name && item.name.toLowerCase().indexOf(q) !== -1
        })
    }

    readonly property int targetY: {
        if (!anchorItem || typeof leftBar === "undefined" || !leftBar) return 100
        var itemCenterY = leftBar.mapFromItem(anchorItem, 0, anchorItem.height / 2).y
        var popupH = 430
        var y = itemCenterY - popupH / 2
        var screenH = root.height > 0 ? root.height : 1080
        if (y < 10) y = 10
        if (y + popupH > screenH - 10) {
            y = screenH - popupH - 10
        }
        return y
    }

    MouseArea {
        anchors.fill: parent
        onClicked: {
            lastCloseTime = Date.now()
            root.visible = false
        }
    }

    Process {
        id: upgradeActionProc
        running: false
    }

    Rectangle {
        id: popupRect
        x: 10
        y: root.targetY

        width: 300
        height: 430
        color: root.bgColor
        border.width: 1
        border.color: root.sepColor

        // Victorian Corner Filigree
        Item {
            anchors.left: parent.left
            anchors.top: parent.top
            anchors.margins: 4
            width: 16; height: 16
            rotation: 270

            Text {
                anchors.right: parent.right
                anchors.top: parent.top
                text: "z"
                font.family: popupCfg?.fontVictorian ?? "Victorian Free Ornaments"
                font.pixelSize: 16
                color: popupCfg?.gold ?? "#b5893f"
                opacity: 0.65
            }
        }
        Item {
            anchors.right: parent.right
            anchors.top: parent.top
            anchors.margins: 4
            width: 16; height: 16

            Text {
                anchors.right: parent.right
                anchors.top: parent.top
                text: "z"
                font.family: popupCfg?.fontVictorian ?? "Victorian Free Ornaments"
                font.pixelSize: 16
                color: popupCfg?.gold ?? "#b5893f"
                opacity: 0.65
            }
        }
        Item {
            anchors.left: parent.left
            anchors.bottom: parent.bottom
            anchors.margins: 4
            width: 16; height: 16
            rotation: 180

            Text {
                anchors.right: parent.right
                anchors.top: parent.top
                text: "z"
                font.family: popupCfg?.fontVictorian ?? "Victorian Free Ornaments"
                font.pixelSize: 16
                color: popupCfg?.gold ?? "#b5893f"
                opacity: 0.65
            }
        }
        Item {
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            anchors.margins: 4
            width: 16; height: 16
            rotation: 90

            Text {
                anchors.right: parent.right
                anchors.top: parent.top
                text: "z"
                font.family: popupCfg?.fontVictorian ?? "Victorian Free Ornaments"
                font.pixelSize: 16
                color: popupCfg?.gold ?? "#b5893f"
                opacity: 0.65
            }
        }

        MouseArea {
            anchors.fill: parent
        }

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 16
            spacing: 8

            // --- HEADER ---
            RowLayout {
                Layout.fillWidth: true

                Item { Layout.fillWidth: true }

                Text {
                    text: "PACMAN UPDATES"
                    color: popupCfg?.fg ?? "#3a2a20"
                    font.pixelSize: 13
                    font.bold: true
                    font.family: popupCfg?.fontTitle ?? "Cinzel"
                }

                Item { Layout.fillWidth: true }
            }

            // Filigree divider
            RowLayout {
                Layout.fillWidth: true
                spacing: 6

                Rectangle { Layout.fillWidth: true; height: 1; color: popupCfg?.bg2 ?? "#d5c4a1"; opacity: 0.7 }
                Text {
                    text: "t"
                    font.family: popupCfg?.fontBodoni ?? "Bodoni Ornaments ITC"
                    font.pixelSize: 14
                    color: popupCfg?.gold ?? "#b5893f"
                }
                Rectangle { Layout.fillWidth: true; height: 1; color: popupCfg?.bg2 ?? "#d5c4a1"; opacity: 0.7 }
            }

            // --- SUBHEADER / QUICK CONTROLS ---
            RowLayout {
                Layout.fillWidth: true
                spacing: 6

                Text {
                    text: "STATUS:"
                    color: popupCfg?.fg ?? "#3a2a20"
                    opacity: 0.75
                    font.pixelSize: 11
                    font.bold: true
                    font.family: popupCfg?.fontTitle ?? "Cinzel"
                }

                Text {
                    text: root.updatesCount > 0 ? (root.updatesCount + " AVAILABLE") : "UP TO DATE"
                    color: root.updatesCount > 0 ? (popupCfg?.gold ?? "#b5893f") : (popupCfg?.green ?? "#7a7038")
                    font.pixelSize: 11
                    font.bold: true
                    font.family: popupCfg?.fontTitle ?? "Cinzel"
                }

                Item { Layout.fillWidth: true }

                Text {
                    text: root.checkTime ? root.checkTime : ""
                    color: popupCfg?.fg ?? "#3a2a20"
                    opacity: 0.6
                    font.pixelSize: 10
                    font.family: popupCfg?.fontWidget ?? "EB Garamond"
                }

                // Refresh scan icon
                Text {
                    id: refreshBtn
                    text: "󰑐"
                    color: root.isChecking ? (popupCfg?.goldBright ?? "#d5a86d") : (refreshMouse.containsMouse ? (popupCfg?.red ?? "#7a362d") : (popupCfg?.fg ?? "#3a2a20"))
                    font.pixelSize: 15
                    font.family: "AurulentSansMNerdFontPropo"
                    Layout.alignment: Qt.AlignVCenter

                    RotationAnimation on rotation {
                        running: root.isChecking
                        from: 0
                        to: 360
                        loops: Animation.Infinite
                        duration: 800
                    }

                    MouseArea {
                        id: refreshMouse
                        anchors.fill: parent
                        anchors.margins: -6
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            if (popupSysData && typeof popupSysData.checkPacmanUpdates === "function") {
                                popupSysData.checkPacmanUpdates()
                            }
                        }
                    }
                }
            }

            // --- SEARCH / FILTER BOX ---
            Rectangle {
                id: searchBoxRect
                Layout.fillWidth: true
                height: 26
                radius: 2
                color: filterInput.activeFocus ? "#dfcbb0" : "#e5d3ba"
                border.width: 1
                border.color: filterInput.activeFocus ? (popupCfg?.gold ?? "#b5893f") : (popupCfg?.bg2 ?? "#d5c4a1")

                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.IBeamCursor
                    onClicked: filterInput.forceActiveFocus()
                }

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 8
                    anchors.rightMargin: 8
                    spacing: 6

                    Text {
                        text: "󰍉"
                        color: filterInput.activeFocus ? (popupCfg?.gold ?? "#b5893f") : (popupCfg?.fg ?? "#3a2a20")
                        opacity: filterInput.activeFocus ? 1.0 : 0.65
                        font.pixelSize: 12
                        font.family: "AurulentSansMNerdFontPropo"
                        Layout.alignment: Qt.AlignVCenter
                    }

                    Item {
                        Layout.fillWidth: true
                        Layout.fillHeight: true

                        TextInput {
                            id: filterInput
                            anchors.fill: parent
                            verticalAlignment: TextInput.AlignVCenter
                            color: popupCfg?.fg ?? "#3a2a20"
                            font.pixelSize: 12
                            font.family: popupCfg?.fontWidget ?? "EB Garamond"
                            selectByMouse: true
                            selectionColor: popupCfg?.gold ?? "#b5893f"
                            selectedTextColor: "#ffffff"
                            clip: true
                            focus: true
                            activeFocusOnTab: true

                            onVisibleChanged: {
                                if (visible) forceActiveFocus()
                            }

                            onTextChanged: root.searchQuery = text

                            Keys.onEscapePressed: {
                                if (text.length > 0) {
                                    text = ""
                                    root.searchQuery = ""
                                } else {
                                    root.lastCloseTime = Date.now()
                                    root.visible = false
                                }
                            }

                            Text {
                                anchors.fill: parent
                                verticalAlignment: Text.AlignVCenter
                                text: "Filter packages..."
                                color: popupCfg?.fg ?? "#3a2a20"
                                opacity: 0.45
                                font.pixelSize: 12
                                font.italic: true
                                font.family: popupCfg?.fontWidget ?? "EB Garamond"
                                visible: !filterInput.text && !filterInput.activeFocus
                            }
                        }
                    }

                    Text {
                        visible: filterInput.text.length > 0
                        text: "✖"
                        color: clearSearchMouse.containsMouse ? (popupCfg?.red ?? "#7a362d") : (popupCfg?.fg ?? "#3a2a20")
                        opacity: 0.7
                        font.pixelSize: 10
                        Layout.alignment: Qt.AlignVCenter

                        MouseArea {
                            id: clearSearchMouse
                            anchors.fill: parent
                            anchors.margins: -6
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                filterInput.text = ""
                                root.searchQuery = ""
                                filterInput.forceActiveFocus()
                            }
                        }
                    }
                }
            }

            // --- PACKAGE LIST VIEW ---
            Item {
                Layout.fillWidth: true
                Layout.fillHeight: true

                // Empty / Up to date state
                ColumnLayout {
                    anchors.centerIn: parent
                    spacing: 8
                    visible: root.filteredList.length === 0

                    Text {
                        text: root.isChecking ? "󰑐" : (root.updatesCount === 0 ? "󰄬" : "󰍉")
                        color: root.updatesCount === 0 ? (popupCfg?.gold ?? "#b5893f") : (popupCfg?.bg2 ?? "#d5c4a1")
                        font.pixelSize: 28
                        font.family: "AurulentSansMNerdFontPropo"
                        Layout.alignment: Qt.AlignHCenter

                        RotationAnimation on rotation {
                            running: root.isChecking
                            from: 0
                            to: 360
                            loops: Animation.Infinite
                            duration: 1000
                        }
                    }

                    Text {
                        text: {
                            if (root.isChecking) return "Checking for updates..."
                            if (root.searchQuery.length > 0) return "No matching packages"
                            return "All packages are up to date"
                        }
                        color: popupCfg?.fg ?? "#3a2a20"
                        font.pixelSize: 12
                        font.bold: true
                        font.family: popupCfg?.fontWidget ?? "EB Garamond"
                        Layout.alignment: Qt.AlignHCenter
                    }
                }

                // Scrollable list
                ListView {
                    id: updatesListView
                    anchors.fill: parent
                    visible: root.filteredList.length > 0
                    clip: true
                    spacing: 3
                    boundsBehavior: Flickable.StopAtBounds
                    model: root.filteredList

                    ScrollBar.vertical: ScrollBar {
                        active: true
                        width: 4
                        contentItem: Rectangle {
                            implicitWidth: 4
                            radius: 2
                            color: popupCfg?.gold ?? "#b5893f"
                            opacity: 0.35
                        }
                    }

                    delegate: Rectangle {
                        id: pkgCard
                        width: updatesListView.width - (updatesListView.contentHeight > updatesListView.height ? 8 : 0)
                        height: pkgCol.implicitHeight + 8
                        radius: 2
                        color: itemMouse.containsMouse ? "#eedbc2" : "transparent"
                        border.color: itemMouse.containsMouse ? (popupCfg?.gold ?? "#b5893f") : "transparent"
                        border.width: 1

                        MouseArea {
                            id: itemMouse
                            anchors.fill: parent
                            hoverEnabled: true
                        }

                        ColumnLayout {
                            id: pkgCol
                            anchors.top: parent.top
                            anchors.left: parent.left
                            anchors.right: parent.right
                            anchors.margins: 4
                            anchors.leftMargin: 6
                            anchors.rightMargin: 6
                            spacing: 3

                            // Package name and optional repo tag
                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 6

                                Text {
                                    text: "󰏔"
                                    color: itemMouse.containsMouse ? (popupCfg?.gold ?? "#b5893f") : (popupCfg?.fg ?? "#3a2a20")
                                    opacity: itemMouse.containsMouse ? 1.0 : 0.75
                                    font.pixelSize: 13
                                    font.family: "AurulentSansMNerdFontPropo"
                                }

                                Text {
                                    text: modelData.name
                                    color: itemMouse.containsMouse ? (popupCfg?.gold ?? "#b5893f") : (popupCfg?.fg ?? "#3a2a20")
                                    font.pixelSize: 13
                                    font.bold: true
                                    font.family: popupCfg?.fontWidget ?? "EB Garamond"
                                    elide: Text.ElideRight
                                    Layout.fillWidth: true
                                }

                                Rectangle {
                                    visible: Boolean(modelData.extra && modelData.extra.length > 0)
                                    height: 14
                                    radius: 2
                                    color: "transparent"
                                    border.width: 1
                                    border.color: popupCfg?.gold ?? "#b5893f"
                                    Layout.preferredWidth: extraText.implicitWidth + 8

                                    Text {
                                        id: extraText
                                        anchors.centerIn: parent
                                        text: modelData.extra ? modelData.extra : ""
                                        color: popupCfg?.gold ?? "#b5893f"
                                        font.pixelSize: 9
                                        font.bold: true
                                        font.family: popupCfg?.fontTitle ?? "Cinzel"
                                    }
                                }
                            }

                            // Version transition: old -> new
                            RowLayout {
                                Layout.fillWidth: true
                                Layout.leftMargin: 19
                                spacing: 4

                                Text {
                                    text: modelData.oldVersion
                                    color: popupCfg?.fg ?? "#3a2a20"
                                    opacity: 0.6
                                    font.pixelSize: 11
                                    font.family: popupCfg?.fontWidget ?? "EB Garamond"
                                    elide: Text.ElideRight
                                    Layout.maximumWidth: 120
                                }

                                Text {
                                    text: "➔"
                                    color: popupCfg?.gold ?? "#b5893f"
                                    font.pixelSize: 9
                                }

                                Text {
                                    text: modelData.newVersion
                                    color: popupCfg?.gold ?? "#b5893f"
                                    font.pixelSize: 11
                                    font.bold: true
                                    font.family: popupCfg?.fontWidget ?? "EB Garamond"
                                    elide: Text.ElideRight
                                    Layout.fillWidth: true
                                }
                            }
                        }
                    }
                }
            }

            // Separator line
            Rectangle {
                Layout.fillWidth: true
                height: 1
                color: popupCfg?.bg2 ?? "#d5c4a1"
                opacity: 0.7
            }

            // --- BOTTOM UPGRADE BUTTON ---
            Rectangle {
                Layout.fillWidth: true
                height: 32
                radius: 2
                color: upgradeBtnMouse.containsMouse ? (popupCfg?.red ?? "#7a362d") : "transparent"
                border.width: 1
                border.color: upgradeBtnMouse.containsMouse ? (popupCfg?.red ?? "#7a362d") : (popupCfg?.gold ?? "#b5893f")

                RowLayout {
                    anchors.centerIn: parent
                    spacing: 8

                    Text {
                        text: "󰮯"
                        color: upgradeBtnMouse.containsMouse ? "#ffffff" : (popupCfg?.gold ?? "#b5893f")
                        font.pixelSize: 14
                        font.family: "AurulentSansMNerdFontPropo"
                    }

                    Text {
                        text: "UPGRADE IN TERMINAL"
                        color: upgradeBtnMouse.containsMouse ? "#ffffff" : (popupCfg?.fg ?? "#3a2a20")
                        font.pixelSize: 10
                        font.bold: true
                        font.family: popupCfg?.fontTitle ?? "Cinzel"
                    }
                }

                MouseArea {
                    id: upgradeBtnMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: {
                        var term = popupCfg ? popupCfg.terminalCmd : "kitty"
                        var cmd = popupCfg ? popupCfg.upgradeCmd : "sudo pacman -Syu"
                        upgradeActionProc.command = [term, "-e", "sh", "-c", cmd + "; echo 'Press enter to exit...'; read"]
                        upgradeActionProc.running = true
                        lastCloseTime = Date.now()
                        root.visible = false
                    }
                }
            }
        }
    }

    // Shadow
    DropShadow {
        source: popupRect
        anchors.fill: popupRect
        horizontalOffset: 12
        verticalOffset: 12
        radius: 14
        samples: 16
        color: "#353a2a20"
        transparentBorder: true
    }

    onVisibleChanged: {
        if (visible) {
            searchQuery = ""
            filterInput.text = ""
            filterInput.forceActiveFocus()
            if (popupSysData && typeof popupSysData.checkPacmanUpdates === "function") {
                popupSysData.checkPacmanUpdates()
            }
        } else {
            searchQuery = ""
            filterInput.text = ""
        }
    }
}
