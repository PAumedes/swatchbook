# Swatchbook — Architecture Summary

## Project Structure

```
swatchbook/
├── meson.build              ← top-level: project, deps, tooling, profiles
├── meson_options.txt        ← 'profile' combo (default | development)
├── Cargo.toml               ← crate manifest + gtk4/adw bindings
├── .gitignore               ← ignores generated config.rs & builds
├── build-aux/
│   └── cargo.sh             ← cargo↔meson build shim
├── data/
│   ├── com.example.Swatchbook.desktop.in
│   ├── com.example.Swatchbook.gschema.xml
│   └── meson.build
└── src/
    ├── main.rs
    ├── window.rs
    ├── window.blp
    ├── config.rs.in         ← Meson→Rust config template
    ├── swatchbook.gresource.xml
    └── meson.build
```

## Key Dependencies

| Crate / Library | Version Floor | Purpose |
|---|---|---|
| `gtk4` (`gtk` alias) | `v4_10` | Widget toolkit |
| `libadwaita` (`adw` alias) | `v1_4` | GNOME HIG components |
| `gettext-rs` | `0.7` | Runtime localisation |
| `glib-2.0` pkg-config | `2.74` | GLib core |
| `libadwaita-1` pkg-config | `1.4` | Unlocks `Adw.NavigationSplitView` |

## Why Four Files Were Added Beyond the Requested Tree

| File | Reason |
|---|---|
| `Cargo.toml` | Meson's `'rust'` language drives `cargo`, which needs a manifest |
| `build-aux/cargo.sh` | `custom_target` runs one program; can't express `cargo build && cp` inline |
| `src/config.rs.in` | Only clean way to inject install paths (`PKGDATADIR`, `LOCALEDIR`) into Rust |
| `src/swatchbook.gresource.xml` | Blueprint UI must be bundled as a GResource for `#[template(resource = …)]` |

## UI Layout

```
Adw.ApplicationWindow
└── Adw.NavigationSplitView  (collapses at <640sp via Adw.Breakpoint)
    ├── sidebar: Adw.NavigationPage "Markdown"
    │   └── Adw.ToolbarView
    │       ├── [top] Adw.HeaderBar  (New Canvas button, Main Menu)
    │       └── ScrolledWindow → Gtk.TextView  (Markdown editor)
    └── content: Adw.NavigationPage "Preview"
        └── Adw.ToolbarView
            ├── [top] Adw.HeaderBar  (Toggle Rendered Preview button)
            └── Adw.StatusPage
                └── Frame → Gtk.DrawingArea  (swatch canvas)
```

Light/Dark mode is handled automatically by Adwaita's header and toolbar widgets — no hardcoded colours in the layout.

## GObject Class Hierarchy

```
SwatchbookWindow
  extends  adw::ApplicationWindow
  extends  gtk::ApplicationWindow
  extends  gtk::Window
  extends  gtk::Widget
  implements  gio::ActionGroup, gio::ActionMap, gtk::Accessible,
              gtk::Buildable, gtk::ConstraintTarget, gtk::Native,
              gtk::Root, gtk::ShortcutManager
```

Subclassing uses `#[glib::object_subclass]` + `CompositeTemplate` from the `adw::subclass::prelude`. All state lives in `imp::SwatchbookWindow` (the `ObjectSubclass` inner type).

## Application Actions

| Action | Accel | Description |
|---|---|---|
| `app.quit` | `Ctrl+Q` | Terminate the process |
| `app.new-canvas` | `Ctrl+N` | Open a fresh window |
| `app.about` | — | Show `Adw.AboutWindow` |

The `--new-canvas` CLI flag is forwarded to the running primary instance via `connect_command_line`, which activates `app.new-canvas` in-process instead of spawning a duplicate.

## GSettings Schema Keys

| Key | Type | Default | Range |
|---|---|---|---|
| `window-width` | `i` | `960` | `360–32767` |
| `window-height` | `i` | `640` | `294–32767` |
| `is-maximized` | `b` | `false` | — |

Persistence: `restore_window_state()` reads on `constructed`, `save_window_state()` writes in `WindowImpl::close_request`.

## Build Profiles

| Profile | Rust target | Notes |
|---|---|---|
| `default` | `release` | LTO + single codegen unit + stripped |
| `development` | `debug` | Verbose cargo output, no stripping |

Set via `-Dprofile=development` at configure time.

## Known Caveats

- `gio::Settings::new(APP_ID)` requires the schema to be compiled into a recognised location. When running from the build tree (without `meson install`), set `GSETTINGS_SCHEMA_DIR=_build/data`.
- The `development` profile does not yet install under a separate `.Devel` application ID — both profiles share `com.example.Swatchbook`. Parallel-installability requires additional wiring in `data/meson.build`.
- There is no `po/` directory yet; gettext merging in `data/meson.build` will fail until at least an empty `po/LINGUAS` exists.
