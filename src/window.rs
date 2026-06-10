//! The main application window.
//!
//! `SwatchbookWindow` is a composite-template subclass of
//! `Adw.ApplicationWindow`. The visual layout lives in `window.blp`; this
//! module owns: GSettings-backed geometry persistence, the live Markdown→swatch
//! pipeline, file I/O actions, and auto-save / crash recovery.

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gio, glib};

use crate::document::Document;
use crate::parser;
use crate::renderer::{self, SwatchItem};

mod imp {
    use super::*;
    use gtk::CompositeTemplate;
    use std::cell::{OnceCell, RefCell};

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/io/github/swatchbook/Swatchbook/window.ui")]
    pub struct SwatchbookWindow {
        #[template_child]
        pub split_view: TemplateChild<adw::NavigationSplitView>,
        #[template_child]
        pub editor: TemplateChild<gtk::TextView>,
        #[template_child]
        pub canvas_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub status_page: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub canvas: TemplateChild<gtk::DrawingArea>,

        pub settings: OnceCell<gio::Settings>,

        /// Parsed swatches waiting to be drawn.
        pub swatches: RefCell<Vec<SwatchItem>>,

        /// Pending debounce timeout — cancelled on each new keystroke.
        pub debounce_id: RefCell<Option<glib::SourceId>>,

        /// Current document state (path, modified flag).
        pub document: RefCell<Document>,

        /// Auto-save periodic timer.
        pub autosave_id: RefCell<Option<glib::SourceId>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SwatchbookWindow {
        const NAME: &'static str = "SwatchbookWindow";
        type Type = super::SwatchbookWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SwatchbookWindow {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.setup_settings();
            obj.restore_window_state();
            obj.setup_canvas();
            obj.setup_editor();
            obj.setup_window_actions();
            obj.setup_autosave();
            obj.check_crash_recovery();
            obj.seed_document();
        }
    }

    impl WidgetImpl for SwatchbookWindow {}

    impl WindowImpl for SwatchbookWindow {
        fn close_request(&self) -> glib::Propagation {
            if let Err(e) = self.obj().save_window_state() {
                eprintln!("swatchbook: failed to save window state: {e}");
            }
            Document::clear_sentinel().ok();
            self.parent_close_request()
        }
    }

