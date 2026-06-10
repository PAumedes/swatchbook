# ----------------------------------------------------------------------------
# Blueprint → GTK Builder XML
#
# blueprint-compiler transpiles window.blp into window.ui inside the build
# directory. `output: '.'` is the documented batch-compile idiom and lets the
# GResource step below locate the generated .ui via its build-dir source root.
# ----------------------------------------------------------------------------
blueprints = custom_target(
  'blueprints',
  input: files('window.blp'),
  output: '.',
  command: [
    blueprint_compiler,
    'batch-compile',
    '@OUTPUT@',
    '@CURRENT_SOURCE_DIR@',
    '@INPUT@',
  ],
)

# ----------------------------------------------------------------------------
# GResource bundle
#
# Bundles the compiled window.ui. `dependencies: blueprints` guarantees the
# .ui exists first; `source_dir` points the resource compiler at the build
# directory where Blueprint emitted it.
# ----------------------------------------------------------------------------
resources = gnome.compile_resources(
  'swatchbook',
  'swatchbook.gresource.xml',
  dependencies: blueprints,
  source_dir: meson.current_build_dir(),
  gresource_bundle: true,
  install: true,
  install_dir: pkgdatadir,
)

# ----------------------------------------------------------------------------
# Generated build configuration (config.rs)
#
# configure_file() substitutes install paths and identity into config.rs.in.
# The result is copied next to the other sources so plain `cargo build` (run
# from the wrapper) can `mod config;` it without custom include paths.
# ----------------------------------------------------------------------------
conf = configuration_data()
conf.set_quoted('APP_ID',          application_id)
conf.set_quoted('VERSION',         version)
conf.set_quoted('GETTEXT_PACKAGE', gettext_package)
conf.set_quoted('LOCALEDIR',       localedir)
conf.set_quoted('PKGDATADIR',      pkgdatadir)

configure_file(
  input: 'config.rs.in',
  output: 'config.rs',
  configuration: conf,
)

# Mirror the generated config.rs back into the source tree for Cargo.
run_command(
  'cp',
  meson.current_build_dir() / 'config.rs',
  meson.current_source_dir() / 'config.rs',
  check: true,
)

# ----------------------------------------------------------------------------
# Cargo build
#
# Drives `cargo build` through build-aux/cargo.sh, which copies the resulting
# binary to @OUTPUT@ and installs it into bindir. `depends: resources` ensures
# the GResource (and therefore the Blueprint UI) is ready before linking.
# ----------------------------------------------------------------------------
cargo_build = custom_target(
  'cargo-build',
  build_by_default: true,
  build_always_stale: true,
  output: meson.project_name(),
  console: true,
  install: true,
  install_dir: bindir,
  depends: resources,
  command: [
    cargo_script,
    meson.project_build_root(),
    meson.project_source_root(),
    '@OUTPUT@',
    rust_target,
    meson.project_name(),
  ],
)
