mod apt;
mod deb;
mod runner;
mod ui;

use std::cell::RefCell;
use std::rc::Rc;

use gtk::gio;
use gtk::prelude::*;
use libadwaita as adw;

thread_local! {
    static WINDOW: RefCell<Option<Rc<ui::window::AppWindow>>> = const { RefCell::new(None) };
}

fn ensure_window(app: &adw::Application) -> Rc<ui::window::AppWindow> {
    WINDOW.with(|slot| {
        if let Some(existing) = slot.borrow().as_ref() {
            return Rc::clone(existing);
        }
        let window = ui::window::AppWindow::build(app);
        *slot.borrow_mut() = Some(Rc::clone(&window));
        window
    })
}

fn load_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_string(include_str!("../resources/style.css"));
    if let Some(display) = gtk::gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

fn main() -> gtk::glib::ExitCode {
    let app = adw::Application::builder()
        .application_id("io.github.debdrop.DebDrop")
        .flags(gio::ApplicationFlags::HANDLES_OPEN)
        .build();

    app.connect_startup(|_| load_css());
    app.connect_activate(|app| {
        ensure_window(app).present();
    });
    app.connect_open(|app, files, _hint| {
        let window = ensure_window(app);
        window.present();
        if let Some(path) = files.first().and_then(|file| file.path()) {
            window.load_path(path);
        }
    });

    app.run()
}
