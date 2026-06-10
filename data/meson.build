# ----------------------------------------------------------------------------
# Desktop launcher
# ----------------------------------------------------------------------------
desktop_file = i18n.merge_file(
  input: '@0@.desktop.in'.format(base_id),
  output: '@0@.desktop'.format(application_id),
  type: 'desktop',
  po_dir: meson.project_source_root() / 'po',
  install: true,
  install_dir: datadir / 'applications',
)

desktop_file_validate = find_program('desktop-file-validate', required: false)
if desktop_file_validate.found()
  test(
    'validate-desktop-file',
    desktop_file_validate,
    args: [desktop_file.full_path()],
    depends: desktop_file,
  )
endif

# ----------------------------------------------------------------------------
# AppStream metainfo
# ----------------------------------------------------------------------------
metainfo_file = i18n.merge_file(
  input: '@0@.metainfo.xml'.format(base_id),
  output: '@0@.metainfo.xml'.format(application_id),
  po_dir: meson.project_source_root() / 'po',
  install: true,
  install_dir: datadir / 'metainfo',
)

appstreamcli = find_program('appstreamcli', required: false)
if appstreamcli.found()
  test(
    'validate-metainfo',
    appstreamcli,
    args: ['validate', '--no-net', metainfo_file.full_path()],
    depends: metainfo_file,
  )
endif

# ----------------------------------------------------------------------------
# Screenshots (installed for local AppStream tooling; also referenced by the
# metainfo URL which points to the raw GitHub copy)
# ----------------------------------------------------------------------------
install_data(
  'screenshots/main.png',
  install_dir: datadir / 'metainfo' / 'screenshots' / application_id,
)

# ----------------------------------------------------------------------------
# Application icon
# ----------------------------------------------------------------------------
install_data(
  'icons/hicolor/scalable/apps/@0@.svg'.format(base_id),
  install_dir: datadir / 'icons' / 'hicolor' / 'scalable' / 'apps',
)

install_data(
  'icons/hicolor/symbolic/apps/@0@-symbolic.svg'.format(base_id),
  install_dir: datadir / 'icons' / 'hicolor' / 'symbolic' / 'apps',
)

install_data(
  'icons/hicolor/64x64/apps/@0@.png'.format(base_id),
  install_dir: datadir / 'icons' / 'hicolor' / '64x64' / 'apps',
)

install_data(
  'icons/hicolor/128x128/apps/@0@.png'.format(base_id),
  install_dir: datadir / 'icons' / 'hicolor' / '128x128' / 'apps',
)

# ----------------------------------------------------------------------------
# GSettings schema
# ----------------------------------------------------------------------------
install_data(
  '@0@.gschema.xml'.format(base_id),
  install_dir: datadir / 'glib-2.0' / 'schemas',
)

test(
  'validate-gschema',
  glib_compile_schemas,
  args: ['--strict', '--dry-run', meson.current_source_dir()],
)
