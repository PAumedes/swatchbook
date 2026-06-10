//! Swatchbook — a Markdown-powered style binder for GNOME.
//!
//! `main.rs` owns process bootstrap: it wires up gettext, registers the
//! compiled GResource bundle, constructs the `Adw.Application`, installs the
//! application-scoped `GAction`s, and hands control to the GLib main loop.

mod config;
mod document;
mod parser;
mod renderer;
mod token;
mod window;

use adw::prelude::*;
use gtk::{gio, glib};

use crate::config::{APP_ID, GETTEXT_PACKAGE, LOCALEDIR, PKGDATADIR, VERSION};
use crate::window::SwatchbookWindow;

fn main() -> glib::ExitCode {
    // -- Logging -----------------------------------------------------------
    // GLib structured logging goes to the journal when running under systemd
    // and to stderr otherwise. SWATCHBOOK_LOG=1 enables debug-level output.
    if std::env::var("SWATCHBOOK_LOG").is_ok() {
        std::env::set_var("G_MESSAGES_DEBUG", "all");
    }

    // -- Localization ------------------------------------------------------
    // Apply any stored language override *before* gettext is initialised so
    // the LANGUAGE env var is already set when setlocale() reads it.
    apply_language_preference();

    gettextrs::setlocale(gettextrs::LocaleCategory::LcAll, "");
    gettextrs::bindtextdomain(GETTEXT_PACKAGE, LOCALEDIR)
        .expect("Unable to bind the gettext text domain");
    gettextrs::bind_textdomain_codeset(GETTEXT_PACKAGE, "UTF-8")
        .expect("Unable to set the gettext codeset to UTF-8");
    gettextrs::textdomain(GETTEXT_PACKAGE)
        .expect("Unable to switch to the gettext text domain");

    glib::set_application_name("Swatchbook");

    // -- Resources ---------------------------------------------------------
    // Load and register the GResource bundle that contains the compiled
    // Blueprint UI (window.ui) before any template is instantiated.
    let resource_path = format!("{PKGDATADIR}/swatchbook.gresource");
    let resources = gio::Resource::load(&resource_path)
        .expect("Failed to load the compiled GResource bundle");
    gio::resources_register(&resources);

    // -- Application -------------------------------------------------------
    let app = adw::Application::builder()
        .application_id(APP_ID)
        .resource_base_path("/io/github/patricioaumedes/Swatchbook")
        .flags(gio::ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();

    // `--new-canvas` powers the desktop file's "New Canvas" jump-list action.
    app.add_main_option(
        "new-canvas",
        glib::Char::from(b'n'),
        glib::OptionFlags::NONE,
        glib::OptionArg::None,
        "Open a fresh canvas window",
        None,
    );

    app.connect_startup(|app| {
        setup_gactions(app);
        setup_accels(app);
    });

    app.connect_activate(build_window);

    // Bridge the CLI option onto the in-process `app.new-canvas` action so a
    // second `swatchbook --new-canvas` invocation is forwarded to the running
    // primary instance instead of spawning a duplicate process.
    //
    // When `--new-canvas` is set we skip `activate()` and go straight to the
    // action; otherwise we'd get two windows on a cold start (activate opens
    // one, the action opens another).
    app.connect_command_line(|app, command_line| {
        let options = command_line.options_dict();
        let new_canvas = options
            .lookup::<bool>("new-canvas")
            .ok()
            .flatten()
            .unwrap_or(false);

        if new_canvas {
            app.activate_action("new-canvas", None);
        } else {
            app.activate();
        }

        glib::ExitCode::SUCCESS.value()
    });

    app.run()
}

/// Registers the application-scoped actions referenced by the menu and the
/// desktop launcher.
fn setup_gactions(app: &adw::Application) {
    let quit = gio::ActionEntry::builder("quit")
        .activate(|app: &adw::Application, _, _| app.quit())
        .build();

    let about = gio::ActionEntry::builder("about")
        .activate(|app: &adw::Application, _, _| show_about(app))
        .build();

    let new_canvas = gio::ActionEntry::builder("new-canvas")
        .activate(|app: &adw::Application, _, _| build_window(app))
        .build();

    let preferences = gio::ActionEntry::builder("preferences")
        .activate(|app: &adw::Application, _, _| show_preferences(app))
        .build();

    app.add_action_entries([quit, about, new_canvas, preferences]);
}

/// Binds keyboard accelerators to their actions.
fn setup_accels(app: &adw::Application) {
    app.set_accels_for_action("app.quit",        &["<Control>q"]);
    app.set_accels_for_action("app.new-canvas",  &["<Control>n"]);
    app.set_accels_for_action("win.open",        &["<Control>o"]);
    app.set_accels_for_action("win.save",        &["<Control>s"]);
    app.set_accels_for_action("win.save-as",     &["<Control><Shift>s"]);
    app.set_accels_for_action("win.export-png",  &["<Control><Shift>e"]);
    app.set_accels_for_action("win.copy-css",    &["<Control><Shift>c"]);
}

/// Constructs a fresh main window and presents it. Used both for normal
/// activation and for the `new-canvas` action (multi-window support).
fn build_window(app: &adw::Application) {
    let window = SwatchbookWindow::new(app);
    window.present();
}

/// Reads the stored language preference and, if set, exports it as the
/// `LANGUAGE` environment variable so gettext picks it up during init.
///
/// This must be called before `setlocale()`.  We guard against missing schemas
/// (dev builds before `meson install`) so the app never panics cold.
fn apply_language_preference() {
    let Some(source) = gio::SettingsSchemaSource::default() else { return };
    if source.lookup(APP_ID, true).is_none() {
        return;
    }
    let settings = gio::Settings::new(APP_ID);
    let lang = settings.string("language");
    if !lang.is_empty() {
        std::env::set_var("LANGUAGE", lang.as_str());
    }
}

/// Opens the Preferences window, currently limited to language selection.
fn show_preferences(app: &adw::Application) {
    const CODES: &[&str] = &["", "en", "es", "fr", "de"];

    let settings = gio::Settings::new(APP_ID);
    let current = settings.string("language");
    let selected = CODES
        .iter()
        .position(|&c| c == current.as_str())
        .unwrap_or(0) as u32;

    let model = gtk::StringList::new(&[
        "System Default",
        "English",
        "Español",
        "Français",
        "Deutsch",
    ]);

    let row = adw::ComboRow::builder()
        .title("Language")
        .model(&model)
        .selected(selected)
        .build();

    row.connect_selected_notify(move |row| {
        let code = CODES.get(row.selected() as usize).copied().unwrap_or("");
        settings.set_string("language", code).ok();
    });

    let group = adw::PreferencesGroup::builder()
        .title("Language")
        .description("Restart Swatchbook to apply the new language")
        .build();
    group.add(&row);

    let page = adw::PreferencesPage::new();
    page.add(&group);

    let prefs_win = adw::PreferencesWindow::builder()
        .title("Preferences")
        .modal(true)
        .build();

    if let Some(w) = app.active_window() {
        prefs_win.set_transient_for(Some(&w));
    }

    prefs_win.add(&page);
    prefs_win.present();
}

/// Presents the standard Adwaita about dialog.
fn show_about(_app: &adw::Application) {
    let about = adw::AboutWindow::builder()
        .application_name("Swatchbook")
        .application_icon(APP_ID)
        .developer_name("Patricio Aumedes")
        .developers(["Patricio Aumedes"])
        .version(VERSION)
        .comments("A Markdown-powered style binder for GNOME.")
        .website("https://github.com/patricioaumedes/swatchbook")
        .issue_url("https://github.com/patricioaumedes/swatchbook/issues")
        .license_type(gtk::License::Gpl30)
        .copyright("© 2026 Patricio Aumedes")
        .build();

    about.present();
}
