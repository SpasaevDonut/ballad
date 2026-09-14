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
        color: popupCfg?.bgColor ?? "#121217"

        MouseArea {
            anchors.fill: parent
        }

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 15
            spacing: 8

            // --- HEADER ---
            RowLayout {
                Layout.fillWidth: true

                Item { Layout.fillWidth: true } // Spacer

                Text {
                    text: "WI-FI NETWORKS"
                    color: popupCfg?.existingColor ?? "#939399"
                    font.pixelSize: 14
                    font.bold: true
                    font.family: "AurulentSansMNerdFontPropo"
                }

                Item { Layout.fillWidth: true } // Spacer

                Text {
                    text: "✖"
                    color: popupCfg?.inactiveColor ?? "#5e5e65"
                    font.pixelSize: 14
                    font.bold: true

                    MouseArea {
                        anchors.fill: parent
                        anchors.margins: -10
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            lastCloseTime = Date.now()
                            root.visible = false
                        }
                    }
                }
            }

            // Accent separator line matching Calendar and Power Popups
            Rectangle {
                Layout.fillWidth: true
                height: 2
                color: popupCfg?.accentColor ?? "#C16FB8"
            }

            // --- SUBHEADER / QUICK CONTROLS ---
            RowLayout {
                Layout.fillWidth: true
                spacing: 6

                Text {
                    text: "STATUS:"
                    color: popupCfg?.inactiveColor ?? "#5e5e65"
                    font.pixelSize: 11
                    font.bold: true
                    font.family: "monospace"
                }

                Text {
                    text: root.wifiEnabled ? "ON" : "OFF"
                    color: root.wifiEnabled ? (popupCfg?.accentColor ?? "#C16FB8") : (popupCfg?.inactiveColor ?? "#5e5e65")
                    font.pixelSize: 11
                    font.bold: true
                    font.family: "monospace"
                }

                Item { Layout.fillWidth: true }

                // Refresh scan icon
                Text {
                    id: refreshIcon
                    text: "󰑐"
                    color: root.isScanning ? (popupCfg?.accentColor ?? "#C16FB8") : (popupCfg?.inactiveColor ?? "#5e5e65")
                    font.pixelSize: 14
                    font.family: "AurulentSansMNerdFontPropo"

                    RotationAnimation on rotation {
                        running: root.isScanning
                        from: 0
                        to: 360
                        loops: Animation.Infinite
                        duration: 800
                    }

                    MouseArea {
                        anchors.fill: parent
                        anchors.margins: -6
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
                    border.color: root.wifiEnabled ? (popupCfg?.accentColor ?? "#C16FB8") : (popupCfg?.inactiveColor ?? "#5e5e65")

                    Text {
                        anchors.centerIn: parent
                        text: root.wifiEnabled ? "ON" : "OFF"
                        color: root.wifiEnabled ? (popupCfg?.accentColor ?? "#C16FB8") : (popupCfg?.inactiveColor ?? "#5e5e65")
                        font.pixelSize: 9
                        font.bold: true
                        font.family: "monospace"
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
                height: visible ? 22 : 0
                visible: root.isConnecting || root.statusMessage !== ""
                color: root.statusIsError ? "#3d1c24" : (root.isConnecting ? "#251c2e" : "#1a2c20")
                clip: true

                RowLayout {
                    anchors.fill: parent
                    anchors.margins: 2
                    anchors.leftMargin: 6
                    anchors.rightMargin: 6
                    spacing: 6

                    Text {
                        text: root.statusIsError ? "⚠" : (root.isConnecting ? "󰤩" : "✓")
                        color: root.statusIsError ? "#ff6b81" : (popupCfg?.accentColor ?? "#C16FB8")
                        font.pixelSize: 11
                    }

                    Text {
                        text: root.statusMessage
                        color: root.statusIsError ? "#ffb8c6" : (popupCfg?.textColor ?? "#A3A0AF")
                        font.pixelSize: 10
                        font.family: "monospace"
                        Layout.fillWidth: true
                        elide: Text.ElideRight
                    }

                    Text {
                        text: "✖"
                        color: popupCfg?.inactiveColor ?? "#5e5e65"
                        font.pixelSize: 9
                        visible: !root.isConnecting
                        MouseArea {
                            anchors.fill: parent
                            anchors.margins: -4
                            cursorShape: Qt.PointingHandCursor
                            onClicked: root.statusMessage = ""
                        }
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
                        color: popupCfg?.inactiveColor ?? "#5e5e65"
                        font.pixelSize: 42
                        anchors.horizontalCenter: parent.horizontalCenter
                    }

                    Text {
                        text: "Wi-Fi is disabled"
                        color: popupCfg?.textColor ?? "#A3A0AF"
                        font.pixelSize: 12
                        font.family: "monospace"
                        font.bold: true
                        anchors.horizontalCenter: parent.horizontalCenter
                    }

                    Rectangle {
                        width: 120
                        height: 26
                        color: popupCfg?.accentColor ?? "#C16FB8"
                        anchors.horizontalCenter: parent.horizontalCenter

                        Text {
                            text: "ENABLE WI-FI"
                            color: "#ffffff"
                            font.pixelSize: 10
                            font.bold: true
                            font.family: "monospace"
                            anchors.centerIn: parent
                        }

                        MouseArea {
                            anchors.fill: parent
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
                        color: popupCfg?.accentColor ?? "#C16FB8"
                        font.pixelSize: 28
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
                        color: popupCfg?.textColor ?? "#A3A0AF"
                        font.pixelSize: 11
                        font.family: "monospace"
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
                            color: isSelected ? "#1c1b26" : (rowMouse.containsMouse ? "#1a1921" : "transparent")
                            border.color: isSelected ? (popupCfg?.accentColor ?? "#C16FB8") : "transparent"
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
                                            color: model.inUse ? (popupCfg?.accentColor ?? "#C16FB8") : (popupCfg?.textColor ?? "#A3A0AF")
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
                                                    color: model.inUse ? (popupCfg?.accentColor ?? "#C16FB8") : (popupCfg?.textColor ?? "#A3A0AF")
                                                    font.pixelSize: 11
                                                    font.bold: model.inUse
                                                    font.family: "monospace"
                                                    elide: Text.ElideRight
                                                    Layout.fillWidth: true
                                                }
                                            }

                                            RowLayout {
                                                spacing: 4
                                                Text {
                                                    text: model.isSaved ? "Saved" : (model.security && model.security !== "--" ? model.security : (model.secured ? "Secured" : "Open"))
                                                    color: model.isSaved ? (popupCfg?.accentColor ?? "#C16FB8") : (popupCfg?.inactiveColor ?? "#5e5e65")
                                                    font.pixelSize: 9
                                                    font.family: "monospace"
                                                }

                                                Text {
                                                    text: "•"
                                                    color: popupCfg?.sepLightColor ?? "#505050"
                                                    font.pixelSize: 8
                                                }

                                                Text {
                                                    text: model.signal + "%"
                                                    color: popupCfg?.inactiveColor ?? "#5e5e65"
                                                    font.pixelSize: 9
                                                    font.family: "monospace"
                                                }
                                            }
                                        }

                                        // Disconnect button on the right for connected network
                                        Rectangle {
                                            visible: model.inUse
                                            width: 72
                                            height: 18
                                            color: disconMouse.containsMouse ? "#451a22" : "transparent"
                                            border.color: disconMouse.containsMouse ? "#ff4757" : (popupCfg?.accentColor ?? "#C16FB8")
                                            border.width: 1

                                            Text {
                                                anchors.centerIn: parent
                                                text: disconMouse.containsMouse ? "DISCONNECT" : "CONNECTED"
                                                color: disconMouse.containsMouse ? "#ff6b81" : (popupCfg?.accentColor ?? "#C16FB8")
                                                font.pixelSize: 8
                                                font.bold: true
                                                font.family: "monospace"
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
                                                color: netCard.isSelected ? (popupCfg?.accentColor ?? "#C16FB8") : (model.isSaved ? (popupCfg?.accentColor ?? "#C16FB8") : (popupCfg?.inactiveColor ?? "#5e5e65"))
                                                font.pixelSize: model.secured && !model.isSaved && !netCard.isSelected ? 11 : 9
                                                font.family: "AurulentSansMNerdFontPropo"
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
                                        color: popupCfg?.sepColor ?? "#2F2F38"
                                    }

                                    Text {
                                        text: model.isSaved ? "New password for " + model.ssid + ":" : "Password:"
                                        color: popupCfg?.textColor ?? "#A3A0AF"
                                        font.pixelSize: 9
                                        font.family: "monospace"
                                    }

                                    Rectangle {
                                        Layout.fillWidth: true
                                        height: 24
                                        color: "#0e0e13"
                                        border.color: pwdInput.activeFocus ? (popupCfg?.accentColor ?? "#C16FB8") : (popupCfg?.inactiveColor ?? "#5e5e65")
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
                                            anchors.leftMargin: 6
                                            anchors.rightMargin: 6
                                            color: popupCfg?.textColor ?? "#A3A0AF"
                                            font.pixelSize: 11
                                            font.family: "monospace"
                                            echoMode: root.showPassword ? TextInput.Normal : TextInput.Password
                                            selectByMouse: true
                                            selectionColor: popupCfg?.accentColor ?? "#C16FB8"
                                            selectedTextColor: "#ffffff"
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
                                                color: popupCfg?.inactiveColor ?? "#5e5e65"
                                                font.pixelSize: 10
                                                font.family: "monospace"
                                                anchors.verticalCenter: parent.verticalCenter
                                                visible: !pwdInput.text && !pwdInput.activeFocus
                                            }

                                            Keys.onReturnPressed: root.connectToNetwork(model.ssid, model.secured, false)
                                            Keys.onEnterPressed: root.connectToNetwork(model.ssid, model.secured, false)
                                        }

                                        Text {
                                            id: showPwdBtn
                                            anchors.right: parent.right
                                            anchors.rightMargin: 6
                                            anchors.verticalCenter: parent.verticalCenter
                                            text: root.showPassword ? "󰈈" : "󰈉"
                                            color: popupCfg?.inactiveColor ?? "#5e5e65"
                                            font.pixelSize: 12
                                            font.family: "AurulentSansMNerdFontPropo"

                                            MouseArea {
                                                anchors.fill: parent
                                                anchors.margins: -4
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
                                            color: connectBtnMouse.containsMouse ? Qt.lighter(popupCfg?.accentColor ?? "#C16FB8", 1.1) : (popupCfg?.accentColor ?? "#C16FB8")
                                            opacity: root.isConnecting ? 0.6 : 1.0

                                            Text {
                                                anchors.centerIn: parent
                                                text: root.isConnecting && root.connectingSsid === model.ssid ? "CONNECTING..." : "CONNECT"
                                                color: "#ffffff"
                                                font.pixelSize: 9
                                                font.bold: true
                                                font.family: "monospace"
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
                                            width: 54
                                            height: 22
                                            color: cancelBtnMouse.containsMouse ? (popupCfg?.inactiveColor ?? "#5e5e65") : "transparent"
                                            border.color: popupCfg?.inactiveColor ?? "#5e5e65"
                                            border.width: 1

                                            Text {
                                                anchors.centerIn: parent
                                                text: "CANCEL"
                                                color: popupCfg?.textColor ?? "#A3A0AF"
                                                font.pixelSize: 9
                                                font.family: "monospace"
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
        horizontalOffset: 15
        verticalOffset: 15
        radius: 15
        samples: 16
        color: "#A6000000"
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
