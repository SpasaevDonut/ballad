import QtQuick

QtObject {
    // --- COLORS AND THEME ---
    property string bgColor: "#121217"
    property string popupBgColor: "#202029"
    property string sepColor: "#2F2F38"
    property string sepLightColor: "#505050"
    property string accentColor: "#C16FB8"
    property string inactiveColor: "#5e5e65"
    property string existingColor: "#939399"
    property string textColor: "#A3A0AF"     
    
    // --- DIMENSIONS ---
    property int barWidth: 46 

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
    property string terminalCmd: "kitty" 
    property string mixerCmd: "pwvucontrol" 

    // --- UPDATES ---
    property int updatesCheckInterval: 600000 // 10 minutes
    property string pacmanUpdateCmd: "pacman -Qu"
    property string upgradeCmd: "sudo pacman -Syu"
}
