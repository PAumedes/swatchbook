//! The main application window.
//!
//! `SwatchbookWindow` is a composite-template subclass of
//! `Adw.ApplicationWindow`. The visual layout lives in `window.blp`; this
//! module owns: GSettings-backed geometry persistence, the live Markdown→swatch
//! pipeline, file I/O actions, and auto-save / crash recovery.

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gdk, gio, glib};

use swatchbook::document::Document;
use swatchbook::parser;
use swatchbook::renderer::{self, RenderCard, SwatchItem};
use swatchbook::token::DesignToken;

mod imp {
    use super::*;
    use gtk::CompositeTemplate;
    use std::cell::{Cell, OnceCell, RefCell};

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

        /// Parsed design-token cards waiting to be drawn.
        pub cards: RefCell<Vec<RenderCard>>,

        /// Pending debounce timeout — cancelled on each new keystroke.
        pub debounce_id: RefCell<Option<glib::SourceId>>,

        /// Current document state (path, modified flag).
        pub document: RefCell<Document>,

        /// Auto-save periodic timer.
        pub autosave_id: RefCell<Option<glib::SourceId>>,

        /// Index of the keyboard-focused swatch on the canvas, if any.
        pub focused_swatch: RefCell<Option<usize>>,

        /// Set once the user has resolved the unsaved-changes prompt, so the
        /// follow-up close request skips the prompt and proceeds.
        pub force_close: Cell<bool>,
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
            let obj = self.obj();

            // Unsaved changes: intercept the close and ask. The prompt's
            // response re-issues the close with `force_close` set.
            if !self.force_close.get() && obj.imp().document.borrow().is_modified {
                obj.confirm_close();
                return glib::Propagation::Stop;
            }

