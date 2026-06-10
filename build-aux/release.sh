#!/usr/bin/env bash
#
# Release automation for Swatchbook.
#
# Usage:
#   release.sh patch [-m "message"]   # 0.1.0 → 0.1.1  (bug fix)
#   release.sh minor [-m "message"]   # 0.1.0 → 0.2.0  (new feature)
#   release.sh major [-m "message"]   # 0.1.0 → 1.0.0  (breaking / milestone)
#
# What it does (in order):
#   1. Bump the version in Cargo.toml, meson.build, build-aux/control
#   2. Prompt for / accept a changelog entry
#   3. Prepend a Debian-format entry to build-aux/changelog
#   4. Ask for confirmation, then run the Incus build
#   5. Create a git tag vX.Y.Z (if inside a git repo)
#
# The ONLY required edit before a release: pick patch / minor / major
# and optionally pass -m "summary of changes".

set -euo pipefail

# ── Colour helpers ───────────────────────────────────────────────────────────
BOLD=$'\033[1m'; RESET=$'\033[0m'; GREEN=$'\033[32m'; CYAN=$'\033[36m'; RED=$'\033[31m'; YELLOW=$'\033[33m'
info()    { printf "${CYAN}▶ %s${RESET}\n" "$*"; }
success() { printf "${GREEN}✔ %s${RESET}\n" "$*"; }
warn()    { printf "${YELLOW}! %s${RESET}\n" "$*"; }
die()     { printf "${RED}✖ %s${RESET}\n" "$*" >&2; exit 1; }

# ── Locate project root ──────────────────────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

CARGO_TOML="$PROJECT_DIR/Cargo.toml"
MESON_BUILD="$PROJECT_DIR/meson.build"
CONTROL="$PROJECT_DIR/build-aux/control"
CHANGELOG="$PROJECT_DIR/build-aux/changelog"

# ── Parse arguments ──────────────────────────────────────────────────────────
BUMP_TYPE="${1:-}"
COMMIT_MSG=""
shift || true

while [[ $# -gt 0 ]]; do
    case "$1" in
        -m|--message) COMMIT_MSG="$2"; shift 2 ;;
        *) die "Unknown argument: $1" ;;
    esac
done

case "$BUMP_TYPE" in
    patch|minor|major) ;;
    "")  die "Usage: release.sh patch|minor|major [-m \"message\"]" ;;
    *)   die "Unknown bump type '$BUMP_TYPE'. Use: patch, minor, or major" ;;
esac

# ── Read current version ─────────────────────────────────────────────────────
CURRENT=$(grep '^version' "$CARGO_TOML" | head -1 | grep -oP '[0-9]+\.[0-9]+\.[0-9]+')
[[ -n "$CURRENT" ]] || die "Could not read version from Cargo.toml"

IFS='.' read -r MAJOR MINOR PATCH <<< "$CURRENT"

case "$BUMP_TYPE" in
    patch) PATCH=$((PATCH + 1)) ;;
    minor) MINOR=$((MINOR + 1)); PATCH=0 ;;
    major) MAJOR=$((MAJOR + 1)); MINOR=0; PATCH=0 ;;
esac

NEW_VERSION="$MAJOR.$MINOR.$PATCH"

printf "\n${BOLD}Release: %s → %s${RESET}\n\n" "$CURRENT" "$NEW_VERSION"

# ── Changelog entry ──────────────────────────────────────────────────────────
if [[ -z "$COMMIT_MSG" ]]; then
    printf "${BOLD}Enter a summary of changes for this release.${RESET}\n"
    printf "(One line, or press Enter to open \$EDITOR)\n\n"
    read -rp "Summary: " COMMIT_MSG

    if [[ -z "$COMMIT_MSG" ]]; then
        # Open a temp file in $EDITOR
        TMPFILE=$(mktemp /tmp/swatchbook-changelog-XXXXX.txt)
        printf "# Enter your changelog entry for %s below.\n# Lines starting with # are ignored.\n\n" "$NEW_VERSION" > "$TMPFILE"
        "${EDITOR:-nano}" "$TMPFILE"
        COMMIT_MSG=$(grep -v '^#' "$TMPFILE" | sed '/^[[:space:]]*$/d' | head -1)
        rm -f "$TMPFILE"
    fi
fi

[[ -n "$COMMIT_MSG" ]] || die "Changelog message cannot be empty."

# ── Preview and confirm ──────────────────────────────────────────────────────
DATESTAMP=$(date -R)
AUTHOR="The Swatchbook Authors <authors@example.com>"

printf "\n${BOLD}Changes to be made:${RESET}\n"
printf "  Version:   %s → %s (%s bump)\n" "$CURRENT" "$NEW_VERSION" "$BUMP_TYPE"
printf "  Changelog: %s\n" "$COMMIT_MSG"
printf "  Tag:       v%s\n\n" "$NEW_VERSION"

read -rp "Proceed? [y/N] " CONFIRM
[[ "${CONFIRM,,}" == "y" ]] || { warn "Aborted."; exit 0; }

# ── Update version in files ──────────────────────────────────────────────────
info "Updating Cargo.toml"
sed -i "0,/^version = \"$CURRENT\"/ s/^version = \"$CURRENT\"/version = \"$NEW_VERSION\"/" "$CARGO_TOML"

info "Updating meson.build"
sed -i "s/version: '$CURRENT'/version: '$NEW_VERSION'/" "$MESON_BUILD"

info "Updating build-aux/control"
sed -i "s/^Version: .*/Version: $NEW_VERSION/" "$CONTROL"

# ── Prepend changelog entry ──────────────────────────────────────────────────
info "Updating build-aux/changelog"
NEW_ENTRY="swatchbook ($NEW_VERSION) 26.04; urgency=medium

  * $COMMIT_MSG

 -- $AUTHOR  $DATESTAMP
"
# Prepend by writing new entry + existing content
EXISTING=$(cat "$CHANGELOG")
printf "%s\n%s" "$NEW_ENTRY" "$EXISTING" > "$CHANGELOG"

success "Version files updated"

# ── Build ────────────────────────────────────────────────────────────────────
info "Running Incus build for v$NEW_VERSION..."
bash "$SCRIPT_DIR/incus-build.sh"

# ── Git tag ──────────────────────────────────────────────────────────────────
if git -C "$PROJECT_DIR" rev-parse --git-dir &>/dev/null; then
    info "Staging version files"
    git -C "$PROJECT_DIR" add \
        "$CARGO_TOML" "$MESON_BUILD" "$CONTROL" "$CHANGELOG"

    git -C "$PROJECT_DIR" commit -m "chore: release v$NEW_VERSION"
    git -C "$PROJECT_DIR" tag -a "v$NEW_VERSION" -m "Release v$NEW_VERSION: $COMMIT_MSG"
    success "Git commit and tag v$NEW_VERSION created"
    printf "\n  Push with: ${CYAN}git push && git push --tags${RESET}\n"
else
    warn "Not a git repository — skipping commit and tag"
fi

printf "\n${BOLD}${GREEN}Release v%s complete!${RESET}\n" "$NEW_VERSION"
printf "Package: %s/swatchbook.deb\n\n" "$PROJECT_DIR"
