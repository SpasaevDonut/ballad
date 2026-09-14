//@ pragma UseQApplication
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Services.UPower
import "./LeftBarWidgets"

Item {
    function formatSpeed(bytes) {
        if (bytes === 0) return "0B"
        if (bytes < 1024) return bytes.toFixed(0) + "B"
        if (bytes < 1048576) return (bytes / 1024).toFixed(0) + "K"
        return (bytes / 1048576).toFixed(1) + "M"
    }

    // --- NETWORK ---
    property real lastRx: 0
    property real lastTx: 0
    property real rawDownload: 0
    property real rawUpload: 0
    property real dlRatio: 0.0
    property real ulRatio: 0.0
    property string downloadStr: "0B"
    property string uploadStr: "0B"
    property string netIcon: "󰤭"
    property bool isConnected: false
    property string wifiSsid: ""
    property int wifiSignal: 0
    property bool wifiEnabled: true

    function updateWifiIcon() {
        if (!wifiEnabled) {
            netIcon = "󰤮"
        } else if (!isConnected) {
            netIcon = "󰤭"
        } else if (wifiSignal >= 75) {
            netIcon = "󰤨"
        } else if (wifiSignal >= 50) {
            netIcon = "󰤥"
        } else if (wifiSignal >= 25) {
            netIcon = "󰤢"
        } else {
            netIcon = "󰤟"
        }
    }

    Process {
        id: netProcess
        command: ["sh", "-c", `awk -v iface="${cfg.netInterface}" '{gsub(/[: ]+/, " "); for(i=1; i<=NF; i++) if($i==iface){print $(i+1) " " $(i+9)}}' /proc/net/dev; cat /sys/class/net/${cfg.netInterface}/operstate 2>/dev/null`]
        stdout: SplitParser {
            onRead: data => {
                let trimmed = data.trim()
                if (trimmed.includes(" ")) {
                    let parts = trimmed.split(/\s+/)
                    let rx = parseFloat(parts[0])
                    let tx = parseFloat(parts[1])
                    if (lastRx > 0 && rx >= lastRx && tx >= lastTx) {
                        rawDownload = (rx - lastRx) / 2
                        rawUpload = (tx - lastTx) / 2
                        dlRatio = Math.min((rawDownload * cfg.dlSensitivity) / cfg.netMaxSpeed, 1.0)
                        ulRatio = Math.min((rawUpload * cfg.ulSensitivity) / cfg.netMaxSpeed, 1.0)
                        downloadStr = formatSpeed(rawDownload)
                        uploadStr = formatSpeed(rawUpload)
                    }
                    lastRx = rx
                    lastTx = tx
                } else if (trimmed === "up") {
                    isConnected = true
                    updateWifiIcon()
                } else if (trimmed === "down") {
                    isConnected = false
                    wifiSsid = ""
                    wifiSignal = 0
                    updateWifiIcon()
                }
            }
        }
    }

    Timer {
        interval: 2000
        running: true
        repeat: true
        triggeredOnStart: true
        onTriggered: {
            netProcess.running = false
            netProcess.running = true
        }
    }

    Process {
        id: netMonitor
        command: ["ip", "monitor", "link", "dev", cfg.netInterface]
        running: true
        stdout: SplitParser {
            onRead: data => {
                if (data.includes("state UP")) {
                    isConnected = true
                    updateWifiIcon()
                } else if (data.includes("state DOWN")) {
                    isConnected = false
                    wifiSsid = ""
                    wifiSignal = 0
                    updateWifiIcon()
                }
            }
        }
        Component.onDestruction: running = false
    }

    // Periodic check of active Wi-Fi connection and signal strength
    PollingData {
        interval: 5000
        command: ["sh", "-c", "nmcli -t -f ACTIVE,SSID,SIGNAL dev wifi list --rescan no 2>/dev/null | awk -F: '$1==\"yes\"{print $2 \":\" $3; exit}'"]
        onDataReady: data => {
            let trimmed = data.trim()
            if (trimmed.includes(":")) {
                let parts = trimmed.split(":")
                wifiSsid = parts[0]
                let sig = parseInt(parts[1])
                wifiSignal = !isNaN(sig) ? sig : 0
                isConnected = true
            } else if (trimmed === "") {
                if (wifiSsid !== "") {
                    wifiSsid = ""
                    wifiSignal = 0
                }
            }
            updateWifiIcon()
        }
    }

    // Check Wi-Fi radio power state (enabled / disabled)
    PollingData {
        interval: 6000
        command: ["nmcli", "radio", "wifi"]
        onDataReady: data => {
            let state = data.trim().toLowerCase()
            wifiEnabled = (state === "enabled")
            updateWifiIcon()
        }
    }

    // --- DISK I/O ---
    property real lastDiskRead: 0
    property real lastDiskWrite: 0
    property real rawDiskRead: 0
    property real rawDiskWrite: 0
    property real diskReadRatio: 0.0
    property real diskWriteRatio: 0.0
    property string diskReadStr: "0.0 B"
    property string diskWriteStr: "0.0 B"

    PollingData {
        command: ["sh", "-c", `awk '$3 == "${cfg.diskInterface}" {print $6 " " $10}' /proc/diskstats 2>/dev/null`]
        onDataReady: data => {
            if (data.includes(" ")) {
                let parts = data.trim().split(/\s+/)
                if (parts.length >= 2) {
                    let rSectors = parseFloat(parts[0])
                    let wSectors = parseFloat(parts[1])
                    let rBytes = rSectors * 512
                    let wBytes = wSectors * 512
                    if (lastDiskRead > 0) {
                        rawDiskRead = (rBytes - lastDiskRead) / 2
                        rawDiskWrite = (wBytes - lastDiskWrite) / 2
                        diskReadRatio = Math.min((rawDiskRead * 5) / cfg.diskMaxSpeed, 1.0)
                        diskWriteRatio = Math.min((rawDiskWrite * 5) / cfg.diskMaxSpeed, 1.0)
                        diskReadStr = formatSpeed(rawDiskRead)
                        diskWriteStr = formatSpeed(rawDiskWrite)
                    }
                    lastDiskRead = rBytes
                    lastDiskWrite = wBytes
                }
            }
        }
    }

    // --- CPU ---
    property int cpuPercent: 0

    PollingData {
        command: ["sh", "-c", "top -bn1 | awk '/^%Cpu/ {print $2+$4}'"]
        onDataReady: data => {
            let val = parseFloat(data.replace(',', '.'))
            if (!isNaN(val)) cpuPercent = Math.round(val)
        }
    }

    // --- RAM ---
    property int ramBlocks: 0

    PollingData {
        command: ["sh", "-c", "free | awk '/Mem:/ {print $3/$2}'"]
        onDataReady: data => {
            let val = parseFloat(data.replace(',', '.'))
            if (!isNaN(val)) ramBlocks = Math.round(val * 24)
        }
    }

    // --- STORAGE USAGE ---
    property int storagePercent: 0

    PollingData {
        interval: 60000
        command: ["sh", "-c", "df / | awk 'NR==2 {print $5}' | tr -d '%'"]
        onDataReady: data => {
            let val = parseInt(data)
            if (!isNaN(val)) storagePercent = val
        }
    }

    // --- TEMPERATURE ---
    property int temp1: 0
    property int temp2: 0

    Process {
        id: tempProc
        property int lineCount: 0
        command: ["sh", "-c", "sensors | grep -E 'Tctl|edge' | awk '{print $2}' | tr -d '+°C'"]
        stdout: SplitParser {
            onRead: data => {
                let val = parseFloat(data)
                if (!isNaN(val)) {
                    if (tempProc.lineCount === 0) temp1 = Math.round(val)
                    else if (tempProc.lineCount === 1) temp2 = Math.round(val)
                    tempProc.lineCount++
                }
            }
        }
        onExited: lineCount = 0
    }

    Timer {
        interval: 2000
        running: true
        repeat: true
        triggeredOnStart: true
        onTriggered: {
            tempProc.running = false
            tempProc.running = true
        }
    }

    // --- FANS ---
    property real fan1Ratio: 0.0
    property real fan2Ratio: 0.0

    PollingData {
        interval: 3000
        command: ["sh", "-c", `sensors | grep -i '${cfg.fan1Name}' | awk '{print $2}'`]
        onDataReady: data => {
            let val = parseFloat(data)
            fan1Ratio = !isNaN(val) ? Math.min(val / cfg.fan1MaxRpm, 1.0) : 0.0
        }
    }

    PollingData {
        interval: 3000
        command: ["sh", "-c", `sensors | grep -i '${cfg.fan2Name}' | awk '{print $2}'`]
        onDataReady: data => {
            let val = parseFloat(data)
            fan2Ratio = !isNaN(val) ? Math.min(val / cfg.fan2MaxRpm, 1.0) : 0.0
        }
    }

    // --- BATTERY (UPower) ---
    property real batteryPercent: UPower.displayDevice.ready ? UPower.displayDevice.percentage * 100 : 0
    property string batteryStatus: UPower.displayDevice.ready ? UPowerDeviceState.toString(UPower.displayDevice.state) : "Unknown"

    // --- PACMAN UPDATES ---
    property int pacmanUpdatesCount: 0
    property var pacmanUpdatesList: []
    property bool isCheckingUpdates: false
    property string lastUpdatesCheckTime: ""
    property var _tempUpdates: []

    function checkPacmanUpdates() {
        if (isCheckingUpdates) return
        isCheckingUpdates = true
        _tempUpdates = []
        pacmanUpdateProc.running = false
        pacmanUpdateProc.running = true
    }

    Process {
        id: pacmanUpdateProc
        command: ["sh", "-c", ((typeof cfg !== "undefined" && cfg && cfg.pacmanUpdateCmd) ? cfg.pacmanUpdateCmd : "pacman -Qu") + " 2>/dev/null || true"]
        stdout: SplitParser {
            onRead: data => {
                let trimmed = data.trim()
                if (!trimmed) return
                try {
                    let m = trimmed.match(/^(\S+)\s+(\S+)\s+->\s+(\S+)(.*)$/)
                    if (m) {
                        _tempUpdates.push({
                            name: m[1],
                            oldVersion: m[2],
                            newVersion: m[3],
                            extra: m[4] ? m[4].trim() : ""
                        })
                    } else {
                        _tempUpdates.push({
                            name: trimmed,
                            oldVersion: "",
                            newVersion: "",
                            extra: ""
                        })
                    }
                } catch(e) {
                    console.warn("Pacman parse error:", e)
                }
            }
        }
        onExited: {
            pacmanUpdatesList = _tempUpdates.slice()
            pacmanUpdatesCount = pacmanUpdatesList.length
            isCheckingUpdates = false
            let d = new Date()
            lastUpdatesCheckTime = ("0" + d.getHours()).slice(-2) + ":" + ("0" + d.getMinutes()).slice(-2)
        }
        Component.onDestruction: running = false
    }

    Timer {
        id: updatesTimer
        interval: (typeof cfg !== "undefined" && cfg && cfg.updatesCheckInterval) ? cfg.updatesCheckInterval : 600000
        running: true
        repeat: true
        triggeredOnStart: true
        onTriggered: {
            checkPacmanUpdates()
        }
    }
}
