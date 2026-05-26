# Vision

`sysApps` is a suite of Linux desktop applications built around a shared platform instead of a pile of unrelated programs.

## Principles

- Linux-first integration over cross-platform abstraction.
- Fast startup and responsive file operations.
- Predictable behavior for keyboard, mouse, and drag/drop workflows.
- Shared services and data models across the app suite.
- Rust owns backend logic and safety-critical operations.
- Qt/QML owns presentation, interaction, and desktop-facing UI.

## Initial App Order

1. `files`
2. `viewer`
3. `editor`
4. `settings`

## Why Start With `files`

The file manager forces the core platform decisions early:

- directory models
- file watching
- MIME/open-with
- thumbnailing
- trash semantics
- long-running task UX
- mount/device awareness
