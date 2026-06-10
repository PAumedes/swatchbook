# Swatchbook

A Markdown-powered style binder for GNOME. Write colour palettes, typography specs, and design tokens in plain Markdown; Swatchbook renders them as an interactive, shareable swatch canvas.

Built with Rust, GTK4, Libadwaita, and Blueprint — looks and behaves like a first-party GNOME application.

![License](https://img.shields.io/github/license/PAumedes/swatchbook)
![Latest release](https://img.shields.io/github/v/release/PAumedes/swatchbook)

---

## Features

- Live Markdown-to-swatch preview with 150 ms debounce
- Supports `#rrggbb`, `#rgb`, `rgb()`, and CSS named colours
- Sections map to headed groups of swatches
- Open, save, and save-as with native file dialogs
- Auto-save every 30 seconds with crash recovery
- Dark mode — follows the system colour scheme automatically
- Responsive layout — collapses to single-column on narrow displays
- Spanish translation included

---

## Install

Download the latest `.deb` from the [Releases page](https://github.com/PAumedes/swatchbook/releases) and install it:

```bash
sudo dpkg -i swatchbook-<version>-amd64.deb
```

**Runtime requirements:** GTK 4.10+ and Libadwaita 1.4+ (included in Ubuntu 24.04+, Fedora 39+).

---

## Usage

Write your palette in the left-hand editor using this syntax:

```markdown
## Brand Colours

- **Primary** — `#4a90d9`
- **Accent** — `#e8a838`
- **Background** — `rgb(245, 245, 245)`
- **Danger** — `red`
```

The canvas on the right updates live as you type. Use `Ctrl+S` to save your binder as a plain `.md` file you can open in any editor.

### Keyboard shortcuts

| Action | Shortcut |
|---|---|
| New window | `Ctrl+N` |
| Open file | `Ctrl+O` |
| Save | `Ctrl+S` |
| Save as | `Ctrl+Shift+S` |
| Quit | `Ctrl+Q` |

---

## Building

All compilation happens inside an [Incus](https://linuxcontainers.org/incus/) container — no build dependencies need to be installed locally.

### Quick start

```bash
make build        # build .deb via Incus container
make install-deb  # install the built package
```

### All Makefile targets

```bash
make              # show help
make build        # build release .deb via Incus
make build-dev    # build debug binary inside container
make rebuild      # clean + full build
make test         # run Meson + Cargo tests inside container
make lint         # cargo clippy + fmt check
make fmt          # auto-format Rust source
make container-up     # start/create the build container
make container-stop   # stop the container
make container-shell  # open a shell inside the container
make release-patch    # bump patch version and publish
make release-minor    # bump minor version and publish
make release-major    # bump major version and publish
make release-watch    # stream the GitHub Actions CI log live
make release-status   # show recent CI runs and published releases
make changelog        # print the full changelog
make clean            # remove build tree inside container
make install-deb      # install swatchbook.deb locally
make uninstall        # uninstall the package
```

### Shell tab-completion

```bash
source build-aux/make-completion.bash
```

Add that line to your `~/.zshrc` or `~/.bashrc` for persistent completion.

### Container setup

The first `make build` creates and configures the container automatically. If your Incus project uses a restricted network, see [docs/BUILD.md](docs/BUILD.md) for the static IP workaround.

---

## Releasing a new version

Swatchbook uses a one-command release flow. When you push a version tag, GitHub Actions builds the `.deb` and publishes it to the Releases page automatically.

```bash
make release-patch   # 0.2.0 → 0.2.1  (bug fix)
make release-minor   # 0.2.0 → 0.3.0  (new feature)
make release-major   # 0.2.0 → 1.0.0  (milestone / breaking change)
```

Each command will:
1. Ask for a one-line changelog entry
2. Bump the version in `Cargo.toml`, `meson.build`, and `build-aux/control`
3. Prepend an entry to `build-aux/changelog`
4. Commit, tag, and push to GitHub
5. GitHub Actions builds the `.deb` in the cloud (~4 min)
6. The `.deb` is attached to a GitHub Release automatically

Watch the build:
```bash
make release-watch    # live log stream
make release-status   # summary of recent runs
```

See [docs/RELEASING.md](docs/RELEASING.md) for the full release process.

---

## Project layout

```
swatchbook/
├── src/
│   ├── main.rs              ← application bootstrap, actions, keyboard shortcuts
│   ├── window.rs            ← SwatchbookWindow subclass, editor, canvas, file I/O
│   ├── window.blp           ← Blueprint UI layout
│   ├── parser.rs            ← Markdown → Document (pulldown-cmark)
│   ├── token.rs             ← colour value extraction (#hex, rgb(), named)
│   ├── renderer.rs          ← swatch layout engine + Cairo drawing
│   ├── document.rs          ← file I/O, auto-save, crash recovery
│   ├── config.rs.in         ← build-time constants (app ID, version, paths)
│   └── swatchbook.gresource.xml
├── data/
│   ├── io.github.swatchbook.Swatchbook.desktop.in
│   ├── io.github.swatchbook.Swatchbook.gschema.xml
│   ├── io.github.swatchbook.Swatchbook.metainfo.xml
│   └── icons/
├── po/                      ← gettext translations (es)
├── tests/                   ← integration tests (parser, renderer, document)
├── build-aux/
│   ├── incus-build.sh       ← container build script
│   ├── release.sh           ← version bump + tag + push
│   ├── make-completion.bash ← shell tab-completion for make
│   ├── control              ← Debian package metadata
│   ├── changelog            ← Debian-format changelog
│   └── copyright
├── docs/                    ← architecture, build, and release docs
├── Makefile
├── Cargo.toml
└── meson.build
```

---

## Contributing

1. Fork the repo and create a branch
2. Make your changes (all builds go through `make build`)
3. Run `make test` and `make lint` before submitting
4. Open a pull request

Please report bugs at the [issue tracker](https://github.com/PAumedes/swatchbook/issues).

---

## License

Swatchbook is free software released under the [GNU General Public License v3.0](LICENSE).

© 2026 Patricio Aumedes
