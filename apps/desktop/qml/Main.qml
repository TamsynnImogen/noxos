import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import QtQuick.Window 2.15

Window {
    id: root
    visible: true
    title: "Desktop"
    color: "black"
    x: Screen.virtualX
    y: Screen.virtualY
    width: Screen.desktopAvailableWidth
    height: Screen.desktopAvailableHeight
    flags: Qt.FramelessWindowHint | Qt.WindowStaysOnBottomHint

    property string selectedPath: ""
    property string contextTargetUrl: ""
    property string contextTargetPath: ""
    property bool contextTargetIsDesktop: true
    property int panelHeight: 74
    property string panelTheme: "neon"
    property date currentTime: new Date()
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

    function iconSource(isDir, fileName, fileUrl) {
        if (isDir)
            return "../../../assets/icons/folders/folder_desktop.svg"

        const lowerName = fileName.toLowerCase()
        if (lowerName.endsWith(".png") || lowerName.endsWith(".jpg") || lowerName.endsWith(".jpeg") || lowerName.endsWith(".webp"))
            return fileUrl

        return "../../../assets/icons/file.svg"
    }

    function openPath(urlValue) {
        Qt.openUrlExternally(urlValue)
    }

    function launchApp(appId) {
        if (desktopModel)
            desktopModel.launch_app(appId)
        startMenu.close()
    }

    function appIconSource(appId) {
        if (appId === "files")
            return "../../../assets/icons/folders/folder.svg"
        if (appId === "image-viewer")
            return "../../../assets/icons/folders/folder_pictures.svg"
        if (appId === "settings")
            return "../../../assets/icons/folders/icon_settings.svg"
        if (appId === "software")
            return "../../../assets/icons/folders/folder_system.svg"
        return ""
    }

    function appInitial(label) {
        if (!label || label.length === 0)
            return "?"
        return label.charAt(0).toUpperCase()
    }

    function appAccent(appId) {
        if (appId === "files")
            return "#26c6ff"
        if (appId === "terminal")
            return "#8b5cf6"
        if (appId === "browser")
            return "#ff7a2f"
        if (appId === "image-viewer")
            return "#f041d8"
        if (appId === "settings")
            return "#59e3a7"
        if (appId === "software")
            return "#45a3ff"
        if (appId === "calculator")
            return "#f7c844"
        return "#b48cff"
    }

    function panelIconSource(iconName, stateName) {
        return "../../../assets/icons/pannel/" + panelTheme + "/" + iconName + "-" + stateName + ".svg"
    }

    function panelBackgroundSource() {
        return "../../../assets/icons/pannel/" + panelTheme + "/panel.svg"
    }

    function panelButtonState(hovered, active) {
        if (active)
            return "active"
        if (hovered)
            return "hover"
        return "default"
    }

    function formattedTime() {
        return currentTime.toLocaleTimeString(Qt.locale(), "hh:mm")
    }

    function formattedDate() {
        return currentTime.toLocaleDateString(Qt.locale(), "ddd d MMM")
    }

    function showDesktopMenu(xPos, yPos) {
        root.selectedPath = ""
        root.contextTargetIsDesktop = true
        root.contextTargetUrl = "file://" + desktopModel.desktop_path
        root.contextTargetPath = desktopModel.desktop_path
        contextMenu.x = xPos
        contextMenu.y = yPos
        contextMenu.open()
    }

    function showItemMenu(urlValue, pathValue, xPos, yPos) {
        root.selectedPath = pathValue
        root.contextTargetIsDesktop = false
        root.contextTargetUrl = urlValue
        root.contextTargetPath = pathValue
        contextMenu.x = xPos
        contextMenu.y = yPos
        contextMenu.open()
    }

    Component.onCompleted: updateBlissWallpaper()

    Timer {
        interval: root.blissRefreshIntervalMs
        repeat: true
        running: true
        onTriggered: {
            root.updateBlissWallpaper()
        }
    }

    Timer {
        interval: 60000
        repeat: true
        running: true
        onTriggered: desktopModel.refresh()
    }

    Timer {
        interval: 1000
        repeat: true
        running: true
        onTriggered: root.currentTime = new Date()
    }

    ListModel {
        id: pinnedAppsModel
        ListElement { appId: "files"; label: "Files"; detail: "Browse local files" }
        ListElement { appId: "terminal"; label: "Terminal"; detail: "Command line" }
        ListElement { appId: "browser"; label: "Browser"; detail: "Open the web" }
        ListElement { appId: "image-viewer"; label: "Images"; detail: "View pictures" }
        ListElement { appId: "settings"; label: "Settings"; detail: "System controls" }
    }

    ListModel {
        id: allAppsModel
        ListElement { appId: "files"; label: "Files"; category: "System" }
        ListElement { appId: "terminal"; label: "Terminal"; category: "System" }
        ListElement { appId: "browser"; label: "Browser"; category: "Internet" }
        ListElement { appId: "image-viewer"; label: "Image Viewer"; category: "Graphics" }
        ListElement { appId: "settings"; label: "Settings"; category: "System" }
        ListElement { appId: "software"; label: "Software"; category: "System" }
        ListElement { appId: "calculator"; label: "Calculator"; category: "Utilities" }
    }

    ListModel {
        id: runningAppsModel
        ListElement { appId: "files"; label: "Files" }
        ListElement { appId: "terminal"; label: "Terminal" }
    }

    Component {
        id: appIconVisual

        Item {
            id: appIcon
            property string appId: ""
            property string label: ""
            property int iconSize: 34
            width: iconSize
            height: iconSize

            Rectangle {
                anchors.fill: parent
                radius: 8
                color: "#cc111827"
                border.width: 1
                border.color: root.appAccent(appIcon.appId)
            }

            Image {
                anchors.centerIn: parent
                width: Math.round(appIcon.iconSize * 0.72)
                height: width
                visible: root.appIconSource(appIcon.appId).length > 0
                source: root.appIconSource(appIcon.appId)
                fillMode: Image.PreserveAspectFit
                smooth: true
                asynchronous: true
            }

            Text {
                anchors.centerIn: parent
                visible: root.appIconSource(appIcon.appId).length === 0
                text: root.appInitial(appIcon.label)
                color: "white"
                font.bold: true
                font.pixelSize: Math.round(appIcon.iconSize * 0.48)
            }
        }
    }

    Popup {
        id: contextMenu
        width: 220
        padding: 8
        modal: false
        focus: true
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
        background: Rectangle {
            radius: 14
            color: "#f8f4ec"
            border.color: "#c9baa3"
        }

        contentItem: Column {
            spacing: 4

            Button {
                width: parent.width
                text: root.contextTargetIsDesktop ? "Open Desktop Folder" : "Open"
                onClicked: {
                    root.openPath(root.contextTargetIsDesktop ? ("file://" + desktopModel.desktop_path) : root.contextTargetUrl)
                    contextMenu.close()
                }
            }

            Button {
                width: parent.width
                text: "Open in Files"
                onClicked: {
                    desktopModel.open_in_files(root.contextTargetPath)
                    contextMenu.close()
                }
            }
        }
    }

    Popup {
        id: startMenu
        width: Math.min(520, root.width - 36)
        height: Math.min(520, root.height - root.panelHeight - 36)
        x: 18
        y: root.height - root.panelHeight - height - 18
        padding: 16
        modal: false
        focus: true
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
        background: Rectangle {
            radius: 14
            color: "#ec0b1220"
            border.width: 1
            border.color: "#8f35d6ff"
        }

        contentItem: ColumnLayout {
            spacing: 14

            RowLayout {
                Layout.fillWidth: true
                spacing: 10

                Rectangle {
                    Layout.preferredWidth: 34
                    Layout.preferredHeight: 34
                    radius: 8
                    color: "transparent"

                    Image {
                        anchors.fill: parent
                        source: root.panelIconSource("meta", "active")
                        fillMode: Image.PreserveAspectFit
                        smooth: true
                        asynchronous: true
                    }
                }

                TextField {
                    Layout.fillWidth: true
                    height: 34
                    placeholderText: "Search Nox..."
                    selectByMouse: true
                    color: "white"
                    placeholderTextColor: "#9bb7c9"
                    background: Rectangle {
                        radius: 8
                        color: "#ba101827"
                        border.color: parent.activeFocus ? "#d946ef" : "#4f6f88a8"
                    }
                }
            }

            Label {
                text: "Pinned"
                color: "#e6f7ff"
                font.bold: true
                font.pixelSize: 14
            }

            GridLayout {
                Layout.fillWidth: true
                columns: 5
                rowSpacing: 10
                columnSpacing: 10

                Repeater {
                    model: pinnedAppsModel

                    delegate: Button {
                        Layout.fillWidth: true
                        Layout.preferredHeight: 82
                        padding: 0
                        onClicked: root.launchApp(appId)

                        background: Rectangle {
                            radius: 9
                            color: parent.hovered ? "#2f1f3150" : "#95101827"
                            border.width: 1
                            border.color: parent.hovered ? root.appAccent(appId) : "#34475a"
                        }

                        contentItem: Column {
                            anchors.centerIn: parent
                            width: parent.width - 8
                            spacing: 7

                            Loader {
                                anchors.horizontalCenter: parent.horizontalCenter
                                sourceComponent: appIconVisual
                                onLoaded: {
                                    item.appId = appId
                                    item.label = label
                                    item.iconSize = 34
                                }
                            }

                            Text {
                                width: parent.width
                                text: label
                                color: "white"
                                horizontalAlignment: Text.AlignHCenter
                                elide: Text.ElideRight
                                font.pixelSize: 12
                            }
                        }
                    }
                }
            }

            RowLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                spacing: 14

                ColumnLayout {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    spacing: 8

                    Label {
                        text: "All Apps"
                        color: "#e6f7ff"
                        font.bold: true
                        font.pixelSize: 14
                    }

                    ListView {
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        model: allAppsModel
                        clip: true
                        spacing: 4

                        delegate: ItemDelegate {
                            width: ListView.view.width
                            height: 40
                            onClicked: root.launchApp(appId)

                            background: Rectangle {
                                radius: 8
                                color: parent.hovered ? "#2b233b5a" : "transparent"
                                border.color: parent.hovered ? "#5b8dd7ff" : "transparent"
                            }

                            contentItem: RowLayout {
                                spacing: 9

                                Loader {
                                    Layout.preferredWidth: 26
                                    Layout.preferredHeight: 26
                                    sourceComponent: appIconVisual
                                    onLoaded: {
                                        item.appId = appId
                                        item.label = label
                                        item.iconSize = 26
                                    }
                                }

                                Column {
                                    Layout.fillWidth: true
                                    spacing: 1

                                    Text {
                                        width: parent.width
                                        text: label
                                        color: "white"
                                        elide: Text.ElideRight
                                        font.pixelSize: 13
                                    }

                                    Text {
                                        width: parent.width
                                        text: category
                                        color: "#9bb7c9"
                                        elide: Text.ElideRight
                                        font.pixelSize: 11
                                    }
                                }
                            }
                        }
                    }
                }

                Rectangle {
                    Layout.preferredWidth: 148
                    Layout.fillHeight: true
                    radius: 10
                    color: "#95101827"
                    border.color: "#34475a"

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: 12
                        spacing: 10

                        Label {
                            text: "Power"
                            color: "#e6f7ff"
                            font.bold: true
                        }

                        Button {
                            Layout.fillWidth: true
                            text: "Lock"
                            onClicked: root.launchApp("lock")
                        }

                        Button {
                            Layout.fillWidth: true
                            text: "Sleep"
                            onClicked: root.launchApp("sleep")
                        }

                        Item { Layout.fillHeight: true }

                        Label {
                            Layout.fillWidth: true
                            text: "nox@NoxOS"
                            color: "#9bb7c9"
                            horizontalAlignment: Text.AlignHCenter
                            elide: Text.ElideRight
                        }
                    }
                }
            }
        }
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

    Rectangle {
        anchors.fill: parent
        gradient: Gradient {
            GradientStop { position: 0.0; color: "#22081117" }
            GradientStop { position: 1.0; color: "#44111822" }
        }
    }

    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        onPressed: function(mouse) {
            if (mouse.button === Qt.RightButton)
                root.showDesktopMenu(mouse.x, mouse.y)
        }
    }

    GridView {
        id: desktopIcons
        anchors.fill: parent
        anchors.margins: 20
        anchors.topMargin: 28
        anchors.bottomMargin: root.panelHeight + 34
        model: desktopModel
        cellWidth: 116
        cellHeight: 124
        flow: GridView.FlowTopToBottom
        layoutDirection: Qt.LeftToRight
        boundsBehavior: Flickable.StopAtBounds
        interactive: false
        clip: true
        delegate: Item {
            id: iconDelegate
            required property string name
            required property string path
            required property string url
            required property bool isDir
            width: desktopIcons.cellWidth
            height: desktopIcons.cellHeight

            Rectangle {
                anchors.centerIn: parent
                width: 104
                height: 112
                radius: 16
                color: root.selectedPath === iconDelegate.path ? "#6a17314a" : "transparent"
                border.color: root.selectedPath === iconDelegate.path ? "#99d7eef7" : "transparent"
                border.width: 1
            }

            Column {
                anchors.centerIn: parent
                width: 96
                spacing: 10

                Item {
                    width: 72
                    height: 72
                    anchors.horizontalCenter: parent.horizontalCenter

                    Rectangle {
                        anchors.fill: parent
                        radius: 22
                        color: "#26000000"
                    }

                    Image {
                        anchors.centerIn: parent
                        width: 56
                        height: 56
                        fillMode: Image.PreserveAspectFit
                        source: root.iconSource(iconDelegate.isDir, iconDelegate.name, iconDelegate.url)
                        asynchronous: true
                        smooth: true
                    }
                }

                Text {
                    width: parent.width
                    text: iconDelegate.name
                    color: "white"
                    horizontalAlignment: Text.AlignHCenter
                    wrapMode: Text.Wrap
                    maximumLineCount: 2
                    elide: Text.ElideRight
                    font.pixelSize: 14
                    style: Text.Outline
                    styleColor: "#88000000"
                }
            }

            MouseArea {
                anchors.fill: parent
                acceptedButtons: Qt.LeftButton | Qt.RightButton
                onClicked: function(mouse) {
                    root.selectedPath = iconDelegate.path
                    if (mouse.button === Qt.RightButton) {
                        root.showItemMenu(iconDelegate.url, iconDelegate.path, iconDelegate.x + mouse.x, iconDelegate.y + mouse.y)
                    }
                }
                onDoubleClicked: root.openPath(iconDelegate.url)
            }
        }
    }

    Rectangle {
        id: taskbar
        z: 20
        height: root.panelHeight
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.leftMargin: 18
        anchors.rightMargin: 18
        anchors.bottomMargin: 14
        color: "transparent"

        Image {
            anchors.fill: parent
            source: root.panelBackgroundSource()
            fillMode: Image.Stretch
            smooth: true
            asynchronous: true
        }

        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 14
            anchors.rightMargin: 14
            spacing: 12

            Button {
                id: startButton
                Layout.preferredWidth: 54
                Layout.preferredHeight: 52
                padding: 0
                onClicked: startMenu.opened ? startMenu.close() : startMenu.open()

                background: Item {}

                contentItem: Item {
                    anchors.centerIn: parent

                    Image {
                        anchors.centerIn: parent
                        width: 48
                        height: 48
                        source: root.panelIconSource("meta", root.panelButtonState(startButton.hovered, startMenu.opened))
                        fillMode: Image.PreserveAspectFit
                        smooth: true
                        asynchronous: true
                    }
                }
            }

            Rectangle {
                Layout.preferredWidth: 1
                Layout.preferredHeight: 38
                color: "#365b7088"
            }

            RowLayout {
                id: pinnedStrip
                spacing: 8

                Repeater {
                    model: pinnedAppsModel

                    delegate: Button {
                        id: pinnedButton
                        Layout.preferredWidth: 54
                        Layout.preferredHeight: 52
                        padding: 0
                        onClicked: root.launchApp(appId)
                        ToolTip.visible: hovered
                        ToolTip.text: label

                        background: Rectangle {
                            radius: 10
                            color: pinnedButton.hovered ? "#2f1f3150" : "#b8101827"
                            border.width: 1
                            border.color: pinnedButton.hovered ? root.appAccent(appId) : "#34475a"
                        }

                        contentItem: Loader {
                            anchors.centerIn: parent
                            sourceComponent: appIconVisual
                            onLoaded: {
                                item.appId = appId
                                item.label = label
                                item.iconSize = 39
                            }
                        }
                    }
                }
            }

            Rectangle {
                Layout.preferredWidth: 1
                Layout.preferredHeight: 38
                color: "#365b7088"
            }

            RowLayout {
                id: runningStrip
                Layout.fillWidth: true
                spacing: 7

                Repeater {
                    model: runningAppsModel

                    delegate: Button {
                        Layout.preferredWidth: 104
                        Layout.preferredHeight: 46
                        padding: 0
                        onClicked: root.launchApp(appId)

                        background: Rectangle {
                            radius: 10
                            color: parent.hovered ? "#2b233b5a" : "#a0101827"
                            border.width: 1
                            border.color: parent.hovered ? "#5bd946ef" : "#29475a"
                        }

                        contentItem: RowLayout {
                            anchors.fill: parent
                            anchors.leftMargin: 8
                            anchors.rightMargin: 8
                            spacing: 7

                            Loader {
                                Layout.preferredWidth: 24
                                Layout.preferredHeight: 24
                                sourceComponent: appIconVisual
                                onLoaded: {
                                    item.appId = appId
                                    item.label = label
                                    item.iconSize = 28
                                }
                            }

                            Text {
                                Layout.fillWidth: true
                                text: label
                                color: "white"
                                elide: Text.ElideRight
                                font.pixelSize: 12
                            }
                        }
                    }
                }
            }

            RowLayout {
                id: workspaceStrip
                spacing: 4

                Repeater {
                    model: 4

                    delegate: Rectangle {
                        width: 32
                        height: 32
                        radius: 8
                        color: index === 1 ? "#7c2cecff" : "#7c111827"
                        border.width: 1
                        border.color: index === 1 ? "#f0d946ef" : "#34475a"

                        Text {
                            anchors.centerIn: parent
                            text: index + 1
                            color: "white"
                            font.bold: index === 1
                            font.pixelSize: 13
                        }
                    }
                }
            }

            Rectangle {
                Layout.preferredWidth: 1
                Layout.preferredHeight: 38
                color: "#365b7088"
            }

            RowLayout {
                spacing: 8

                Repeater {
                    model: ["VOL", "NET", "BT"]

                    delegate: Rectangle {
                        width: 38
                        height: 32
                        radius: 8
                        color: "#7c111827"
                        border.width: 1
                        border.color: "#34475a"

                        Text {
                            anchors.centerIn: parent
                            text: modelData
                            color: "#d9f7ff"
                            font.pixelSize: 10
                            font.bold: true
                        }
                    }
                }
            }

            Button {
                id: clockButton
                Layout.preferredWidth: 118
                Layout.preferredHeight: 52
                padding: 0

                background: Rectangle {
                    radius: 12
                    color: clockButton.hovered ? "#2b233b5a" : "#95101827"
                    border.width: 1
                    border.color: clockButton.hovered ? "#35d6ff" : "#34475a"
                }

                contentItem: Column {
                    anchors.centerIn: parent
                    width: parent.width - 10
                    spacing: 1

                    Text {
                        width: parent.width
                        text: root.formattedTime()
                        color: "white"
                        horizontalAlignment: Text.AlignHCenter
                        font.bold: true
                        font.pixelSize: 17
                    }

                    Text {
                        width: parent.width
                        text: root.formattedDate()
                        color: "#9bb7c9"
                        horizontalAlignment: Text.AlignHCenter
                        elide: Text.ElideRight
                        font.pixelSize: 10
                    }
                }
            }
        }
    }
}
