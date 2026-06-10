# ----------------------------------------------------------------------------
# Desktop launcher
#
# The .desktop.in carries gettext-translatable keys; i18n.merge_file() folds in
# the translations from po/ and emits the final, validated .desktop file.
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
# GSettings schema
# ----------------------------------------------------------------------------
install_data(
  '@0@.gschema.xml'.format(base_id),
  install_dir: datadir / 'glib-2.0' / 'schemas',
)

# Validate the schema in-tree so a malformed key fails `meson test` early.
test(
  'validate-gschema',
  glib_compile_schemas,
  args: ['--strict', '--dry-run', meson.current_source_dir()],
)
