# Releasing a new version

Swatchbook uses a fully automated release pipeline. You run one command locally; GitHub Actions builds the `.deb` and publishes it to the Releases page.

---

## How it works

```
make release-patch
       │
       ├── bumps version in Cargo.toml, meson.build, build-aux/control
       ├── prepends entry to build-aux/changelog
       ├── prepends <release> entry to data/…metainfo.xml
       ├── commits + tags vX.Y.Z
       ├── git push origin main + git push origin vX.Y.Z
       │
       └── GitHub Actions triggers (.github/workflows/release.yml)
                 │
                 ├── installs build deps on ubuntu-24.04
                 ├── meson setup + ninja compile (~4 min)
                 ├── assembles swatchbook-X.Y.Z-amd64.deb
                 └── creates GitHub Release with .deb attached
```

---

## Choosing the bump type

| Command | When to use | Example |
|---|---|---|
| `make release-patch` | Bug fixes, typos, minor tweaks | `0.2.0 → 0.2.1` |
| `make release-minor` | New features, backward-compatible | `0.2.0 → 0.3.0` |
| `make release-major` | Breaking changes, major milestones | `0.2.0 → 1.0.0` |

---

## Step-by-step

### 1. Make sure main is clean

```bash
git status          # should be clean
make test           # all tests pass
```

### 2. Run the release command

```bash
make release-patch   # or release-minor / release-major
```

The script will ask for a one-line changelog entry:

```
Release: 0.2.0 → 0.2.1

Enter a summary of changes for this release.
Summary: Fix colour parsing for shorthand hex values
```

Type `y` to confirm, and the script handles everything else.

### 3. Watch the CI build

```bash
make release-watch    # live log stream in your terminal
```

Or open [github.com/patricioaumedes/swatchbook/actions](https://github.com/patricioaumedes/swatchbook/actions) in a browser.

### 4. Verify the release

```bash
make release-status   # shows recent runs and published releases
```

Or visit [github.com/patricioaumedes/swatchbook/releases](https://github.com/patricioaumedes/swatchbook/releases).

---

## What gets updated automatically

| File | What changes |
|---|---|
| `Cargo.toml` | `version = "X.Y.Z"` |
| `meson.build` | `version: 'X.Y.Z'` (also sets the `VERSION` constant shown in the About dialog) |
| `build-aux/control` | `Version: X.Y.Z` |
| `build-aux/changelog` | New Debian-format entry prepended |
| `data/…metainfo.xml` | New `<release>` block prepended inside `<releases>` |

---

## If CI fails

1. Check the run log: `make release-watch` or the Actions tab on GitHub
2. Fix the issue on `main` and push: `git push origin main`
3. To re-trigger the release build, delete and re-push the tag:

```bash
git tag -d vX.Y.Z
git push origin :vX.Y.Z
git tag -a vX.Y.Z -m "Release vX.Y.Z: <message>"
git push origin vX.Y.Z
```

---

## GPG signing (optional)

If you want to sign the `.deb` locally before distributing it outside GitHub:

```bash
# Create build-aux/signing.conf with:
GPG_KEY_ID=YOUR_KEY_ID

# Then build manually:
bash build-aux/incus-build.sh
```

The CI build on GitHub does not sign packages (no private key in CI).
