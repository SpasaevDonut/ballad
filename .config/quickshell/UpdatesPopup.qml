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
    readonly property color bgColor: popupCfg ? popupCfg.bgColor : "#121217"
    readonly property color accentColor: popupCfg ? popupCfg.accentColor : "#C16FB8"
    readonly property color textColor: popupCfg ? popupCfg.textColor : "#A3A0AF"
    readonly property color inactiveColor: popupCfg ? popupCfg.inactiveColor : "#5e5e65"
    readonly property color existingColor: popupCfg ? popupCfg.existingColor : "#939399"
    readonly property color sepColor: popupCfg ? popupCfg.sepColor : "#2F2F38"
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
        var popupH = 480
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

        width: 320
        height: 480
        color: root.bgColor

        MouseArea {
            anchors.fill: parent
        }

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 14
            spacing: 8

            // --- HEADER ---
            RowLayout {
                Layout.fillWidth: true
                spacing: 6

                Text {
                    text: "󰮯"
                    color: root.accentColor
                    font.pixelSize: 18
                    font.family: "AurulentSansMNerdFontPropo"
                }

                Text {
                    text: "PACMAN UPDATES"
                    color: root.existingColor
                    font.pixelSize: 13
                    font.bold: true
                    font.family: "AurulentSansMNerdFontPropo"
                }

                Item { Layout.fillWidth: true } // Spacer

                // Refresh button
                Text {
                    id: refreshBtn
                    text: "󰑐"
                    color: root.isChecking ? root.accentColor : (refreshMouse.containsMouse ? root.accentColor : root.inactiveColor)
                    font.pixelSize: 15
                    font.family: "AurulentSansMNerdFontPropo"

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

                // Close button
                Text {
                    text: "✖"
                    color: closeMouse.containsMouse ? root.accentColor : root.inactiveColor
                    font.pixelSize: 13
                    font.bold: true

                    MouseArea {
                        id: closeMouse
                        anchors.fill: parent
                        anchors.margins: -6
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            lastCloseTime = Date.now()
                            root.visible = false
                        }
                    }
                }
            }

            // Accent separator line
            Rectangle {
                Layout.fillWidth: true
                height: 2
                color: root.accentColor
            }

            // --- STATUS ROW ---
            RowLayout {
                Layout.fillWidth: true
                spacing: 6

                Rectangle {
                    height: 20
                    radius: 3
                    color: root.updatesCount > 0 ? "#331c2d" : "#1c2e22"
                    border.width: 1
                    border.color: root.updatesCount > 0 ? root.accentColor : "#44aa66"
                    Layout.preferredWidth: badgeText.implicitWidth + 12

                    Text {
                        id: badgeText
                        anchors.centerIn: parent
                        text: root.updatesCount > 0 ? (root.updatesCount + " available") : "Up to date 󰄬"
                        color: root.updatesCount > 0 ? root.accentColor : "#66cc88"
                        font.pixelSize: 10
                        font.bold: true
                        font.family: "monospace"
                    }
                }

                Item { Layout.fillWidth: true }

                Text {
                    text: root.checkTime ? ("Checked " + root.checkTime) : ""
                    color: root.inactiveColor
                    font.pixelSize: 10
                    font.family: "monospace"
                }
            }

            // --- SEARCH / FILTER BOX ---
            Rectangle {
                id: searchBoxRect
                Layout.fillWidth: true
                height: 28
                color: "#181820"
                border.width: 1
                border.color: filterInput.activeFocus ? root.accentColor : root.sepColor

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
                        color: filterInput.activeFocus ? root.accentColor : root.inactiveColor
                        font.pixelSize: 13
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
                            color: root.textColor
                            font.pixelSize: 11
                            font.family: "monospace"
                            selectByMouse: true
                            selectionColor: root.accentColor
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
                                color: root.inactiveColor
                                font.pixelSize: 11
                                font.family: "monospace"
                                visible: !filterInput.text && !filterInput.activeFocus
                            }
                        }
                    }

                    Text {
                        visible: filterInput.text.length > 0
                        text: "✖"
                        color: clearSearchMouse.containsMouse ? root.accentColor : root.inactiveColor
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
                        color: root.updatesCount === 0 ? root.accentColor : root.inactiveColor
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
                        color: root.textColor
                        font.pixelSize: 12
                        font.bold: true
                        font.family: "monospace"
                        Layout.alignment: Qt.AlignHCenter
                    }
                }

                // Scrollable list
                ListView {
                    id: updatesListView
                    anchors.fill: parent
                    visible: root.filteredList.length > 0
                    clip: true
                    spacing: 4
                    boundsBehavior: Flickable.StopAtBounds
                    model: root.filteredList

                    ScrollBar.vertical: ScrollBar {
                        active: true
                        width: 4
                        contentItem: Rectangle {
                            implicitWidth: 4
                            radius: 2
                            color: root.accentColor
                            opacity: 0.5
                        }
                    }

                    delegate: Rectangle {
                        width: updatesListView.width - (updatesListView.contentHeight > updatesListView.height ? 8 : 0)
                        height: pkgCol.implicitHeight + 10
                        color: itemMouse.containsMouse ? "#1c1c24" : "#16161d"
                        border.width: 1
                        border.color: itemMouse.containsMouse ? root.accentColor : root.sepColor

                        MouseArea {
                            id: itemMouse
                            anchors.fill: parent
                            hoverEnabled: true
                        }

                        ColumnLayout {
                            id: pkgCol
                            anchors.fill: parent
                            anchors.margins: 6
                            spacing: 3

                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 6

                                Text {
                                    text: "󰏔"
                                    color: root.accentColor
                                    font.pixelSize: 12
                                    font.family: "AurulentSansMNerdFontPropo"
                                }

                                Text {
                                    text: modelData.name
                                    color: root.textColor
                                    font.pixelSize: 11
                                    font.bold: true
                                    font.family: "monospace"
                                    elide: Text.ElideRight
                                    Layout.fillWidth: true
                                }

                                Rectangle {
                                    visible: Boolean(modelData.extra && modelData.extra.length > 0)
                                    height: 14
                                    radius: 2
                                    color: "#282024"
                                    border.width: 1
                                    border.color: root.inactiveColor
                                    Layout.preferredWidth: extraText.implicitWidth + 6

                                    Text {
                                        id: extraText
                                        anchors.centerIn: parent
                                        text: modelData.extra ? modelData.extra : ""
                                        color: root.inactiveColor
                                        font.pixelSize: 8
                                        font.family: "monospace"
                                    }
                                }
                            }

                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 4

                                Text {
                                    text: modelData.oldVersion
                                    color: root.inactiveColor
                                    font.pixelSize: 10
                                    font.family: "monospace"
                                    elide: Text.ElideRight
                                    Layout.maximumWidth: 120
                                }

                                Text {
                                    text: "➔"
                                    color: root.accentColor
                                    font.pixelSize: 9
                                }

                                Text {
                                    text: modelData.newVersion
                                    color: root.accentColor
                                    font.pixelSize: 10
                                    font.bold: true
                                    font.family: "monospace"
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
                color: root.sepColor
            }

            // --- BOTTOM UPGRADE BUTTON ---
            Rectangle {
                Layout.fillWidth: true
                height: 36
                color: upgradeBtnMouse.containsMouse ? "#281e30" : "transparent"
                border.width: 1
                border.color: upgradeBtnMouse.containsMouse ? root.accentColor : root.sepLightColor

                RowLayout {
                    anchors.centerIn: parent
                    spacing: 8

                    Text {
                        text: "󰮯"
                        color: upgradeBtnMouse.containsMouse ? root.accentColor : root.textColor
                        font.pixelSize: 14
                        font.family: "AurulentSansMNerdFontPropo"
                    }

                    Text {
                        text: "UPGRADE IN TERMINAL"
                        color: upgradeBtnMouse.containsMouse ? root.accentColor : root.textColor
                        font.pixelSize: 11
                        font.bold: true
                        font.family: "monospace"
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
        horizontalOffset: 15
        verticalOffset: 15
        radius: 15
        samples: 16
        color: "#A6000000"
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
