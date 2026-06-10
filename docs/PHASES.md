# Swatchbook — Implementation Phases

Each phase delivers a shippable increment. Phases build on each other; nothing is thrown away.

---

## Phase 0 — Buildable Shell (current state)

**Goal:** The project compiles, installs, and opens a window.

- [x] Meson build system with profile support (`default` / `development`)
- [x] Cargo workspace wired through `build-aux/cargo.sh`
- [x] Blueprint UI compiled and bundled as a GResource
- [x] `SwatchbookWindow` composite-template subclass
- [x] `Adw.NavigationSplitView` with editor sidebar and preview pane
- [x] `Adw.Breakpoint` collapse at 640 sp
- [x] GSettings schema with window geometry persistence
- [x] Application actions: `app.quit`, `app.new-canvas`, `app.about`
- [x] `Adw.AboutWindow` wired up
- [x] Placeholder swatch `Gtk.DrawingArea` with a Cairo draw function
- [x] Seeded editor with sample Markdown
- [x] Desktop file with `NewCanvas` jump-list action
- [x] `--new-canvas` CLI flag forwarded to the running primary instance

**Exit criteria:** `meson compile && meson install` succeeds; the app opens and the placeholder canvas renders five coloured swatches.

---

## Phase 1 — Markdown Parsing & Live Preview

**Goal:** Typing in the editor updates the canvas in real time.

### 1.1 Markdown parser integration

- [ ] Add `pulldown-cmark` to `Cargo.toml`
- [ ] Create `src/parser.rs` — wraps `pulldown-cmark` and exposes a `Document` type representing headings, paragraphs, code blocks, and colour token spans
- [ ] Unit-test the parser with a set of fixtures in `tests/parser.rs`

### 1.2 Document model

- [ ] Define `SwatchToken` enum: `NamedColour`, `HexColour`, `RgbColour`, `Variable`
- [ ] Write `src/token.rs` — regex-based extraction of colour values from inline code spans and fenced blocks
- [ ] Unit-test token extraction

### 1.3 Live update pipeline

- [ ] Connect `GtkTextBuffer::changed` signal in `window.rs`
- [ ] Debounce with `glib::timeout_add_local` (150 ms) to avoid re-parsing on every keystroke
- [ ] Pass parsed `Document` to the `DrawingArea` draw function via a `RefCell<Document>` on the window `imp`
- [ ] Call `canvas.queue_draw()` after each parse

**Exit criteria:** Typing `- **Red** — \`#E53935\`` in the editor causes a red swatch to appear in the canvas within 200 ms.

---

## Phase 2 — Canvas Renderer

**Goal:** The canvas renders a polished, accurate representation of the parsed document.

### 2.1 Swatch grid layout engine

- [ ] Create `src/renderer.rs` — pure layout logic, no GTK imports, takes a `Document` and a `(width, height)` and returns a list of positioned `SwatchRect` structs
- [ ] Support variable swatch counts per row with configurable column count
- [ ] Add gap, padding, and label metrics
- [ ] Unit-test layout arithmetic independently of Cairo

### 2.2 Cairo rendering

- [ ] Render `SwatchRect` list in the `DrawingArea` draw function
- [ ] Draw the colour fill with a rounded rectangle
- [ ] Draw the colour name label below each swatch using Pango layout
- [ ] Draw the hex value in a lighter secondary font
- [ ] Clip the canvas at its allocation boundaries

### 2.3 Typography and style

- [ ] Use `gtk::StyleContext` + Pango to respect the system monospace font for hex values
- [ ] Scale label font size relative to the swatch size
- [ ] Honour the system accent colour for the selection highlight

### 2.4 Dark mode correctness

- [ ] Detect `Adw.StyleManager::color-scheme` and flip the label contrast colour (dark text on light swatches, light text on dark swatches)
- [ ] Connect to `Adw.StyleManager::notify::dark` to queue a redraw on scheme change

**Exit criteria:** A document with ten named colours renders a clean grid; labels are legible in both Light and Dark mode; resizing the window reflows the grid.

---

## Phase 3 — File I/O

**Goal:** Users can open, save, and manage binder documents as `.md` files.

### 3.1 Document state

