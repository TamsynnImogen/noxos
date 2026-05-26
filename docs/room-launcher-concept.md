# Room Launcher Concept

This is a spiritual successor to Packard Bell Navigator: a practical Linux app launcher organized as a lived-in house rather than a grid of icons.

The goal is not to clone the original assets or behavior exactly. The goal is to keep the strong idea: rooms are categories, and apps are represented by objects that belong naturally in those rooms.

## Core Metaphor

- The launcher is a house.
- Rooms are folders, categories, or work contexts.
- Apps are furniture, tools, boxes, books, devices, posters, shelf items, or other physical objects.
- Navigation should feel like moving through a place, not opening nested menus.

The room itself is the UI. Objects should not feel like icons pasted onto a wallpaper; desks, shelves, drawers, cabinets, walls, tables, and devices should define where launchable things live.

## Primary Views

### House View

An isometric or dollhouse overview shows the available rooms.

Possible rooms:

- Living Room: media, browser, chat, casual apps
- Study: documents, notes, office apps
- Workshop: development tools, terminal, editors
- Game Room: games, emulators, Steam, launchers
- Library: docs, PDFs, reference tools
- Utility Closet: settings and system utilities
- Gallery: photos, art, design apps

Clicking a room zooms into that room.

### Room View

A fixed illustrated room scene contains interactive objects. Objects launch apps, folders, URLs, or commands.

Possible object mappings:

- Browser: globe, magazine, window, modem, desk computer
- Terminal: CRT, black console screen, toolbox
- File manager: filing cabinet, cardboard box, desk drawer
- Music: stereo stack, vinyl shelf, headphones
- Games: cartridges, board games, arcade cabinet, boxed software
- Settings: fuse box, wall panel, remote, wrench
- Trash: bin, shredder, recycling box
- Subcategories: shelves, drawers, cupboards, notice boards

The software-room shelf pattern is especially strong: apps can appear as box art, books, cartridges, or discs arranged on shelves, with subcategories as drawers or side tabs.

## MVP

- Fullscreen launcher window.
- House overview with clickable rooms.
- Room background image.
- Positioned hotspots/items in each room.
- Click item to launch a configured command.
- Hover item to show a label and short description.
- Bottom status bar in the style of old Navigator screens.
- Top toolbar for home, room map, search, settings, and exit.
- Static JSON or TOML configuration for rooms and items.

## Second Pass

- Edit mode.
- Drag items onto shelves, desks, walls, or floors.
- Add apps from Linux `.desktop` files.
- Choose object type for each app.
- Rename, move, hide, or delete items.
- Per-room themes.
- Ambient sounds and small object animations.
- Search overlay styled as index cards, a catalog, or a Rolodex.
- Optional startup/session mode so it can behave like a desktop shell.

## Technical Direction

This should fit the existing `sysApps` direction:

- Linux-first behavior.
- Rust for app launching, `.desktop` parsing, configuration, and system integration.
- Qt/QML for the interactive visual UI.
- Shared app-discovery or launch services can later support other sysApps components.

The implementation should start as a launcher app, not a full desktop shell. Replacing the desktop/session can be explored only after launching, room management, and editing are solid.

## Data Model Sketch

```json
{
  "rooms": [
    {
      "id": "games",
      "name": "Games Room",
      "background": "assets/rooms/games.png",
      "items": [
        {
          "id": "steam",
          "label": "Steam",
          "description": "Open the game library.",
          "object": "console",
          "x": 420,
          "y": 310,
          "command": "steam"
        }
      ]
    }
  ]
}
```

## Design Constraints

- Keep it useful first: it should launch real apps quickly.
- Keep the visuals consistent: prefer one coherent illustrated style over mixed asset sources.
- Avoid using original Packard Bell assets unless the project is strictly private archival work.
- Make categories legible without turning everything into visible text labels.
- Keep object hit areas generous and predictable.
- The launcher should be charming, but not slow or confusing for daily use.

## Open Questions

- Should the first version use generated room art, hand-authored QML scenes, or simple placeholder backgrounds?
- Should each room use fixed object slots first, before free dragging is implemented?
- Should this be a separate `apps/room-launcher` app or eventually become part of `apps/desktop`?
- Should app discovery come from `.desktop` files only, or also user-defined shell commands and URLs from day one?
