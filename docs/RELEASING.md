# Releasing Swatchbook

This document is the authoritative guide for cutting a release. It is written
for developers and AI assistants — every step is explicit and every invariant is
stated. Read it fully before running anything.

---

## How the pipeline works

```
local machine                          GitHub Actions
─────────────────────────────────────  ───────────────────────────────────
1. merge feature branch → main
2. bash build-aux/release.sh minor     triggered by push to main → ci.yml
     ├─ bumps version in 5 files          fmt check, clippy, meson build, tests
     ├─ commits "chore: release vX.Y.Z"
     ├─ tags vX.Y.Z                    triggered by tag push → release.yml
     ├─ git push origin main               meson build, assemble .deb
     └─ git push origin vX.Y.Z            create GitHub Release + attach .deb
3. cargo update → push Cargo.lock
```

The `.deb` is built by CI, not locally. The local `build-aux/incus-build.sh`
(or `incus-build build`) is for development iteration only.

---

## Files the release script touches

| File | What changes |
|---|---|
| `Cargo.toml` | `version = "X.Y.Z"` on the first occurrence |
| `meson.build` | `version: 'X.Y.Z'` (drives the VERSION constant shown in About) |
| `build-aux/control` | `Version: X.Y.Z` |
| `build-aux/changelog` | New Debian-format stanza prepended |
| `data/io.github.patricioaumedes.Swatchbook.metainfo.xml` | New `<release>` block prepended inside `<releases>` |

`Cargo.lock` is **not** touched by the script. Update it separately (see step 4).

---

## Choosing the bump type

| Type | When | Example |
|---|---|---|
| `patch` | Bug fixes, typos, no new features | `1.1.0 → 1.1.1` |
| `minor` | New features, backwards-compatible | `1.1.0 → 1.2.0` |
| `major` | Breaking changes, major milestones | `1.1.0 → 2.0.0` |

---

## Step-by-step

### 1. All changes must be on `main`

The release script ends with `git push origin main`. If you are on a feature
branch, it will push the wrong branch or fail.

```bash
git checkout main
git merge --ff-only <feature-branch>   # fast-forward; no merge commits
git log --oneline -5                   # verify the right commits are here
```

### 2. Pre-release checklist

```bash
git status          # must be completely clean — no staged or unstaged changes
git diff HEAD       # double-check
```

Run the tests inside the container:

```bash
make test
# or, if incus-build is installed:
incus-build build --clean   # full build + tests
```

**Metainfo invariant** — the release script prepends a new `<release>` block.
If `metainfo.xml` already contains a block for the version you are about to
release, the script will create a duplicate. Check before running:

```bash
grep '<release version=' data/io.github.patricioaumedes.Swatchbook.metainfo.xml
```

If the target version already appears (e.g., from a premature commit during
development), remove that block, commit, and proceed.

### 3. Run the release script

```bash
bash build-aux/release.sh patch    # or minor / major
# or via make:
make release-patch                 # make release-minor / make release-major
```

The script will prompt for a one-line changelog summary. Pass it with `-m` to
skip the prompt:

```bash
bash build-aux/release.sh minor \
  -m "New feature summary in plain English for the release notes"
```

When asked `Proceed? [y/N]`, review the printed summary and type `y`.

The script then:
1. Updates the five files listed above
2. Runs `git add` on exactly those five files
3. Commits `chore: release vX.Y.Z`
4. Creates annotated tag `vX.Y.Z`
5. Pushes `main` and the tag — **this is when CI fires**

### 4. Update `Cargo.lock`

The release script bumps `Cargo.toml` but not `Cargo.lock`. Update it so CI
uses the locked dependency graph and the committed lock file stays honest:

```bash
# Cargo is not installed on the host — run inside the build container:
incus exec swatchbook-builder -- bash -c "
  cd /root/swatchbook-check
  CARGO_HOME=/root/cargo-home CARGO_TARGET_DIR=/root/cargo-target \
  PATH=\$PATH:/root/.rustup-cargo/bin \
  cargo update -p swatchbook
"
incus file pull swatchbook-builder/root/swatchbook-check/Cargo.lock Cargo.lock

git add Cargo.lock
git commit -m "chore: update Cargo.lock for vX.Y.Z"
git push origin main
```