- [ ] Create `src/document.rs` — holds the raw Markdown string and the on-disk path (`Option<PathBuf>`)
- [ ] Track `is_modified` flag
- [ ] Set the window title to `filename — Swatchbook` (unsaved: `• Untitled — Swatchbook`)
- [ ] Block close with `Adw.MessageDialog` when `is_modified` is true

### 3.2 File actions

- [ ] `app.open` — `Gtk.FileDialog::open()` filtered to `*.md`
- [ ] `app.save` — writes in place; falls through to Save As if no path
- [ ] `app.save-as` — `Gtk.FileDialog::save()`
- [ ] Wire accelerators: `Ctrl+O`, `Ctrl+S`, `Ctrl+Shift+S`
- [ ] Add Recent Files support via `Gtk.RecentManager`

### 3.3 Auto-save

- [ ] Save to `$XDG_DATA_HOME/swatchbook/autosave.md` every 30 s while modified
- [ ] Restore from auto-save on next launch if a crash is detected (write a sentinel file cleared on clean exit)

**Exit criteria:** Create, save, close, and reopen a binder; content is identical. Crashing mid-edit recovers the document on next launch.

---

## Phase 4 — Export

**Goal:** Users can export their binder as a shareable artefact.

### 4.1 PNG export

- [ ] Render the canvas to an off-screen `cairo::ImageSurface` at 2× DPI
- [ ] Save via `cairo::ImageSurface::write_to_png`
- [ ] Wire to `app.export-png` with `Ctrl+Shift+E`
- [ ] Show a `Gtk.FileDialog::save` filtered to `*.png`

### 4.2 SVG export

- [ ] Render to `cairo::SvgSurface`
- [ ] Embed colour names and hex values as SVG `<title>` and `aria-label` attributes for accessibility

### 4.3 CSS variable export

- [ ] Generate a `--color-{slug}: #{hex};` block from the parsed tokens
- [ ] Copy to clipboard via `gdk::Clipboard` with a toast notification (`Adw.Toast`)
- [ ] Optionally save as `.css`

**Exit criteria:** Exporting a five-colour binder produces a pixel-accurate PNG and a valid CSS variables file.

---

## Phase 5 — Polish & Distribution

**Goal:** The app is ready for submission to GNOME Circle or Flathub.

### 5.1 Accessibility

- [ ] Audit all widgets with Accerciser
- [ ] Ensure every `DrawingArea` has an accessible name and description
- [ ] Add keyboard navigation for the canvas (focus ring, arrow-key swatch selection, `Enter` to copy hex value)

### 5.2 Localisation

- [ ] Set up `po/` directory with `LINGUAS`, `POTFILES`, and a `meson.build`
- [ ] Run `xgettext` / `blueprint-compiler --pot` to extract strings
- [ ] Add at minimum `es`, `fr`, `de` translations as stubs

### 5.3 Flatpak manifest

- [ ] Write `com.example.Swatchbook.json` (or `.yaml`) targeting the GNOME runtime
- [ ] Add `cargo` as a Flatpak build module using the offline source mirror pattern
- [ ] Validate with `flatpak-builder --sandbox`

### 5.4 CI

- [ ] GitHub Actions workflow: `meson setup`, `meson test`, `cargo test`, `cargo clippy`, `cargo fmt --check`
- [ ] Flatpak build job using `flatpak-github-actions`

### 5.5 Final HIG audit

- [ ] Verify every string is translatable
- [ ] Verify keyboard shortcuts match the GNOME HIG appendix
- [ ] Verify the app icon exists at 32, 48, 64, 128, 256, and 512 px
- [ ] Verify `DBusActivatable` works correctly (single-instance enforcement)

**Exit criteria:** `flatpak-builder` produces a working Flatpak; the app passes `desktop-file-validate` and `appstreamcli validate`.

---

## Dependency Map

```
Phase 0 (shell)
    └── Phase 1 (Markdown + live preview)
            └── Phase 2 (canvas renderer)
                    ├── Phase 3 (file I/O)    ← can start in parallel with Phase 2
                    └── Phase 4 (export)      ← depends on Phase 2
                            └── Phase 5 (polish + distribution)
```

Phase 3 can begin once Phase 1 is complete (it only needs the `Document` type, not the full renderer).
