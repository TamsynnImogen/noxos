import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import QtMultimedia 5.15
import QtQuick.Window 2.15
import Qt.labs.platform 1.1 as Platform

ApplicationWindow {
    id: root
    width: 1480
    height: 920
    visible: true
    title: modelRef ? modelRef.window_title : "Image Viewer"
    color: "#111418"
    property var modelRef: (typeof imageViewerModel !== "undefined") ? imageViewerModel : null
    property string homePath: Platform.StandardPaths.writableLocation(Platform.StandardPaths.HomeLocation)
    property string picturesPath: Platform.StandardPaths.writableLocation(Platform.StandardPaths.PicturesLocation)
    property bool showChrome: true
    property string uiFontFamily: "Noto Sans"

    function currentMediaKind() {
        return modelRef && modelRef.current_media_kind ? modelRef.current_media_kind.toString() : ""
    }

    function currentIsVideo() {
        return currentMediaKind() === "video"
    }

    function currentIsAnimated() {
        return currentMediaKind() === "animated"
    }

    function currentIsStillImage() {
        return currentMediaKind() === "image"
    }

    function galleryItems() {
        if (!modelRef || !modelRef.items_json || modelRef.items_json.length === 0)
            return []
        try {
            return JSON.parse(modelRef.items_json)
        } catch (error) {
            console.log("Failed to parse gallery items:", error)
            return []
        }
    }

    function fileUrlToPath(urlValue) {
        const urlText = urlValue.toString()
        if (urlText.indexOf("file://") === 0)
            return decodeURIComponent(urlText.slice(7))
        return urlText
    }

    function pathToFileUrl(path) {
        if (!path || path.length === 0)
            return ""
        const encoded = path.split("/").map(function(part, index) {
            if (index === 0 && part === "")
                return ""
            return encodeURIComponent(part)
        }).join("/")
        return "file://" + encoded
    }

    function suggestedExportPath() {
        if (!modelRef || !modelRef.has_image)
            return ""

        let baseName = modelRef.current_name && modelRef.current_name.length > 0 ? modelRef.current_name : "image"
        const dotIndex = baseName.lastIndexOf(".")
        if (dotIndex > 0)
            baseName = baseName.slice(0, dotIndex)

        const currentPath = modelRef.current_path ? modelRef.current_path.toString() : ""
        let folder = picturesPath && picturesPath.length > 0 ? picturesPath : homePath

        if (currentPath.indexOf("archive://") !== 0 && currentPath.indexOf("/") >= 0) {
            const slashIndex = currentPath.lastIndexOf("/")
            if (slashIndex > 0)
                folder = currentPath.slice(0, slashIndex)
        }

        return folder + "/" + baseName + "-export.png"
    }

    function openExportDialog() {
        if (!modelRef || !modelRef.has_image)
            return
        exportDialog.currentFile = root.pathToFileUrl(root.suggestedExportPath())
        exportDialog.open()
    }

    function openSourceDialog() {
        openDialog.folder = root.pathToFileUrl(picturesPath && picturesPath.length > 0 ? picturesPath : homePath)
        openDialog.open()
    }

    Shortcut {
        context: Qt.ApplicationShortcut
        sequence: StandardKey.Open
        onActivated: root.openSourceDialog()
    }

    Shortcut {
        context: Qt.ApplicationShortcut
        sequence: StandardKey.Save
        enabled: root.modelRef && root.modelRef.has_image
        onActivated: root.openExportDialog()
    }

    Shortcut {
        context: Qt.ApplicationShortcut
        sequence: "Left"
        enabled: root.modelRef && root.modelRef.can_go_previous
        onActivated: root.modelRef.previous_image()
    }

    Shortcut {
        context: Qt.ApplicationShortcut
        sequence: "Right"
        enabled: root.modelRef && root.modelRef.can_go_next
        onActivated: root.modelRef.next_image()
    }

    Shortcut {
        context: Qt.ApplicationShortcut
        sequence: "Space"
        enabled: root.modelRef && root.modelRef.has_image
        onActivated: root.modelRef.set_slideshow_running(!root.modelRef.slideshow_running)
    }

    Shortcut {
        context: Qt.ApplicationShortcut
        sequence: "R"
        enabled: root.modelRef && root.modelRef.has_image && !root.currentIsVideo()
        onActivated: root.modelRef.rotate_right()
    }

    Shortcut {
        context: Qt.ApplicationShortcut
        sequence: "Shift+R"
        enabled: root.modelRef && root.modelRef.has_image && !root.currentIsVideo()
        onActivated: root.modelRef.rotate_left()
    }

    Shortcut {
        context: Qt.ApplicationShortcut
        sequence: "L"
        enabled: root.modelRef && root.modelRef.has_image && !root.currentIsVideo()
        onActivated: root.modelRef.rotate_left()
    }

    Shortcut {
        context: Qt.ApplicationShortcut
        sequence: "0"
        enabled: root.modelRef && root.modelRef.has_image && root.modelRef.current_rotation !== 0
        onActivated: root.modelRef.reset_rotation()
    }

    Shortcut {
        context: Qt.ApplicationShortcut
        sequence: "F11"
        onActivated: root.visibility = root.visibility === Window.FullScreen ? Window.Windowed : Window.FullScreen
    }

    Shortcut {
        context: Qt.ApplicationShortcut
        sequence: "Escape"
        enabled: root.visibility === Window.FullScreen
        onActivated: root.visibility = Window.Windowed
    }

    Shortcut {
        context: Qt.ApplicationShortcut
        sequence: "Tab"
        onActivated: root.showChrome = !root.showChrome
    }

    Timer {
        interval: Math.max(2, modelRef ? modelRef.slideshow_interval_seconds : 4) * 1000
        repeat: true
        running: modelRef && modelRef.slideshow_running
        onTriggered: {
            if (modelRef)
                modelRef.advance_slideshow()
        }
    }

    Platform.FileDialog {
        id: openDialog
        title: "Open Media or Archive"
        fileMode: Platform.FileDialog.OpenFile
        nameFilters: [
            "Media and archives (*.png *.jpg *.jpeg *.gif *.webp *.bmp *.svg *.mp4 *.mkv *.webm *.mov *.avi *.m4v *.ogv *.zip *.rar *.7z *.tar *.gz *.bz2 *.xz *.tgz *.tbz2 *.txz)",
            "Images and animation (*.png *.jpg *.jpeg *.gif *.webp *.bmp *.svg)",
            "Videos (*.mp4 *.mkv *.webm *.mov *.avi *.m4v *.ogv)",
            "Archives (*.zip *.rar *.7z *.tar *.gz *.bz2 *.xz *.tgz *.tbz2 *.txz)"
        ]
        onAccepted: {
            if (root.modelRef)
                root.modelRef.open_target(root.fileUrlToPath(openDialog.file))
        }
    }

    Platform.FileDialog {
        id: exportDialog
        title: "Save Image Copy"
        fileMode: Platform.FileDialog.SaveFile
        nameFilters: [
            "PNG image (*.png)",
            "JPEG image (*.jpg *.jpeg)",
            "WEBP image (*.webp)",
            "BMP image (*.bmp)",
            "GIF image (*.gif)"
        ]
        onAccepted: {
            if (root.modelRef)
                root.modelRef.export_current(root.fileUrlToPath(currentFile))
        }
    }

    Rectangle {
        anchors.fill: parent
        gradient: Gradient {
            GradientStop { position: 0.0; color: "#181d25" }
            GradientStop { position: 0.5; color: "#0f1318" }
            GradientStop { position: 1.0; color: "#171b20" }
        }
    }

    Rectangle {
        anchors.fill: parent
        color: "transparent"
        border.color: "#22303b"
        border.width: 1
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 18
        spacing: 14

        Rectangle {
            visible: root.showChrome
            Layout.fillWidth: true
            radius: 22
            color: "#d8c2a2"
            border.color: "#f0dfc7"
            border.width: 1
            implicitHeight: toolbarLayout.implicitHeight + 22

            RowLayout {
                id: toolbarLayout
                anchors.fill: parent
                anchors.margins: 11
                spacing: 10

                Label {
                    text: modelRef && modelRef.has_image ? modelRef.current_name : "Image Viewer"
                    color: "#1f1a15"
                    font.family: root.uiFontFamily
                    font.pixelSize: 20
                    font.bold: true
                    Layout.preferredWidth: 260
                    Layout.minimumWidth: 180
                    elide: Text.ElideRight
                }

                Button {
                    text: "Open"
                    onClicked: root.openSourceDialog()
                }

                Button {
                    text: "Prev"
                    enabled: modelRef && modelRef.can_go_previous
                    onClicked: modelRef.previous_image()
                }

                Button {
                    text: "Next"
                    enabled: modelRef && modelRef.can_go_next
                    onClicked: modelRef.next_image()
                }

                Button {
                    text: modelRef && modelRef.slideshow_running ? "Pause" : "Slideshow"
                    enabled: modelRef && modelRef.has_image
                    onClicked: modelRef.set_slideshow_running(!modelRef.slideshow_running)
                }

                SpinBox {
                    id: slideshowSeconds
                    from: 2
                    to: 60
                    editable: true
                    enabled: modelRef !== null
                    value: modelRef ? modelRef.slideshow_interval_seconds : 4
                    onValueChanged: {
                        if (root.modelRef && root.modelRef.slideshow_interval_seconds !== value)
                            root.modelRef.set_slideshow_interval_seconds(value)
                    }
                }

                        Button {
                            text: "Rotate Left"
                            enabled: modelRef && modelRef.has_image && !root.currentIsVideo()
                            onClicked: modelRef.rotate_left()
                        }

                        Button {
                            text: "Rotate Right"
                            enabled: modelRef && modelRef.has_image && !root.currentIsVideo()
                            onClicked: modelRef.rotate_right()
                        }

                        Button {
                            text: "Reset"
                            enabled: modelRef && modelRef.has_image && !root.currentIsVideo() && modelRef.current_rotation !== 0
                            onClicked: modelRef.reset_rotation()
                        }

                Button {
                    text: "Export"
                    enabled: modelRef && modelRef.has_image && !root.currentIsVideo()
                    onClicked: root.openExportDialog()
                }

                Button {
                    text: modelRef && modelRef.translation_busy ? "Processing..." : "OCR / Translate"
                    enabled: modelRef && modelRef.has_image && root.currentIsStillImage() && !modelRef.translation_busy
                    onClicked: modelRef.translate_current_text()
                }

                Button {
                    text: root.visibility === Window.FullScreen ? "Windowed" : "Fullscreen"
                    onClicked: root.visibility = root.visibility === Window.FullScreen ? Window.Windowed : Window.FullScreen
                }

                Item {
                    Layout.fillWidth: true
                }

                ColumnLayout {
                    spacing: 2

                    Label {
                        text: modelRef && modelRef.has_image ? ((modelRef.current_index + 1) + " / " + modelRef.current_count) : "0 / 0"
                        color: "#1f1a15"
                        font.family: root.uiFontFamily
                        font.pixelSize: 16
                        font.bold: true
                        horizontalAlignment: Text.AlignRight
                        Layout.alignment: Qt.AlignRight
                    }

                    Label {
                        text: modelRef && modelRef.has_image ? (modelRef.current_format + "  •  " + modelRef.current_dimensions) : "No image loaded"
                        color: "#3a322a"
                        font.family: root.uiFontFamily
                        font.pixelSize: 13
                        horizontalAlignment: Text.AlignRight
                        Layout.alignment: Qt.AlignRight
                    }
                }
            }
        }

        Rectangle {
            visible: modelRef && modelRef.error_message.length > 0
            Layout.fillWidth: true
            radius: 14
            color: "#6a201b"
            border.color: "#af5b51"
            border.width: 1
            implicitHeight: errorLabel.implicitHeight + 18

            Label {
                id: errorLabel
                anchors.fill: parent
                anchors.margins: 9
                text: modelRef ? modelRef.error_message : ""
                color: "#f8e9e3"
                wrapMode: Text.Wrap
                font.family: root.uiFontFamily
                font.pixelSize: 13
            }
        }

        Rectangle {
            visible: root.showChrome && modelRef && modelRef.has_image && root.currentIsStillImage()
            Layout.fillWidth: true
            radius: 18
            color: "#182028"
            border.color: "#33404c"
            border.width: 1
            implicitHeight: translationPanel.implicitHeight + 18

            ColumnLayout {
                id: translationPanel
                anchors.fill: parent
                anchors.margins: 9
                spacing: 8

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 8

                    Label {
                        text: "Translate Still Image"
                        color: "#efe4d1"
                        font.family: root.uiFontFamily
                        font.pixelSize: 15
                        font.bold: true
                    }

                    Label {
                        text: "Target"
                        color: "#aab8c6"
                        font.family: root.uiFontFamily
                        font.pixelSize: 12
                    }

                    TextField {
                        id: targetLanguageField
                        text: modelRef ? modelRef.translation_target_language : "en"
                        placeholderText: "en"
                        Layout.preferredWidth: 90
                        onEditingFinished: {
                            if (root.modelRef)
                                root.modelRef.set_translation_target_language(text)
                        }
                    }

                    Button {
                        text: modelRef && modelRef.translation_busy ? "Working..." : "Run OCR / Translate"
                        enabled: modelRef && !modelRef.translation_busy
                        onClicked: {
                            if (root.modelRef) {
                                root.modelRef.set_translation_target_language(targetLanguageField.text)
                                root.modelRef.translate_current_text()
                            }
                        }
                    }

                    Label {
                        text: modelRef && modelRef.translation_detected_language.length > 0
                            ? ("Detected: " + modelRef.translation_detected_language)
                            : "Detected: -"
                        color: "#aab8c6"
                        font.family: root.uiFontFamily
                        font.pixelSize: 12
                        Layout.fillWidth: true
                    }
                }

                Label {
                    text: modelRef ? modelRef.translation_status : ""
                    color: modelRef && modelRef.translation_busy ? "#d8c2a2" : "#9db1c3"
                    font.family: root.uiFontFamily
                    font.pixelSize: 12
                    wrapMode: Text.Wrap
                    Layout.fillWidth: true
                }

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 8

                    Rectangle {
                        Layout.fillWidth: true
                        Layout.preferredHeight: 110
                        radius: 12
                        color: "#10161c"
                        border.color: "#2c3742"
                        border.width: 1

                        ColumnLayout {
                            anchors.fill: parent
                            anchors.margins: 8
                            spacing: 6

                            Label {
                                text: "OCR Text"
                                color: "#dfe8f1"
                                font.family: root.uiFontFamily
                                font.pixelSize: 12
                                font.bold: true
                            }

                            ScrollView {
                                Layout.fillWidth: true
                                Layout.fillHeight: true

                                TextArea {
                                    readOnly: true
                                    wrapMode: TextEdit.Wrap
                                    text: modelRef ? modelRef.translation_extracted_text : ""
                                    color: "#d6dee7"
                                    background: null
                                }
                            }
                        }
                    }

                    Rectangle {
                        Layout.fillWidth: true
                        Layout.preferredHeight: 110
                        radius: 12
                        color: "#10161c"
                        border.color: "#2c3742"
                        border.width: 1

                        ColumnLayout {
                            anchors.fill: parent
                            anchors.margins: 8
                            spacing: 6

                            Label {
                                text: "Translated Text"
                                color: "#dfe8f1"
                                font.family: root.uiFontFamily
                                font.pixelSize: 12
                                font.bold: true
                            }

                            ScrollView {
                                Layout.fillWidth: true
                                Layout.fillHeight: true

                                TextArea {
                                    readOnly: true
                                    wrapMode: TextEdit.Wrap
                                    text: modelRef ? modelRef.translation_text : ""
                                    color: "#efe4d1"
                                    background: null
                                }
                            }
                        }
                    }
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            radius: 28
            color: "#0a0d10"
            border.color: "#26303a"
            border.width: 1
            clip: true

            Rectangle {
                anchors.fill: parent
                gradient: Gradient {
                    GradientStop { position: 0.0; color: "#131b22" }
                    GradientStop { position: 1.0; color: "#080a0d" }
                }
            }

            Item {
                anchors.fill: parent
                anchors.margins: root.showChrome ? 24 : 10

                Rectangle {
                    anchors.fill: parent
                    color: "#10161c"
                    radius: 20
                }

                Item {
                    id: viewport
                    anchors.fill: parent
                    anchors.margins: 20
                    property bool quarterTurn: modelRef && modelRef.current_rotation % 180 !== 0

                    Image {
                        id: stageImage
                        visible: modelRef && modelRef.has_image && root.currentIsStillImage()
                        anchors.centerIn: parent
                        width: viewport.quarterTurn ? parent.height : parent.width
                        height: viewport.quarterTurn ? parent.width : parent.height
                        source: modelRef ? modelRef.current_source_url : ""
                        asynchronous: true
                        cache: false
                        smooth: true
                        fillMode: Image.PreserveAspectFit
                        rotation: modelRef ? modelRef.current_rotation : 0

                        Behavior on rotation {
                            NumberAnimation {
                                duration: 140
                                easing.type: Easing.OutCubic
                            }
                        }

                        Behavior on opacity {
                            NumberAnimation {
                                duration: 180
                                easing.type: Easing.OutCubic
                            }
                        }
                    }

                    AnimatedImage {
                        id: animatedStageImage
                        visible: modelRef && modelRef.has_image && root.currentIsAnimated()
                        anchors.centerIn: parent
                        width: viewport.quarterTurn ? parent.height : parent.width
                        height: viewport.quarterTurn ? parent.width : parent.height
                        source: modelRef ? modelRef.current_source_url : ""
                        cache: false
                        fillMode: Image.PreserveAspectFit
                        playing: visible
                        paused: !visible
                        rotation: modelRef ? modelRef.current_rotation : 0

                        Behavior on rotation {
                            NumberAnimation {
                                duration: 140
                                easing.type: Easing.OutCubic
                            }
                        }
                    }

                    Video {
                        id: videoStage
                        visible: modelRef && modelRef.has_image && root.currentIsVideo()
                        anchors.fill: parent
                        source: modelRef ? modelRef.current_source_url : ""
                        autoPlay: visible
                        autoLoad: true
                        fillMode: VideoOutput.PreserveAspectFit
                        muted: false
                        focus: visible
                    }

                    Column {
                        visible: !(modelRef && modelRef.has_image)
                        anchors.centerIn: parent
                        spacing: 10

                        Label {
                            anchors.horizontalCenter: parent.horizontalCenter
                            text: "No image to display"
                            color: "#efe4d1"
                            font.family: root.uiFontFamily
                            font.pixelSize: 28
                            font.bold: true
                        }

                        Label {
                            anchors.horizontalCenter: parent.horizontalCenter
                            text: "Open media from Files, press Ctrl+O, or launch the app with a file path."
                            color: "#aab8c6"
                            font.family: root.uiFontFamily
                            font.pixelSize: 14
                        }

                        Button {
                            anchors.horizontalCenter: parent.horizontalCenter
                            text: "Open Media"
                            onClicked: root.openSourceDialog()
                        }
                    }
                }
            }
        }

        Rectangle {
            visible: root.showChrome
            Layout.fillWidth: true
            radius: 20
            color: "#171d24"
            border.color: "#28323d"
            border.width: 1
            implicitHeight: filmstripColumn.implicitHeight + 18

            ColumnLayout {
                id: filmstripColumn
                anchors.fill: parent
                anchors.margins: 9
                spacing: 8

                RowLayout {
                    Layout.fillWidth: true

                    Label {
                        text: "Filmstrip"
                        color: "#efe4d1"
                        font.family: root.uiFontFamily
                        font.pixelSize: 15
                        font.bold: true
                    }

                    Label {
                        text: modelRef ? modelRef.status_text : ""
                        color: "#aab8c6"
                        font.family: root.uiFontFamily
                        font.pixelSize: 12
                        elide: Text.ElideRight
                        Layout.fillWidth: true
                    }

                    Label {
                        text: "Tab hides chrome"
                        color: "#7f8d9b"
                        font.family: root.uiFontFamily
                        font.pixelSize: 12
                    }
                }

                Flickable {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 126
                    contentWidth: stripRow.width
                    contentHeight: stripRow.height
                    clip: true
                    boundsBehavior: Flickable.StopAtBounds

                    Row {
                        id: stripRow
                        spacing: 8

                        Repeater {
                            model: root.galleryItems()

                            delegate: MouseArea {
                                required property int index
                                required property var modelData
                                width: 112
                                height: 112
                                hoverEnabled: true
                                onClicked: {
                                    if (root.modelRef)
                                        root.modelRef.select_index(index)
                                }

                                Rectangle {
                                    anchors.fill: parent
                                    radius: 16
                                    color: root.modelRef && root.modelRef.current_index === index
                                        ? "#d8c2a2"
                                        : (parent.containsMouse ? "#27313b" : "#1c242c")
                                    border.color: root.modelRef && root.modelRef.current_index === index
                                        ? "#f4e8d7"
                                        : "#35424f"
                                    border.width: 1
                                }

                                Column {
                                    anchors.fill: parent
                                    anchors.margins: 8
                                    spacing: 6

                                    Rectangle {
                                        width: parent.width
                                        height: 72
                                        radius: 12
                                        color: "#0e1216"
                                        clip: true

                                        Image {
                                            anchors.fill: parent
                                            visible: modelData.media_kind !== "video"
                                            source: modelData.source_url
                                            fillMode: Image.PreserveAspectCrop
                                            asynchronous: true
                                            cache: false
                                            smooth: true
                                        }

                                        Rectangle {
                                            anchors.fill: parent
                                            visible: modelData.media_kind === "video"
                                            color: "#10161c"

                                            Label {
                                                anchors.centerIn: parent
                                                text: "Video"
                                                color: "#efe4d1"
                                                font.family: root.uiFontFamily
                                                font.pixelSize: 12
                                                font.bold: true
                                            }
                                        }
                                    }

                                    Label {
                                        width: parent.width
                                        text: modelData.name
                                        color: root.modelRef && root.modelRef.current_index === index ? "#1d1a16" : "#dce5ef"
                                        font.family: root.uiFontFamily
                                        font.pixelSize: 12
                                        maximumLineCount: 1
                                        elide: Text.ElideRight
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
