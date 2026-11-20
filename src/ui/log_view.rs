use gtk::prelude::*;

const MAX_LINES: i32 = 5000;

pub struct LogView {
    pub scrolled: gtk::ScrolledWindow,
    buffer: gtk::TextBuffer,
}

impl LogView {
    pub fn new() -> Self {
        let buffer = gtk::TextBuffer::new(None);
        let view = gtk::TextView::builder()
            .buffer(&buffer)
            .editable(false)
            .cursor_visible(false)
            .monospace(true)
            .wrap_mode(gtk::WrapMode::Char)
            .top_margin(8)
            .bottom_margin(8)
            .left_margin(10)
            .right_margin(10)
            .build();
        view.add_css_class("debdrop-log");
        view.update_property(&[gtk::accessible::Property::Label("Installation log")]);

        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .child(&view)
            .vexpand(true)
            .build();
        scrolled.add_css_class("card");

        Self { scrolled, buffer }
    }

    pub fn clear(&self) {
        self.buffer.set_text("");
    }

    pub fn append(&self, text: &str) {
        let stick = self.at_bottom();
        let mut end = self.buffer.end_iter();
        if self.buffer.char_count() > 0 {
            self.buffer.insert(&mut end, "\n");
        }
        self.buffer.insert(&mut end, text);
        self.trim();
        if stick {
            let adj = self.scrolled.vadjustment();
            gtk::glib::idle_add_local_once(move || {
                adj.set_value(adj.upper());
            });
        }
    }

    pub fn text(&self) -> String {
        self.buffer
            .text(&self.buffer.start_iter(), &self.buffer.end_iter(), false)
            .to_string()
    }

    fn at_bottom(&self) -> bool {
        let adj = self.scrolled.vadjustment();
        adj.value() + adj.page_size() >= adj.upper() - 32.0
    }

    fn trim(&self) {
        let lines = self.buffer.line_count();
        if lines > MAX_LINES {
            let mut start = self.buffer.start_iter();
            let mut end = self
                .buffer
                .iter_at_line(lines - MAX_LINES)
                .unwrap_or_else(|| self.buffer.end_iter());
            self.buffer.delete(&mut start, &mut end);
        }
    }
}

impl Default for LogView {
    fn default() -> Self {
        Self::new()
    }
}
