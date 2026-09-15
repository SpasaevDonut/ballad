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
    WlrLayershell.namespace: "quickshell:wifi-popup"

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

    readonly property int targetY: {
        if (!anchorItem || typeof leftBar === "undefined" || !leftBar) return 100
        var itemCenterY = leftBar.mapFromItem(anchorItem, 0, anchorItem.height / 2).y
        var popupH = 420
        var y = itemCenterY - popupH / 2
        var screenH = root.height > 0 ? root.height : 1080
        if (y < 10) y = 10
        if (y + popupH > screenH - 10) {
            y = screenH - popupH - 10
        }
        return y
    }

    property bool isScanning: false
    property bool isConnecting: false
    property string connectingSsid: ""
    property string expandedSsid: ""
    property string enteredPassword: ""
    property bool showPassword: false
    property string statusMessage: ""
    property bool statusIsError: false
    property bool wifiEnabled: popupSysData?.wifiEnabled ?? true

    property var savedConnections: ({})
    property var rawScanResults: []

    function triggerScan(rescan) {
        if (isScanning) return
        isScanning = true
        statusMessage = ""
        rawScanResults = []
        savedConnections = {}
        savedConProc.running = true
        var rescanFlag = rescan ? "--rescan yes" : "--rescan no"
        wifiScanProc.command = ["sh", "-c", "nmcli -t -f IN-USE,SIGNAL,SECURITY,SSID dev wifi list " + rescanFlag]
        wifiScanProc.running = true
    }

    function toggleWifiPower() {
        var targetState = wifiEnabled ? "off" : "on"
        wifiRadioSetProc.command = ["nmcli", "radio", "wifi", targetState]
        wifiRadioSetProc.running = true
    }

    function disconnectCurrent(ssid) {
        statusMessage = "Disconnecting..."
        statusIsError = false
        var iface = popupCfg?.netInterface ?? "wlan0"
        wifiDisconnectProc.command = ["sh", "-c", `nmcli dev disconnect ${iface} 2>/dev/null || nmcli con down id "${ssid}" 2>/dev/null`]
        wifiDisconnectProc.running = true
    }

    function connectToNetwork(ssid, isSecured, isSaved) {
        if (isConnecting) return

        var safeSsid = ssid.replace(/'/g, "'\\''")

        // If the network is already saved and no password entered manually, connect using saved profile
        if (isSaved && enteredPassword.length === 0) {
            isConnecting = true
            connectingSsid = ssid
            statusMessage = "Connecting to " + ssid + "..."
            statusIsError = false
            wifiConnectProc.command = ["sh", "-c", `nmcli con up id '${safeSsid}' 2>/dev/null || nmcli dev wifi connect '${safeSsid}'`]
            wifiConnectProc.running = true
            return
        }

        // If the network is secured and password is empty
        if (isSecured && enteredPassword.length === 0) {
            statusMessage = "Enter password"
            statusIsError = true
            root.expandedSsid = ssid
            return
        }

        isConnecting = true
        connectingSsid = ssid
        statusMessage = "Connecting to " + ssid + "..."
        statusIsError = false

        if (isSecured) {
            var safePwd = enteredPassword.replace(/'/g, "'\\''")
            wifiConnectProc.command = ["sh", "-c", `nmcli dev wifi connect '${safeSsid}' password '${safePwd}'`]
        } else {
            wifiConnectProc.command = ["sh", "-c", `nmcli dev wifi connect '${safeSsid}'`]
        }
        wifiConnectProc.running = true
    }

    MouseArea {
        anchors.fill: parent
        onClicked: {
            lastCloseTime = Date.now()
            root.visible = false
        }
    }

    Rectangle {
        id: wifiRect
        x: 10
        y: root.targetY

        width: 280
        height: 420
        color: popupCfg?.popupBgColor ?? popupCfg?.bg0 ?? "#f1e5ce"
        border.width: 1
        border.color: popupCfg?.bg2 ?? "#d5c4a1"

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

                Item { Layout.fillWidth: true } // Spacer

                Text {
                    text: "WI-FI NETWORKS"
                    color: popupCfg?.fg ?? "#3a2a20"
                    font.pixelSize: 13
                    font.bold: true
                    font.family: popupCfg?.fontTitle ?? "Cinzel"
                }

                Item { Layout.fillWidth: true } // Spacer
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
                    text: root.wifiEnabled ? "ON" : "OFF"
                    color: root.wifiEnabled ? (popupCfg?.gold ?? "#b5893f") : (popupCfg?.red ?? "#7a362d")
                    font.pixelSize: 11
                    font.bold: true
                    font.family: popupCfg?.fontTitle ?? "Cinzel"
                }

                Item { Layout.fillWidth: true }

                // Refresh scan icon
                Text {
                    id: refreshIcon
                    text: "󰑐"
                    color: root.isScanning ? (popupCfg?.goldBright ?? "#d5a86d") : (refreshMouse.containsMouse ? (popupCfg?.red ?? "#7a362d") : (popupCfg?.fg ?? "#3a2a20"))
                    font.pixelSize: 16
                    font.family: "AurulentSansMNerdFontPropo"

                    RotationAnimation on rotation {
                        running: root.isScanning
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
                        onClicked: root.triggerScan(true)
                    }
                }

                // Sharp ON / OFF button
                Rectangle {
                    width: 36
                    height: 18
                    color: "transparent"
                    border.width: 1
                    border.color: root.wifiEnabled ? (popupCfg?.gold ?? "#b5893f") : (popupCfg?.bg2 ?? "#d5c4a1")

                    Text {
                        anchors.centerIn: parent
                        text: root.wifiEnabled ? "ON" : "OFF"
                        color: root.wifiEnabled ? (popupCfg?.gold ?? "#b5893f") : (popupCfg?.fg ?? "#3a2a20")
                        font.pixelSize: 9
                        font.bold: true
                        font.family: popupCfg?.fontTitle ?? "Cinzel"
                    }

                    MouseArea {
                        anchors.fill: parent
                        cursorShape: Qt.PointingHandCursor
                        onClicked: root.toggleWifiPower()
                    }
                }
            }

            // --- STATUS / NOTIFICATION MESSAGE ---
            Rectangle {
                Layout.fillWidth: true
                height: visible ? 24 : 0
                visible: root.isConnecting || root.statusMessage !== ""
                color: root.statusIsError ? "#f4dcd9" : (popupCfg?.bg0 ?? "#f1e5ce")
                border.color: root.statusIsError ? (popupCfg?.red ?? "#7a362d") : (popupCfg?.gold ?? "#b5893f")
                border.width: 1
                clip: true

                RowLayout {
                    anchors.fill: parent
                    anchors.margins: 2
                    anchors.leftMargin: 8
                    anchors.rightMargin: 8
                    spacing: 6

                    Text {
                        text: root.statusIsError ? "⚠" : (root.isConnecting ? "󰤩" : "✓")
                        color: root.statusIsError ? (popupCfg?.red ?? "#7a362d") : (popupCfg?.gold ?? "#b5893f")
                        font.pixelSize: 12
                        font.family: popupCfg?.fontIcons ?? "AurulentSansMNerdFontPropo"
                    }

                    Text {
                        text: root.statusMessage
                        color: root.statusIsError ? (popupCfg?.red ?? "#7a362d") : (popupCfg?.fg ?? "#3a2a20")
                        font.pixelSize: 11
                        font.family: popupCfg?.fontWidget ?? "EB Garamond"
                        font.italic: root.isConnecting
                        Layout.fillWidth: true
                        elide: Text.ElideRight
                    }
                }
            }

            // --- WI-FI DISABLED VIEW ---
            Item {
                Layout.fillWidth: true
                Layout.fillHeight: true
                visible: !root.wifiEnabled

                Column {
                    anchors.centerIn: parent
                    spacing: 12

                    Text {
                        text: "󰤮"
                        color: popupCfg?.gold ?? "#b5893f"
                        font.pixelSize: 42
                        font.family: popupCfg?.fontIcons ?? "AurulentSansMNerdFontPropo"
                        anchors.horizontalCenter: parent.horizontalCenter
                    }

                    Text {
                        text: "Wi-Fi is disabled"
                        color: popupCfg?.fg ?? "#3a2a20"
                        font.pixelSize: 13
                        font.family: popupCfg?.fontWidget ?? "EB Garamond"
                        font.bold: true
                        anchors.horizontalCenter: parent.horizontalCenter
                    }

                    Rectangle {
                        width: 120
                        height: 26
                        color: enableBtnMouse.containsMouse ? (popupCfg?.goldBright ?? "#d5a86d") : (popupCfg?.gold ?? "#b5893f")
                        border.color: popupCfg?.gold ?? "#b5893f"
                        border.width: 1
                        anchors.horizontalCenter: parent.horizontalCenter

                        Text {
                            text: "ENABLE WI-FI"
                            color: popupCfg?.bg0 ?? "#f1e5ce"
                            font.pixelSize: 10
                            font.bold: true
                            font.family: popupCfg?.fontTitle ?? "Cinzel"
                            anchors.centerIn: parent
                        }

                        MouseArea {
                            id: enableBtnMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: root.toggleWifiPower()
                        }
                    }
                }
            }

            // --- NETWORKS LIST ---
            Item {
                Layout.fillWidth: true
                Layout.fillHeight: true
                visible: root.wifiEnabled
                clip: true

                // Scanning / empty indicator
                Column {
                    anchors.centerIn: parent
                    spacing: 8
                    visible: wifiModel.count === 0

                    Text {
                        text: root.isScanning ? "󰑐" : "󰤭"
                        color: popupCfg?.gold ?? "#b5893f"
                        font.pixelSize: 28
                        font.family: popupCfg?.fontIcons ?? "AurulentSansMNerdFontPropo"
                        anchors.horizontalCenter: parent.horizontalCenter

                        RotationAnimation on rotation {
                            running: root.isScanning && wifiModel.count === 0
                            from: 0
                            to: 360
                            loops: Animation.Infinite
                            duration: 1000
                        }
                    }

                    Text {
                        text: root.isScanning ? "Scanning..." : "No networks found"
                        color: popupCfg?.fg ?? "#3a2a20"
                        font.pixelSize: 12
                        font.family: popupCfg?.fontWidget ?? "EB Garamond"
                        font.italic: true
                        anchors.horizontalCenter: parent.horizontalCenter
                    }
                }

                ScrollView {
                    anchors.fill: parent
                    clip: true
                    visible: wifiModel.count > 0

                    ListView {
                        id: networkListView
                        model: wifiModel
                        spacing: 2
                        boundsBehavior: Flickable.StopAtBounds

                        delegate: Rectangle {
                            id: netCard
                            width: ListView.view.width
                            height: contentColumn.implicitHeight + 8
                            color: isSelected ? "#dfcbb0" : (rowMouse.containsMouse ? "#eedbc2" : "transparent")
                            border.color: isSelected ? (popupCfg?.gold ?? "#b5893f") : "transparent"
                            border.width: isSelected ? 1 : 0

                            property bool isSelected: root.expandedSsid === model.ssid

                            ColumnLayout {
                                id: contentColumn
                                anchors.top: parent.top
                                anchors.left: parent.left
                                anchors.right: parent.right
                                anchors.margins: 4
                                spacing: 4

                                // Main network row
                                Item {
                                    Layout.fillWidth: true
                                    height: 28

                                    RowLayout {
                                        anchors.fill: parent
                                        spacing: 6

                                        // Signal icon
                                        Text {
                                            text: {
                                                if (model.inUse) return "󰤨"
                                                if (model.signal >= 75) return "󰤨"
                                                if (model.signal >= 50) return "󰤥"
                                                if (model.signal >= 25) return "󰤢"
                                                return "󰤟"
                                            }
                                            color: model.inUse ? (popupCfg?.gold ?? "#b5893f") : (popupCfg?.fg ?? "#3a2a20")
                                            font.pixelSize: 14
                                            font.family: "AurulentSansMNerdFontPropo"
                                        }

                                        // Network name and subtitle
                                        ColumnLayout {
                                            Layout.fillWidth: true
                                            spacing: 1

                                            RowLayout {
                                                Layout.fillWidth: true
                                                spacing: 4

                                                Text {
                                                    text: model.ssid
                                                    color: model.inUse ? (popupCfg?.gold ?? "#b5893f") : (popupCfg?.fg ?? "#3a2a20")
                                                    font.pixelSize: 13
                                                    font.bold: model.inUse
                                                    font.family: popupCfg?.fontWidget ?? "EB Garamond"
                                                    elide: Text.ElideRight
                                                    Layout.fillWidth: true
                                                }
                                            }

                                            RowLayout {
                                                spacing: 4
                                                Text {
                                                    text: model.isSaved ? "Saved" : (model.security && model.security !== "--" ? model.security : (model.secured ? "Secured" : "Open"))
                                                    color: model.isSaved ? (popupCfg?.gold ?? "#b5893f") : (popupCfg?.fg ?? "#3a2a20")
                                                    opacity: model.isSaved ? 1.0 : 0.65
                                                    font.pixelSize: 10
                                                    font.family: popupCfg?.fontWidget ?? "EB Garamond"
                                                }

                                                Text {
                                                    text: "•"
                                                    color: popupCfg?.sepColor ?? "#d5c4a1"
                                                    font.pixelSize: 8
                                                }

                                                Text {
                                                    text: model.signal + "%"
                                                    color: popupCfg?.fg ?? "#3a2a20"
                                                    opacity: 0.65
                                                    font.pixelSize: 10
                                                    font.family: popupCfg?.fontWidget ?? "EB Garamond"
                                                }
                                            }
                                        }

                                        // Disconnect button on the right for connected network
                                        Rectangle {
                                            visible: model.inUse
                                            width: 76
                                            height: 18
                                            color: disconMouse.containsMouse ? (popupCfg?.red ?? "#7a362d") : "transparent"
                                            border.color: disconMouse.containsMouse ? (popupCfg?.red ?? "#7a362d") : (popupCfg?.gold ?? "#b5893f")
                                            border.width: 1

                                            Text {
                                                anchors.centerIn: parent
                                                text: disconMouse.containsMouse ? "DISCONNECT" : "CONNECTED"
                                                color: disconMouse.containsMouse ? "#ffffff" : (popupCfg?.gold ?? "#b5893f")
                                                font.pixelSize: 9
                                                font.bold: true
                                                font.family: popupCfg?.fontTitle ?? "Cinzel"
                                            }

                                            MouseArea {
                                                id: disconMouse
                                                anchors.fill: parent
                                                hoverEnabled: true
                                                cursorShape: Qt.PointingHandCursor
                                                onClicked: root.disconnectCurrent(model.ssid)
                                            }
                                        }

                                        // Action icon on the right for disconnected network
                                        Rectangle {
                                            visible: !model.inUse
                                            width: 18
                                            height: 18
                                            color: "transparent"

                                            Text {
                                                anchors.centerIn: parent
                                                text: {
                                                    if (netCard.isSelected) return "▲"
                                                    if (model.isSaved) return "▶"
                                                    if (model.secured) return "󰌾"
                                                    return "▶"
                                                }
                                                color: netCard.isSelected ? (popupCfg?.goldBright ?? "#d5a86d") : (model.isSaved ? (popupCfg?.gold ?? "#b5893f") : (popupCfg?.fg ?? "#3a2a20"))
                                                opacity: (!netCard.isSelected && !model.isSaved) ? 0.45 : 1.0
                                                font.pixelSize: model.secured && !model.isSaved && !netCard.isSelected ? 11 : 9
                                                font.family: popupCfg?.fontIcons ?? "AurulentSansMNerdFontPropo"
                                            }
                                        }
                                    }

                                    // Click on main row
                                    MouseArea {
                                        id: rowMouse
                                        anchors.fill: parent
                                        hoverEnabled: true
                                        cursorShape: Qt.PointingHandCursor
                                        z: 0

                                        onClicked: {
                                            if (model.inUse) return

                                            if (model.isSaved) {
                                                // Saved network: connect immediately with saved profile
                                                root.connectToNetwork(model.ssid, false, true)
                                            } else if (model.secured) {
                                                // New secured network: toggle password input
                                                if (root.expandedSsid === model.ssid) {
                                                    root.expandedSsid = ""
                                                } else {
                                                    root.expandedSsid = model.ssid
                                                    root.enteredPassword = ""
                                                    root.statusMessage = ""
                                                }
                                            } else {
                                                // Open network: connect
                                                root.connectToNetwork(model.ssid, false, false)
                                            }
                                        }
                                    }
                                }

                                // --- INLINE PASSWORD INPUT ---
                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: 4
                                    visible: netCard.isSelected && !model.inUse

                                    Rectangle {
                                        Layout.fillWidth: true
                                        height: 1
                                        color: popupCfg?.bg2 ?? "#d5c4a1"
                                    }

                                    Text {
                                        text: model.isSaved ? "New password for " + model.ssid + ":" : "Password:"
                                        color: popupCfg?.fg ?? "#3a2a20"
                                        font.pixelSize: 11
                                        font.family: popupCfg?.fontWidget ?? "EB Garamond"
                                    }

                                    Rectangle {
                                        Layout.fillWidth: true
                                        height: 26
                                        color: popupCfg?.bg0 ?? "#f1e5ce"
                                        border.color: pwdInput.activeFocus ? (popupCfg?.goldBright ?? "#d5a86d") : (popupCfg?.gold ?? "#b5893f")
                                        border.width: 1

                                        MouseArea {
                                            anchors.fill: parent
                                            onClicked: pwdInput.forceActiveFocus()
                                        }

                                        TextInput {
                                            id: pwdInput
                                            anchors.left: parent.left
                                            anchors.right: showPwdBtn.left
                                            anchors.verticalCenter: parent.verticalCenter
                                            anchors.leftMargin: 8
                                            anchors.rightMargin: 8
                                            color: popupCfg?.fg ?? "#3a2a20"
                                            font.pixelSize: 12
                                            font.family: popupCfg?.fontWidget ?? "EB Garamond"
                                            echoMode: root.showPassword ? TextInput.Normal : TextInput.Password
                                            selectByMouse: true
                                            selectionColor: popupCfg?.goldBright ?? "#d5a86d"
                                            selectedTextColor: popupCfg?.fg ?? "#3a2a20"
                                            clip: true
                                            enabled: !root.isConnecting
                                            focus: true
                                            activeFocusOnTab: true

                                            onVisibleChanged: {
                                                if (visible) forceActiveFocus()
                                            }

                                            onTextChanged: root.enteredPassword = text

                                            Text {
                                                text: "Enter password..."
                                                color: popupCfg?.gold ?? "#b5893f"
                                                opacity: 0.7
                                                font.pixelSize: 11
                                                font.italic: true
                                                font.family: popupCfg?.fontWidget ?? "EB Garamond"
                                                anchors.verticalCenter: parent.verticalCenter
                                                visible: !pwdInput.text && !pwdInput.activeFocus
                                            }

                                            Keys.onReturnPressed: root.connectToNetwork(model.ssid, model.secured, false)
                                            Keys.onEnterPressed: root.connectToNetwork(model.ssid, model.secured, false)
                                        }

                                        Text {
                                            id: showPwdBtn
                                            anchors.right: parent.right
                                            anchors.rightMargin: 8
                                            anchors.verticalCenter: parent.verticalCenter
                                            text: root.showPassword ? "󰈈" : "󰈉"
                                            color: showPwdMouse.containsMouse ? (popupCfg?.fg ?? "#3a2a20") : (popupCfg?.gold ?? "#b5893f")
                                            font.pixelSize: 13
                                            font.family: popupCfg?.fontIcons ?? "AurulentSansMNerdFontPropo"

                                            MouseArea {
                                                id: showPwdMouse
                                                anchors.fill: parent
                                                anchors.margins: -4
                                                hoverEnabled: true
                                                cursorShape: Qt.PointingHandCursor
                                                onClicked: root.showPassword = !root.showPassword
                                            }
                                        }
                                    }

                                    // Connect / Cancel Buttons
                                    RowLayout {
                                        Layout.fillWidth: true
                                        spacing: 6

                                        Rectangle {
                                            Layout.fillWidth: true
                                            height: 22
                                            color: connectBtnMouse.containsMouse ? (popupCfg?.goldBright ?? "#d5a86d") : (popupCfg?.gold ?? "#b5893f")
                                            border.color: popupCfg?.gold ?? "#b5893f"
                                            border.width: 1
                                            opacity: root.isConnecting ? 0.6 : 1.0

                                            Text {
                                                anchors.centerIn: parent
                                                text: root.isConnecting && root.connectingSsid === model.ssid ? "CONNECTING..." : "CONNECT"
                                                color: popupCfg?.bg0 ?? "#f1e5ce"
                                                font.pixelSize: 9
                                                font.bold: true
                                                font.family: popupCfg?.fontTitle ?? "Cinzel"
                                            }

                                            MouseArea {
                                                id: connectBtnMouse
                                                anchors.fill: parent
                                                enabled: !root.isConnecting
                                                hoverEnabled: true
                                                cursorShape: Qt.PointingHandCursor
                                                onClicked: root.connectToNetwork(model.ssid, model.secured, false)
                                            }
                                        }

                                        Rectangle {
                                            width: 58
                                            height: 22
                                            color: cancelBtnMouse.containsMouse ? (popupCfg?.bg1 ?? "#e8d2b4") : "transparent"
                                            border.color: popupCfg?.gold ?? "#b5893f"
                                            border.width: 1

                                            Text {
                                                anchors.centerIn: parent
                                                text: "CANCEL"
                                                color: popupCfg?.gold ?? "#b5893f"
                                                font.pixelSize: 9
                                                font.bold: true
                                                font.family: popupCfg?.fontTitle ?? "Cinzel"
                                            }

                                            MouseArea {
                                                id: cancelBtnMouse
                                                anchors.fill: parent
                                                hoverEnabled: true
                                                cursorShape: Qt.PointingHandCursor
                                                onClicked: {
                                                    root.expandedSsid = ""
                                                    root.enteredPassword = ""
                                                    root.statusMessage = ""
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
    }

    // Shadow
    DropShadow {
        source: wifiRect
        anchors.fill: wifiRect
        horizontalOffset: 12
        verticalOffset: 12
        radius: 14
        samples: 16
        color: "#353a2a20"
        transparentBorder: true
    }

    ListModel {
        id: wifiModel
    }

    // Process to query saved NetworkManager connections
    Process {
        id: savedConProc
        command: ["nmcli", "-t", "-f", "NAME,TYPE", "connection", "show"]
        stdout: SplitParser {
            onRead: data => {
                let trimmed = data.trim()
                if (!trimmed) return

                let placeholder = "\u0000"
                let safeLine = trimmed.replace(/\\:/g, placeholder)
                let parts = safeLine.split(":")
                if (parts.length >= 2) {
                    let name = parts[0].replace(new RegExp(placeholder, "g"), ":").trim()
                    let type = parts[1].trim()
                    if (type === "802-11-wireless" || type === "wifi") {
                        root.savedConnections[name] = true
                    }
                }
            }
        }
    }

    // Process to scan networks (nmcli)
    Process {
        id: wifiScanProc
        stdout: SplitParser {
            onRead: data => {
                let trimmed = data.trim()
                if (!trimmed) return

                // Parse nmcli -t -f IN-USE,SIGNAL,SECURITY,SSID dev wifi
                let placeholder = "\u0000"
                let safeLine = trimmed.replace(/\\:/g, placeholder)
                let parts = safeLine.split(":")
                if (parts.length >= 4) {
                    let inUse = parts[0].trim() === "*"
                    let signal = parseInt(parts[1]) || 0
                    let sec = parts[2].replace(new RegExp(placeholder, "g"), ":").trim()
                    let ssid = parts.slice(3).join(":").replace(new RegExp(placeholder, "g"), ":").trim()

                    if (ssid && ssid !== "--" && ssid.length > 0) {
                        let isSaved = Boolean(root.savedConnections[ssid])
                        root.rawScanResults.push({
                            inUse: inUse,
                            signal: signal,
                            secured: (sec.length > 0 && sec !== "--"),
                            security: sec,
                            ssid: ssid,
                            isSaved: isSaved
                        })
                    }
                }
            }
        }

        onExited: {
            root.isScanning = false

            // Deduplicate and sort
            let uniqueMap = {}
            for (let i = 0; i < root.rawScanResults.length; i++) {
                let item = root.rawScanResults[i]
                if (!uniqueMap[item.ssid] || item.inUse || item.signal > uniqueMap[item.ssid].signal) {
                    uniqueMap[item.ssid] = item
                }
            }

            let resultList = []
            for (let k in uniqueMap) {
                let item = uniqueMap[k]
                item.isSaved = Boolean(root.savedConnections[item.ssid])
                resultList.push(item)
            }

            resultList.sort((a, b) => {
                if (a.inUse && !b.inUse) return -1
                if (!a.inUse && b.inUse) return 1
                if (a.isSaved && !b.isSaved) return -1
                if (!a.isSaved && b.isSaved) return 1
                return b.signal - a.signal
            })

            wifiModel.clear()
            for (let i = 0; i < resultList.length; i++) {
                wifiModel.append(resultList[i])
            }
        }
    }

    // Process to connect to network
    Process {
        id: wifiConnectProc
        property string errorOutput: ""

        stdout: SplitParser {
            onRead: data => {
                console.log("wifi connect out:", data)
            }
        }

        stderr: SplitParser {
            onRead: data => {
                wifiConnectProc.errorOutput += data + " "
            }
        }

        onExited: (exitCode) => {
            root.isConnecting = false
            if (exitCode === 0) {
                root.statusMessage = "Connected to " + root.connectingSsid
                root.statusIsError = false
                root.expandedSsid = ""
                root.enteredPassword = ""
                root.triggerScan(false)
            } else {
                root.statusIsError = true
                root.statusMessage = wifiConnectProc.errorOutput.trim() || "Failed to connect to " + root.connectingSsid
                root.expandedSsid = root.connectingSsid
            }
            wifiConnectProc.errorOutput = ""
        }
    }

    // Process to disconnect from network
    Process {
        id: wifiDisconnectProc
        onExited: {
            root.statusMessage = "Disconnected"
            root.statusIsError = false
            root.triggerScan(false)
        }
    }

    // Process to toggle Wi-Fi radio power
    Process {
        id: wifiRadioSetProc
        onExited: {
            wifiRadioCheckProc.running = true
        }
    }

    // Process to check Wi-Fi radio power status
    Process {
        id: wifiRadioCheckProc
        command: ["nmcli", "radio", "wifi"]
        stdout: SplitParser {
            onRead: data => {
                root.wifiEnabled = (data.trim().toLowerCase() === "enabled")
                if (root.wifiEnabled) {
                    root.triggerScan(false)
                }
            }
        }
    }

    onVisibleChanged: {
        if (visible) {
            root.statusMessage = ""
            root.statusIsError = false
            wifiRadioCheckProc.running = true
            // Quick load from cache, then scan
            root.triggerScan(false)
        } else {
            expandedSsid = ""
            enteredPassword = ""
            showPassword = false
            statusMessage = ""
        }
    }
}
