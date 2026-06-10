//! The main application window.
//!
//! `SwatchbookWindow` is a composite-template subclass of
//! `Adw.ApplicationWindow`. The visual layout lives in `window.blp`; this
//! module owns: GSettings-backed geometry persistence, the live Markdown→swatch
//! pipeline, file I/O actions, and auto-save / crash recovery.

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gdk, gio, glib};

use crate::document::Document;
use crate::parser;
use crate::renderer::{self, SwatchItem};

mod imp {
    use super::*;
    use gtk::CompositeTemplate;
    use std::cell::{OnceCell, RefCell};

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/io/github/patricioaumedes/Swatchbook/window.ui")]
    pub struct SwatchbookWindow {
        #[template_child]
        pub toast_overlay: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub split_view: TemplateChild<adw::NavigationSplitView>,
        #[template_child]
        pub editor: TemplateChild<gtk::TextView>,
        #[template_child]
        pub canvas_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub status_page: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub canvas_scroll: TemplateChild<gtk::ScrolledWindow>,
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

        /// Index of the keyboard-focused swatch on the canvas, if any.
        pub focused_swatch: RefCell<Option<usize>>,
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
            obj.check_crash_recovery(); // must run before setup_autosave writes the sentinel
            obj.setup_autosave();
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
        let style_manager = adw::StyleManager::default();
        let style_manager2 = style_manager.clone();
        let window_weak = self.downgrade();

        imp.canvas.set_draw_func(move |_area, cr, width, height| {
            let Some(win) = window_weak.upgrade() else { return };
            let items = win.imp().swatches.borrow();
            let focused = *win.imp().focused_swatch.borrow();
            let dark = style_manager.is_dark();
            renderer::render(cr, &items, width as f64, height as f64, dark, focused);
        });

        // Update canvas height whenever the allocated width changes so the
        // content_height calculation uses the real width, not the initial guess.
        let win_weak = self.downgrade();
        imp.canvas.connect_resize(move |canvas, width, _height| {
            let Some(win) = win_weak.upgrade() else { return };
            let count = win.imp().swatches.borrow().len();
            let h = renderer::content_height(count, width as f64).ceil() as i32;
            canvas.set_content_height(h.max(360));
        });

        // Redraw on colour-scheme change.
        let canvas = imp.canvas.get();
        style_manager2.connect_dark_notify(move |_| {
            canvas.queue_draw();
        });

        // Click anywhere on the canvas to grab keyboard focus.
        let click = gtk::GestureClick::new();
        let canvas_ref = imp.canvas.get();
        click.connect_pressed(move |_, _, _, _| {
            canvas_ref.grab_focus();
        });
        imp.canvas.set_focusable(true);
        imp.canvas.add_controller(click);

        // Keyboard navigation: arrows move the focus indicator, Enter copies the hex.
        let key_ctrl = gtk::EventControllerKey::new();
        let win_weak2 = self.downgrade();
        key_ctrl.connect_key_pressed(move |_, key, _, _| {
            let Some(win) = win_weak2.upgrade() else {
                return glib::Propagation::Proceed;
            };
            let imp = win.imp();
            let count = imp.swatches.borrow().len();
            if count == 0 {
                return glib::Propagation::Proceed;
            }

            match key {
                gdk::Key::Right | gdk::Key::Down => {
                    let next = imp
                        .focused_swatch
                        .borrow()
                        .map_or(0, |i| (i + 1) % count);
                    *imp.focused_swatch.borrow_mut() = Some(next);
                    imp.canvas.queue_draw();
                    glib::Propagation::Stop
                }
                gdk::Key::Left | gdk::Key::Up => {
                    let prev = imp.focused_swatch.borrow().map_or(count - 1, |i| {
                        if i == 0 { count - 1 } else { i - 1 }
                    });
                    *imp.focused_swatch.borrow_mut() = Some(prev);
                    imp.canvas.queue_draw();
                    glib::Propagation::Stop
                }
                gdk::Key::Return | gdk::Key::KP_Enter => {
                    if let Some(idx) = *imp.focused_swatch.borrow() {
                        if idx < count {
                            let hex = imp.swatches.borrow()[idx].hex.clone();
                            win.clipboard().set_text(&hex);
                            win.show_toast(&format!("Copied {hex}"));
                        }
                    }
                    glib::Propagation::Stop
                }
                _ => glib::Propagation::Proceed,
            }
        });
        imp.canvas.add_controller(key_ctrl);
    }

    // ── Editor / live-preview pipeline ────────────────────────────────────────