> If you have cargo on the host, run `cargo update -p swatchbook` directly.

### 5. Watch CI

```bash
make release-watch      # streams the live log; Ctrl-C is safe
# or
make release-status     # shows recent runs and published assets
```

The `.deb` appears at `https://github.com/PAumedes/swatchbook/releases/tag/vX.Y.Z`
within ~5 minutes of the tag push.

---

## What CI validates

### `ci.yml` — runs on every push to `main` and on PRs

1. `cargo fmt --check` — must pass (run `cargo fmt` locally first if unsure)
2. `cargo clippy -- -D warnings` — zero warnings allowed
3. `meson setup` + `meson compile`
4. `meson test` (runs all Rust unit tests via the meson test harness)

### `release.yml` — runs when a `v*.*.*` tag is pushed

1. Same build + test as CI
2. Assembles `swatchbook-X.Y.Z-amd64.deb`
3. Extracts release notes from `build-aux/changelog` (Debian format)
4. Creates a GitHub Release and attaches the `.deb`

---

## Recovering from a failed release

### CI fails after the tag is pushed

Fix the issue on `main`, push the fix, then re-push the tag:

```bash
git push origin main         # push the fix first

git tag -d vX.Y.Z            # delete local tag
git push origin :vX.Y.Z      # delete remote tag
git tag -a vX.Y.Z -m "Release vX.Y.Z: <original message>"
git push origin vX.Y.Z       # re-triggers release.yml
```

### You pushed the wrong version

Same as above: delete local + remote tag, create the correct tag, re-push.

### The release script aborted mid-run

Check which files were partially updated (`git diff`). The script is not
atomic — if it fails after editing files but before committing, reset:

```bash
git checkout -- Cargo.toml meson.build build-aux/control \
  build-aux/changelog \
  data/io.github.patricioaumedes.Swatchbook.metainfo.xml
```

Then fix the root cause and re-run.

---

## Known invariants — do not break these

- **GitHub username is `PAumedes`** (not `patricioaumedes`). All URLs in the
  manifest, metainfo, and Makefile use `PAumedes/swatchbook`. Do not change
  the casing.
- **Never pre-write `<release>` entries in metainfo.xml.** The release script
  owns that file's `<releases>` section. Adding a block manually during
  development creates duplicates.
- **The release script must run from the `main` branch.** It pushes to
  `origin main` unconditionally.
- **`cargo fmt` must be clean before tagging.** CI runs `cargo fmt --check`
  with `-D warnings`; a formatting failure blocks the GitHub Release.
- **`Cargo.lock` must be committed and up to date.** CI uses
  `actions/cache` keyed on `Cargo.lock`; a stale lock wastes cache hits and
  may cause subtle version skew.

---

## Flatpak (future Flathub submission)

When `incus-build` is installed, the following commands automate the Flatpak
release steps:

```bash
# 1. Regenerate cargo-sources.json from the current Cargo.lock
incus-build flatpak regen

# 2. Fetch the release tarball, compute sha256, update the manifest
incus-build flatpak bump 1.1.0

# 3. Validate metainfo, desktop file, and manifest syntax
incus-build flatpak validate

# 4. Prepare a Flathub PR (dry-run by default; add --push to open it)
incus-build flatpak submit 1.1.0
incus-build flatpak submit 1.1.0 --push
```

These commands read `incus-build.toml` in the project root for paths and
repo names. The Flathub submission repo is
`flathub/io.github.patricioaumedes.Swatchbook`.

---

## GPG signing (optional, local only)

CI does not sign packages. To sign a locally-built `.deb`:

```bash
# Create build-aux/signing.conf (git-ignored):
echo "GPG_KEY_ID=YOUR_KEY_ID" > build-aux/signing.conf

# Build with signing:
bash build-aux/incus-build.sh
# or, if incus-build is installed:
incus-build build --sign YOUR_KEY_ID
```

`debsigs` must be installed on the host (`sudo apt install debsigs`).
