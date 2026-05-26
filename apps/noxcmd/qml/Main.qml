import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15

ApplicationWindow {
    id: root
    width: 1120
    height: 760
    visible: true
    title: "NOXcmd"

    property var modelRef: (typeof commandBlocksModel !== "undefined") ? commandBlocksModel : null
    property color windowColor: "#070b12"
    property color surfaceColor: "#101827"
    property color panelColor: "#0c1220"
    property color borderColor: "#2e4058"
    property color textColor: "#f3f7fb"
    property color mutedTextColor: "#9fb4ca"
    property color accentColor: "#35d6ff"
    property color dangerColor: "#ff6688"
    property color successColor: "#54e0a2"

    function runEditorCommand() {
        if (!root.modelRef)
            return
        const command = commandEditor.text.trim()
        if (command.length === 0)
            return
        root.modelRef.run_command(command)
        commandEditor.text = ""
        commandEditor.forceActiveFocus()
        Qt.callLater(function() {
            blocksView.positionViewAtEnd()
        })
    }

    palette {
        window: root.windowColor
        base: root.surfaceColor
        text: root.textColor
        button: root.panelColor
        buttonText: root.textColor
        highlight: root.accentColor
    }

    Rectangle {
        anchors.fill: parent
        color: root.windowColor

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 14
            spacing: 12

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 56
                radius: 10
                color: root.panelColor
                border.color: root.borderColor

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 14
                    anchors.rightMargin: 14
                    spacing: 12

                    Rectangle {
                        Layout.preferredWidth: 34
                        Layout.preferredHeight: 34
                        radius: 8
                        color: "#1b1032"
                        border.color: "#8b5cf6"

                        Text {
                            anchors.centerIn: parent
                            text: "N"
                            color: root.textColor
                            font.bold: true
                            font.pixelSize: 18
                        }
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 1

                        Label {
                            text: "NOXcmd"
                            color: root.textColor
                            font.bold: true
                            font.pixelSize: 16
                        }

                        Label {
                            Layout.fillWidth: true
                            text: root.modelRef ? root.modelRef.current_directory : ""
                            color: root.mutedTextColor
                            elide: Text.ElideMiddle
                            font.pixelSize: 12
                        }
                    }

                    Button {
                        text: "Clear"
                        onClicked: if (root.modelRef) root.modelRef.clear_blocks()
                    }
                }
            }

            ListView {
                id: blocksView
                Layout.fillWidth: true
                Layout.fillHeight: true
                model: root.modelRef
                clip: true
                spacing: 10
                boundsBehavior: Flickable.StopAtBounds

                delegate: Rectangle {
                    required property string command
                    required property string cwd
                    required property string output
                    required property int exitCode
                    required property double durationMs
                    required property double startedAt
                    required property bool success
                    width: blocksView.width
                    implicitHeight: blockLayout.implicitHeight + 24
                    radius: 10
                    color: root.surfaceColor
                    border.width: 1
                    border.color: success ? "#294d54" : "#643244"

                    ColumnLayout {
                        id: blockLayout
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.top: parent.top
                        anchors.margins: 12
                        spacing: 8

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 8

                            Label {
                                text: "$"
                                color: root.accentColor
                                font.bold: true
                                font.family: "monospace"
                                font.pixelSize: 14
                            }

                            TextEdit {
                                Layout.fillWidth: true
                                text: command
                                readOnly: true
                                selectByMouse: true
                                color: root.textColor
                                selectionColor: "#7c3aed"
                                selectedTextColor: "white"
                                wrapMode: TextEdit.Wrap
                                font.family: "monospace"
                                font.pixelSize: 14
                            }

                            Label {
                                text: success ? "ok" : ("exit " + exitCode)
                                color: success ? root.successColor : root.dangerColor
                                font.pixelSize: 12
                            }

                            Label {
                                text: Math.max(0, Math.round(durationMs)) + " ms"
                                color: root.mutedTextColor
                                font.pixelSize: 12
                            }
                        }

                        Label {
                            Layout.fillWidth: true
                            text: cwd
                            color: root.mutedTextColor
                            elide: Text.ElideMiddle
                            font.pixelSize: 11
                        }

                        TextArea {
                            Layout.fillWidth: true
                            visible: output.length > 0
                            text: output
                            readOnly: true
                            selectByMouse: true
                            wrapMode: TextEdit.WrapAnywhere
                            color: root.textColor
                            selectionColor: "#7c3aed"
                            selectedTextColor: "white"
                            font.family: "monospace"
                            font.pixelSize: 13
                            background: Rectangle {
                                radius: 8
                                color: "#0a101b"
                                border.color: "#223348"
                            }
                        }
                    }
                }
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 136
                radius: 10
                color: root.panelColor
                border.color: commandEditor.activeFocus ? root.accentColor : root.borderColor

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: 10
                    spacing: 8

                    RowLayout {
                        Layout.fillWidth: true

                        Label {
                            text: "$"
                            color: root.accentColor
                            font.bold: true
                            font.family: "monospace"
                            font.pixelSize: 14
                        }

                        Label {
                            Layout.fillWidth: true
                            text: "Click to place cursor. Select text and type, Delete, or Backspace to replace/remove."
                            color: root.mutedTextColor
                            elide: Text.ElideRight
                            font.pixelSize: 12
                        }

                        Button {
                            text: "Run"
                            onClicked: root.runEditorCommand()
                        }
                    }

                    TextArea {
                        id: commandEditor
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        selectByMouse: true
                        persistentSelection: true
                        wrapMode: TextEdit.WrapAnywhere
                        color: root.textColor
                        selectionColor: "#7c3aed"
                        selectedTextColor: "white"
                        placeholderText: "Enter command"
                        placeholderTextColor: root.mutedTextColor
                        font.family: "monospace"
                        font.pixelSize: 15
                        Keys.onPressed: function(event) {
                            if ((event.key === Qt.Key_Return || event.key === Qt.Key_Enter) && !(event.modifiers & Qt.ShiftModifier)) {
                                root.runEditorCommand()
                                event.accepted = true
                            }
                        }
                        background: Rectangle {
                            radius: 8
                            color: "#0a101b"
                            border.color: "transparent"
                        }
                    }
                }
            }

            Label {
                visible: root.modelRef && root.modelRef.error_message.length > 0
                Layout.fillWidth: true
                text: root.modelRef ? root.modelRef.error_message : ""
                color: root.dangerColor
                wrapMode: Text.Wrap
            }
        }
    }
}