    fn setup_editor(&self) {
        let window_weak = self.downgrade();
        let buffer = self.imp().editor.buffer();

        buffer.connect_changed(move |buf| {
            let Some(win) = window_weak.upgrade() else { return };
            let imp = win.imp();

            imp.document.borrow_mut().is_modified = true;
            win.update_title();

            // Cancel any pending debounce timer. The take() ensures we only
            // call remove() on IDs that haven't already fired and removed themselves.
            if let Some(id) = imp.debounce_id.borrow_mut().take() {
                // remove() can only fail if the source already fired — ignore that.
                let _ = id.remove();
            }

            let (start, end) = buf.bounds();
            let text = buf.text(&start, &end, false).to_string();
            let window_weak2 = win.downgrade();

            // Re-parse 150 ms after the last keystroke.
            // Clear debounce_id from inside the closure so the next keystroke
            // won't try to remove an already-fired source.
            let id = glib::timeout_add_local(std::time::Duration::from_millis(150), move || {
                if let Some(win) = window_weak2.upgrade() {
                    win.imp().debounce_id.borrow_mut().take();
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
        let canvas_w = self.imp().canvas.allocated_width().max(480) as f64;
        let canvas_h = renderer::content_height(items.len(), canvas_w).ceil() as i32;
        let new_count = items.len();
        *self.imp().swatches.borrow_mut() = items;

        // Keep focused index in bounds after a re-parse.
        let mut focused = self.imp().focused_swatch.borrow_mut();
        if let Some(idx) = *focused {
            if idx >= new_count {
                *focused = if new_count == 0 { None } else { Some(new_count - 1) };
            }
        }
        self.imp().canvas.set_content_height(canvas_h.max(360));

        if has_swatches {
            self.imp().canvas_stack.set_visible_child(&*self.imp().canvas_scroll);
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
        let export_png = gio::ActionEntry::builder("export-png")
            .activate(|win: &Self, _, _| win.action_export_png())
            .build();
        let export_svg = gio::ActionEntry::builder("export-svg")
            .activate(|win: &Self, _, _| win.action_export_svg())
            .build();
        let copy_css = gio::ActionEntry::builder("copy-css")
            .activate(|win: &Self, _, _| win.action_copy_css())
            .build();

        self.add_action_entries([open, save, save_as, export_png, export_svg, copy_css]);
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
                // set_text fires `changed` synchronously, which sets is_modified=true.
                // Reset it immediately — a just-opened file is not modified.
                self.imp().editor.buffer().set_text(&content);
                self.imp().document.borrow_mut().is_modified = false;
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

    fn canvas_export_size(&self) -> (u32, u32) {
        let w = self.imp().canvas.allocated_width().max(480) as u32;
        let items = self.imp().swatches.borrow();
        let h = renderer::content_height(items.len(), w as f64).ceil() as u32;
        (w, h.max(360))
    }

    fn show_toast(&self, message: &str) {
        self.imp().toast_overlay.add_toast(adw::Toast::new(message));
    }

    fn action_export_png(&self) {
        if self.imp().swatches.borrow().is_empty() {
            self.show_toast("No swatches to export.");
            return;
        }

        let filter = gtk::FileFilter::new();
        filter.set_name(Some("PNG image"));
        filter.add_pattern("*.png");
        let filters = gio::ListStore::new::<gtk::FileFilter>();
        filters.append(&filter);

        let dialog = gtk::FileDialog::builder()
            .title("Export as PNG")
            .filters(&filters)
            .initial_name("swatchbook.png")
            .build();

        let win = self.clone();
        dialog.save(Some(self), gio::Cancellable::NONE, move |result| {
            if let Ok(file) = result {
                if let Some(path) = file.path() {
                    let items = win.imp().swatches.borrow().clone();
                    let (w, h) = win.canvas_export_size();
                    match renderer::export_png(&items, w, h, &path) {
                        Ok(()) => win.show_toast("PNG exported."),
                        Err(e) => eprintln!("swatchbook: PNG export failed: {e}"),
                    }
                }
            }
        });
    }

    fn action_export_svg(&self) {
        if self.imp().swatches.borrow().is_empty() {
            self.show_toast("No swatches to export.");
            return;
        }

        let filter = gtk::FileFilter::new();
        filter.set_name(Some("SVG image"));
        filter.add_pattern("*.svg");
        let filters = gio::ListStore::new::<gtk::FileFilter>();
        filters.append(&filter);

        let dialog = gtk::FileDialog::builder()
            .title("Export as SVG")
            .filters(&filters)
            .initial_name("swatchbook.svg")
            .build();

        let win = self.clone();
        dialog.save(Some(self), gio::Cancellable::NONE, move |result| {
            if let Ok(file) = result {
                if let Some(path) = file.path() {
                    let items = win.imp().swatches.borrow().clone();
                    let (w, h) = win.canvas_export_size();
                    match renderer::export_svg(&items, w, h, &path) {
                        Ok(()) => win.show_toast("SVG exported."),
                        Err(e) => eprintln!("swatchbook: SVG export failed: {e}"),
                    }
                }
            }
        });
    }

    fn action_copy_css(&self) {
        let items = self.imp().swatches.borrow();
        if items.is_empty() {
            self.show_toast("No swatches to copy.");
            return;
        }
        let css = renderer::to_css_variables(&items);
        drop(items);
        self.clipboard().set_text(&css);
        self.show_toast("CSS variables copied to clipboard.");
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
        let text = "# Swatchbook\n\n\
             A *Markdown-powered* style binder.\n\n\
             ## Palette\n\n\
             - **Primary** — `#3482E3`\n\
             - **Success** — `#2EC27E`\n\
             - **Warning** — `#F5C211`\n\
             - **Error** — `#E53935`\n\
             - **Purple** — `#9C27B0`\n";
        // set_text fires `changed` synchronously; reset the flag so a fresh
        // window doesn't start life with the unsaved-changes bullet.
        buf.set_text(text);
        self.imp().document.borrow_mut().is_modified = false;
        self.update_title();
        // Render immediately — don't wait for the debounce timer.
        self.reparse(text);
    }
}
