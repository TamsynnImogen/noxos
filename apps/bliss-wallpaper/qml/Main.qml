import QtQuick 2.15
import QtQuick.Window 2.15

Window {
    id: root
    visible: true
    title: "Bliss Wallpaper"
    color: "black"
    x: Screen.virtualX
    y: Screen.virtualY
    width: Screen.width
    height: Screen.height
    flags: Qt.FramelessWindowHint | Qt.WindowStaysOnBottomHint | Qt.WindowDoesNotAcceptFocus

    property var blissSchedule: [
        { minute: 300, source: "../../../assets/Bliss/1.Bliss_Dawn.png" },
        { minute: 390, source: "../../../assets/Bliss/2.Bliss_Sunrise.png" },
        { minute: 540, source: "../../../assets/Bliss/3.Bliss_Morning.png" },
        { minute: 720, source: "../../../assets/Bliss/4.Bliss_Midday.png" },
        { minute: 900, source: "../../../assets/Bliss/5.Bliss_Afternoon.png" },
        { minute: 1110, source: "../../../assets/Bliss/6.Bliss_Sunset.png" },
        { minute: 1230, source: "../../../assets/Bliss/7.Bliss_Dusk.png" },
        { minute: 1320, source: "../../../assets/Bliss/8.Bliss_Night.png" }
    ]
    property string blissCurrentSource: blissSchedule[0].source
    property string blissNextSource: blissSchedule[1].source
    property real blissBlendProgress: 0.0
    property int blissRefreshIntervalMs: 60000

    function minutesSinceMidnight(dateValue) {
        return (dateValue.getHours() * 60) + dateValue.getMinutes() + (dateValue.getSeconds() / 60.0)
    }

    function resolveBlissWallpaper(dateValue) {
        const nowMinutes = minutesSinceMidnight(dateValue)
        let currentIndex = blissSchedule.length - 1

        for (let i = 0; i < blissSchedule.length; ++i) {
            if (nowMinutes >= blissSchedule[i].minute)
                currentIndex = i
            else
                break
        }

        const nextIndex = (currentIndex + 1) % blissSchedule.length
        const currentEntry = blissSchedule[currentIndex]
        const nextEntry = blissSchedule[nextIndex]
        const startMinute = currentEntry.minute
        let endMinute = nextEntry.minute
        let adjustedNow = nowMinutes

        if (endMinute <= startMinute)
            endMinute += 1440
        if (adjustedNow < startMinute)
            adjustedNow += 1440

        const duration = Math.max(1, endMinute - startMinute)
        return {
            currentSource: currentEntry.source,
            nextSource: nextEntry.source,
            progress: Math.max(0, Math.min(1, (adjustedNow - startMinute) / duration)),
            refreshIntervalMs: Math.max(10000, Math.round((duration * 60 * 1000) / 200))
        }
    }

    function updateBlissWallpaper() {
        const state = resolveBlissWallpaper(new Date())
        blissCurrentSource = state.currentSource
        blissNextSource = state.nextSource
        blissBlendProgress = state.progress
        blissRefreshIntervalMs = state.refreshIntervalMs
    }

    Component.onCompleted: updateBlissWallpaper()

    Timer {
        interval: root.blissRefreshIntervalMs
        repeat: true
        running: true
        onTriggered: root.updateBlissWallpaper()
    }

    Rectangle {
        anchors.fill: parent
        color: "black"
    }

    Image {
        anchors.fill: parent
        source: root.blissCurrentSource
        fillMode: Image.PreserveAspectCrop
        asynchronous: true
        cache: true
    }

    Image {
        anchors.fill: parent
        source: root.blissNextSource
        fillMode: Image.PreserveAspectCrop
        asynchronous: true
        cache: true
        opacity: root.blissBlendProgress

        Behavior on opacity {
            NumberAnimation {
                duration: 1200
                easing.type: Easing.Linear
            }
        }
    }
}
