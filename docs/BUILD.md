# Building Swatchbook

All compilation runs inside an [Incus](https://linuxcontainers.org/incus/) container. Nothing needs to be installed on your local machine except Incus itself.

---

## Prerequisites

- Incus installed and your user added to the `incus-admin` group
- `make` available on the host

---

## Quick build

```bash
make build
```

On first run this creates an Ubuntu 24.04 container named `swatchbook-builder`, installs all build dependencies inside it, compiles the app, and copies `swatchbook.deb` to the project root.

Subsequent runs reuse the existing container — startup takes a few seconds instead of minutes.

---

## Build targets

| Target | What it does |
|---|---|
| `make build` | Release build → `swatchbook.deb` |
| `make build-dev` | Debug build (no .deb, binary stays in container) |
| `make rebuild` | `make clean` + `make build` |
| `make test` | Meson + Cargo tests inside the container |
| `make lint` | `cargo clippy` + `cargo fmt --check` |
| `make fmt` | Auto-format Rust source with `cargo fmt` |

---

## Container management

```bash
make container-up      # start/create the container
make container-stop    # stop it (keeps state)
make container-delete  # delete it entirely (frees ~1 GB)
make container-shell   # open a bash shell inside it
make container-status  # show container state and network info
```

---

## Network setup (restricted Incus projects)

If your Incus project has restricted network management (common on shared hosts), the container won't get DHCP. You need to configure NAT on the host bridge and set a static IP inside the container manually.

### One-time host setup (requires root)

```bash
# Replace incusbr-1000 with your actual bridge name (check: ip link show)
sudo iptables-legacy -t nat -A POSTROUTING -s 10.100.100.0/24 -o <your-uplink> -j MASQUERADE
sudo iptables-legacy -A FORWARD -i incusbr-1000 -j ACCEPT
sudo iptables-legacy -A FORWARD -o incusbr-1000 -j ACCEPT
```

### Each session (applied automatically by incus-build.sh)

The build script re-applies the static IP each time the container restarts:

```bash
ip addr add 10.100.100.2/24 dev eth0
ip route add default via 10.100.100.1 dev eth0
echo 'nameserver 1.1.1.1' > /etc/resolv.conf
```

This is done automatically inside `build-aux/incus-build.sh` — you don't need to run it manually.

---

## Build dependencies installed in the container

| Package | Purpose |
|---|---|
| `build-essential` | GCC, make |
| `meson`, `ninja-build` | Build system |
| `cargo`, `rustc` | Rust compiler |
| `libgtk-4-dev` | GTK4 headers |
| `libadwaita-1-dev` | Libadwaita headers |
| `libpango1.0-dev` | Pango/Cairo text layout |
| `blueprint-compiler` | Compiles `.blp` UI files to XML |
| `gettext` | Translation tooling |
| `desktop-file-utils` | `desktop-file-validate` |
| `dpkg-dev` | `.deb` packaging tools |
| `libxml2-utils` | `xmllint` for GResource validation |
| `appstream` | `appstreamcli validate` |

---

## Running without installing

If you built a debug binary and want to test it without `dpkg -i`:

```bash
# Inside the container (make container-shell)
GSETTINGS_SCHEMA_DIR=/tmp/swatchbook/_build/data \
  /tmp/swatchbook/_build/src/swatchbook
```

The `GSETTINGS_SCHEMA_DIR` override is needed because the compiled schema isn't in a system location.
