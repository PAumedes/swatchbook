#!/usr/bin/env bash
#
# Tool to build Swatchbook inside an Incus container.
# This ensures a clean build environment without installing dependencies locally.

set -e

CONTAINER_NAME="swatchbook-builder"
IMAGE="images:ubuntu/26.04"
PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Automatically load signing configuration if the file exists
if [ -f "$PROJECT_DIR/build-aux/signing.conf" ]; then
    source "$PROJECT_DIR/build-aux/signing.conf"
fi

GPG_KEY_ID="${GPG_KEY_ID:-}" # Set this env var to sign the package

echo "── Checking Incus container status ──"
if ! incus list "$CONTAINER_NAME" --format csv | grep -q "^$CONTAINER_NAME,"; then
    echo "Creating container '$CONTAINER_NAME' using $IMAGE..."
    incus launch "$IMAGE" "$CONTAINER_NAME"
    sleep 5

    echo "Configuring network..."
    # The restricted user project bridge needs a static IP assigned manually
    # because DHCP may not work until the host admin configures NAT.
    incus exec "$CONTAINER_NAME" -- bash -c "
        ip addr add 10.100.100.2/24 dev eth0 2>/dev/null || true
        ip route add default via 10.100.100.1 dev eth0 2>/dev/null || true
        echo 'nameserver 1.1.1.1' > /etc/resolv.conf
    "

    echo "Installing build dependencies..."
    incus exec "$CONTAINER_NAME" -- apt-get update -q
    incus exec "$CONTAINER_NAME" -- apt-get install -y \
        build-essential meson ninja-build cargo rustc \
        libgtk-4-dev libadwaita-1-dev blueprint-compiler \
        libpango1.0-dev \
        gettext desktop-file-utils dpkg-dev libxml2-utils
fi

# Ensure the container is running
if [[ "$(incus list "$CONTAINER_NAME" --format csv -c s)" != "RUNNING" ]]; then
    incus start "$CONTAINER_NAME"
    sleep 3
fi

# Re-apply static network config (lost on restart)
incus exec "$CONTAINER_NAME" -- bash -c "
    ip addr show eth0 | grep -q '10.100.100.2' || {
        ip addr add 10.100.100.2/24 dev eth0
        ip route add default via 10.100.100.1 dev eth0
        echo 'nameserver 1.1.1.1' > /etc/resolv.conf
    }
" 2>/dev/null || true

echo "── Mounting project source ──"
if ! incus config device show "$CONTAINER_NAME" | grep -q "path: /src"; then
    incus config device add "$CONTAINER_NAME" project-src disk \
        source="$PROJECT_DIR" path=/src
fi

echo "── Running Build ──"
incus exec "$CONTAINER_NAME" -- bash -c "
    # Work in /tmp — the /src mount is read-only for root in a restricted project
    rm -rf /tmp/swatchbook
    cp -r /src /tmp/swatchbook

    cd /tmp/swatchbook

    # 1. Configure + compile
    rm -rf _build
    meson setup _build --prefix=/usr
    meson compile -C _build

    # 2. Stage installation
    PKG_ROOT=/tmp/swatchbook-pkg
    rm -rf \"\$PKG_ROOT\"
    DESTDIR=\"\$PKG_ROOT\" meson install -C _build

    # 3. Debian metadata
    mkdir -p \"\$PKG_ROOT/DEBIAN\"
    cp /tmp/swatchbook/build-aux/control \"\$PKG_ROOT/DEBIAN/\"
    cp /tmp/swatchbook/build-aux/postinst \"\$PKG_ROOT/DEBIAN/\"
    cp /tmp/swatchbook/build-aux/postrm \"\$PKG_ROOT/DEBIAN/\"
    chmod 0755 \"\$PKG_ROOT/DEBIAN/postinst\" \"\$PKG_ROOT/DEBIAN/postrm\"

    # 4. Documentation
    DOC_DIR=\"\$PKG_ROOT/usr/share/doc/swatchbook\"
    mkdir -p \"\$DOC_DIR\"
    cp /tmp/swatchbook/build-aux/copyright \"\$DOC_DIR/\"
    cp /tmp/swatchbook/build-aux/changelog \"\$DOC_DIR/\"
    gzip -n -9 \"\$DOC_DIR/changelog\"

    # 5. Build .deb into /tmp (writable)
    dpkg-deb --build \"\$PKG_ROOT\" /tmp/swatchbook.deb
    echo 'Build complete.'
"

# Pull .deb out of the container onto the host
echo "── Retrieving package ──"
incus file pull "$CONTAINER_NAME/tmp/swatchbook.deb" "$PROJECT_DIR/swatchbook.deb"

if [ -n "$GPG_KEY_ID" ]; then
    if ! command -v debsigs >/dev/null 2>&1; then
        echo "Error: 'debsigs' is not installed on your host system."
        echo "Please install it with: sudo apt install debsigs"
        exit 1
    fi
    echo "── Signing package with key $GPG_KEY_ID ──"
    debsigs --sign=origin -k "$GPG_KEY_ID" "$PROJECT_DIR/swatchbook.deb"
    echo "── Done! Signed package: $PROJECT_DIR/swatchbook.deb ──"
else
    echo "── Done! Unsigned package: $PROJECT_DIR/swatchbook.deb ──"
fi
