# Swatchbook — Implementation Phases

Each phase delivers a shippable increment. Phases build on each other.

---

## Phase 0 — Buildable Shell ✅

- [x] Meson build system with profile support (`default` / `development`)
- [x] Cargo wired through Meson via `build-aux/cargo.sh`
- [x] Blueprint UI compiled and bundled as a GResource
- [x] `SwatchbookWindow` composite-template subclass
- [x] `Adw.NavigationSplitView` with editor sidebar and preview pane
- [x] `Adw.Breakpoint` collapse at 640sp
- [x] GSettings schema with window geometry persistence
- [x] Application actions: `app.quit`, `app.new-canvas`, `app.about`
- [x] Desktop file with `NewCanvas` jump-list action
- [x] `--new-canvas` CLI flag forwarded to the running primary instance

---

## Phase 1 — Markdown Parsing & Live Preview ✅

- [x] `pulldown-cmark` integrated for Markdown parsing
- [x] `src/parser.rs` — `Document { sections }` with `all_swatches()` iterator
- [x] `src/token.rs` — `ColorValue` enum, `extract_color()` for `#rrggbb`, `#rgb`, `rgb()`, CSS named colours
- [x] `GtkTextBuffer::changed` signal with 150 ms debounce via `glib::timeout_add_local`
- [x] Parsed document passed to `DrawingArea` via `RefCell` on the window `imp`
- [x] `canvas.queue_draw()` triggered after each parse

---

## Phase 2 — Canvas Renderer ✅

- [x] `src/renderer.rs` — pure layout engine, no GTK imports
- [x] `layout(count, width) -> Vec<SwatchRect>` — unit-testable geometry
- [x] Cairo rounded rectangles for colour fills (8px radius)
- [x] Pango labels below each swatch (11pt bold name, 10pt mono hex)
- [x] Max 5 columns, 40×40px swatches, responsive reflow on resize
- [x] Dark mode — detects `Adw.StyleManager` colour scheme, adapts label contrast
- [x] `Adw.StatusPage` shown when no swatches are parsed yet

---

## Phase 3 — File I/O ✅

- [x] `src/document.rs` — holds Markdown content, `Option<PathBuf>`, `is_modified` flag
- [x] Window title: `filename — Swatchbook` / `• Untitled — Swatchbook` when modified
- [x] `win.open` (`Ctrl+O`) — `Gtk.FileDialog` filtered to `*.md`
- [x] `win.save` (`Ctrl+S`) — in-place write; falls through to Save As if no path
- [x] `win.save-as` (`Ctrl+Shift+S`) — `Gtk.FileDialog::save`
- [x] Auto-save every 30 seconds to `$XDG_DATA_HOME/swatchbook/autosave.md`
- [x] Crash recovery via sentinel file — restores auto-save on next launch

---

## Phase 4 — Export ✅

- [x] PNG export — render canvas to off-screen `cairo::ImageSurface` at 2× DPI (`win.export-png`, `Ctrl+Shift+E`)
- [x] SVG export — render to `cairo::SvgSurface` (`win.export-svg`)
- [x] CSS variables — generate `--color-{slug}: #{hex};` block, copy to clipboard (`win.copy-css`, `Ctrl+Shift+C`)
- [x] `Adw.ToastOverlay` notification on export/copy success

---

## Phase 5 — Polish & Distribution ⏳

- [x] Flatpak manifest targeting the GNOME runtime (`io.github.patricioaumedes.Swatchbook.json` + `make flatpak`)
- [x] CI: `cargo clippy`, `cargo fmt --check`, `meson test` on every PR (`.github/workflows/ci.yml`)
- [ ] Accessibility audit with Accerciser
- [x] Keyboard navigation for the canvas (arrow keys move focus ring, `Enter` copies hex)
- [x] Additional translations (fr, de stubs in `po/`)
- [ ] GNOME Circle submission checklist

---

## Dependency map

```
Phase 0 (shell)
    └── Phase 1 (Markdown + live preview)
            └── Phase 2 (canvas renderer)
                    ├── Phase 3 (file I/O)    ← runs in parallel with Phase 2
                    └── Phase 4 (export)      ← depends on Phase 2
                            └── Phase 5 (polish + distribution)
```
