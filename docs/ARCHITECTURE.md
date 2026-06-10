# Swatchbook — Architecture

## Project structure

```
swatchbook/
├── src/
│   ├── main.rs              ← process bootstrap, GActions, keyboard shortcuts
│   ├── window.rs            ← SwatchbookWindow subclass, editor, canvas, file I/O
│   ├── window.blp           ← Blueprint UI layout
│   ├── parser.rs            ← Markdown → Document (pulldown-cmark)
│   ├── token.rs             ← colour value extraction (#hex, rgb(), named)
│   ├── renderer.rs          ← swatch layout engine + Cairo/Pango drawing
│   ├── document.rs          ← file I/O, auto-save, crash recovery
│   ├── lib.rs               ← crate root (exposes modules for integration tests)
│   ├── config.rs.in         ← Meson→Rust build-time constants
│   └── swatchbook.gresource.xml
├── data/
│   ├── io.github.patricioaumedes.Swatchbook.desktop.in
│   ├── io.github.patricioaumedes.Swatchbook.gschema.xml
│   ├── io.github.patricioaumedes.Swatchbook.metainfo.xml
│   ├── icons/hicolor/scalable/apps/    ← SVG app icon
│   ├── icons/hicolor/symbolic/apps/    ← monochrome symbolic icon
│   └── meson.build
├── po/
│   ├── es.po                ← Spanish translation
│   ├── POTFILES
│   ├── LINGUAS
│   └── meson.build
├── tests/
│   ├── parser_tests.rs
│   ├── renderer_tests.rs
│   └── document_tests.rs
├── build-aux/
│   ├── incus-build.sh       ← container build + .deb packaging
│   ├── release.sh           ← version bump + changelog + tag + push
│   ├── make-completion.bash ← bash/zsh tab-completion for make targets
│   ├── cargo.sh             ← Cargo↔Meson build bridge
│   ├── control              ← Debian package metadata
│   ├── changelog            ← Debian-format changelog
│   └── copyright
├── .github/workflows/
│   └── release.yml          ← CI: build .deb + publish GitHub Release on tag push
├── docs/
├── Makefile
├── Cargo.toml
└── meson.build
```

---

## Key dependencies

| Crate / Library | Version | Purpose |
|---|---|---|
| `gtk4` (`gtk` alias) | `v4_10` | Widget toolkit |
| `libadwaita` (`adw` alias) | `v1_4` | GNOME HIG components |
| `gettext-rs` | `0.7` | Runtime localisation |
| `pulldown-cmark` | `0.12` | Markdown parsing |
| `pangocairo` | `0.20` | Text layout in Cairo draw functions |

---

## Data flow

```
GtkTextBuffer (editor)
       │  changed signal (150 ms debounce)
       ▼
   parser::parse()          pulldown-cmark → Document { sections }
       │
       ▼
   token::extract_color()   "red" / "#4a90d9" / "rgb()" → ColorValue
       │
       ▼
   renderer::layout()       pure geometry → Vec<SwatchRect>
       │
       ▼
   renderer::render()       Cairo + Pango → pixels on DrawingArea
```

The parser, token, and renderer modules are pure functions with no GTK imports — they can be unit-tested without a display.

---

## UI layout

```
Adw.ApplicationWindow
└── Adw.NavigationSplitView  (collapses at <640sp via Adw.Breakpoint)
    ├── sidebar: Adw.NavigationPage "Markdown"
    │   └── Adw.ToolbarView
    │       ├── [top] Adw.HeaderBar  (menu button)
    │       └── ScrolledWindow → Gtk.TextView  (Markdown editor)
    └── content: Adw.NavigationPage "Preview"
        └── Adw.ToolbarView
            ├── [top] Adw.HeaderBar
            └── stack
                ├── Adw.StatusPage   (shown when no swatches parsed)
                └── Gtk.DrawingArea  (swatch canvas)
```

---

## GObject class hierarchy

```
SwatchbookWindow
  extends  adw::ApplicationWindow
  extends  gtk::ApplicationWindow
  extends  gtk::Window
  extends  gtk::Widget
```

Subclassing uses `#[glib::object_subclass]` + `CompositeTemplate`. All mutable state lives in `imp::SwatchbookWindow` behind `RefCell` (required because GObject can't hold `&mut` references across signal boundaries).

---

## Application actions

| Action | Scope | Shortcut | Description |
|---|---|---|---|
| `app.quit` | app | `Ctrl+Q` | Terminate the process |
| `app.new-canvas` | app | `Ctrl+N` | Open a fresh window |
| `app.about` | app | — | Show the about dialog |
| `win.open` | window | `Ctrl+O` | Open a `.md` file |
| `win.save` | window | `Ctrl+S` | Save current file |
| `win.save-as` | window | `Ctrl+Shift+S` | Save to a new path |

---

## GSettings schema

Schema ID: `io.github.patricioaumedes.Swatchbook`

| Key | Type | Default | Description |
|---|---|---|---|
| `window-width` | `i` | `960` | Saved window width |
| `window-height` | `i` | `640` | Saved window height |
| `is-maximized` | `b` | `false` | Saved maximised state |

State is restored in `constructed()` and saved in `close_request()`.

---

## Build system

Meson orchestrates the full build; Cargo handles Rust compilation only.

```
meson setup _build
    │
    ├── blueprint-compiler  →  window.ui  (bundled into .gresource)
    ├── glib-compile-resources  →  swatchbook.gresource
    ├── cargo build --release  →  swatchbook binary
    ├── msgfmt  →  es/LC_MESSAGES/swatchbook.mo
    ├── i18n.merge_file  →  .desktop, .metainfo.xml
    └── glib-compile-schemas  →  schema validation
```

The local build uses an Incus container (`make build`). The CI build runs on GitHub Actions (`ubuntu-24.04`) and is triggered by version tag pushes.

---

## Build profiles

| Profile | Rust target | Notes |
|---|---|---|
| `default` | `release` | LTO, single codegen unit, stripped binary |
| `development` | `debug` | No stripping, verbose output |

Set via `meson setup _build -Dprofile=development`.
