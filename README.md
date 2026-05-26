# sysApps

`sysApps` is a Linux-first desktop app suite intended to replace common native utilities with a coherent Qt/QML + Rust stack.

The first target is `files`, a file manager for Linux Mint Cinnamon with a pragmatic MVP:

- browse directories
- inspect file metadata
- navigate with breadcrumbs
- create, rename, trash, copy, cut, and paste items
- support trash semantics instead of hard deletes by default
- open files with system defaults

## Layout

- `apps/files`: file manager application shell and QML UI
- `apps/image-viewer`: image viewer with folder/archive navigation and export controls
- `apps/noxgames`: ROM/native-game launcher CLI and future library app
- `crates/file-ops`: filesystem listing and operation primitives
- `docs`: product and architecture notes

Relevant concept notes:

- `docs/room-launcher-concept.md`: Navigator-inspired room launcher concept
- `docs/future-ideas.md`: deferred NOXcmd, project creation, image tooling, and NOXfiles TUI ideas

## Current Status

This repository currently contains:

- a Rust workspace
- a first backend crate for directory listing
- a CLI fallback app binary to exercise the backend
- a feature-gated QML file manager with a Rust-backed directory model
- persistent per-folder view settings
- list, details, and icon views with sorting, grouping, context menus, and keyboard navigation
- local directory, device, and archive browsing support

## Local Requirements

The Rust/QML UI currently targets the Qt 5 stack available on this Linux Mint machine.

- Default build: `cargo test`
- CLI runner: `cargo run -p files-app -- .`
- QML runner: `cargo run -p files-app --features qml -- <path>`
- Image viewer runner: `cargo run -p image-viewer-app --features qml -- <image-path-or-archive-uri>`

To compile the QML build locally, install Qt Quick development headers. On Debian-based systems this is typically `qtdeclarative5-dev`.

## Next Steps

1. Improve large-folder and archive-loading performance.
2. Move copy/cut/paste from the app-local clipboard toward system clipboard integration.
3. Add operation progress reporting for long-running copy, move, trash, and archive work.
4. Continue tightening Dolphin-like navigation behavior, especially tree/sidebar polish.
