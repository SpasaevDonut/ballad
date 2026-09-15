import QtQuick

QtObject {
    // --- AUTHENTIC DISDOC PALETTE (s.a.t.8 / Koleda Wallpaper) ---
    property string bg0: "#f1e5ce"         // Основной пергамент
    property string bg1: "#e8d2b4"         // Карточки и панели, чуть плотнее
    property string bg2: "#d5c4a1"         // Разделители, неактивные бордеры
    property string fg: "#3a2a20"          // Текст: тёплая темная умбра
    property string red: "#7a362d"         // Винный акцент (юбка)
    property string redDim: "#4c2d26"      // Тень / неактивное состояние красного
    property string gold: "#b5893f"        // Золото (волосы/солнце/латунь)
    property string goldBright: "#d5a86d"   // Хайлайт, активные элементы, фокус
    property string green: "#7a7038"       // Приглушённый оливково-золотой (трава)
    property string greenDim: "#49442c"    // Тень травы, неактивные риски
    property string blueGray: "#9a9d9c"    // Приглушённый серо-голубой акцент (небо)

    // --- COMPATIBLE THEME BINDINGS ---
    property string bgColor: bg0
    property string popupBgColor: bg0
    property string sepColor: bg2
    property string sepLightColor: bg0
    property string accentColor: gold
    property string inactiveColor: bg2
    property string existingColor: redDim
    property string textColor: fg

    // --- TYPOGRAPHY AND ORNAMENTS ---
    property string fontVictorian: "Victorian Free Ornaments"
    property string fontBodoni: "Bodoni Ornaments ITC"
    property string fontTitle: "Cinzel"
    property string fontRoman: "Cinzel Decorative"
    property string fontAccent: "Berkshire Swash"
    property string fontWidget: "EB Garamond"
    property string fontIcons: "AurulentSansMNerdFontPropo"     
    
    // --- DIMENSIONS ---
    property int barWidth: 50 

    // --- NETWORK ---
    property string netInterface: "wlan0"
    property real netMaxSpeed: 20971520 // 20 MB/s max speed for visual meter scaling
    property real dlSensitivity: 1.5 
    property real ulSensitivity: 2.0 

    // --- DISK ---
    property string diskInterface: "nvme0n1"
    property real diskMaxSpeed: 524288000 // 500 MB/s max speed for visual I/O meter

    // --- FANS ---
    property string fan1Name: "fan1" 
    property real fan1MaxRpm: 5000   

    property string fan2Name: "fan2"
    property real fan2MaxRpm: 5000

    // --- COMMANDS (Terminal & Mixer) ---
    property string terminalCmd: "foot" 
    property string mixerCmd: "pwvucontrol" 

    // --- UPDATES ---
    property int updatesCheckInterval: 2400000 // 40 minutes
    property string pacmanUpdateCmd: "pacman -Qu"
    property string upgradeCmd: "sudo pacman -Syu"
}
