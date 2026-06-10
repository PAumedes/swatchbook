project(
  'swatchbook',
  'rust',
  version: '0.3.1',
  license: 'GPL-3.0-or-later',
  meson_version: '>= 1.0.0',
  default_options: [
    'warning_level=2',
  ],
)

# ----------------------------------------------------------------------------
# Imported Meson modules
# ----------------------------------------------------------------------------
i18n  = import('i18n')
gnome = import('gnome')

# ----------------------------------------------------------------------------
# Core identity
# ----------------------------------------------------------------------------
base_id         = 'io.github.swatchbook.Swatchbook'
gettext_package = meson.project_name()
version         = meson.project_version()

# ----------------------------------------------------------------------------
# Runtime dependencies
#
# These are the libraries our gtk4-rs / libadwaita-rs bindings link against.
# Meson resolves them via pkg-config so the build fails early (with a clear
# message) on a machine that is missing the development headers.
# ----------------------------------------------------------------------------
dependency('glib-2.0',       version: '>= 2.74')
dependency('gio-2.0',        version: '>= 2.74')
dependency('gtk4',           version: '>= 4.10')
dependency('libadwaita-1',   version: '>= 1.4')

# ----------------------------------------------------------------------------
# Build tooling
# ----------------------------------------------------------------------------
rustc                  = find_program('rustc',                  required: true)
cargo                  = find_program('cargo',                  required: true)
blueprint_compiler     = find_program('blueprint-compiler',     required: true)
glib_compile_resources = find_program('glib-compile-resources', required: true)
glib_compile_schemas   = find_program('glib-compile-schemas',   required: true)

# Cargo build wrapper (see build-aux/cargo.sh). Meson cannot chain a
# `cargo build` and a `cp` in a single custom_target command, so a tiny,
# well-understood shim does the work. This mirrors the official
# gtk-rust-template approach.
cargo_script = find_program('build-aux/cargo.sh', required: true)

# ----------------------------------------------------------------------------
# Profile handling: a "development" build gets debug symbols and the unmodified
# binary name, while the default build is an optimised release artifact.
# ----------------------------------------------------------------------------
application_id = base_id

if get_option('profile') == 'development'
  profile     = 'Devel'
  rust_target = 'debug'
  message('Building Swatchbook in DEVELOPMENT mode')
else
  profile     = ''
  rust_target = 'release'
  message('Building Swatchbook in RELEASE mode')
endif

# ----------------------------------------------------------------------------
# Installation paths
# ----------------------------------------------------------------------------
prefix     = get_option('prefix')
bindir     = prefix / get_option('bindir')
datadir    = prefix / get_option('datadir')
localedir  = prefix / get_option('localedir')
pkgdatadir = datadir / meson.project_name()

# ----------------------------------------------------------------------------
# Sub-directories
# ----------------------------------------------------------------------------
subdir('data')
subdir('po')
subdir('src')

# ----------------------------------------------------------------------------
# Post-install: refresh the icon cache, desktop database and the compiled
# GSettings schemas so the freshly installed app is immediately usable.
# ----------------------------------------------------------------------------
gnome.post_install(
  glib_compile_schemas: true,
  gtk_update_icon_cache: true,
  update_desktop_database: true,
)