            if let Err(e) = obj.save_window_state() {
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
        self.imp()
            .settings
            .set(settings)
            .expect("settings set once");
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
            let Some(win) = window_weak.upgrade() else {
                return;
            };
            let cards = win.imp().cards.borrow();
            let focused = *win.imp().focused_swatch.borrow();
            let dark = style_manager.is_dark();
            renderer::render(cr, &cards, width as f64, height as f64, dark, focused);
        });

        // Update canvas height whenever the allocated width changes so the
        // content_height calculation uses the real width, not the initial guess.
        let win_weak = self.downgrade();
        imp.canvas.connect_resize(move |canvas, width, _height| {
            let Some(win) = win_weak.upgrade() else {
                return;
            };
            let count = win.imp().cards.borrow().len();
            let h = renderer::content_height(count, width as f64).ceil() as i32;
            canvas.set_content_height(h.max(360));
        });

        // Redraw on colour-scheme change.
        let canvas = imp.canvas.get();
        style_manager2.connect_dark_notify(move |_| {
            canvas.queue_draw();
        });

        // Click a swatch to focus it and copy its hex; click empty space to
        // just grab keyboard focus.
        let click = gtk::GestureClick::new();
        let win_weak_click = self.downgrade();
        click.connect_pressed(move |_, _, x, y| {
            let Some(win) = win_weak_click.upgrade() else {
                return;
            };
            win.imp().canvas.grab_focus();
            if let Some(idx) = win.swatch_at(x, y) {
                *win.imp().focused_swatch.borrow_mut() = Some(idx);
                let value = win.imp().cards.borrow()[idx].copy_value();
                win.clipboard().set_text(&value);
                win.show_toast(&format!("Copied {value}"));
                win.imp().canvas.queue_draw();
            }
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
            let count = imp.cards.borrow().len();
            if count == 0 {
                return glib::Propagation::Proceed;
            }

            match key {
                gdk::Key::Right | gdk::Key::Down => {
                    let next = imp.focused_swatch.borrow().map_or(0, |i| (i + 1) % count);
                    *imp.focused_swatch.borrow_mut() = Some(next);
                    imp.canvas.queue_draw();
                    glib::Propagation::Stop
                }
                gdk::Key::Left | gdk::Key::Up => {
                    let prev = imp.focused_swatch.borrow().map_or(count - 1, |i| {
                        if i == 0 {
                            count - 1
                        } else {
                            i - 1
                        }
                    });
                    *imp.focused_swatch.borrow_mut() = Some(prev);
                    imp.canvas.queue_draw();
                    glib::Propagation::Stop
                }
                gdk::Key::Return | gdk::Key::KP_Enter => {
                    if let Some(idx) = *imp.focused_swatch.borrow() {
                        if idx < count {
                            let value = imp.cards.borrow()[idx].copy_value();
                            win.clipboard().set_text(&value);
                            win.show_toast(&format!("Copied {value}"));
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
        self.setup_highlight_tags();

        let window_weak = self.downgrade();
        let buffer = self.imp().editor.buffer();

        buffer.connect_changed(move |buf| {
            let Some(win) = window_weak.upgrade() else {
                return;
            };
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

            // Highlight immediately — it's cheap and makes the editor feel responsive.
            win.highlight(&text);

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
        let cards: Vec<RenderCard> = parsed
            .all_swatches()
            .map(|e| match &e.token {
                DesignToken::Color(color) => {
                    let (r, g, b, a) = color.to_rgba();
                    RenderCard::Color(SwatchItem {
                        name: e.name.clone(),
                        hex: color.to_hex_string(),
                        r,
                        g,
                        b,
                        a,
                    })
                }
                DesignToken::Font {
                    family,
                    size_px,
                    weight,
                    line_height,
                    display,
                } => RenderCard::Font {
                    name: e.name.clone(),
                    family: family.clone(),
                    size_px: *size_px,
                    weight: *weight,
                    line_height: *line_height,
                    display: display.clone(),
                },
                DesignToken::Space { value_px, display } => RenderCard::Space {
                    name: e.name.clone(),
                    value_px: *value_px,
                    display: display.clone(),
                },
                DesignToken::Radius { value_px, display } => RenderCard::Radius {
                    name: e.name.clone(),
                    value_px: *value_px,
                    display: display.clone(),
                },
                DesignToken::Shadow(css) => RenderCard::Shadow {
                    name: e.name.clone(),
                    css: css.clone(),
                },
            })
            .collect();

        let has_swatches = !cards.is_empty();
        let canvas_w = self.imp().canvas.allocated_width().max(480) as f64;
        let canvas_h = renderer::content_height(cards.len(), canvas_w).ceil() as i32;
        let new_count = cards.len();
        *self.imp().cards.borrow_mut() = cards;

        // Keep focused index in bounds after a re-parse.
        let mut focused = self.imp().focused_swatch.borrow_mut();
        if let Some(idx) = *focused {
            if idx >= new_count {
                *focused = if new_count == 0 {
                    None
                } else {
                    Some(new_count - 1)
                };
            }
        }
        self.imp().canvas.set_content_height(canvas_h.max(360));

        if has_swatches {
            self.imp()
                .canvas_stack
                .set_visible_child(&*self.imp().canvas_scroll);
            self.imp().canvas.queue_draw();
        } else {
            self.imp()
                .canvas_stack
                .set_visible_child(&*self.imp().status_page);
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
        let export_json = gio::ActionEntry::builder("export-json")
            .activate(|win: &Self, _, _| win.action_export_json())
            .build();
        let export_gpl = gio::ActionEntry::builder("export-gpl")
            .activate(|win: &Self, _, _| win.action_export_gpl())
            .build();
        let copy_tailwind = gio::ActionEntry::builder("copy-tailwind")
            .activate(|win: &Self, _, _| win.action_copy_tailwind())
            .build();

        self.add_action_entries([
            open,
            save,
            save_as,
            export_png,
            export_svg,
            copy_css,
            export_json,
            export_gpl,
            copy_tailwind,
        ]);
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
                self.register_recent(path);
            }
            Err(e) => self.show_error(&gettextrs::gettext("Could not open file"), &e.to_string()),
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
            self.show_error(&gettextrs::gettext("Could not save file"), &e.to_string());
            return;
        }
        if let Some(path) = self.imp().document.borrow().path.clone() {
            self.register_recent(&path);
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
                    if let Err(e) = win.imp().document.borrow_mut().save_to(path.clone()) {
                        win.show_error(&gettextrs::gettext("Could not save file"), &e.to_string());
                    } else {
                        win.register_recent(&path);
                    }
                    win.update_title();
                }
            }
        });
    }

    /// Hit-test a canvas point against the swatch layout, returning the index
    /// of the swatch rectangle under `(x, y)`, if any.
    fn swatch_at(&self, x: f64, y: f64) -> Option<usize> {
        let count = self.imp().cards.borrow().len();
        if count == 0 {
            return None;
        }
        let width = self.imp().canvas.allocated_width().max(480) as f64;
        renderer::layout(count, width).into_iter().position(|rect| {
            x >= rect.x && x <= rect.x + rect.w && y >= rect.y && y <= rect.y + rect.h
        })
    }

    fn canvas_export_size(&self) -> (u32, u32) {
        let w = self.imp().canvas.allocated_width().max(480) as u32;
        let count = self.imp().cards.borrow().len();
        let h = renderer::content_height(count, w as f64).ceil() as u32;
        (w, h.max(360))
    }

    fn setup_highlight_tags(&self) {
        let table = self.imp().editor.buffer().tag_table();

        // Heading lines (starting with #) — bold.
        let heading = gtk::TextTag::new(Some("sb-heading"));
        heading.set_property("weight", 700i32); // pango::Weight::Bold
        table.add(&heading);

        // Inline code spans (`...`) — Adwaita purple, monospace.
        let code = gtk::TextTag::new(Some("sb-code"));
        code.set_property("foreground", "#7764d8");
        code.set_property("family", "monospace");
        table.add(&code);
    }

    /// Apply heading and code-span highlighting to the editor buffer.
    ///
    /// Runs on every keystroke (before the reparse debounce) so edits feel
    /// immediate. Does not trigger a `changed` signal, so no re-entrancy risk.
    fn highlight(&self, text: &str) {
        let buf = self.imp().editor.buffer();
        let table = buf.tag_table();

        // Clear previous highlights.
        let (doc_start, doc_end) = buf.bounds();
        for name in ["sb-heading", "sb-code"] {
            if let Some(tag) = table.lookup(name) {
                buf.remove_tag(&tag, &doc_start, &doc_end);
            }
        }

        for (line_no, line) in text.lines().enumerate() {
            let li = line_no as i32;

            // Heading: bold the whole line.
            if line.starts_with('#') {
                if let Some(ls) = buf.iter_at_line(li) {
                    let mut le = ls.clone();
                    le.forward_to_line_end();
                    if let Some(tag) = table.lookup("sb-heading") {
                        buf.apply_tag(&tag, &ls, &le);
                    }
                }
            }

            // Code spans: colour each `...` pair on the line.
            let chars: Vec<char> = line.chars().collect();
            let n = chars.len();
            let mut ci = 0usize;
            while ci < n {
                if chars[ci] == '`' && ci + 1 < n {
                    if let Some(offset) = chars[ci + 1..].iter().position(|&c| c == '`') {
                        let cs = ci as i32;
                        let ce = (ci + offset + 2) as i32;
                        if let (Some(ts), Some(te)) = (
                            buf.iter_at_line_offset(li, cs),
                            buf.iter_at_line_offset(li, ce),
                        ) {
                            if let Some(tag) = table.lookup("sb-code") {
                                buf.apply_tag(&tag, &ts, &te);
                            }
                        }
                        ci += offset + 2;
                        continue;
                    }
                }
                ci += 1;
            }
        }
    }

    fn register_recent(&self, path: &std::path::Path) {
        let uri = gio::File::for_path(path).uri();
        gtk::RecentManager::default().add_item(&uri);
    }

    fn show_toast(&self, message: &str) {
        self.imp().toast_overlay.add_toast(adw::Toast::new(message));
    }

    /// Show a modal error dialog for failures the user must not miss (e.g. a
    /// failed save). Less serious feedback should use `show_toast` instead.
    fn show_error(&self, heading: &str, detail: &str) {
        let dialog = adw::MessageDialog::new(Some(self), Some(heading), Some(detail));
        dialog.add_response("ok", &gettextrs::gettext("OK"));
        dialog.set_default_response(Some("ok"));
        dialog.present();
    }

    /// Prompt before closing a window with unsaved changes. Resolves to one of
    /// three outcomes: cancel (stay open), discard (close losing edits), or save
    /// (persist then close). The chosen path re-issues the close with
    /// `force_close` set so the prompt isn't shown twice.
    fn confirm_close(&self) {
        let dialog = adw::MessageDialog::new(
            Some(self),
            Some(&gettextrs::gettext("Save changes?")),
            Some(&gettextrs::gettext(
                "Your binder has unsaved changes. They will be lost if you close without saving.",
            )),
        );
        dialog.add_response("cancel", &gettextrs::gettext("Cancel"));
        dialog.add_response("discard", &gettextrs::gettext("Discard"));
        dialog.add_response("save", &gettextrs::gettext("Save"));
        dialog.set_response_appearance("discard", adw::ResponseAppearance::Destructive);
        dialog.set_response_appearance("save", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("save"));
        dialog.set_close_response("cancel");

        let win = self.clone();
        dialog.connect_response(None, move |_, response| match response {
            "discard" => {
                win.imp().force_close.set(true);
                win.close();
            }
            "save" => {
                let had_path = win.imp().document.borrow().path.is_some();
                win.action_save();
                // With a known path the save is synchronous, so close now. Without
                // one, `action_save` opened the Save-As dialog asynchronously;
                // leave the window open and let the user close again once saved.
                if had_path && !win.imp().document.borrow().is_modified {
                    win.imp().force_close.set(true);
                    win.close();
                }
            }
            _ => {} // cancel: stay open
        });

        dialog.present();
    }

    fn action_export_png(&self) {
        if self.imp().cards.borrow().is_empty() {
            self.show_toast("No tokens to export.");
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
                    let cards = win.imp().cards.borrow().clone();
                    let (w, h) = win.canvas_export_size();
                    match renderer::export_png(&cards, w, h, &path) {
                        Ok(()) => win.show_toast(&gettextrs::gettext("PNG exported.")),
                        Err(e) => win.show_toast(&format!(
                            "{}: {e}",
                            gettextrs::gettext("PNG export failed")
                        )),
                    }
                }
            }
        });
    }

    fn action_export_svg(&self) {
        if self.imp().cards.borrow().is_empty() {
            self.show_toast("No tokens to export.");
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
                    let cards = win.imp().cards.borrow().clone();
                    let (w, h) = win.canvas_export_size();
                    match renderer::export_svg(&cards, w, h, &path) {
                        Ok(()) => win.show_toast(&gettextrs::gettext("SVG exported.")),
                        Err(e) => win.show_toast(&format!(
                            "{}: {e}",
                            gettextrs::gettext("SVG export failed")
                        )),
                    }
                }
            }
        });
    }

    fn action_copy_css(&self) {
        let cards = self.imp().cards.borrow();
        let color_items: Vec<SwatchItem> = cards.iter().filter_map(|c| c.as_color().cloned()).collect();
        drop(cards);
        if color_items.is_empty() {
            self.show_toast("No colour tokens to copy.");
            return;
        }
        let css = renderer::to_css_variables(&color_items);
        self.clipboard().set_text(&css);
        let n = color_items.len();
        self.show_toast(&format!(
            "{} CSS variable{} copied.",
            n,
            if n == 1 { "" } else { "s" }
        ));
    }

    fn action_export_json(&self) {
        if self.imp().cards.borrow().is_empty() {
            self.show_toast("No tokens to export.");
            return;
        }

        let filter = gtk::FileFilter::new();
        filter.set_name(Some("JSON design tokens"));
        filter.add_pattern("*.json");
        let filters = gio::ListStore::new::<gtk::FileFilter>();
        filters.append(&filter);

        let dialog = gtk::FileDialog::builder()
            .title("Export Design Tokens as JSON")
            .filters(&filters)
            .initial_name("tokens.json")
            .build();

        let win = self.clone();
        dialog.save(Some(self), gio::Cancellable::NONE, move |result| {
            if let Ok(file) = result {
                if let Some(path) = file.path() {
                    let cards = win.imp().cards.borrow().clone();
                    let json = renderer::to_design_tokens_json(&cards);
                    match std::fs::write(&path, json) {
                        Ok(()) => win.show_toast(&gettextrs::gettext("Design tokens exported.")),
                        Err(e) => win.show_toast(&format!("JSON export failed: {e}")),
                    }
                }
            }
        });
    }

    fn action_export_gpl(&self) {
        let cards = self.imp().cards.borrow();
        let color_count = cards.iter().filter(|c| c.as_color().is_some()).count();
        drop(cards);
        if color_count == 0 {
            self.show_toast("No colour tokens to export.");
            return;
        }

        let filter = gtk::FileFilter::new();
        filter.set_name(Some("GIMP palette"));
        filter.add_pattern("*.gpl");
        let filters = gio::ListStore::new::<gtk::FileFilter>();
        filters.append(&filter);

        let palette_name = self
            .imp()
            .document
            .borrow()
            .title()
            .trim_end_matches(".md")
            .to_string();

        let dialog = gtk::FileDialog::builder()
            .title("Export as GIMP Palette")
            .filters(&filters)
            .initial_name("palette.gpl")
            .build();

        let win = self.clone();
        dialog.save(Some(self), gio::Cancellable::NONE, move |result| {
            if let Ok(file) = result {
                if let Some(path) = file.path() {
                    let cards = win.imp().cards.borrow().clone();
                    let gpl = renderer::to_gimp_palette(&cards, &palette_name);
                    match std::fs::write(&path, gpl) {
                        Ok(()) => win.show_toast(&gettextrs::gettext("GIMP palette exported.")),
                        Err(e) => win.show_toast(&format!("GPL export failed: {e}")),
                    }
                }
            }
        });
    }

    fn action_copy_tailwind(&self) {
        let cards = self.imp().cards.borrow();
        let color_count = cards.iter().filter(|c| c.as_color().is_some()).count();
        if color_count == 0 {
            self.show_toast("No colour tokens to copy.");
            drop(cards);
            return;
        }
        let tw = renderer::to_tailwind_config(&cards);
        drop(cards);
        self.clipboard().set_text(&tw);
        self.show_toast(&format!(
            "Tailwind config with {color_count} colour{} copied.",
            if color_count == 1 { "" } else { "s" }
        ));
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
        let text = "# My Design System\n\n\
             ## Palette\n\n\
             - **Primary** — `#3482E3`\n\
             - **Success** — `#2EC27E`\n\
             - **Warning** — `#F5C211`\n\
             - **Error** — `#E53935`\n\
             - **Purple** — `#9C27B0`\n\n\
             ## Typography\n\n\
             - **Body** — `font: sans-serif 16px/1.5`\n\
             - **Heading** — `font: sans-serif Bold 24px`\n\
             - **Mono** — `font: monospace 14px`\n\n\
             ## Spacing\n\n\
             - **xs** — `4px`\n\
             - **sm** — `8px`\n\
             - **md** — `16px`\n\
             - **lg** — `32px`\n\n\
             ## Radius\n\n\
             - **button** — `radius: 6px`\n\
             - **card** — `radius: 12px`\n\n\
             ## Shadow\n\n\
             - **card** — `shadow: 0 2px 8px rgba(0,0,0,0.12)`\n\
             - **modal** — `shadow: 0 8px 24px rgba(0,0,0,0.2)`\n";
        // set_text fires `changed` synchronously (which calls highlight() and
        // starts the debounce timer). Reset the modified flag immediately.
        buf.set_text(text);
        self.imp().document.borrow_mut().is_modified = false;
        self.update_title();
        // Render immediately — don't wait for the debounce timer.
        self.reparse(text);
    }
}
