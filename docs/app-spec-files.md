# `files` MVP Spec

## Target

A Linux Mint Cinnamon file manager that is good enough for daily local file browsing before advanced features are added.

## MVP Features

- open a starting directory from CLI or default to the current directory
- list directory contents with name, type, size, hidden state, and modified time
- navigate into directories and back up to parent directories
- show breadcrumbs for the current path
- toggle hidden files
- sort by name, size, type, and modified time
- open files with the system default handler
- move items to trash
- create folders
- rename single items
- copy, cut, and paste single items through the file view context menu
- remember folder view settings such as view mode, sorting, grouping, and hidden-file visibility
- browse supported archive formats as read-only locations
- show mounted devices and a folder tree in the sidebar

## Current Implementation Notes

- The QML UI has details, list, and icon views, plus per-folder persisted view settings.
- Context menus currently support open, copy, cut, paste, create folder/file, rename, and move to trash.
- Copy/cut/paste uses an app-local clipboard. It does not yet integrate with the desktop clipboard or multi-select.
- Folder and archive loading is asynchronous, with basic archive listing cache support.
- Trash uses desktop trash behavior where available, with a local Trash fallback.

## Explicitly Deferred

- split view
- tabs
- network shares
- full-text search
- git integration
- plugin system
- multi-select file operations
- system clipboard file-copy integration
- visible progress and cancellation for long-running file operations

## Backend Boundaries

The Rust layer should own:

- filesystem reads and metadata normalization
- path validation and error mapping
- file operation scheduling
- trash implementation
- MIME lookup and opener dispatch

The QML layer should own:

- window layout
- list/grid presentation
- keyboard shortcuts
- breadcrumbs, context menus, dialogs, and progress UI

## Completed First Technical Milestone

The initial foundation is in place:

1. a Rust API that can list a directory safely and predictably
2. a small executable that exercises that API
3. a QML shell with real navigation, file views, dialogs, and a Rust-backed model

## Near-Term Work

1. improve large-folder and archive performance
2. add visible progress and cancellation for long-running file operations
3. add multi-select and desktop clipboard integration for copy/cut/paste
4. continue refining the sidebar tree and device navigation behavior
