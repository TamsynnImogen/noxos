import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import QtQuick.Window 2.15
import Qt.labs.platform 1.1 as Platform
import Qt.labs.folderlistmodel 2.15 as Foldering

ApplicationWindow {
    id: root
    width: 1360
    height: 860
    visible: true
    title: "Files"
    property var modelRef: (typeof filesModel !== "undefined") ? filesModel : null
    property var devicesRef: (typeof devicesModel !== "undefined") ? devicesModel : null
    property string selectedPath: ""
    property string selectedName: ""
    property var selectedPaths: []
    property var selectedPathSet: ({})
    property int selectedCount: selectedPaths.length
    property string contextMenuPath: ""
    property string contextMenuName: ""
    property bool contextMenuIsDirectory: false
    property bool contextMenuHasTarget: false
    property string activeDragPath: ""
    property string dropTargetPath: ""
    property string themeMode: "System"
    property string homePath: Platform.StandardPaths.writableLocation(Platform.StandardPaths.HomeLocation)
    property string favoritesUri: "virtual://favorites"
    property string devicesUri: "virtual://devices"
    property bool treeChildrenExpanded: true
    property int favoritesHeight: 210
    property int devicesHeight: 140
    property int sidebarWidth: 220
    property int infoPanelWidth: 250
    property int terminalPanelHeight: 220
    property int detailsIconColumnWidth: 28
    property int detailsNameColumnWidth: 340
    property int detailsTypeColumnWidth: 88
    property int detailsSizeColumnWidth: 88
    property int detailsModifiedColumnWidth: 150
    property int detailsColumnSpacing: 12
    property int detailsHorizontalPadding: 10
    property int toolbarButtonSize: 32
    property int toolbarIconSize: 20
    property int treeRowHeight: 22
    property int treeIndentSize: 14
    property int treeChevronSize: 16
    property int treeFolderIconSize: 16
    property int detailsColumnsWidth: detailsIconColumnWidth
        + detailsNameColumnWidth
        + detailsTypeColumnWidth
        + detailsSizeColumnWidth
        + detailsModifiedColumnWidth
        + (detailsColumnSpacing * 4)
    property int detailsContentWidth: detailsColumnsWidth + (detailsHorizontalPadding * 2)
    property int detailsNameColumnX: detailsIconColumnWidth + detailsColumnSpacing
    property int detailsTypeColumnX: detailsNameColumnX + detailsNameColumnWidth + detailsColumnSpacing
    property int detailsSizeColumnX: detailsTypeColumnX + detailsTypeColumnWidth + detailsColumnSpacing
    property int detailsModifiedColumnX: detailsSizeColumnX + detailsSizeColumnWidth + detailsColumnSpacing
    property string viewMode: root.modelRef ? root.modelRef.folder_view_mode : "Details"
    property bool darkTheme: themeMode === "Dark" || (themeMode === "System" && isDarkColor(systemPalette.window))
    property color windowColor: darkTheme ? "#1d2128" : "#f4f1ea"
    property color surfaceColor: darkTheme ? "#232831" : "#fffdf9"
    property color sidebarColor: darkTheme ? "#262c35" : "#fffaf2"
    property color chromeColor: darkTheme ? "#2a2f39" : "#2d2f34"
    property color borderColor: darkTheme ? "#3b4556" : "#d8cebb"
    property color textColor: darkTheme ? "#ebe7de" : "#1f1d1a"
    property color mutedTextColor: darkTheme ? "#b8c0cd" : "#635b50"
    property color accentColor: darkTheme ? "#7db0b6" : "#4f7c82"
    property color rowEvenColor: darkTheme ? "#29303a" : "#fcfaf5"
    property color rowOddColor: darkTheme ? "#242b35" : "#f5efe4"
    property color rowSelectedColor: darkTheme ? "#32434c" : "#dbe8ea"
    property color buttonColor: darkTheme ? "#374252" : "#d7c3a5"
    property color inputColor: darkTheme ? "#181c22" : "#fffdf9"
    property color backgroundStart: darkTheme ? "#1f242c" : "#f6f0e6"
    property color backgroundEnd: darkTheme ? "#171b22" : "#e3dccd"
    property color scrollbarTrackColor: darkTheme ? "#10141a" : "#e2d7c4"
    property color scrollbarThumbColor: darkTheme ? "#b7cdd4" : "#426d75"
    property color scrollbarThumbBorderColor: darkTheme ? "#d9e6ea" : "#24474f"

    SystemPalette {
        id: systemPalette
        colorGroup: SystemPalette.Active
    }

    Timer {
        interval: 1200
        repeat: true
        running: true
        onTriggered: {
            if (root.modelRef && root.modelRef.thumbnail_jobs_pending() > 0)
                root.modelRef.refresh()
        }
    }

    Connections {
        target: root.modelRef
        function onCurrent_path_changed() {
            root.clearSelection()
        }
    }

    function normalizedPathParts(path) {
        if (!path || path.length === 0)
            return []
        if (path.indexOf("virtual://") === 0)
            return [{ label: path.slice("virtual://".length).charAt(0).toUpperCase() + path.slice("virtual://".length + 1), path: path, depth: 0 }]
        if (path === "/")
            return [{ label: "/", path: "/", depth: 0 }]

        const parts = path.split("/").filter(function(part) { return part.length > 0 })
        const items = [{ label: "/", path: "/", depth: 0 }]
        let current = ""
        for (let i = 0; i < parts.length; ++i) {
            current += "/" + parts[i]
            items.push({
                label: parts[i],
                path: current,
                depth: i + 1
            })
        }
        return items
    }

    function treeRootPath(path) {
        const currentPath = root.fileUrlToPath(path || "")
        const home = root.fileUrlToPath(root.homePath)
        if (home && home.length > 0 && (currentPath === home || currentPath.indexOf(home + "/") === 0))
            return home

        const parts = currentPath.split("/").filter(function(part) { return part.length > 0 })
        if (parts.length >= 3 && parts[0] === "media")
            return "/" + parts.slice(0, 3).join("/")
        if (parts.length >= 4 && parts[0] === "run" && parts[1] === "media")
            return "/" + parts.slice(0, 4).join("/")
        if (parts.length >= 2 && parts[0] === "mnt")
            return "/" + parts.slice(0, 2).join("/")

        return "/"
    }

    function isVirtualPath(path) {
        return !!path && path.indexOf("virtual://") === 0
    }

    function relativeTreeParts(path) {
        if (root.isVirtualPath(path)) {
            const label = path === root.favoritesUri ? "Favourites" : (path === root.devicesUri ? "Devices" : path.slice("virtual://".length))
            return [{ label: label, path: path, depth: 0 }]
        }
        const treeRoot = root.treeRootPath(path)
        if (!path || path.length === 0)
            return []
        if (treeRoot === "/")
            return root.normalizedPathParts(path)
        if (path === treeRoot)
            return [{ label: treeRoot.split("/").pop() || treeRoot, path: treeRoot, depth: 0 }]
        if (path.indexOf(treeRoot + "/") !== 0)
            return [{ label: treeRoot.split("/").pop() || treeRoot, path: treeRoot, depth: 0 }]

        const rootLabel = treeRoot.split("/").pop() || treeRoot
        const suffix = path.slice(treeRoot.length + 1)
        const parts = suffix.split("/").filter(function(part) { return part.length > 0 })
        const items = [{ label: rootLabel, path: treeRoot, depth: 0 }]
        let current = treeRoot
        for (let i = 0; i < parts.length; ++i) {
            current += "/" + parts[i]
            items.push({
                label: parts[i],
                path: current,
                depth: i + 1
            })
        }
        return items
    }

    function breadcrumbParts(path) {
        if (!path || path.length === 0)
            return []
        return normalizedPathParts(path)
    }

    function groupedEntries() {
        if (!root.modelRef || !root.modelRef.grouped_entries_json || root.modelRef.grouped_entries_json.length === 0)
            return []
        try {
            return JSON.parse(root.modelRef.grouped_entries_json)
        } catch (error) {
            console.log("Failed to parse grouped entries:", error)
            return []
        }
    }

    function formatModifiedValue(modifiedMs, modifiedText) {
        return modifiedMs > 0
            ? new Date(modifiedMs).toLocaleString(Qt.locale(), Locale.ShortFormat)
            : modifiedText
    }

    function isDarkColor(color) {
        return ((0.299 * color.r) + (0.587 * color.g) + (0.114 * color.b)) < 0.5
    }

    function fileUrlToPath(urlValue) {
        const urlText = urlValue.toString()
        if (urlText.indexOf("file://") === 0)
            return decodeURIComponent(urlText.slice(7))
        return urlText
    }

    function pathToFileUrl(path) {
        if (!path || path.length === 0 || root.isVirtualPath(path))
            return ""
        const encoded = path.split("/").map(function(part, index) {
            if (index === 0 && part === "")
                return ""
            return encodeURIComponent(part)
        }).join("/")
        return "file://" + encoded
    }

    function filePreviewSource(path, thumbnailUrl) {
        if (thumbnailUrl && thumbnailUrl.length > 0)
            return thumbnailUrl
        if (root.isImageFile(path))
            return root.pathToFileUrl(path)
        return ""
    }

    function isArchiveFile(path) {
        if (!path)
            return false
        const lowerPath = path.toString().toLowerCase()
        return lowerPath.endsWith(".zip")
            || lowerPath.endsWith(".rar")
            || lowerPath.endsWith(".7z")
            || lowerPath.endsWith(".tar")
            || lowerPath.endsWith(".gz")
            || lowerPath.endsWith(".bz2")
            || lowerPath.endsWith(".xz")
            || lowerPath.endsWith(".tgz")
            || lowerPath.endsWith(".tbz2")
            || lowerPath.endsWith(".txz")
    }

    function isImageFile(path) {
        if (!path || root.isVirtualPath(path))
            return false
        const lowerPath = path.toLowerCase()
        return lowerPath.endsWith(".png")
            || lowerPath.endsWith(".jpg")
            || lowerPath.endsWith(".jpeg")
            || lowerPath.endsWith(".gif")
            || lowerPath.endsWith(".webp")
            || lowerPath.endsWith(".bmp")
            || lowerPath.endsWith(".svg")
    }

    function folderIconSource(name, path) {
        const base = "../../../assets/icons/folders/"
        const nameText = (name === undefined || name === null) ? "" : name.toString()
        const pathText = (path === undefined || path === null) ? "" : path.toString()
        const lowerName = nameText.toLowerCase()
        const lowerPath = pathText.toLowerCase()

        if (pathText === root.favoritesUri)
            return base + "folder_favourites.svg"
        if (pathText === root.devicesUri)
            return base + "folder_system.svg"
        if (lowerName === "desktop" || lowerPath.endsWith("/desktop"))
            return base + "folder_desktop.svg"
        if (lowerName === "documents" || lowerPath.endsWith("/documents"))
            return base + "folder_documents.svg"
        if (lowerName === "downloads" || lowerPath.endsWith("/downloads"))
            return base + "folder_downloads.svg"
        if (lowerName === "pictures" || lowerPath.endsWith("/pictures"))
            return base + "folder_pictures.svg"
        if (lowerName === "music" || lowerPath.endsWith("/music"))
            return base + "folder_music.svg"
        if (lowerName === "videos" || lowerName === "videoes" || lowerPath.endsWith("/videos"))
            return base + "folder_videoes.svg"
        if (lowerName === "games" || lowerPath.indexOf("/games") >= 0)
            return base + "folder_games.svg"
        if (lowerName === "code" || lowerName === "crates" || lowerName === "apps" || lowerName === "src" || lowerName === "qml" || lowerName === "docs")
            return base + "folder_code.svg"
        if (lowerName.indexOf("archive") >= 0)
            return base + "folder_archieve.svg"
        if (lowerName === "trash" || lowerName.indexOf("waste") >= 0)
            return base + "folder_trash_empty.svg"
        if (lowerName === "home" || lowerPath === root.fileUrlToPath(root.homePath).toLowerCase())
            return base + "folder.svg"

        return base + "folder.svg"
    }

    function fileIconSource(path) {
        const base = "../../../assets/icons/folders/"
        if (root.isArchiveFile(path))
            return base + "folder_archieve.svg"
        return ""
    }

    function activateCurrentSelection(path, isDirectory) {
        if (root.modelRef && path.length > 0)
            root.modelRef.activate_entry(path, isDirectory)
    }

    function trashCurrentSelection(path) {
        if (!root.modelRef)
            return
        const paths = root.selectedPathsForAction(path)
        if (paths.length > 1)
            root.modelRef.move_paths_to_trash(JSON.stringify(paths))
        else if (paths.length === 1)
            root.modelRef.move_to_trash(paths[0])
    }

    function copyCurrentSelection(path) {
        if (!root.modelRef)
            return
        const paths = root.selectedPathsForAction(path)
        if (paths.length > 1)
            root.modelRef.copy_paths(JSON.stringify(paths))
        else if (paths.length === 1)
            root.modelRef.copy_path(paths[0])
    }

    function cutCurrentSelection(path) {
        if (!root.modelRef)
            return
        const paths = root.selectedPathsForAction(path)
        if (paths.length > 1)
            root.modelRef.cut_paths(JSON.stringify(paths))
        else if (paths.length === 1)
            root.modelRef.cut_path(paths[0])
    }

    function selectedPathsForAction(path) {
        if (path && path.length > 0 && root.isPathSelected(path) && root.selectedPaths.length > 0)
            return root.selectedPaths.slice()
        if (root.selectedPaths.length > 0 && (!path || path.length === 0))
            return root.selectedPaths.slice()
        return path && path.length > 0 ? [path] : []
    }

    function dragPathsFor(path) {
        if (path && root.isPathSelected(path) && root.selectedPaths.length > 0)
            return root.selectedPaths.slice()
        return path && path.length > 0 ? [path] : []
    }

    function dragMimeText(path) {
        const paths = root.dragPathsFor(path)
        return paths.map(function(item) { return root.pathToFileUrl(item) }).join("\n")
    }

    function isPathSelected(path) {
        return !!path && root.selectedPathSet[path] === true
    }

    function setSelection(paths, focusPath, focusName) {
        const nextPaths = []
        const nextSet = {}
        for (let i = 0; i < paths.length; ++i) {
            const path = paths[i]
            if (!path || path.length === 0 || nextSet[path])
                continue
            nextPaths.push(path)
            nextSet[path] = true
        }
        root.selectedPaths = nextPaths
        root.selectedPathSet = nextSet
        root.selectedPath = focusPath || (nextPaths.length === 1 ? nextPaths[0] : "")
        root.selectedName = focusName || ""
        if (root.modelRef)
            root.modelRef.set_selected_path(root.selectedPath)
    }

    function clearSelection() {
        root.setSelection([], "", "")
    }

    function selectEntry(path, name, additive) {
        if (!additive) {
            root.setSelection([path], path, name)
            return
        }

        const paths = root.selectedPaths.slice()
        if (!root.isPathSelected(path))
            paths.push(path)
        root.setSelection(paths, path, name)
    }

    function toggleEntrySelection(path, name) {
        const paths = []
        const selected = root.isPathSelected(path)
        for (let i = 0; i < root.selectedPaths.length; ++i) {
            if (root.selectedPaths[i] !== path)
                paths.push(root.selectedPaths[i])
        }
        if (!selected)
            paths.push(path)
        root.setSelection(paths, selected ? "" : path, selected ? "" : name)
    }

    function selectAllEntries() {
        if (!root.modelRef || !root.modelRef.current_paths_json)
            return
        try {
            const paths = JSON.parse(root.modelRef.current_paths_json())
            root.setSelection(paths, "", "")
        } catch (error) {
            console.log("Failed to select all entries:", error)
        }
    }

    function selectionMarkerText(path) {
        return root.isPathSelected(path) ? "✓" : "+"
    }

    function selectionMarkerVisible(path, hovered) {
        return root.isPathSelected(path) || hovered
    }

    function selectionMarkerColor(path) {
        return root.isPathSelected(path) ? root.accentColor : root.surfaceColor
    }

    function selectionMarkerTextColor(path) {
        return root.isPathSelected(path) ? "white" : root.accentColor
    }

    function isReadOnlyPath(path) {
        return root.isVirtualPath(path) || (!!path && path.indexOf("archive://") === 0)
    }

    function pathsFromDrop(drop) {
        const paths = []
        if (root.activeDragPath.length > 0) {
            return root.dragPathsFor(root.activeDragPath)
        }
        if (drop.source && drop.source.path && drop.source.path.length > 0) {
            return root.dragPathsFor(drop.source.path)
        }

        if (drop.hasUrls && drop.urls) {
            for (let i = 0; i < drop.urls.length; ++i) {
                const path = root.fileUrlToPath(drop.urls[i])
                if (path.length > 0 && !root.isReadOnlyPath(path))
                    paths.push(path)
            }
        }
        return paths
    }

    function canDropPathsInto(paths, destinationPath) {
        if (!root.modelRef || !destinationPath || destinationPath.length === 0 || root.isReadOnlyPath(destinationPath))
            return false
        if (!paths || paths.length === 0)
            return false

        for (let i = 0; i < paths.length; ++i) {
            const sourcePath = paths[i]
            if (!sourcePath || sourcePath.length === 0 || root.isReadOnlyPath(sourcePath))
                return false
            if (sourcePath === destinationPath)
                return false
            if (destinationPath.indexOf(sourcePath + "/") === 0)
                return false
        }
        return true
    }

    function updateDropTarget(drop, destinationPath) {
        const paths = root.pathsFromDrop(drop)
        const accepted = root.canDropPathsInto(paths, destinationPath)
        drop.accepted = accepted
        root.dropTargetPath = accepted ? destinationPath : ""
    }

    function clearDropTarget(destinationPath) {
        if (!destinationPath || root.dropTargetPath === destinationPath)
            root.dropTargetPath = ""
    }

    function handleDrop(drop, destinationPath) {
        const paths = root.pathsFromDrop(drop)
        if (!root.canDropPathsInto(paths, destinationPath)) {
            root.dropTargetPath = ""
            return
        }

        const internalDrag = root.activeDragPath.length > 0 || (drop.source && drop.source.path && drop.source.path.length > 0)
        const copy = !internalDrag || drop.proposedAction === Qt.CopyAction
        root.modelRef.drop_paths_into_directory(JSON.stringify(paths), destinationPath, copy)
        root.activeDragPath = ""
        root.dropTargetPath = ""
        drop.acceptProposedAction()
    }

    function openFileContextMenu(path, name, isDirectory, item, mouse) {
        if (path && path.length > 0 && !root.isPathSelected(path))
            root.selectEntry(path, name, false)
        root.contextMenuPath = path || ""
        root.contextMenuName = name || ""
        root.contextMenuIsDirectory = !!isDirectory
        root.contextMenuHasTarget = root.contextMenuPath.length > 0
        const pos = item.mapToItem(root.contentItem, mouse.x, mouse.y)
        fileContextMenu.x = pos.x
        fileContextMenu.y = pos.y
        fileContextMenu.open()
    }

    function openFolderContextMenu(item, mouse) {
        root.contextMenuPath = ""
        root.contextMenuName = ""
        root.contextMenuIsDirectory = false
        root.contextMenuHasTarget = false
        if (root.modelRef)
            root.modelRef.set_selected_path("")
        root.clearSelection()
        const pos = item.mapToItem(root.contentItem, mouse.x, mouse.y)
        fileContextMenu.x = pos.x
        fileContextMenu.y = pos.y
        fileContextMenu.open()
    }

    function handleFileShortcut(event, path) {
        if (!root.modelRef || !(event.modifiers & Qt.ControlModifier))
            return false

        const targetPath = path && path.length > 0 ? path : root.selectedPath
        if (event.key === Qt.Key_C && targetPath.length > 0 && !root.isReadOnlyPath(targetPath)) {
            root.copyCurrentSelection(targetPath)
            return true
        }
        if (event.key === Qt.Key_X && targetPath.length > 0 && !root.isReadOnlyPath(targetPath)) {
            root.cutCurrentSelection(targetPath)
            return true
        }
        if (event.key === Qt.Key_V && root.modelRef.can_paste && !root.isReadOnlyPath(root.modelRef.current_path)) {
            root.modelRef.paste_into_current()
            return true
        }
        if (event.key === Qt.Key_A) {
            root.selectAllEntries()
            return true
        }
        return false
    }

    function toolbarIconSource(fileName) {
        return "../../../assets/icons/folders/" + fileName
    }

    palette {
        window: root.windowColor
        base: root.surfaceColor
        alternateBase: root.rowOddColor
        text: root.textColor
        button: root.buttonColor
        buttonText: root.textColor
        highlight: root.accentColor
        highlightedText: root.surfaceColor
    }

    Component {
        id: detailsViewComponent

        ColumnLayout {
            id: detailsColumn
            property real headerContentX: 0
            property real scrollbarWidth: 14
            property real availableColumnsWidth: Math.max(
                root.detailsColumnsWidth,
                fileList.width - scrollbarWidth - (root.detailsHorizontalPadding * 2)
            )
            property real extraColumnsWidth: Math.max(0, availableColumnsWidth - root.detailsColumnsWidth)
            property real effectiveNameColumnWidth: root.detailsNameColumnWidth + extraColumnsWidth
            property real effectiveTypeColumnWidth: root.detailsTypeColumnWidth
            property real effectiveSizeColumnWidth: root.detailsSizeColumnWidth
            property real effectiveModifiedColumnWidth: root.detailsModifiedColumnWidth
            property real effectiveNameColumnX: root.detailsIconColumnWidth + root.detailsColumnSpacing
            property real effectiveTypeColumnX: effectiveNameColumnX + effectiveNameColumnWidth + root.detailsColumnSpacing
            property real effectiveSizeColumnX: effectiveTypeColumnX + effectiveTypeColumnWidth + root.detailsColumnSpacing
            property real effectiveModifiedColumnX: effectiveSizeColumnX + effectiveSizeColumnWidth + root.detailsColumnSpacing
            property real effectiveColumnsWidth: root.detailsIconColumnWidth
                + effectiveNameColumnWidth
                + effectiveTypeColumnWidth
                + effectiveSizeColumnWidth
                + effectiveModifiedColumnWidth
                + (root.detailsColumnSpacing * 4)
            property real effectiveContentWidth: effectiveColumnsWidth + (root.detailsHorizontalPadding * 2)
            spacing: 0

            Item {
                Layout.fillWidth: true
                Layout.preferredHeight: 32
                Layout.minimumHeight: 32
                clip: true

                Item {
                    id: detailsHeaderRow
                    x: root.detailsHorizontalPadding - detailsColumn.headerContentX
                    width: detailsColumn.effectiveColumnsWidth
                    height: parent.height
                    anchors.verticalCenter: parent.verticalCenter

                    Label {
                        x: detailsColumn.effectiveNameColumnX
                        width: detailsColumn.effectiveNameColumnWidth
                        anchors.top: parent.top
                        anchors.bottom: parent.bottom
                        text: "Name"
                        color: root.mutedTextColor
                        font.bold: true
                        font.pixelSize: 13
                        horizontalAlignment: Text.AlignLeft
                        verticalAlignment: Text.AlignVCenter
                    }

                    Label {
                        x: detailsColumn.effectiveTypeColumnX
                        width: detailsColumn.effectiveTypeColumnWidth
                        anchors.top: parent.top
                        anchors.bottom: parent.bottom
                        text: "Type"
                        color: root.mutedTextColor
                        font.bold: true
                        font.pixelSize: 13
                        horizontalAlignment: Text.AlignLeft
                        verticalAlignment: Text.AlignVCenter
                    }

                    Label {
                        x: detailsColumn.effectiveSizeColumnX
                        width: detailsColumn.effectiveSizeColumnWidth
                        anchors.top: parent.top
                        anchors.bottom: parent.bottom
                        text: "Size"
                        color: root.mutedTextColor
                        font.bold: true
                        font.pixelSize: 13
                        horizontalAlignment: Text.AlignLeft
                        verticalAlignment: Text.AlignVCenter
                    }

                    Label {
                        x: detailsColumn.effectiveModifiedColumnX
                        width: detailsColumn.effectiveModifiedColumnWidth
                        anchors.top: parent.top
                        anchors.bottom: parent.bottom
                        text: "Modified"
                        color: root.mutedTextColor
                        font.bold: true
                        font.pixelSize: 13
                        horizontalAlignment: Text.AlignLeft
                        verticalAlignment: Text.AlignVCenter
                    }
                }

                Item {
                    x: detailsHeaderRow.x
                    anchors.top: parent.top
                    anchors.bottom: parent.bottom
                    width: detailsColumn.effectiveColumnsWidth

                    Item {
                        x: detailsColumn.effectiveTypeColumnX - (root.detailsColumnSpacing / 2) - (width / 2)
                        width: 14
                        anchors.top: parent.top
                        anchors.bottom: parent.bottom

                        Rectangle {
                            anchors.horizontalCenter: parent.horizontalCenter
                            anchors.top: parent.top
                            anchors.bottom: parent.bottom
                            width: 3
                            radius: 1
                            color: root.accentColor
                            opacity: 0.75
                        }

                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.SizeHorCursor
                            property real startSceneX: 0
                            property int startLeftWidth: 0
                            property int startRightWidth: 0
                            onPressed: {
                                startSceneX = mapToItem(root.contentItem, mouse.x, mouse.y).x
                                startLeftWidth = root.detailsNameColumnWidth
                                startRightWidth = root.detailsTypeColumnWidth
                            }
                            onPositionChanged: {
                                if (!pressed)
                                    return
                                const rawDelta = mapToItem(root.contentItem, mouse.x, mouse.y).x - startSceneX
                                const delta = Math.max(
                                    Math.max(140 - startLeftWidth, startRightWidth - 260),
                                    Math.min(Math.min(900 - startLeftWidth, startRightWidth - 70), rawDelta)
                                )
                                root.detailsNameColumnWidth = startLeftWidth + delta
                                root.detailsTypeColumnWidth = startRightWidth - delta
                            }
                        }
                    }

                    Item {
                        x: detailsColumn.effectiveSizeColumnX - (root.detailsColumnSpacing / 2) - (width / 2)
                        width: 14
                        anchors.top: parent.top
                        anchors.bottom: parent.bottom

                        Rectangle {
                            anchors.horizontalCenter: parent.horizontalCenter
                            anchors.top: parent.top
                            anchors.bottom: parent.bottom
                            width: 3
                            radius: 1
                            color: root.accentColor
                            opacity: 0.75
                        }

                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.SizeHorCursor
                            property real startSceneX: 0
                            property int startLeftWidth: 0
                            property int startRightWidth: 0
                            onPressed: {
                                startSceneX = mapToItem(root.contentItem, mouse.x, mouse.y).x
                                startLeftWidth = root.detailsTypeColumnWidth
                                startRightWidth = root.detailsSizeColumnWidth
                            }
                            onPositionChanged: {
                                if (!pressed)
                                    return
                                const rawDelta = mapToItem(root.contentItem, mouse.x, mouse.y).x - startSceneX
                                const delta = Math.max(
                                    Math.max(70 - startLeftWidth, startRightWidth - 260),
                                    Math.min(Math.min(260 - startLeftWidth, startRightWidth - 70), rawDelta)
                                )
                                root.detailsTypeColumnWidth = startLeftWidth + delta
                                root.detailsSizeColumnWidth = startRightWidth - delta
                            }
                        }
                    }

                    Item {
                        x: detailsColumn.effectiveModifiedColumnX - (root.detailsColumnSpacing / 2) - (width / 2)
                        width: 14
                        anchors.top: parent.top
                        anchors.bottom: parent.bottom

                        Rectangle {
                            anchors.horizontalCenter: parent.horizontalCenter
                            anchors.top: parent.top
                            anchors.bottom: parent.bottom
                            width: 3
                            radius: 1
                            color: root.accentColor
                            opacity: 0.75
                        }

                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.SizeHorCursor
                            property real startSceneX: 0
                            property int startLeftWidth: 0
                            property int startRightWidth: 0
                            onPressed: {
                                startSceneX = mapToItem(root.contentItem, mouse.x, mouse.y).x
                                startLeftWidth = root.detailsSizeColumnWidth
                                startRightWidth = root.detailsModifiedColumnWidth
                            }
                            onPositionChanged: {
                                if (!pressed)
                                    return
                                const rawDelta = mapToItem(root.contentItem, mouse.x, mouse.y).x - startSceneX
                                const delta = Math.max(
                                    Math.max(70 - startLeftWidth, startRightWidth - 360),
                                    Math.min(Math.min(260 - startLeftWidth, startRightWidth - 100), rawDelta)
                                )
                                root.detailsSizeColumnWidth = startLeftWidth + delta
                                root.detailsModifiedColumnWidth = startRightWidth - delta
                            }
                        }
                    }
                }
            }

            ListView {
                id: fileList
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                spacing: 6
                model: root.modelRef
                contentWidth: Math.max(width, detailsColumn.effectiveContentWidth)
                flickableDirection: Flickable.AutoFlickIfNeeded
                boundsBehavior: Flickable.StopAtBounds
                currentIndex: -1
                focus: true
                onContentXChanged: detailsColumn.headerContentX = contentX
                ScrollBar.vertical: ScrollBar {
                    policy: ScrollBar.AlwaysOn
                    width: 14
                    visible: true
                    opacity: 1.0
                    active: true
                    interactive: true
                    background: Rectangle {
                        implicitWidth: 14
                        implicitHeight: 14
                        color: root.scrollbarTrackColor
                        radius: 4
                    }
                    contentItem: Rectangle {
                        width: 12
                        implicitWidth: 12
                        implicitHeight: 36
                        radius: 6
                        color: root.scrollbarThumbColor
                        border.width: 1
                        border.color: root.scrollbarThumbBorderColor
                    }
                }
                ScrollBar.horizontal: ScrollBar {
                    policy: ScrollBar.AlwaysOn
                    height: 14
                    visible: true
                    opacity: 1.0
                    active: true
                    interactive: true
                    background: Rectangle {
                        implicitWidth: 14
                        implicitHeight: 14
                        color: root.scrollbarTrackColor
                        radius: 4
                    }
                    contentItem: Rectangle {
                        height: 12
                        implicitWidth: 36
                        implicitHeight: 12
                        radius: 6
                        color: root.scrollbarThumbColor
                        border.width: 1
                        border.color: root.scrollbarThumbBorderColor
                    }
                }
                section.property: root.modelRef && root.modelRef.grouping_name !== "None" ? "groupLabel" : ""
                section.criteria: ViewSection.FullString
                section.labelPositioning: ViewSection.InlineLabels
                section.delegate: Item {
                    visible: root.modelRef && root.modelRef.grouping_name !== "None" && section.length > 0
                    width: fileList.width
                    height: visible ? 26 : 0

                    Text {
                        anchors.left: parent.left
                        anchors.leftMargin: 2
                        anchors.verticalCenter: parent.verticalCenter
                        text: section
                        color: root.mutedTextColor
                        font.pixelSize: 13
                    }

                    Rectangle {
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.bottom: parent.bottom
                        height: 1
                        color: root.borderColor
                    }
                }

                property string currentItemPath: currentItem ? currentItem.path : ""
                property bool currentItemIsDirectory: currentItem ? currentItem.isDirectory : false

                Keys.onPressed: function(event) {
                    if (!root.modelRef)
                        return

                    if (root.handleFileShortcut(event, currentItemPath)) {
                        event.accepted = true
                        return
                    }

                    if ((event.key === Qt.Key_Return || event.key === Qt.Key_Enter) && currentItem) {
                        root.activateCurrentSelection(currentItemPath, currentItemIsDirectory)
                        event.accepted = true
                        return
                    }

                    if (event.key === Qt.Key_Delete && currentItem) {
                        root.trashCurrentSelection(currentItemPath)
                        event.accepted = true
                        return
                    }

                    if (event.key === Qt.Key_F2 && currentItem) {
                        renameDialog.open()
                        event.accepted = true
                        return
                    }

                    if (event.key === Qt.Key_Backspace || (event.key === Qt.Key_Up && (event.modifiers & Qt.AltModifier))) {
                        root.modelRef.go_up()
                        event.accepted = true
                        return
                    }

                    if (event.key === Qt.Key_Left && (event.modifiers & Qt.AltModifier) && root.modelRef.can_go_back) {
                        root.modelRef.go_back()
                        event.accepted = true
                        return
                    }

                    if (event.key === Qt.Key_Right && (event.modifiers & Qt.AltModifier) && root.modelRef.can_go_forward) {
                        root.modelRef.go_forward()
                        event.accepted = true
                    }
                }

                MouseArea {
                    anchors.fill: parent
                    z: -1
                    acceptedButtons: Qt.RightButton
                    onClicked: {
                        root.openFolderContextMenu(this, mouse)
                        fileList.forceActiveFocus()
                    }
                }

                delegate: Item {
                    id: detailsDelegate
                    required property int index
                    required property string name
                    required property string path
                    required property string kind
                    required property string sizeText
                    required property string modifiedText
                    required property double modifiedMs
                    required property string thumbnailUrl
                    required property bool isDirectory
                    width: fileList.contentWidth
                    height: 40

                    Item {
                        id: detailsDragAnchor
                        width: 1
                        height: 1
                        opacity: 0
                        Drag.active: detailsFileMouse.drag.active
                        Drag.source: detailsDelegate
                        Drag.supportedActions: Qt.CopyAction | Qt.MoveAction
                        Drag.proposedAction: Qt.MoveAction
                        Drag.mimeData: { "text/uri-list": root.dragMimeText(path) }
                        onXChanged: if (!detailsFileMouse.drag.active) x = 0
                        onYChanged: if (!detailsFileMouse.drag.active) y = 0
                    }

                    Rectangle {
                        anchors.fill: parent
                        radius: 8
                        color: root.isPathSelected(path) || fileList.currentIndex === index
                            ? root.rowSelectedColor
                            : (index % 2 === 0 ? root.rowEvenColor : root.rowOddColor)
                        border.width: root.dropTargetPath === path ? 2 : (root.isPathSelected(path) || fileList.currentIndex === index ? 1 : 0)
                        border.color: root.accentColor
                    }

                    Item {
                        anchors.fill: parent

                        Item {
                            x: root.detailsHorizontalPadding
                            width: root.detailsIconColumnWidth
                            height: 24
                            anchors.verticalCenter: parent.verticalCenter

                            Image {
                                anchors.fill: parent
                                visible: isDirectory
                                source: root.folderIconSource(name, path)
                                fillMode: Image.PreserveAspectFit
                                smooth: true
                                asynchronous: true
                            }

                            Image {
                                anchors.fill: parent
                                visible: !isDirectory && root.filePreviewSource(path, thumbnailUrl).length > 0
                                source: root.filePreviewSource(path, thumbnailUrl)
                                fillMode: Image.PreserveAspectCrop
                                smooth: true
                                asynchronous: true
                                cache: false
                            }

                            Image {
                                anchors.fill: parent
                                visible: !isDirectory
                                    && root.filePreviewSource(path, thumbnailUrl).length === 0
                                    && root.fileIconSource(path).length > 0
                                source: root.fileIconSource(path)
                                fillMode: Image.PreserveAspectFit
                                smooth: true
                                asynchronous: true
                            }

                            Rectangle {
                                anchors.fill: parent
                                visible: !isDirectory
                                    && root.filePreviewSource(path, thumbnailUrl).length === 0
                                    && root.fileIconSource(path).length === 0
                                radius: 6
                                color: root.darkTheme ? "#5f7b85" : "#c0d6d8"
                            }
                        }

                        Label {
                            x: root.detailsHorizontalPadding + detailsColumn.effectiveNameColumnX
                            width: detailsColumn.effectiveNameColumnWidth
                            anchors.top: parent.top
                            anchors.bottom: parent.bottom
                            text: name
                            font.pixelSize: 14
                            color: root.textColor
                            elide: Text.ElideRight
                            maximumLineCount: 1
                            verticalAlignment: Text.AlignVCenter
                        }

                        Label {
                            x: root.detailsHorizontalPadding + detailsColumn.effectiveTypeColumnX
                            width: detailsColumn.effectiveTypeColumnWidth
                            anchors.top: parent.top
                            anchors.bottom: parent.bottom
                            text: kind
                            color: root.mutedTextColor
                            font.pixelSize: 13
                            horizontalAlignment: Text.AlignLeft
                            verticalAlignment: Text.AlignVCenter
                        }

                        Label {
                            x: root.detailsHorizontalPadding + detailsColumn.effectiveSizeColumnX
                            width: detailsColumn.effectiveSizeColumnWidth
                            anchors.top: parent.top
                            anchors.bottom: parent.bottom
                            text: sizeText
                            color: root.mutedTextColor
                            font.pixelSize: 13
                            horizontalAlignment: Text.AlignRight
                            verticalAlignment: Text.AlignVCenter
                        }

                        Label {
                            x: root.detailsHorizontalPadding + detailsColumn.effectiveModifiedColumnX
                            width: detailsColumn.effectiveModifiedColumnWidth
                            anchors.top: parent.top
                            anchors.bottom: parent.bottom
                            text: modifiedMs > 0
                                ? new Date(modifiedMs).toLocaleString(Qt.locale(), Locale.ShortFormat)
                                : modifiedText
                            color: root.mutedTextColor
                            font.pixelSize: 13
                            horizontalAlignment: Text.AlignRight
                            verticalAlignment: Text.AlignVCenter
                            elide: Text.ElideLeft
                        }
                    }

                    MouseArea {
                        id: detailsFileMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        acceptedButtons: Qt.LeftButton | Qt.RightButton
                        drag.target: detailsDragAnchor
                        onClicked: {
                            if (mouse.button === Qt.RightButton) {
                                fileList.currentIndex = index
                                root.openFileContextMenu(path, name, isDirectory, detailsFileMouse, mouse)
                                fileList.forceActiveFocus()
                                return
                            }
                            fileList.currentIndex = index
                            root.selectEntry(path, name, mouse.modifiers & Qt.ControlModifier)
                            fileList.forceActiveFocus()
                        }
                        onPressed: {
                            if (mouse.button === Qt.LeftButton && !root.isReadOnlyPath(path))
                                root.activeDragPath = path
                        }
                        onReleased: {
                            detailsDragAnchor.x = 0
                            detailsDragAnchor.y = 0
                            root.activeDragPath = ""
                        }
                        onCanceled: {
                            detailsDragAnchor.x = 0
                            detailsDragAnchor.y = 0
                            root.activeDragPath = ""
                        }
                        onDoubleClicked: root.activateCurrentSelection(path, isDirectory)
                    }

                    Rectangle {
                        width: 18
                        height: 18
                        radius: 9
                        x: 4
                        anchors.verticalCenter: parent.verticalCenter
                        z: 5
                        visible: root.selectionMarkerVisible(path, detailsFileMouse.containsMouse)
                        color: root.selectionMarkerColor(path)
                        border.width: 1
                        border.color: root.accentColor

                        Text {
                            anchors.centerIn: parent
                            text: root.selectionMarkerText(path)
                            color: root.selectionMarkerTextColor(path)
                            font.pixelSize: 13
                            font.bold: true
                        }

                        MouseArea {
                            anchors.fill: parent
                            acceptedButtons: Qt.LeftButton
                            onClicked: {
                                root.toggleEntrySelection(path, name)
                                fileList.currentIndex = index
                                fileList.forceActiveFocus()
                            }
                        }
                    }

                    DropArea {
                        anchors.fill: parent
                        enabled: isDirectory && !root.isReadOnlyPath(path)
                        onEntered: function(drag) { root.updateDropTarget(drag, path) }
                        onPositionChanged: function(drag) { root.updateDropTarget(drag, path) }
                        onExited: root.clearDropTarget(path)
                        onDropped: function(drop) { root.handleDrop(drop, path) }
                    }
                }
            }
        }
    }

    Component {
        id: listViewComponent
        Loader {
            Layout.fillWidth: true
            Layout.fillHeight: true
            sourceComponent: root.modelRef && root.modelRef.grouping_name !== "None"
                ? groupedListViewComponent
                : flatListViewComponent
        }
    }

    Component {
        id: iconsViewComponent
        Loader {
            Layout.fillWidth: true
            Layout.fillHeight: true
            sourceComponent: root.modelRef && root.modelRef.grouping_name !== "None"
                ? groupedIconsViewComponent
                : flatIconsViewComponent
        }
    }

    Component {
        id: flatListViewComponent

        GridView {
            id: compactList
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            model: root.modelRef
            cellWidth: 240
            cellHeight: 32
            flow: GridView.FlowTopToBottom
            layoutDirection: Qt.LeftToRight
            flickableDirection: Flickable.HorizontalFlick
            boundsBehavior: Flickable.StopAtBounds
            currentIndex: -1
            focus: true
            ScrollBar.horizontal: ScrollBar {
                policy: ScrollBar.AlwaysOn
                height: 14
                visible: true
                opacity: 1.0
                active: true
                interactive: true
                background: Rectangle {
                    implicitWidth: 14
                    implicitHeight: 14
                    color: root.scrollbarTrackColor
                    radius: 4
                }
                contentItem: Rectangle {
                    height: 12
                    implicitWidth: 36
                    implicitHeight: 12
                    radius: 6
                    color: root.scrollbarThumbColor
                    border.width: 1
                    border.color: root.scrollbarThumbBorderColor
                }
            }

            property string currentItemPath: currentItem ? currentItem.path : ""
            property bool currentItemIsDirectory: currentItem ? currentItem.isDirectory : false

            Keys.onPressed: function(event) {
                if (!root.modelRef)
                    return
                if (root.handleFileShortcut(event, currentItemPath)) {
                    event.accepted = true
                    return
                }
                if ((event.key === Qt.Key_Return || event.key === Qt.Key_Enter) && currentItem) {
                    root.activateCurrentSelection(currentItemPath, currentItemIsDirectory)
                    event.accepted = true
                    return
                }
                if (event.key === Qt.Key_Delete && currentItem) {
                    root.trashCurrentSelection(currentItemPath)
                    event.accepted = true
                }
            }

            MouseArea {
                anchors.fill: parent
                z: -1
                acceptedButtons: Qt.RightButton
                onClicked: {
                    root.openFolderContextMenu(this, mouse)
                    compactList.forceActiveFocus()
                }
            }

            MouseArea {
                anchors.fill: parent
                acceptedButtons: Qt.NoButton
                propagateComposedEvents: true
                onWheel: function(wheel) {
                    let delta = 0
                    if (wheel.pixelDelta.x !== 0)
                        delta = wheel.pixelDelta.x
                    else if (wheel.pixelDelta.y !== 0)
                        delta = -wheel.pixelDelta.y
                    else if (wheel.angleDelta.x !== 0)
                        delta = wheel.angleDelta.x
                    else
                        delta = -wheel.angleDelta.y

                    if (delta === 0)
                        return

                    const maxContentX = Math.max(0, compactList.contentWidth - compactList.width)
                    compactList.contentX = Math.max(0, Math.min(maxContentX, compactList.contentX + delta))
                    wheel.accepted = true
                }
            }

            delegate: Item {
                id: compactDelegate
                required property int index
                required property string name
                required property string path
                required property string thumbnailUrl
                required property bool isDirectory
                width: compactList.cellWidth
                height: compactList.cellHeight

                Item {
                    id: compactDragAnchor
                    width: 1
                    height: 1
                    opacity: 0
                    Drag.active: compactFileMouse.drag.active
                    Drag.source: compactDelegate
                    Drag.supportedActions: Qt.CopyAction | Qt.MoveAction
                    Drag.proposedAction: Qt.MoveAction
                    Drag.mimeData: { "text/uri-list": root.dragMimeText(path) }
                }

                Rectangle {
                    anchors.fill: parent
                    radius: 4
                    color: root.isPathSelected(path) || compactList.currentIndex === index ? root.rowSelectedColor : "transparent"
                    border.width: root.dropTargetPath === path ? 2 : (root.isPathSelected(path) || compactList.currentIndex === index ? 1 : 0)
                    border.color: root.accentColor
                }

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 8
                    anchors.rightMargin: 8
                    spacing: 8

                    Item {
                        Layout.preferredWidth: 16
                        Layout.minimumWidth: 16
                        width: 16
                        height: 16
                        Layout.alignment: Qt.AlignVCenter

                        Image {
                            anchors.fill: parent
                            visible: isDirectory
                            source: root.folderIconSource(name, path)
                            fillMode: Image.PreserveAspectFit
                            smooth: true
                            asynchronous: true
                        }

                        Image {
                            anchors.fill: parent
                            visible: !isDirectory && root.filePreviewSource(path, thumbnailUrl).length > 0
                            source: root.filePreviewSource(path, thumbnailUrl)
                            fillMode: Image.PreserveAspectCrop
                            smooth: true
                            asynchronous: true
                            cache: false
                        }

                        Image {
                            anchors.fill: parent
                            visible: !isDirectory
                                && root.filePreviewSource(path, thumbnailUrl).length === 0
                                && root.fileIconSource(path).length > 0
                            source: root.fileIconSource(path)
                            fillMode: Image.PreserveAspectFit
                            smooth: true
                            asynchronous: true
                        }

                        Rectangle {
                            anchors.fill: parent
                            visible: !isDirectory
                                && root.filePreviewSource(path, thumbnailUrl).length === 0
                                && root.fileIconSource(path).length === 0
                            radius: 4
                            color: root.darkTheme ? "#5f7b85" : "#c0d6d8"
                        }
                    }

                    Text {
                        text: name
                        color: root.textColor
                        font.pixelSize: 14
                        elide: Text.ElideRight
                        Layout.fillWidth: true
                        Layout.alignment: Qt.AlignVCenter
                    }
                }

                MouseArea {
                    id: compactFileMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    acceptedButtons: Qt.LeftButton | Qt.RightButton
                    drag.target: compactDragAnchor
                    onClicked: {
                        if (mouse.button === Qt.RightButton) {
                            compactList.currentIndex = index
                            root.openFileContextMenu(path, name, isDirectory, compactFileMouse, mouse)
                            compactList.forceActiveFocus()
                            return
                        }
                        compactList.currentIndex = index
                        root.selectEntry(path, name, mouse.modifiers & Qt.ControlModifier)
                        compactList.forceActiveFocus()
                    }
                    onPressed: {
                        if (mouse.button === Qt.LeftButton && !root.isReadOnlyPath(path))
                            root.activeDragPath = path
                    }
                    onReleased: {
                        compactDragAnchor.x = 0
                        compactDragAnchor.y = 0
                        root.activeDragPath = ""
                    }
                    onCanceled: {
                        compactDragAnchor.x = 0
                        compactDragAnchor.y = 0
                        root.activeDragPath = ""
                    }
                    onDoubleClicked: root.activateCurrentSelection(path, isDirectory)
                }

                Rectangle {
                    width: 18
                    height: 18
                    radius: 9
                    anchors.left: parent.left
                    anchors.leftMargin: 4
                    anchors.verticalCenter: parent.verticalCenter
                    z: 5
                    visible: root.selectionMarkerVisible(path, compactFileMouse.containsMouse)
                    color: root.selectionMarkerColor(path)
                    border.width: 1
                    border.color: root.accentColor

                    Text {
                        anchors.centerIn: parent
                        text: root.selectionMarkerText(path)
                        color: root.selectionMarkerTextColor(path)
                        font.pixelSize: 13
                        font.bold: true
                    }

                    MouseArea {
                        anchors.fill: parent
                        acceptedButtons: Qt.LeftButton
                        onClicked: {
                            root.toggleEntrySelection(path, name)
                            compactList.currentIndex = index
                            compactList.forceActiveFocus()
                        }
                    }
                }

                DropArea {
                    anchors.fill: parent
                    enabled: isDirectory && !root.isReadOnlyPath(path)
                    onEntered: function(drag) { root.updateDropTarget(drag, path) }
                    onPositionChanged: function(drag) { root.updateDropTarget(drag, path) }
                    onExited: root.clearDropTarget(path)
                    onDropped: function(drop) { root.handleDrop(drop, path) }
                }
            }
        }
    }

    Component {
        id: groupedListViewComponent

        ScrollView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            ScrollBar.horizontal: ScrollBar {
                policy: ScrollBar.AlwaysOn
                height: 14
                visible: true
                opacity: 1.0
                active: true
                interactive: true
                background: Rectangle {
                    implicitWidth: 14
                    implicitHeight: 14
                    color: root.scrollbarTrackColor
                    radius: 4
                }
                contentItem: Rectangle {
                    height: 12
                    implicitWidth: 36
                    implicitHeight: 12
                    radius: 6
                    color: root.scrollbarThumbColor
                    border.width: 1
                    border.color: root.scrollbarThumbBorderColor
                }
            }

            Row {
                id: groupedListRow
                spacing: 0

                Repeater {
                    model: root.groupedEntries()

                    delegate: Item {
                        required property var modelData
                        width: 170
                        height: groupedListColumn.implicitHeight

                        Column {
                            id: groupedListColumn
                            width: parent.width
                            spacing: 0

                            Item {
                                width: parent.width
                                height: 28

                                Text {
                                    anchors.left: parent.left
                                    anchors.leftMargin: 6
                                    anchors.verticalCenter: parent.verticalCenter
                                    text: modelData.label
                                    color: root.mutedTextColor
                                    font.pixelSize: 13
                                }
                            }

                            Rectangle {
                                width: parent.width
                                height: 1
                                color: root.borderColor
                            }

                            Repeater {
                                model: modelData.items

                                delegate: Item {
                                    id: groupedListDelegate
                                    required property var modelData
                                    property string path: modelData.path
                                    width: groupedListColumn.width
                                    height: 22

                                    Item {
                                        id: groupedListDragAnchor
                                        width: 1
                                        height: 1
                                        opacity: 0
                                        Drag.active: groupedListFileMouse.drag.active
                                        Drag.source: groupedListDelegate
                                        Drag.supportedActions: Qt.CopyAction | Qt.MoveAction
                                        Drag.proposedAction: Qt.MoveAction
                                        Drag.mimeData: { "text/uri-list": root.dragMimeText(modelData.path) }
                                    }

                                    Row {
                                        anchors.fill: parent
                                        anchors.leftMargin: 8
                                        anchors.rightMargin: 8
                                        spacing: 6

                                        Item {
                                            width: 12
                                            height: 12
                                            anchors.verticalCenter: parent.verticalCenter

                                            Image {
                                                anchors.fill: parent
                                                visible: modelData.is_directory
                                                source: root.folderIconSource(modelData.name, modelData.path)
                                                fillMode: Image.PreserveAspectFit
                                                smooth: true
                                                asynchronous: true
                                            }

                                            Image {
                                                anchors.fill: parent
                                                visible: !modelData.is_directory && root.filePreviewSource(modelData.path, modelData.thumbnail_url).length > 0
                                                source: root.filePreviewSource(modelData.path, modelData.thumbnail_url)
                                                fillMode: Image.PreserveAspectCrop
                                                smooth: true
                                                asynchronous: true
                                                cache: false
                                            }

                                            Image {
                                                anchors.fill: parent
                                                visible: !modelData.is_directory
                                                    && root.filePreviewSource(modelData.path, modelData.thumbnail_url).length === 0
                                                    && root.fileIconSource(modelData.path).length > 0
                                                source: root.fileIconSource(modelData.path)
                                                fillMode: Image.PreserveAspectFit
                                                smooth: true
                                                asynchronous: true
                                            }

                                            Rectangle {
                                                anchors.fill: parent
                                                visible: !modelData.is_directory
                                                    && root.filePreviewSource(modelData.path, modelData.thumbnail_url).length === 0
                                                    && root.fileIconSource(modelData.path).length === 0
                                                radius: 3
                                                color: root.darkTheme ? "#5f7b85" : "#c0d6d8"
                                            }
                                        }

                                        Text {
                                            width: parent.width - 26
                                            anchors.verticalCenter: parent.verticalCenter
                                            text: modelData.name
                                            color: root.selectedPath === modelData.path ? root.accentColor : root.textColor
                                            font.pixelSize: 13
                                            elide: Text.ElideRight
                                        }
                                    }

                                    Rectangle {
                                        anchors.fill: parent
                                        visible: root.isPathSelected(modelData.path)
                                        z: -1
                                        color: root.rowSelectedColor
                                        opacity: 0.55
                                        radius: 4
                                    }

                                    MouseArea {
                                        id: groupedListFileMouse
                                        anchors.fill: parent
                                        hoverEnabled: true
                                        acceptedButtons: Qt.LeftButton | Qt.RightButton
                                        drag.target: groupedListDragAnchor
                                        onClicked: {
                                            if (mouse.button === Qt.RightButton) {
                                                root.openFileContextMenu(modelData.path, modelData.name, modelData.is_directory, groupedListFileMouse, mouse)
                                                return
                                            }
                                            root.selectEntry(modelData.path, modelData.name, mouse.modifiers & Qt.ControlModifier)
                                        }
                                        onPressed: {
                                            if (mouse.button === Qt.LeftButton && !root.isReadOnlyPath(modelData.path))
                                                root.activeDragPath = modelData.path
                                        }
                                        onReleased: {
                                            groupedListDragAnchor.x = 0
                                            groupedListDragAnchor.y = 0
                                            root.activeDragPath = ""
                                        }
                                        onCanceled: {
                                            groupedListDragAnchor.x = 0
                                            groupedListDragAnchor.y = 0
                                            root.activeDragPath = ""
                                        }
                                        onDoubleClicked: root.activateCurrentSelection(modelData.path, modelData.is_directory)
                                    }

                                    Rectangle {
                                        width: 16
                                        height: 16
                                        radius: 8
                                        anchors.left: parent.left
                                        anchors.leftMargin: 2
                                        anchors.verticalCenter: parent.verticalCenter
                                        z: 5
                                        visible: root.selectionMarkerVisible(modelData.path, groupedListFileMouse.containsMouse)
                                        color: root.selectionMarkerColor(modelData.path)
                                        border.width: 1
                                        border.color: root.accentColor

                                        Text {
                                            anchors.centerIn: parent
                                            text: root.selectionMarkerText(modelData.path)
                                            color: root.selectionMarkerTextColor(modelData.path)
                                            font.pixelSize: 12
                                            font.bold: true
                                        }

                                        MouseArea {
                                            anchors.fill: parent
                                            acceptedButtons: Qt.LeftButton
                                            onClicked: root.toggleEntrySelection(modelData.path, modelData.name)
                                        }
                                    }

                                    Rectangle {
                                        anchors.fill: parent
                                        visible: root.dropTargetPath === modelData.path
                                        color: "transparent"
                                        border.width: 2
                                        border.color: root.accentColor
                                        radius: 4
                                    }

                                    DropArea {
                                        anchors.fill: parent
                                        enabled: modelData.is_directory && !root.isReadOnlyPath(modelData.path)
                                        onEntered: function(drag) { root.updateDropTarget(drag, modelData.path) }
                                        onPositionChanged: function(drag) { root.updateDropTarget(drag, modelData.path) }
                                        onExited: root.clearDropTarget(modelData.path)
                                        onDropped: function(drop) { root.handleDrop(drop, modelData.path) }
                                    }
                                }
                            }
                        }

                        Rectangle {
                            anchors.top: parent.top
                            anchors.bottom: parent.bottom
                            anchors.right: parent.right
                            width: 1
                            color: root.borderColor
                        }
                    }
                }
            }
        }
    }

    Component {
        id: flatIconsViewComponent

        GridView {
            id: iconGrid
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            model: root.modelRef
            cellWidth: 110
            cellHeight: 110
            currentIndex: -1
            focus: true

            property string currentItemPath: currentItem ? currentItem.path : ""
            property bool currentItemIsDirectory: currentItem ? currentItem.isDirectory : false

            Keys.onPressed: function(event) {
                if (!root.modelRef)
                    return
                if (root.handleFileShortcut(event, currentItemPath)) {
                    event.accepted = true
                    return
                }
                if ((event.key === Qt.Key_Return || event.key === Qt.Key_Enter) && currentItem) {
                    root.activateCurrentSelection(currentItemPath, currentItemIsDirectory)
                    event.accepted = true
                    return
                }
                if (event.key === Qt.Key_Delete && currentItem) {
                    root.trashCurrentSelection(currentItemPath)
                    event.accepted = true
                }
            }

            MouseArea {
                anchors.fill: parent
                z: -1
                acceptedButtons: Qt.RightButton
                onClicked: {
                    root.openFolderContextMenu(this, mouse)
                    iconGrid.forceActiveFocus()
                }
            }

            delegate: Item {
                id: iconDelegate
                required property int index
                required property string name
                required property string path
                required property string thumbnailUrl
                required property bool isDirectory
                width: iconGrid.cellWidth
                height: iconGrid.cellHeight

                Item {
                    id: iconDragAnchor
                    width: 1
                    height: 1
                    opacity: 0
                    Drag.active: iconFileMouse.drag.active
                    Drag.source: iconDelegate
                    Drag.supportedActions: Qt.CopyAction | Qt.MoveAction
                    Drag.proposedAction: Qt.MoveAction
                    Drag.mimeData: { "text/uri-list": root.dragMimeText(path) }
                }

                Rectangle {
                    anchors.fill: parent
                    radius: 10
                    color: root.isPathSelected(path) || iconGrid.currentIndex === index ? root.rowSelectedColor : "transparent"
                    border.width: root.dropTargetPath === path ? 2 : (root.isPathSelected(path) || iconGrid.currentIndex === index ? 1 : 0)
                    border.color: root.accentColor
                }

                Column {
                    anchors.top: parent.top
                    anchors.topMargin: 8
                    anchors.horizontalCenter: parent.horizontalCenter
                    spacing: 8
                    width: parent.width - 12

                    Item {
                        width: 42
                        height: 42
                        anchors.horizontalCenter: parent.horizontalCenter

                        Image {
                            anchors.fill: parent
                            visible: isDirectory
                            source: root.folderIconSource(name, path)
                            fillMode: Image.PreserveAspectFit
                            smooth: true
                            asynchronous: true
                        }

                        Image {
                            anchors.fill: parent
                            visible: !isDirectory && root.filePreviewSource(path, thumbnailUrl).length > 0
                            source: root.filePreviewSource(path, thumbnailUrl)
                            fillMode: Image.PreserveAspectCrop
                            smooth: true
                            asynchronous: true
                            cache: false
                        }

                        Image {
                            anchors.fill: parent
                            visible: !isDirectory
                                && root.filePreviewSource(path, thumbnailUrl).length === 0
                                && root.fileIconSource(path).length > 0
                            source: root.fileIconSource(path)
                            fillMode: Image.PreserveAspectFit
                            smooth: true
                            asynchronous: true
                        }

                        Rectangle {
                            anchors.fill: parent
                            visible: !isDirectory
                                && root.filePreviewSource(path, thumbnailUrl).length === 0
                                && root.fileIconSource(path).length === 0
                            radius: 10
                            color: root.darkTheme ? "#5f7b85" : "#c0d6d8"
                        }
                    }

                    Text {
                        text: name
                        color: root.textColor
                        font.pixelSize: 13
                        horizontalAlignment: Text.AlignHCenter
                        wrapMode: Text.Wrap
                        maximumLineCount: 2
                        elide: Text.ElideRight
                        width: parent.width
                    }
                }

                MouseArea {
                    id: iconFileMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    acceptedButtons: Qt.LeftButton | Qt.RightButton
                    drag.target: iconDragAnchor
                    onClicked: {
                        if (mouse.button === Qt.RightButton) {
                            iconGrid.currentIndex = index
                            root.openFileContextMenu(path, name, isDirectory, iconFileMouse, mouse)
                            iconGrid.forceActiveFocus()
                            return
                        }
                        iconGrid.currentIndex = index
                        root.selectEntry(path, name, mouse.modifiers & Qt.ControlModifier)
                        iconGrid.forceActiveFocus()
                    }
                    onPressed: {
                        if (mouse.button === Qt.LeftButton && !root.isReadOnlyPath(path))
                            root.activeDragPath = path
                    }
                    onReleased: {
                        iconDragAnchor.x = 0
                        iconDragAnchor.y = 0
                        root.activeDragPath = ""
                    }
                    onCanceled: {
                        iconDragAnchor.x = 0
                        iconDragAnchor.y = 0
                        root.activeDragPath = ""
                    }
                    onDoubleClicked: root.activateCurrentSelection(path, isDirectory)
                }

                Rectangle {
                    width: 20
                    height: 20
                    radius: 10
                    anchors.left: parent.left
                    anchors.leftMargin: 6
                    anchors.top: parent.top
                    anchors.topMargin: 6
                    z: 5
                    visible: root.selectionMarkerVisible(path, iconFileMouse.containsMouse)
                    color: root.selectionMarkerColor(path)
                    border.width: 1
                    border.color: root.accentColor

                    Text {
                        anchors.centerIn: parent
                        text: root.selectionMarkerText(path)
                        color: root.selectionMarkerTextColor(path)
                        font.pixelSize: 14
                        font.bold: true
                    }

                    MouseArea {
                        anchors.fill: parent
                        acceptedButtons: Qt.LeftButton
                        onClicked: {
                            root.toggleEntrySelection(path, name)
                            iconGrid.currentIndex = index
                            iconGrid.forceActiveFocus()
                        }
                    }
                }

                DropArea {
                    anchors.fill: parent
                    enabled: isDirectory && !root.isReadOnlyPath(path)
                    onEntered: function(drag) { root.updateDropTarget(drag, path) }
                    onPositionChanged: function(drag) { root.updateDropTarget(drag, path) }
                    onExited: root.clearDropTarget(path)
                    onDropped: function(drop) { root.handleDrop(drop, path) }
                }
            }
        }
    }

    Component {
        id: groupedIconsViewComponent

        ScrollView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            ScrollBar.vertical: ScrollBar {
                policy: ScrollBar.AlwaysOn
                width: 14
                visible: true
                opacity: 1.0
                active: true
                interactive: true
                background: Rectangle {
                    implicitWidth: 14
                    implicitHeight: 14
                    color: root.scrollbarTrackColor
                    radius: 4
                }
                contentItem: Rectangle {
                    width: 12
                    implicitWidth: 12
                    implicitHeight: 36
                    radius: 6
                    color: root.scrollbarThumbColor
                    border.width: 1
                    border.color: root.scrollbarThumbBorderColor
                }
            }

            Column {
                width: parent.width
                spacing: 18

                Repeater {
                    model: root.groupedEntries()

                    delegate: Column {
                        required property var modelData
                        width: parent.width
                        spacing: 10

                        Item {
                            width: parent.width
                            height: 26

                            Text {
                                anchors.left: parent.left
                                anchors.leftMargin: 4
                                anchors.verticalCenter: parent.verticalCenter
                                text: modelData.label
                                color: root.mutedTextColor
                                font.pixelSize: 13
                            }

                            Rectangle {
                                anchors.left: parent.left
                                anchors.right: parent.right
                                anchors.bottom: parent.bottom
                                height: 1
                                color: root.borderColor
                            }
                        }

                        Flow {
                            width: parent.width
                            spacing: 18

                            Repeater {
                                model: modelData.items

                                delegate: Item {
                                    id: groupedIconDelegate
                                    required property var modelData
                                    property string path: modelData.path
                                    width: 112
                                    height: 108

                                    Item {
                                        id: groupedIconDragAnchor
                                        width: 1
                                        height: 1
                                        opacity: 0
                                        Drag.active: groupedIconFileMouse.drag.active
                                        Drag.source: groupedIconDelegate
                                        Drag.supportedActions: Qt.CopyAction | Qt.MoveAction
                                        Drag.proposedAction: Qt.MoveAction
                                        Drag.mimeData: { "text/uri-list": root.dragMimeText(modelData.path) }
                                    }

                                    Rectangle {
                                        anchors.fill: parent
                                        radius: 10
                                        color: root.isPathSelected(modelData.path) ? root.rowSelectedColor : "transparent"
                                        border.width: root.dropTargetPath === modelData.path ? 2 : (root.isPathSelected(modelData.path) ? 1 : 0)
                                        border.color: root.accentColor
                                    }

                                    Column {
                                        anchors.top: parent.top
                                        anchors.topMargin: 8
                                        anchors.horizontalCenter: parent.horizontalCenter
                                        spacing: 8
                                        width: parent.width - 10

                                        Item {
                                            width: 56
                                            height: 56
                                            anchors.horizontalCenter: parent.horizontalCenter

                                            Image {
                                                anchors.fill: parent
                                                visible: modelData.is_directory
                                                source: root.folderIconSource(modelData.name, modelData.path)
                                                fillMode: Image.PreserveAspectFit
                                                smooth: true
                                                asynchronous: true
                                            }

                                            Image {
                                                anchors.fill: parent
                                                visible: !modelData.is_directory && root.filePreviewSource(modelData.path, modelData.thumbnail_url).length > 0
                                                source: root.filePreviewSource(modelData.path, modelData.thumbnail_url)
                                                fillMode: Image.PreserveAspectCrop
                                                smooth: true
                                                asynchronous: true
                                                cache: false
                                            }

                                            Image {
                                                anchors.fill: parent
                                                visible: !modelData.is_directory
                                                    && root.filePreviewSource(modelData.path, modelData.thumbnail_url).length === 0
                                                    && root.fileIconSource(modelData.path).length > 0
                                                source: root.fileIconSource(modelData.path)
                                                fillMode: Image.PreserveAspectFit
                                                smooth: true
                                                asynchronous: true
                                            }

                                            Rectangle {
                                                anchors.fill: parent
                                                visible: !modelData.is_directory
                                                    && root.filePreviewSource(modelData.path, modelData.thumbnail_url).length === 0
                                                    && root.fileIconSource(modelData.path).length === 0
                                                radius: 10
                                                color: root.darkTheme ? "#5f7b85" : "#c0d6d8"
                                            }
                                        }

                                        Text {
                                            width: parent.width
                                            text: modelData.name
                                            color: root.textColor
                                            font.pixelSize: 12
                                            horizontalAlignment: Text.AlignHCenter
                                            wrapMode: Text.Wrap
                                            maximumLineCount: 3
                                            elide: Text.ElideRight
                                        }
                                    }

                                    MouseArea {
                                        id: groupedIconFileMouse
                                        anchors.fill: parent
                                        hoverEnabled: true
                                        acceptedButtons: Qt.LeftButton | Qt.RightButton
                                        drag.target: groupedIconDragAnchor
                                        onClicked: {
                                            if (mouse.button === Qt.RightButton) {
                                                root.openFileContextMenu(modelData.path, modelData.name, modelData.is_directory, groupedIconFileMouse, mouse)
                                                return
                                            }
                                            root.selectEntry(modelData.path, modelData.name, mouse.modifiers & Qt.ControlModifier)
                                        }
                                        onPressed: {
                                            if (mouse.button === Qt.LeftButton && !root.isReadOnlyPath(modelData.path))
                                                root.activeDragPath = modelData.path
                                        }
                                        onReleased: {
                                            groupedIconDragAnchor.x = 0
                                            groupedIconDragAnchor.y = 0
                                            root.activeDragPath = ""
                                        }
                                        onCanceled: {
                                            groupedIconDragAnchor.x = 0
                                            groupedIconDragAnchor.y = 0
                                            root.activeDragPath = ""
                                        }
                                        onDoubleClicked: root.activateCurrentSelection(modelData.path, modelData.is_directory)
                                    }

                                    Rectangle {
                                        width: 20
                                        height: 20
                                        radius: 10
                                        anchors.left: parent.left
                                        anchors.leftMargin: 6
                                        anchors.top: parent.top
                                        anchors.topMargin: 6
                                        z: 5
                                        visible: root.selectionMarkerVisible(modelData.path, groupedIconFileMouse.containsMouse)
                                        color: root.selectionMarkerColor(modelData.path)
                                        border.width: 1
                                        border.color: root.accentColor

                                        Text {
                                            anchors.centerIn: parent
                                            text: root.selectionMarkerText(modelData.path)
                                            color: root.selectionMarkerTextColor(modelData.path)
                                            font.pixelSize: 14
                                            font.bold: true
                                        }

                                        MouseArea {
                                            anchors.fill: parent
                                            acceptedButtons: Qt.LeftButton
                                            onClicked: root.toggleEntrySelection(modelData.path, modelData.name)
                                        }
                                    }

                                    DropArea {
                                        anchors.fill: parent
                                        enabled: modelData.is_directory && !root.isReadOnlyPath(modelData.path)
                                        onEntered: function(drag) { root.updateDropTarget(drag, modelData.path) }
                                        onPositionChanged: function(drag) { root.updateDropTarget(drag, modelData.path) }
                                        onExited: root.clearDropTarget(modelData.path)
                                        onDropped: function(drop) { root.handleDrop(drop, modelData.path) }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    header: ToolBar {
        contentHeight: 40

        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 10
            anchors.rightMargin: 10
            spacing: 8

            ToolButton {
                text: "Back"
                enabled: root.modelRef ? root.modelRef.can_go_back : false
                display: AbstractButton.IconOnly
                icon.source: root.toolbarIconSource("icon_nav_back.svg")
                icon.width: root.toolbarIconSize
                icon.height: root.toolbarIconSize
                Layout.preferredWidth: root.toolbarButtonSize
                Layout.preferredHeight: root.toolbarButtonSize
                ToolTip.visible: hovered
                ToolTip.text: text
                onClicked: if (root.modelRef) root.modelRef.go_back()
            }

            ToolButton {
                text: "Forward"
                enabled: root.modelRef ? root.modelRef.can_go_forward : false
                display: AbstractButton.IconOnly
                icon.source: root.toolbarIconSource("icon_nav_forward.svg")
                icon.width: root.toolbarIconSize
                icon.height: root.toolbarIconSize
                Layout.preferredWidth: root.toolbarButtonSize
                Layout.preferredHeight: root.toolbarButtonSize
                ToolTip.visible: hovered
                ToolTip.text: text
                onClicked: if (root.modelRef) root.modelRef.go_forward()
            }

            ToolButton {
                text: "Up"
                enabled: root.modelRef !== null
                display: AbstractButton.IconOnly
                icon.source: root.toolbarIconSource("icon_nav_up.svg")
                icon.width: root.toolbarIconSize
                icon.height: root.toolbarIconSize
                Layout.preferredWidth: root.toolbarButtonSize
                Layout.preferredHeight: root.toolbarButtonSize
                ToolTip.visible: hovered
                ToolTip.text: text
                onClicked: if (root.modelRef) root.modelRef.go_up()
            }

            Frame {
                Layout.fillWidth: true
                Layout.preferredHeight: 30
                Layout.minimumHeight: 30
                Layout.maximumHeight: 30
                padding: 5
                background: Rectangle {
                    radius: 6
                    color: root.inputColor
                    border.color: root.borderColor
                }

                Item {
                    anchors.fill: parent
                    clip: true

                    Row {
                        id: breadcrumbContent
                        anchors.left: parent.left
                        anchors.leftMargin: 6
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: 6

                        Repeater {
                            model: root.modelRef ? root.breadcrumbParts(root.modelRef.current_path) : []

                            delegate: Row {
                                required property var modelData
                                required property int index
                                spacing: 6

                                Rectangle {
                                    width: modelData.label === "/" ? 30 : crumbLabel.implicitWidth + 18
                                    height: 26
                                    radius: 6
                                    color: index === (root.modelRef ? root.breadcrumbParts(root.modelRef.current_path).length - 1 : -1)
                                        ? root.rowSelectedColor
                                        : root.inputColor
                                    border.width: 1
                                    border.color: index === (root.modelRef ? root.breadcrumbParts(root.modelRef.current_path).length - 1 : -1)
                                        ? root.accentColor
                                        : root.borderColor

                                    Image {
                                        anchors.centerIn: parent
                                        width: 16
                                        height: 16
                                        visible: modelData.label === "/"
                                        source: root.toolbarIconSource("icon_nav_home.svg")
                                        fillMode: Image.PreserveAspectFit
                                        smooth: true
                                        asynchronous: true
                                    }

                                    Text {
                                        id: crumbLabel
                                        anchors.centerIn: parent
                                        visible: modelData.label !== "/"
                                        text: modelData.label
                                        color: root.textColor
                                        font.pixelSize: 13
                                        font.bold: index === (root.modelRef ? root.breadcrumbParts(root.modelRef.current_path).length - 1 : -1)
                                    }

                                    MouseArea {
                                        anchors.fill: parent
                                        onClicked: if (root.modelRef) root.modelRef.load_path(modelData.path)
                                    }
                                }

                                Loader {
                                    id: childFoldersLoader
                                    active: !root.isVirtualPath(modelData.path)
                                    sourceComponent: Foldering.FolderListModel {
                                        folder: "file://" + modelData.path
                                        showDirs: true
                                        showFiles: false
                                        showDotAndDotDot: false
                                        sortField: Foldering.FolderListModel.Name
                                    }
                                }

                                ToolButton {
                                    id: crumbArrow
                                    anchors.verticalCenter: parent.verticalCenter
                                    icon.source: root.toolbarIconSource("icon_down.svg")
                                    icon.width: 14
                                    icon.height: 14
                                    display: AbstractButton.IconOnly
                                    visible: !root.isVirtualPath(modelData.path)
                                    width: 18
                                    height: 24
                                    leftPadding: 2
                                    rightPadding: 2
                                    topPadding: 0
                                    bottomPadding: 0
                                    onClicked: crumbMenu.open()
                                }

                                Menu {
                                    id: crumbMenu

                                    Instantiator {
                                        model: childFoldersLoader.item

                                        delegate: MenuItem {
                                            required property string fileName
                                            required property url fileURL
                                            text: fileName
                                            onTriggered: if (root.modelRef) root.modelRef.load_path(root.fileUrlToPath(fileURL))
                                        }

                                        onObjectAdded: function(index, object) {
                                            crumbMenu.insertItem(index, object)
                                        }

                                        onObjectRemoved: function(index, object) {
                                            crumbMenu.removeItem(object)
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            ToolButton {
                text: "Home"
                enabled: root.modelRef !== null
                display: AbstractButton.IconOnly
                icon.source: root.toolbarIconSource("icon_home.svg")
                icon.width: root.toolbarIconSize
                icon.height: root.toolbarIconSize
                Layout.preferredWidth: root.toolbarButtonSize
                Layout.preferredHeight: root.toolbarButtonSize
                ToolTip.visible: hovered
                ToolTip.text: text
                onClicked: if (root.modelRef) root.modelRef.load_path(Platform.StandardPaths.writableLocation(Platform.StandardPaths.HomeLocation))
            }

            ToolButton {
                text: "Refresh"
                enabled: root.modelRef !== null
                display: AbstractButton.IconOnly
                icon.source: root.toolbarIconSource("icon_refresh.svg")
                icon.width: root.toolbarIconSize
                icon.height: root.toolbarIconSize
                Layout.preferredWidth: root.toolbarButtonSize
                Layout.preferredHeight: root.toolbarButtonSize
                ToolTip.visible: hovered
                ToolTip.text: text
                onClicked: {
                    if (root.modelRef)
                        root.modelRef.refresh()
                    if (root.devicesRef)
                        root.devicesRef.refresh()
                }
            }

            Button {
                id: themeMenuButton
                text: root.themeMode
                Layout.preferredWidth: 112
                Layout.preferredHeight: root.toolbarButtonSize
                contentItem: Item {
                    Label {
                        anchors.left: parent.left
                        anchors.right: themeArrow.left
                        anchors.top: parent.top
                        anchors.bottom: parent.bottom
                        anchors.leftMargin: 10
                        anchors.rightMargin: 6
                        text: themeMenuButton.text
                        color: root.textColor
                        verticalAlignment: Text.AlignVCenter
                        elide: Text.ElideRight
                    }

                    Image {
                        id: themeArrow
                        anchors.right: parent.right
                        anchors.rightMargin: 8
                        anchors.verticalCenter: parent.verticalCenter
                        width: 14
                        height: 14
                        source: root.toolbarIconSource("icon_down.svg")
                        fillMode: Image.PreserveAspectFit
                        smooth: true
                    }
                }
                background: Rectangle {
                    radius: 4
                    color: root.buttonColor
                    border.color: root.borderColor
                }
                onClicked: themeMenu.open()

                Menu {
                    id: themeMenu
                    y: themeMenuButton.height
                    width: themeMenuButton.width
                    MenuItem { text: "System"; onTriggered: root.themeMode = text }
                    MenuItem { text: "Light"; onTriggered: root.themeMode = text }
                    MenuItem { text: "Dark"; onTriggered: root.themeMode = text }
                }
            }

            Button {
                id: sortMenuButton
                text: root.modelRef ? root.modelRef.sort_field_name : "Name"
                Layout.preferredWidth: 112
                Layout.preferredHeight: root.toolbarButtonSize
                contentItem: Item {
                    Label {
                        anchors.left: parent.left
                        anchors.right: sortArrow.left
                        anchors.top: parent.top
                        anchors.bottom: parent.bottom
                        anchors.leftMargin: 10
                        anchors.rightMargin: 6
                        text: sortMenuButton.text
                        color: root.textColor
                        verticalAlignment: Text.AlignVCenter
                        elide: Text.ElideRight
                    }

                    Image {
                        id: sortArrow
                        anchors.right: parent.right
                        anchors.rightMargin: 8
                        anchors.verticalCenter: parent.verticalCenter
                        width: 14
                        height: 14
                        source: root.toolbarIconSource("icon_down.svg")
                        fillMode: Image.PreserveAspectFit
                        smooth: true
                    }
                }
                background: Rectangle {
                    radius: 4
                    color: root.buttonColor
                    border.color: root.borderColor
                }
                onClicked: sortMenu.open()

                Menu {
                    id: sortMenu
                    y: sortMenuButton.height
                    width: sortMenuButton.width
                    MenuItem { text: "Name"; onTriggered: if (root.modelRef) root.modelRef.set_sort_field(text) }
                    MenuItem { text: "Size"; onTriggered: if (root.modelRef) root.modelRef.set_sort_field(text) }
                    MenuItem { text: "Type"; onTriggered: if (root.modelRef) root.modelRef.set_sort_field(text) }
                    MenuItem { text: "Modified"; onTriggered: if (root.modelRef) root.modelRef.set_sort_field(text) }
                }
            }

            ToolButton {
                text: root.modelRef && root.modelRef.sort_descending ? "Sort descending" : "Sort ascending"
                display: AbstractButton.IconOnly
                icon.source: root.toolbarIconSource("icon_sort.svg")
                icon.width: root.toolbarIconSize
                icon.height: root.toolbarIconSize
                Layout.preferredWidth: root.toolbarButtonSize
                Layout.preferredHeight: root.toolbarButtonSize
                ToolTip.visible: hovered
                ToolTip.text: text
                onClicked: if (root.modelRef) root.modelRef.set_sort_descending(!(root.modelRef.sort_descending))
            }

            ToolButton {
                text: "Details view"
                checkable: true
                checked: root.viewMode === "Details"
                display: AbstractButton.IconOnly
                icon.source: root.toolbarIconSource("icon_view_details.svg")
                icon.width: root.toolbarIconSize
                icon.height: root.toolbarIconSize
                Layout.preferredWidth: root.toolbarButtonSize
                Layout.preferredHeight: root.toolbarButtonSize
                ToolTip.visible: hovered
                ToolTip.text: text
                onClicked: if (root.modelRef) root.modelRef.set_folder_view_mode("Details")
            }

            ToolButton {
                text: "List view"
                checkable: true
                checked: root.viewMode === "List"
                display: AbstractButton.IconOnly
                icon.source: root.toolbarIconSource("icon_view_list.svg")
                icon.width: root.toolbarIconSize
                icon.height: root.toolbarIconSize
                Layout.preferredWidth: root.toolbarButtonSize
                Layout.preferredHeight: root.toolbarButtonSize
                ToolTip.visible: hovered
                ToolTip.text: text
                onClicked: if (root.modelRef) root.modelRef.set_folder_view_mode("List")
            }

            ToolButton {
                text: "Icons view"
                checkable: true
                checked: root.viewMode === "Icons"
                display: AbstractButton.IconOnly
                icon.source: root.toolbarIconSource("icon_view_icons.svg")
                icon.width: root.toolbarIconSize
                icon.height: root.toolbarIconSize
                Layout.preferredWidth: root.toolbarButtonSize
                Layout.preferredHeight: root.toolbarButtonSize
                ToolTip.visible: hovered
                ToolTip.text: text
                onClicked: if (root.modelRef) root.modelRef.set_folder_view_mode("Icons")
            }

            Button {
                id: groupingMenuButton
                text: root.modelRef ? root.modelRef.grouping_name : "None"
                Layout.preferredWidth: 112
                Layout.preferredHeight: root.toolbarButtonSize
                contentItem: Item {
                    Label {
                        anchors.left: parent.left
                        anchors.right: groupingArrow.left
                        anchors.top: parent.top
                        anchors.bottom: parent.bottom
                        anchors.leftMargin: 10
                        anchors.rightMargin: 6
                        text: groupingMenuButton.text
                        color: root.textColor
                        verticalAlignment: Text.AlignVCenter
                        elide: Text.ElideRight
                    }

                    Image {
                        id: groupingArrow
                        anchors.right: parent.right
                        anchors.rightMargin: 8
                        anchors.verticalCenter: parent.verticalCenter
                        width: 14
                        height: 14
                        source: root.toolbarIconSource("icon_down.svg")
                        fillMode: Image.PreserveAspectFit
                        smooth: true
                    }
                }
                background: Rectangle {
                    radius: 4
                    color: root.buttonColor
                    border.color: root.borderColor
                }
                onClicked: groupingMenu.open()

                Menu {
                    id: groupingMenu
                    y: groupingMenuButton.height
                    width: groupingMenuButton.width
                    MenuItem { text: "None"; onTriggered: if (root.modelRef) root.modelRef.set_grouping(text) }
                    MenuItem { text: "Type"; onTriggered: if (root.modelRef) root.modelRef.set_grouping(text) }
                }
            }

            ToolButton {
                text: root.modelRef && root.modelRef.show_hidden ? "Hide hidden files" : "Show hidden files"
                checkable: true
                checked: root.modelRef ? root.modelRef.show_hidden : false
                display: AbstractButton.IconOnly
                icon.source: root.toolbarIconSource(root.modelRef && root.modelRef.show_hidden ? "icon_hide-hidden.svg" : "icon_show_hidden.svg")
                icon.width: root.toolbarIconSize
                icon.height: root.toolbarIconSize
                Layout.preferredWidth: root.toolbarButtonSize
                Layout.preferredHeight: root.toolbarButtonSize
                ToolTip.visible: hovered
                ToolTip.text: text
                onClicked: if (root.modelRef) root.modelRef.set_show_hidden(!root.modelRef.show_hidden)
            }

            ToolButton {
                text: "New Folder"
                enabled: root.modelRef !== null && !root.isVirtualPath(root.modelRef.current_path)
                display: AbstractButton.IconOnly
                icon.source: root.toolbarIconSource("icon_new_folder.svg")
                icon.width: root.toolbarIconSize
                icon.height: root.toolbarIconSize
                Layout.preferredWidth: root.toolbarButtonSize
                Layout.preferredHeight: root.toolbarButtonSize
                ToolTip.visible: hovered
                ToolTip.text: text
                onClicked: newFolderDialog.open()
            }

            ToolButton {
                text: "New File"
                enabled: root.modelRef !== null && !root.isVirtualPath(root.modelRef.current_path)
                display: AbstractButton.IconOnly
                icon.source: root.toolbarIconSource("icon_new_file.svg")
                icon.width: root.toolbarIconSize
                icon.height: root.toolbarIconSize
                Layout.preferredWidth: root.toolbarButtonSize
                Layout.preferredHeight: root.toolbarButtonSize
                ToolTip.visible: hovered
                ToolTip.text: text
                onClicked: newFileDialog.open()
            }

            ToolButton {
                text: "Rename"
                enabled: root.modelRef !== null && root.selectedCount === 1 && !root.isVirtualPath(root.modelRef.current_path)
                display: AbstractButton.IconOnly
                icon.source: root.toolbarIconSource("icon_rename.svg")
                icon.width: root.toolbarIconSize
                icon.height: root.toolbarIconSize
                Layout.preferredWidth: root.toolbarButtonSize
                Layout.preferredHeight: root.toolbarButtonSize
                ToolTip.visible: hovered
                ToolTip.text: text
                onClicked: renameDialog.open()
            }

            ToolButton {
                text: "Settings"
                enabled: false
                display: AbstractButton.IconOnly
                icon.source: root.toolbarIconSource("icon_settings.svg")
                icon.width: root.toolbarIconSize
                icon.height: root.toolbarIconSize
                Layout.preferredWidth: root.toolbarButtonSize
                Layout.preferredHeight: root.toolbarButtonSize
                ToolTip.visible: hovered
                ToolTip.text: "Settings"
            }
        }
    }

    Dialog {
        id: newFolderDialog
        title: "New Folder"
        modal: true
        anchors.centerIn: Overlay.overlay
        standardButtons: Dialog.Ok | Dialog.Cancel

        onOpened: newFolderField.forceActiveFocus()
        onAccepted: {
            if (root.modelRef)
                root.modelRef.create_folder(newFolderField.text)
            newFolderField.text = ""
        }
        onRejected: newFolderField.text = ""

        contentItem: ColumnLayout {
            spacing: 10
            width: 320

            Label {
                text: "Folder name"
                color: root.textColor
            }

            TextField {
                id: newFolderField
                Layout.fillWidth: true
                placeholderText: "New folder"
                selectByMouse: true
            }
        }
    }

    Dialog {
        id: newFileDialog
        title: "New File"
        modal: true
        anchors.centerIn: Overlay.overlay
        standardButtons: Dialog.Ok | Dialog.Cancel

        onOpened: newFileField.forceActiveFocus()
        onAccepted: {
            if (root.modelRef)
                root.modelRef.create_file(newFileField.text)
            newFileField.text = ""
        }
        onRejected: newFileField.text = ""

        contentItem: ColumnLayout {
            spacing: 10
            width: 320

            Label {
                text: "File name"
                color: root.textColor
            }

            TextField {
                id: newFileField
                Layout.fillWidth: true
                placeholderText: "new-file.txt"
                selectByMouse: true
            }
        }
    }

    Dialog {
        id: renameDialog
        title: "Rename"
        modal: true
        anchors.centerIn: Overlay.overlay
        standardButtons: Dialog.Ok | Dialog.Cancel

        onOpened: {
            renameField.text = root.selectedName
            renameField.selectAll()
            renameField.forceActiveFocus()
        }
        onAccepted: {
            if (root.modelRef && root.selectedPath.length > 0)
                root.modelRef.rename_path(root.selectedPath, renameField.text)
            renameField.text = ""
        }
        onRejected: renameField.text = ""

        contentItem: ColumnLayout {
            spacing: 10
            width: 320

            Label {
                text: "New name"
                color: root.textColor
            }

            TextField {
                id: renameField
                Layout.fillWidth: true
                selectByMouse: true
            }
        }
    }

    Menu {
        id: fileContextMenu

        MenuItem {
            text: "Open"
            enabled: root.contextMenuHasTarget
            onTriggered: root.activateCurrentSelection(root.contextMenuPath, root.contextMenuIsDirectory)
        }
        MenuSeparator {}
        MenuItem {
            text: root.selectedPathsForAction(root.contextMenuPath).length > 1 ? "Copy Selected" : "Copy"
            enabled: root.contextMenuHasTarget && !root.isReadOnlyPath(root.contextMenuPath)
            onTriggered: root.copyCurrentSelection(root.contextMenuPath)
        }
        MenuItem {
            text: root.selectedPathsForAction(root.contextMenuPath).length > 1 ? "Cut Selected" : "Cut"
            enabled: root.contextMenuHasTarget && !root.isReadOnlyPath(root.contextMenuPath)
            onTriggered: root.cutCurrentSelection(root.contextMenuPath)
        }
        MenuItem {
            text: "Paste"
            enabled: root.modelRef && root.modelRef.can_paste && !root.isReadOnlyPath(root.modelRef.current_path)
            onTriggered: if (root.modelRef) root.modelRef.paste_into_current()
        }
        MenuSeparator {}
        MenuItem {
            text: "New Folder"
            enabled: root.modelRef && !root.isReadOnlyPath(root.modelRef.current_path)
            onTriggered: newFolderDialog.open()
        }
        MenuItem {
            text: "New File"
            enabled: root.modelRef && !root.isReadOnlyPath(root.modelRef.current_path)
            onTriggered: newFileDialog.open()
        }
        MenuSeparator {}
        MenuItem {
            text: "Rename"
            enabled: root.contextMenuHasTarget && root.selectedPathsForAction(root.contextMenuPath).length === 1 && !root.isReadOnlyPath(root.contextMenuPath)
            onTriggered: renameDialog.open()
        }
        MenuItem {
            text: root.selectedPathsForAction(root.contextMenuPath).length > 1 ? "Move Selected to Trash" : "Move to Trash"
            enabled: root.contextMenuHasTarget && !root.isReadOnlyPath(root.contextMenuPath)
            onTriggered: root.trashCurrentSelection(root.contextMenuPath)
        }
    }

    Rectangle {
        anchors.fill: parent
        gradient: Gradient {
            GradientStop { position: 0.0; color: root.backgroundStart }
            GradientStop { position: 1.0; color: root.backgroundEnd }
        }

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 14
            spacing: 14

            RowLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true

                Frame {
                    Layout.preferredWidth: root.sidebarWidth
                    Layout.minimumWidth: 160
                    Layout.maximumWidth: 420
                    Layout.fillHeight: true
                    background: Rectangle {
                        radius: 12
                        color: root.sidebarColor
                        border.color: root.borderColor
                    }

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: 10
                        spacing: 10

                        Label {
                            text: "Favourites"
                            font.pixelSize: 14
                            font.bold: true
                            color: root.textColor

                            MouseArea {
                                anchors.fill: parent
                                cursorShape: Qt.PointingHandCursor
                                onClicked: if (root.modelRef) root.modelRef.load_path(root.favoritesUri)
                            }
                        }

                        ScrollView {
                            Layout.fillWidth: true
                            Layout.preferredHeight: root.favoritesHeight
                            Layout.minimumHeight: 100
                            Layout.maximumHeight: 420
                            clip: true
                            ScrollBar.vertical: ScrollBar {
                                policy: ScrollBar.AlwaysOn
                                width: 14
                                visible: true
                                opacity: 1.0
                                active: true
                                interactive: true
                                background: Rectangle {
                                    implicitWidth: 14
                                    implicitHeight: 14
                                    color: root.scrollbarTrackColor
                                    radius: 4
                                }
                                contentItem: Rectangle {
                                    width: 12
                                    implicitWidth: 12
                                    implicitHeight: 36
                                    radius: 6
                                    color: root.scrollbarThumbColor
                                    border.width: 1
                                    border.color: root.scrollbarThumbBorderColor
                                }
                            }

                            Column {
                                width: parent.width
                                spacing: 2

                                Repeater {
                                model: [
                                    { label: "Home", path: Platform.StandardPaths.writableLocation(Platform.StandardPaths.HomeLocation) },
                                    { label: "Desktop", path: Platform.StandardPaths.writableLocation(Platform.StandardPaths.DesktopLocation) },
                                    { label: "Documents", path: Platform.StandardPaths.writableLocation(Platform.StandardPaths.DocumentsLocation) },
                                    { label: "Downloads", path: Platform.StandardPaths.writableLocation(Platform.StandardPaths.DownloadLocation) },
                                        { label: "sysApps", path: "/home/tamsynn/sysApps" }
                                    ]
                                delegate: ItemDelegate {
                                    id: favoriteDelegate
                                    width: parent.width
                                    highlighted: root.dropTargetPath === modelData.path || (root.modelRef && modelData.path === root.modelRef.current_path)
                                    onClicked: if (root.modelRef) root.modelRef.load_path(modelData.path)
                                    implicitHeight: 30

                                    contentItem: RowLayout {
                                        spacing: 8

                                        Image {
                                            Layout.preferredWidth: 16
                                            Layout.preferredHeight: 16
                                            source: root.folderIconSource(modelData.label, modelData.path)
                                            fillMode: Image.PreserveAspectFit
                                            smooth: true
                                            asynchronous: true
                                        }

                                        Label {
                                            text: modelData.label
                                            color: root.textColor
                                            elide: Text.ElideRight
                                            Layout.fillWidth: true
                                        }
                                    }

                                    DropArea {
                                        anchors.fill: parent
                                        enabled: !root.isReadOnlyPath(modelData.path)
                                        onEntered: function(drag) { root.updateDropTarget(drag, modelData.path) }
                                        onPositionChanged: function(drag) { root.updateDropTarget(drag, modelData.path) }
                                        onExited: root.clearDropTarget(modelData.path)
                                        onDropped: function(drop) { root.handleDrop(drop, modelData.path) }
                                    }
                                }
                            }
                        }
                        }

                        Rectangle {
                            Layout.fillWidth: true
                            height: 6
                            radius: 3
                            color: "#00000000"

                            Rectangle {
                                anchors.centerIn: parent
                                width: parent.width - 12
                                height: 1
                                color: root.borderColor
                            }

                            MouseArea {
                                anchors.fill: parent
                                cursorShape: Qt.SizeVerCursor
                                property real startY: 0
                                property int startHeight: 0
                                onPressed: {
                                    startY = mouse.y
                                    startHeight = root.favoritesHeight
                                }
                                onPositionChanged: {
                                    if (!pressed)
                                        return
                                    const delta = mouse.y - startY
                                    root.favoritesHeight = Math.max(100, Math.min(420, startHeight + delta))
                                }
                            }
                        }

                        Label {
                            text: "Devices"
                            font.pixelSize: 14
                            font.bold: true
                            color: root.textColor

                            MouseArea {
                                anchors.fill: parent
                                cursorShape: Qt.PointingHandCursor
                                onClicked: if (root.modelRef) root.modelRef.load_path(root.devicesUri)
                            }
                        }

                        ListView {
                            Layout.fillWidth: true
                            Layout.preferredHeight: root.devicesHeight
                            Layout.minimumHeight: 100
                            Layout.maximumHeight: 260
                            clip: true
                            spacing: 2
                            model: root.devicesRef

                            delegate: ItemDelegate {
                                id: deviceDelegate
                                required property string label
                                required property string mountPath
                                required property string details
                                required property double usagePercent
                                required property bool mounted
                                width: ListView.view.width
                                enabled: mounted
                                highlighted: root.dropTargetPath === mountPath || (root.modelRef && mounted && mountPath === root.modelRef.current_path)

                                contentItem: Column {
                                    spacing: 5

                                    Label {
                                        text: label
                                        color: root.textColor
                                        elide: Text.ElideRight
                                        width: parent.width
                                        font.pixelSize: 13
                                        leftPadding: 22

                                        Image {
                                            anchors.left: parent.left
                                            anchors.verticalCenter: parent.verticalCenter
                                            width: 16
                                            height: 16
                                            source: root.folderIconSource(label, root.devicesUri)
                                            fillMode: Image.PreserveAspectFit
                                            smooth: true
                                            asynchronous: true
                                        }
                                    }

                                    Column {
                                        width: parent.width
                                        spacing: 2

                                        Rectangle {
                                            width: parent.width
                                            height: 6
                                            radius: 3
                                            color: root.inputColor
                                            border.width: 1
                                            border.color: root.borderColor

                                            Rectangle {
                                                width: Math.max(0, Math.min(parent.width, parent.width * (usagePercent / 100.0)))
                                                height: parent.height
                                                radius: parent.radius
                                                color: root.accentColor
                                            }
                                        }

                                        Label {
                                            text: details
                                            color: root.mutedTextColor
                                            font.pixelSize: 11
                                            elide: Text.ElideRight
                                            width: parent.width
                                        }
                                    }
                                }
                                implicitHeight: 48

                                onClicked: {
                                    if (mounted && root.modelRef)
                                        root.modelRef.load_path(mountPath)
                                }

                                DropArea {
                                    anchors.fill: parent
                                    enabled: mounted && !root.isReadOnlyPath(mountPath)
                                    onEntered: function(drag) { root.updateDropTarget(drag, mountPath) }
                                    onPositionChanged: function(drag) { root.updateDropTarget(drag, mountPath) }
                                    onExited: root.clearDropTarget(mountPath)
                                    onDropped: function(drop) { root.handleDrop(drop, mountPath) }
                                }
                            }
                        }

                        Rectangle {
                            Layout.fillWidth: true
                            height: 6
                            radius: 3
                            color: "#00000000"

                            Rectangle {
                                anchors.centerIn: parent
                                width: parent.width - 12
                                height: 1
                                color: root.borderColor
                            }

                            MouseArea {
                                anchors.fill: parent
                                cursorShape: Qt.SizeVerCursor
                                property real startY: 0
                                property int startHeight: 0
                                onPressed: {
                                    startY = mouse.y
                                    startHeight = root.devicesHeight
                                }
                                onPositionChanged: {
                                    if (!pressed)
                                        return
                                    const delta = mouse.y - startY
                                    root.devicesHeight = Math.max(100, Math.min(260, startHeight + delta))
                                }
                            }
                        }

                        Label {
                            text: "Tree"
                            font.pixelSize: 14
                            font.bold: true
                            color: root.textColor
                        }

                        ScrollView {
                            id: treeScrollView
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            clip: true
                            ScrollBar.vertical: ScrollBar {
                                id: treeVerticalScrollBar
                                parent: treeScrollView
                                anchors.top: treeScrollView.top
                                anchors.right: treeScrollView.right
                                anchors.bottom: treeHorizontalScrollBar.top
                                policy: ScrollBar.AlwaysOn
                                width: 8
                                visible: true
                                opacity: 1.0
                                active: true
                                interactive: true
                                background: Rectangle {
                                    implicitWidth: 8
                                    implicitHeight: 8
                                    color: root.scrollbarTrackColor
                                    radius: 3
                                }
                                contentItem: Rectangle {
                                    width: 6
                                    implicitWidth: 6
                                    implicitHeight: 36
                                    radius: 3
                                    color: root.scrollbarThumbColor
                                    border.width: 1
                                    border.color: root.scrollbarThumbBorderColor
                                }
                            }
                            ScrollBar.horizontal: ScrollBar {
                                id: treeHorizontalScrollBar
                                parent: treeScrollView
                                anchors.left: treeScrollView.left
                                anchors.right: treeVerticalScrollBar.left
                                anchors.bottom: treeScrollView.bottom
                                policy: ScrollBar.AlwaysOn
                                height: 8
                                visible: true
                                opacity: 1.0
                                active: true
                                interactive: true
                                background: Rectangle {
                                    implicitWidth: 8
                                    implicitHeight: 8
                                    color: root.scrollbarTrackColor
                                    radius: 3
                                }
                                contentItem: Rectangle {
                                    height: 6
                                    implicitWidth: 36
                                    implicitHeight: 6
                                    radius: 3
                                    color: root.scrollbarThumbColor
                                    border.width: 1
                                    border.color: root.scrollbarThumbBorderColor
                                }
                            }

                            Column {
                                width: parent.width
                                spacing: 0

                                Repeater {
                                    model: root.modelRef ? root.relativeTreeParts(root.modelRef.current_path) : []

                                    delegate: Item {
                                        id: treePathDelegate
                                        required property var modelData
                                        required property int index
                                        property bool isBranchEnd: index === (root.modelRef ? root.relativeTreeParts(root.modelRef.current_path).length - 1 : -1)
                                        property int rowIndent: 4 + (modelData.depth * root.treeIndentSize)
                                        width: parent.width
                                        height: root.treeRowHeight

                                        Row {
                                            x: treePathDelegate.rowIndent
                                            width: parent.width - treePathDelegate.rowIndent - 4
                                            height: parent.height
                                            spacing: 4

                                            Image {
                                                source: treePathDelegate.isBranchEnd
                                                    ? root.toolbarIconSource(root.treeChildrenExpanded ? "icon_nav_down.svg" : "icon_nav_forward.svg")
                                                    : root.toolbarIconSource("icon_nav_down.svg")
                                                width: root.treeChevronSize
                                                height: root.treeChevronSize
                                                anchors.verticalCenter: parent.verticalCenter
                                                fillMode: Image.PreserveAspectFit
                                                smooth: true
                                                asynchronous: true
                                            }

                                            Image {
                                                width: root.treeFolderIconSize
                                                height: root.treeFolderIconSize
                                                source: root.folderIconSource(modelData.label, modelData.path)
                                                anchors.verticalCenter: parent.verticalCenter
                                                fillMode: Image.PreserveAspectFit
                                                smooth: true
                                                asynchronous: true
                                            }

                                            Text {
                                                width: Math.max(0, parent.width - root.treeChevronSize - root.treeFolderIconSize - 16)
                                                text: modelData.label
                                                color: root.textColor
                                                font.pixelSize: 13
                                                elide: Text.ElideRight
                                                verticalAlignment: Text.AlignVCenter
                                            }
                                        }

                                        MouseArea {
                                            id: treePathMouse
                                            anchors.fill: parent
                                            hoverEnabled: true
                                            onClicked: {
                                                if (treePathDelegate.isBranchEnd)
                                                    root.treeChildrenExpanded = !root.treeChildrenExpanded
                                                if (root.modelRef)
                                                    root.modelRef.load_path(modelData.path)
                                            }
                                        }

                                        Rectangle {
                                            anchors.fill: parent
                                            visible: root.dropTargetPath === modelData.path
                                            color: root.rowSelectedColor
                                            opacity: 0.75
                                            radius: 4
                                        }

                                        DropArea {
                                            anchors.fill: parent
                                            enabled: !root.isReadOnlyPath(modelData.path)
                                            onEntered: function(drag) { root.updateDropTarget(drag, modelData.path) }
                                            onPositionChanged: function(drag) { root.updateDropTarget(drag, modelData.path) }
                                            onExited: root.clearDropTarget(modelData.path)
                                            onDropped: function(drop) { root.handleDrop(drop, modelData.path) }
                                        }
                                    }
                                }

                                Repeater {
                                    model: root.treeChildrenExpanded && root.modelRef ? root.modelRef : null

                                    delegate: Item {
                                        id: treeChildDelegate
                                        required property string name
                                        required property string path
                                        required property bool isDirectory
                                        property int rowIndent: 4 + (root.relativeTreeParts(root.modelRef ? root.modelRef.current_path : "").length * root.treeIndentSize)
                                        visible: isDirectory
                                        width: parent.width
                                        height: visible ? root.treeRowHeight : 0

                                        Rectangle {
                                            anchors.fill: parent
                                            visible: treeChildMouse.containsMouse
                                            color: root.rowOddColor
                                            opacity: 0.7
                                        }

                                        Row {
                                            x: treeChildDelegate.rowIndent
                                            width: parent.width - treeChildDelegate.rowIndent - 4
                                            height: parent.height
                                            spacing: 4

                                            Image {
                                                source: root.toolbarIconSource("icon_nav_forward.svg")
                                                width: root.treeChevronSize
                                                height: root.treeChevronSize
                                                anchors.verticalCenter: parent.verticalCenter
                                                fillMode: Image.PreserveAspectFit
                                                smooth: true
                                                asynchronous: true
                                            }

                                            Image {
                                                width: root.treeFolderIconSize
                                                height: root.treeFolderIconSize
                                                source: root.folderIconSource(name, path)
                                                anchors.verticalCenter: parent.verticalCenter
                                                fillMode: Image.PreserveAspectFit
                                                smooth: true
                                                asynchronous: true
                                            }

                                            Text {
                                                text: name
                                                color: root.textColor
                                                font.pixelSize: 13
                                                elide: Text.ElideRight
                                                width: Math.max(0, parent.width - root.treeChevronSize - root.treeFolderIconSize - 16)
                                                verticalAlignment: Text.AlignVCenter
                                            }
                                        }

                                        MouseArea {
                                            id: treeChildMouse
                                            anchors.fill: parent
                                            hoverEnabled: true
                                            onClicked: if (root.modelRef) root.modelRef.load_path(path)
                                        }

                                        Rectangle {
                                            anchors.fill: parent
                                            visible: root.dropTargetPath === path
                                            color: root.rowSelectedColor
                                            opacity: 0.75
                                            radius: 4
                                        }

                                        DropArea {
                                            anchors.fill: parent
                                            enabled: isDirectory && !root.isReadOnlyPath(path)
                                            onEntered: function(drag) { root.updateDropTarget(drag, path) }
                                            onPositionChanged: function(drag) { root.updateDropTarget(drag, path) }
                                            onExited: root.clearDropTarget(path)
                                            onDropped: function(drop) { root.handleDrop(drop, path) }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                Rectangle {
                    Layout.preferredWidth: 8
                    Layout.minimumWidth: 8
                    Layout.maximumWidth: 8
                    Layout.fillHeight: true
                    color: "#00000000"

                    Rectangle {
                        anchors.centerIn: parent
                        width: 1
                        height: parent.height - 24
                        color: root.borderColor
                    }

                    MouseArea {
                        anchors.fill: parent
                        cursorShape: Qt.SizeHorCursor
                        property real startX: 0
                        property int startWidth: 0

                        onPressed: {
                            startX = mouse.x
                            startWidth = root.sidebarWidth
                        }

                        onPositionChanged: {
                            if (!pressed)
                                return
                            const delta = mouse.x - startX
                            root.sidebarWidth = Math.max(160, Math.min(420, startWidth + delta))
                        }
                    }
                }

                Frame {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    background: Rectangle {
                        radius: 12
                        color: root.surfaceColor
                        border.color: root.borderColor
                    }

                    Item {
                        id: filePanelContent
                        anchors.fill: parent
                        anchors.margins: 14
                        clip: true

                        DropArea {
                            anchors.fill: parent
                            enabled: root.modelRef && !root.isReadOnlyPath(root.modelRef.current_path)
                            onEntered: function(drag) {
                                if (root.dropTargetPath.length === 0)
                                    root.updateDropTarget(drag, root.modelRef.current_path)
                            }
                            onPositionChanged: function(drag) {
                                if (root.dropTargetPath.length === 0 || root.dropTargetPath === root.modelRef.current_path)
                                    root.updateDropTarget(drag, root.modelRef.current_path)
                            }
                            onExited: root.clearDropTarget(root.modelRef ? root.modelRef.current_path : "")
                            onDropped: function(drop) {
                                if (root.dropTargetPath.length === 0 || root.dropTargetPath === root.modelRef.current_path)
                                    root.handleDrop(drop, root.modelRef.current_path)
                            }
                        }

                        Rectangle {
                            anchors.fill: parent
                            radius: 8
                            color: "transparent"
                            border.width: root.modelRef && root.dropTargetPath === root.modelRef.current_path ? 2 : 0
                            border.color: root.accentColor
                            opacity: 0.75
                        }

                        ColumnLayout {
                            id: middleContent
                            z: 1
                            width: filePanelContent.width
                            height: parent.height
                            spacing: 10

                            Label {
                                text: "Current Folder"
                                font.pixelSize: 16
                                font.bold: true
                                color: root.textColor
                            }

                            Rectangle {
                                Layout.fillWidth: true
                                height: 1
                                color: root.borderColor
                            }

                            Loader {
                                Layout.fillWidth: true
                                Layout.fillHeight: true
                                sourceComponent: root.viewMode === "Icons"
                                    ? iconsViewComponent
                                    : (root.viewMode === "List" ? listViewComponent : detailsViewComponent)
                            }

                            Label {
                                visible: root.modelRef ? root.modelRef.error_message.length > 0 : false
                                text: root.modelRef ? root.modelRef.error_message : ""
                                color: root.darkTheme ? "#ff9aa2" : "#8c2f39"
                                wrapMode: Text.Wrap
                                Layout.fillWidth: true
                            }
                        }
                    }
                }

                Rectangle {
                    Layout.preferredWidth: 8
                    Layout.minimumWidth: 8
                    Layout.maximumWidth: 8
                    Layout.fillHeight: true
                    color: "#00000000"

                    Rectangle {
                        anchors.centerIn: parent
                        width: 1
                        height: parent.height - 24
                        color: root.borderColor
                    }

                    MouseArea {
                        anchors.fill: parent
                        cursorShape: Qt.SizeHorCursor
                        property real startX: 0
                        property int startWidth: 0

                        onPressed: {
                            startX = mouse.x
                            startWidth = root.infoPanelWidth
                        }

                        onPositionChanged: {
                            if (!pressed)
                                return
                            const delta = mouse.x - startX
                            root.infoPanelWidth = Math.max(220, Math.min(560, startWidth - delta))
                        }
                    }
                }

                Frame {
                    Layout.preferredWidth: root.infoPanelWidth
                    Layout.minimumWidth: 220
                    Layout.maximumWidth: 560
                    Layout.fillHeight: true
                    background: Rectangle {
                        radius: 12
                        color: root.surfaceColor
                        border.color: root.borderColor
                    }

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: 14
                        spacing: 12

                        Label {
                            text: "Details"
                            font.pixelSize: 16
                            font.bold: true
                            color: root.textColor
                        }

                        Rectangle {
                            Layout.fillWidth: true
                            height: 1
                            color: root.borderColor
                        }

                        Item {
                            Layout.alignment: Qt.AlignHCenter
                            width: 120
                            height: 96

                            Image {
                                anchors.fill: parent
                                visible: root.modelRef && root.modelRef.selected_info_is_directory
                                source: root.folderIconSource(
                                    root.modelRef ? root.modelRef.selected_info_name : "",
                                    root.modelRef ? root.modelRef.selected_info_path : ""
                                )
                                fillMode: Image.PreserveAspectFit
                                smooth: true
                                asynchronous: true
                            }

                            Image {
                                anchors.fill: parent
                                visible: root.modelRef
                                    && !root.modelRef.selected_info_is_directory
                                    && root.filePreviewSource(root.modelRef.selected_info_path, root.modelRef.selected_info_thumbnail_url).length > 0
                                source: root.modelRef ? root.filePreviewSource(root.modelRef.selected_info_path, root.modelRef.selected_info_thumbnail_url) : ""
                                fillMode: Image.PreserveAspectFit
                                smooth: true
                                asynchronous: true
                                cache: false
                            }

                            Image {
                                anchors.fill: parent
                                visible: root.modelRef
                                    && !root.modelRef.selected_info_is_directory
                                    && root.filePreviewSource(root.modelRef.selected_info_path, root.modelRef.selected_info_thumbnail_url).length === 0
                                    && root.fileIconSource(root.modelRef.selected_info_path).length > 0
                                source: root.modelRef ? root.fileIconSource(root.modelRef.selected_info_path) : ""
                                fillMode: Image.PreserveAspectFit
                                smooth: true
                                asynchronous: true
                            }

                            Rectangle {
                                anchors.fill: parent
                                visible: !(root.modelRef && root.modelRef.selected_info_is_directory)
                                    && !(root.modelRef && root.filePreviewSource(root.modelRef.selected_info_path, root.modelRef.selected_info_thumbnail_url).length > 0)
                                    && !(root.modelRef && root.fileIconSource(root.modelRef.selected_info_path).length > 0)
                                radius: 16
                                color: root.darkTheme ? "#5f7b85" : "#c0d6d8"
                            }
                        }

                        Label {
                            Layout.fillWidth: true
                            text: root.modelRef ? root.modelRef.selected_info_name : ""
                            color: root.textColor
                            font.pixelSize: 20
                            font.bold: true
                            horizontalAlignment: Text.AlignHCenter
                            wrapMode: Text.Wrap
                        }

                        GridLayout {
                            Layout.fillWidth: true
                            columns: 2
                            columnSpacing: 10
                            rowSpacing: 8

                            Label { text: "Type:"; color: root.mutedTextColor; font.bold: true }
                            Label { text: root.modelRef ? root.modelRef.selected_info_kind : ""; color: root.textColor; Layout.fillWidth: true; elide: Text.ElideRight }

                            Label { text: "Size:"; color: root.mutedTextColor; font.bold: true }
                            Label { text: root.modelRef ? root.modelRef.selected_info_size : ""; color: root.textColor; Layout.fillWidth: true; elide: Text.ElideRight }

                            Label { text: "Modified:"; color: root.mutedTextColor; font.bold: true }
                            Label {
                                text: root.modelRef
                                    ? root.formatModifiedValue(root.modelRef.selected_info_modified_ms, root.modelRef.selected_info_modified)
                                    : ""
                                color: root.textColor
                                Layout.fillWidth: true
                                wrapMode: Text.Wrap
                            }
                        }

                        Label {
                            text: "Path"
                            color: root.mutedTextColor
                            font.bold: true
                        }

                        TextArea {
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            readOnly: true
                            wrapMode: TextEdit.WrapAnywhere
                            text: root.modelRef ? root.modelRef.selected_info_path : ""
                            color: root.textColor
                            background: Rectangle {
                                radius: 8
                                color: root.inputColor
                                border.color: root.borderColor
                            }
                        }
                    }
                }
            }

            Frame {
                Layout.fillWidth: true
                Layout.preferredHeight: root.terminalPanelHeight
                Layout.minimumHeight: root.terminalPanelHeight
                Layout.maximumHeight: root.terminalPanelHeight
                background: Rectangle {
                    radius: 12
                    color: root.surfaceColor
                    border.color: root.borderColor
                }

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: 12
                    spacing: 8

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 8

                        Label {
                            Layout.fillWidth: true
                            text: "NOXcmd Mini"
                            color: root.textColor
                            font.bold: true
                            font.pixelSize: 13
                        }

                        Button {
                            text: "Pop Out"
                            enabled: root.modelRef !== null
                            onClicked: if (root.modelRef) root.modelRef.pop_out_terminal()
                        }

                        Button {
                            text: "Clear"
                            enabled: root.modelRef !== null
                            onClicked: if (root.modelRef) root.modelRef.clear_terminal_output()
                        }
                    }

                    TextArea {
                        id: terminalOutput
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        readOnly: true
                        wrapMode: TextEdit.Wrap
                        text: root.modelRef ? root.modelRef.terminal_output : ""
                        color: root.textColor
                        font.family: "monospace"
                        font.pixelSize: 12
                        background: Rectangle {
                            radius: 8
                            color: root.inputColor
                            border.color: root.borderColor
                        }
                    }

                    TextField {
                        id: terminalInput
                        Layout.fillWidth: true
                        placeholderText: "Enter shell command"
                        selectByMouse: true
                        font.family: "monospace"
                        onAccepted: {
                            if (root.modelRef && text.trim().length > 0) {
                                root.modelRef.execute_terminal_command(text)
                                text = ""
                            }
                        }
                    }
                }
            }
        }
    }
}
