# Future Ideas

This file captures product ideas that are worth preserving but are not immediate implementation commitments.

## NOXcmd

NOXcmd should exist in two forms:

- NOXcmd Desktop: a full terminal app with tabs, splits, profiles, command palette, searchable history, bookmarks/snippets, command blocks, side panels, theme-aware styling, and later SSH/session management.
- NOXcmd Mini: a small folder-aware drawer embedded in NOXfiles, around 10 lines high by default, with quick commands and a pop-out action.

Shared direction:

- Use a shared PTY/session backend long term.
- Make command input feel like a text editor: click to place cursor, drag/select text, type to replace selected text, Backspace/Delete remove selected text, and support normal word/cursor navigation.
- Command blocks should group command, working directory, output, duration, and exit status.
- Mini pop-out should eventually transfer the live session into NOXcmd Desktop, not just open a new shell in the same folder.

## Project Creation And GitHub

New project creation should be a shared NOX feature rather than a NOXcmd-only feature.

Possible shared crate/app layer:

- create project folder
- initialize local Git repo
- optional first commit
- optional GitHub repo creation
- templates later

Use `gh auth login` and `gh repo create` first instead of storing GitHub secrets ourselves. Native OAuth/device-flow can come later.

Potential entry points:

- NOXfiles: New Project action
- NOXcmd: command palette action
- Desktop/start menu: New Project action

## Image Conversion And Compression

Switcheroo and Curtail are useful reference points:

- Switcheroo-style conversion and resizing
- Curtail-style compression/optimization

This should be integrated into existing tools rather than becoming separate apps at first.

NOXfiles context menu:

- Convert Image
- Resize Image
- Compress Image
- Optimize for Web
- Strip Metadata
- Batch Convert/Compress/Resize for multi-select

NOXimage toolbar:

- Convert
- Resize
- Compress
- Save As
- Strip Metadata
- Compare original/compressed size

Implementation direction:

- Add a shared `crates/image-ops` later.
- Start with external tools where available: ImageMagick, oxipng, jpegoptim/mozjpeg, cwebp, scour/svgo.
- Replace with Rust libraries only where it improves reliability or packaging.

## NOXfiles TUI

Consider a terminal version of NOXfiles for SSH, recovery, and keyboard-first workflows.

It should be a focused TUI, not a second unrelated file manager.

Possible app:

- `apps/noxfiles-tui`

Likely stack:

- `ratatui`
- `crossterm`

Desired MVP:

- directory listing/navigation
- open file/folder
- copy/cut/paste
- rename
- mkdir/touch
- trash
- current-folder filter/search
- metadata/preview panel
- embedded NOXcmd line

Important prerequisite:

- Move file operation logic out of the GUI bridge and into shared crates so GUI NOXfiles and NOXfiles TUI cannot drift apart.