    impl ApplicationWindowImpl for SwatchbookWindow {}
    impl AdwApplicationWindowImpl for SwatchbookWindow {}
}

glib::wrapper! {
    pub struct SwatchbookWindow(ObjectSubclass<imp::SwatchbookWindow>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl SwatchbookWindow {
    pub fn new(app: &adw::Application) -> Self {
        glib::Object::builder().property("application", app).build()
    }

    // ── GSettings ─────────────────────────────────────────────────────────────

    fn setup_settings(&self) {
        let settings = gio::Settings::new(crate::config::APP_ID);
        self.imp().settings.set(settings).expect("settings set once");
    }

    fn settings(&self) -> &gio::Settings {
        self.imp().settings.get().expect("settings initialised")
    }

    fn restore_window_state(&self) {
        let s = self.settings();
        self.set_default_size(s.int("window-width"), s.int("window-height"));
        if s.boolean("is-maximized") {
            self.maximize();
        }
    }

    fn save_window_state(&self) -> Result<(), glib::BoolError> {
        let s = self.settings();
        let (w, h) = self.default_size();
        s.set_int("window-width", w)?;
        s.set_int("window-height", h)?;
        s.set_boolean("is-maximized", self.is_maximized())?;
        Ok(())
    }

    // ── Canvas ────────────────────────────────────────────────────────────────

    fn setup_canvas(&self) {
        let imp = self.imp();
        let swatches = imp.swatches.clone();
        let style_manager = adw::StyleManager::default();
        let style_manager2 = style_manager.clone();

        imp.canvas.set_draw_func(move |_area, cr, width, height| {
            let items = swatches.borrow();
            let dark = style_manager.is_dark();
            renderer::render(cr, &items, width as f64, height as f64, dark);
        });

        // Redraw on colour-scheme change.
        let canvas = imp.canvas.get();
        style_manager2.connect_dark_notify(move |_| {
            canvas.queue_draw();
        });
    }

    // ── Editor / live-preview pipeline ────────────────────────────────────────

    fn setup_editor(&self) {
        let window_weak = self.downgrade();
        let buffer = self.imp().editor.buffer();

        buffer.connect_changed(move |buf| {
            let Some(win) = window_weak.upgrade() else { return };
            let imp = win.imp();

            imp.document.borrow_mut().is_modified = true;

            // Cancel any pending debounce timer.
            if let Some(id) = imp.debounce_id.borrow_mut().take() {
                id.remove();
            }

            let (start, end) = buf.bounds();
            let text = buf.text(&start, &end, false).to_string();
            let window_weak2 = win.downgrade();

            // Re-parse 150 ms after the last keystroke.
            let id = glib::timeout_add_local(std::time::Duration::from_millis(150), move || {
                if let Some(win) = window_weak2.upgrade() {
                    win.reparse(&text);
                }
                glib::ControlFlow::Break
            });

            *imp.debounce_id.borrow_mut() = Some(id);
        });
    }

    /// Re-parse `markdown` and refresh the canvas draw function's data.
    fn reparse(&self, markdown: &str) {
        let parsed = parser::parse(markdown);
        let items: Vec<SwatchItem> = parsed
            .all_swatches()
            .map(|e| {
                let (r, g, b) = e.color.to_rgb();
                SwatchItem {
                    name: e.name.clone(),
                    hex: e.color.to_hex_string(),
                    r,
                    g,
                    b,
                }
            })
            .collect();

        let has_swatches = !items.is_empty();
        *self.imp().swatches.borrow_mut() = items;

        if has_swatches {
            self.imp().canvas_stack.set_visible_child(&*self.imp().canvas);
            self.imp().canvas.queue_draw();
        } else {
            self.imp().canvas_stack.set_visible_child(&*self.imp().status_page);
        }
    }

    // ── Window-scoped actions (open / save / save-as) ─────────────────────────

    fn setup_window_actions(&self) {
        let open = gio::ActionEntry::builder("open")
            .activate(|win: &Self, _, _| win.action_open())
            .build();
        let save = gio::ActionEntry::builder("save")
            .activate(|win: &Self, _, _| win.action_save())
            .build();
        let save_as = gio::ActionEntry::builder("save-as")
            .activate(|win: &Self, _, _| win.action_save_as())
            .build();

        self.add_action_entries([open, save, save_as]);
    }

    fn action_open(&self) {
        let filter = gtk::FileFilter::new();
        filter.set_name(Some("Markdown files"));
        filter.add_pattern("*.md");
        filter.add_pattern("*.markdown");

        let filters = gio::ListStore::new::<gtk::FileFilter>();
        filters.append(&filter);

        let dialog = gtk::FileDialog::builder()
            .title("Open Binder")
            .filters(&filters)
            .build();

        let win = self.clone();
        dialog.open(Some(self), gio::Cancellable::NONE, move |result| {
            if let Ok(file) = result {
                if let Some(path) = file.path() {
                    win.load_file(&path);
                }
            }
        });
    }

    fn load_file(&self, path: &std::path::Path) {
        match Document::from_file(path) {
            Ok(doc) => {
                let content = doc.content.clone();
                *self.imp().document.borrow_mut() = doc;
                self.imp().editor.buffer().set_text(&content);
                self.update_title();
            }
            Err(e) => eprintln!("swatchbook: failed to open file: {e}"),
        }
    }

    fn action_save(&self) {
        if self.imp().document.borrow().path.is_some() {
            self.do_save();
        } else {
            self.action_save_as();
        }
    }

    fn do_save(&self) {
        let buf = self.imp().editor.buffer();
        let (start, end) = buf.bounds();
        let text = buf.text(&start, &end, false).to_string();
        self.imp().document.borrow_mut().content = text;
        if let Err(e) = self.imp().document.borrow_mut().save() {
            eprintln!("swatchbook: save failed: {e}");
        }
        self.update_title();
    }

    fn action_save_as(&self) {
        let filter = gtk::FileFilter::new();
        filter.set_name(Some("Markdown files"));
        filter.add_pattern("*.md");

        let filters = gio::ListStore::new::<gtk::FileFilter>();
        filters.append(&filter);

        let dialog = gtk::FileDialog::builder()
            .title("Save Binder As")
            .filters(&filters)
            .initial_name("untitled.md")
            .build();

        let win = self.clone();
        dialog.save(Some(self), gio::Cancellable::NONE, move |result| {
            if let Ok(file) = result {
                if let Some(path) = file.path() {
                    let buf = win.imp().editor.buffer();
                    let (start, end) = buf.bounds();
                    let text = buf.text(&start, &end, false).to_string();
                    win.imp().document.borrow_mut().content = text;
                    if let Err(e) = win.imp().document.borrow_mut().save_to(path) {
                        eprintln!("swatchbook: save-as failed: {e}");
                    }
                    win.update_title();
                }
            }
        });
    }

    fn update_title(&self) {
        let title = self.imp().document.borrow().window_title();
        self.set_title(Some(&title));
    }

    // ── Auto-save ─────────────────────────────────────────────────────────────

    fn setup_autosave(&self) {
        Document::write_sentinel().ok();

        let win_weak = self.downgrade();
        let id = glib::timeout_add_local(std::time::Duration::from_secs(30), move || {
            let Some(win) = win_weak.upgrade() else {
                return glib::ControlFlow::Break;
            };
            let imp = win.imp();
            if imp.document.borrow().is_modified {
                let buf = imp.editor.buffer();
                let (start, end) = buf.bounds();
                let text = buf.text(&start, &end, false).to_string();
                imp.document.borrow_mut().content = text;
                imp.document.borrow().write_autosave().ok();
            }
            glib::ControlFlow::Continue
        });
        *self.imp().autosave_id.borrow_mut() = Some(id);
    }

    fn check_crash_recovery(&self) {
        if !Document::has_crash_recovery() {
            return;
        }
        if let Ok(recovered) = Document::recover() {
            let content = recovered.content.clone();
            *self.imp().document.borrow_mut() = recovered;
            self.imp().editor.buffer().set_text(&content);
            self.update_title();
        }
    }

    // ── Seed ──────────────────────────────────────────────────────────────────

    fn seed_document(&self) {
        let buf = self.imp().editor.buffer();
        let (start, end) = buf.bounds();
        if !buf.text(&start, &end, false).is_empty() {
            return;
        }
        buf.set_text(
            "# Swatchbook\n\n\
             A *Markdown-powered* style binder.\n\n\
             ## Palette\n\n\
             - **Primary** — `#3482E3`\n\
             - **Success** — `#2EC27E`\n\
             - **Warning** — `#F5C211`\n\
             - **Error** — `#E53935`\n\
             - **Purple** — `#9C27B0`\n",
        );
    }
}
