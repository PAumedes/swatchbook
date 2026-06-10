# Swatchbook

A Markdown-powered style binder for GNOME. Write colour palettes, typography specs, and design tokens in plain Markdown; Swatchbook renders them as an interactive, shareable canvas.

Built with Rust, GTK4, Libadwaita, and Blueprint — looks and behaves like a first-party GNOME application.

## Features

- Split-pane interface: live Markdown editor alongside an interactive swatch canvas
- Responsive layout: collapses to single-column on narrow displays (phones, half-tiled windows)
- Persistent window state via GSettings
- Native Light/Dark mode — follows the system style automatically
- Multi-window support (`Ctrl+N` / right-click desktop action)
- Fully localised (gettext)

## Requirements

### Runtime

- GNOME runtime with GTK 4.10+ and Libadwaita 1.4+

### Build

| Tool | Minimum version |
|---|---|
| Rust / Cargo | 1.75 |
| Meson | 1.0 |
| blueprint-compiler | 0.12 |
| GLib / GIO | 2.74 |
| GTK4 development headers | 4.10 |
| Libadwaita development headers | 1.4 |

On a Fedora / GNOME OS system:

```bash
sudo dnf install meson rust cargo blueprint-compiler \
    gtk4-devel libadwaita-devel
```

On Ubuntu 24.04+:

```bash
sudo apt install meson cargo blueprint-compiler \
    libgtk-4-dev libadwaita-1-dev
```

## Building

```bash
# Configure (release build)
meson setup _build

# Configure (development build — debug symbols, verbose output)
meson setup _build -Dprofile=development

# Compile
meson compile -C _build

# Run validation tests (desktop file + GSettings schema)
meson test -C _build

# Install
meson install -C _build
```

### Running from the build tree (without installing)

GSettings needs to find the compiled schema:

```bash
GSETTINGS_SCHEMA_DIR=_build/data ./_build/src/swatchbook
```

## Project Layout

```
swatchbook/
├── build-aux/cargo.sh       ← Cargo↔Meson build bridge
├── data/                    ← desktop file, GSettings schema
├── src/
│   ├── main.rs              ← application bootstrap, actions, accels
│   ├── window.rs            ← SwatchbookWindow subclass, canvas, settings
│   ├── window.blp           ← Blueprint UI layout
│   ├── config.rs.in         ← build-time config template
│   └── swatchbook.gresource.xml
├── Cargo.toml
└── meson.build
```

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for a detailed breakdown of the design decisions, class hierarchy, and GSettings schema.

## Roadmap

See [docs/PHASES.md](docs/PHASES.md) for the phased implementation plan.

## License

Swatchbook is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
