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
        unsafe { std::env::set_var("G_MESSAGES_DEBUG", "all") };
    }

    // -- Localization ------------------------------------------------------
    // Initialise gettext so `_()`-equivalent lookups in the resources and the
    // bundled translations resolve at runtime.
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
        .resource_base_path("/io/github/swatchbook/Swatchbook")
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
    app.connect_command_line(|app, command_line| {
        let options = command_line.options_dict();
        app.activate();

        if options
            .lookup::<bool>("new-canvas")
            .ok()
            .flatten()
            .unwrap_or(false)
        {
            app.activate_action("new-canvas", None);
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

    app.add_action_entries([quit, about, new_canvas]);
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

/// Presents the standard Adwaita about dialog.
fn show_about(app: &adw::Application) {
    let window = app.active_window();

    let about = adw::AboutWindow::builder()
        .application_name("Swatchbook")
        .application_icon(APP_ID)
        .developer_name("Patricio Aumedes")
        .developers(["Patricio Aumedes"])
        .version(VERSION)
        .comments("A Markdown-powered style binder for GNOME.")
        .website("https://github.com/PAumedes/swatchbook")
        .issue_url("https://github.com/PAumedes/swatchbook/issues")
        .license_type(gtk::License::Gpl30)
        .copyright("© 2026 Patricio Aumedes")
        .build();

    if let Some(window) = window {
        about.set_transient_for(Some(&window));
    }

    about.present();
}
