# Architecture Notes

## Stack

- UI: Qt Quick / QML
- Backend: Rust
- Initial integration direction: Rust backend crate with a thin Qt bridge layer

## Layering

- `crates/file-ops`: pure Rust filesystem domain logic
- `apps/files`: app-specific bridge code and QML

The important constraint is that the filesystem logic stays testable without Qt.

Current file-manager implementation notes:

- `apps/files/src/main.rs` owns the Qt bridge model, path navigation, archive loading, folder settings, trash, and app-local copy/cut/paste operations.
- `apps/files/qml/Main.qml` owns the window layout, toolbar, sidebar, file views, details panel, dialogs, context menu, and keyboard shortcuts.
- `crates/file-ops` remains the testable Rust crate for directory listing, sorting, filtering, and grouping primitives.

## Bridge Decision

The current bridge choice is `qmetaobject` for Qt 5. The main requirement is exposing a directory model from Rust to QML without burying core logic inside UI glue.

That means the bridge layer should be thin and replaceable:

- Rust owns listing, sorting, filtering, and operation semantics.
- QML consumes a model and sends user intents back down.

## Environment Note

The current bridge uses `qmetaobject`, which targets the Qt 5 stack available on this machine. The `files` app supports:

- default CLI builds without Qt integration
- optional QML builds through the `qml` Cargo feature when Qt Quick development headers are installed
