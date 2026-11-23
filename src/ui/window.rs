use std::cell::{Cell, RefCell};
use std::ffi::OsString;
use std::path::PathBuf;
use std::rc::Rc;

use adw::prelude::*;
use gtk::gdk;
use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use libadwaita as adw;

use crate::ui::log_view::LogView;
use crate::ui::package_view::PackageCard;
use crate::{apt, deb, runner};

#[derive(Clone)]
enum RunKind {
    Install {
        name: String,
        version: String,
        action: apt::Action,
    },
    Remove {
        name: String,
    },
    Update,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BannerMode {
    None,
    Details,
    Refresh,
}

pub struct AppWindow {
    pub window: adw::ApplicationWindow,
    stack: gtk::Stack,
    overlay: adw::ToastOverlay,
    banner: adw::Banner,
    bar: gtk::Box,
    card: PackageCard,
    log: LogView,
    spinner: gtk::Spinner,
    busy_label: gtk::Label,
    primary: gtk::Button,
    remove: gtk::Button,
    launch: gtk::Button,
    choose_bar: gtk::Button,

    info: RefCell<Option<deb::DebInfo>>,
    plan: RefCell<Option<apt::Plan>>,
    installed: RefCell<Option<apt::InstalledState>>,
    action: Cell<apt::Action>,
    busy: Cell<bool>,
    pid: Cell<Option<i32>>,
    seq: Cell<u64>,
    phase: Cell<u8>,
    plan_ok: Cell<Option<bool>>,
    installed_ready: Cell<bool>,
    banner_mode: Cell<BannerMode>,
    last_dir: RefCell<Option<PathBuf>>,
}

impl AppWindow {
    pub fn build(app: &adw::Application) -> Rc<Self> {
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("DebDrop")
            .default_width(760)
            .default_height(620)
            .build();

        let choose_empty = gtk::Button::with_label("Choose File…");
        choose_empty.add_css_class("pill");
        choose_empty.add_css_class("suggested-action");
        let empty = adw::StatusPage::builder()
            .icon_name("package-x-generic")
            .title("Drop a .deb to inspect it")
            .description("Nothing is installed until you click Install.")
            .child(&choose_empty)
            .hexpand(true)
            .vexpand(true)
            .build();
        empty.add_css_class("debdrop-drop-zone");

        let card = PackageCard::new();
        let card_page = card.page.clone();
        card_page.set_vexpand(true);
        // GTK marks a scrolled window as focusable while it can scroll, which
        // turns the page into a single Tab stop that swallows the key before
        // any row is reached. The rows are the destinations; the viewport is not.
        if let Some(scrolled) = first_scrolled_window(card_page.upcast_ref()) {
            scrolled.set_focusable(false);
        }

        let spinner = gtk::Spinner::new();
        let busy_label = gtk::Label::new(Some("Working…"));
        busy_label.add_css_class("title-4");
        busy_label.set_wrap(true);
        busy_label.set_justify(gtk::Justification::Center);

        let busy_header = gtk::Box::new(gtk::Orientation::Vertical, 10);
        busy_header.set_halign(gtk::Align::Center);
        busy_header.set_margin_top(24);
        busy_header.set_margin_bottom(4);
        busy_header.append(&spinner);
        busy_header.append(&busy_label);

        let log = LogView::new();
        let busy_page = gtk::Box::new(gtk::Orientation::Vertical, 10);
        busy_page.set_margin_top(10);
        busy_page.set_margin_bottom(10);
        busy_page.set_margin_start(18);
        busy_page.set_margin_end(18);
        busy_page.append(&busy_header);
        busy_page.append(&log.scrolled);

        let stack = gtk::Stack::builder()
            .transition_type(gtk::StackTransitionType::Crossfade)
            .transition_duration(140)
            .vexpand(true)
            .build();
        stack.add_named(&empty, Some("empty"));
        stack.add_named(&card_page, Some("card"));
        stack.add_named(&busy_page, Some("busy"));
        stack.set_visible_child_name("empty");

        let header = adw::HeaderBar::new();
        let menu = gio::Menu::new();
        menu.append(Some("Open…"), Some("app.open"));
        menu.append(Some("Refresh package lists"), Some("app.refresh"));
        menu.append(Some("Show last log"), Some("app.show-log"));
        menu.append(Some("About DebDrop"), Some("app.about"));
        let menu_button = gtk::MenuButton::new();
        menu_button.set_icon_name("open-menu-symbolic");
        menu_button.set_menu_model(Some(&menu));
        menu_button.set_tooltip_text(Some("Menu"));
        header.pack_end(&menu_button);

        let open_button = gtk::Button::from_icon_name("document-open-symbolic");
        open_button.set_tooltip_text(Some("Open a .deb file (Ctrl+O)"));
        open_button.update_property(&[gtk::accessible::Property::Label("Open a .deb file")]);
        header.pack_end(&open_button);

        let banner = adw::Banner::new("");
        banner.set_revealed(false);
        banner.set_button_label(Some("Details"));

        let clamp = adw::Clamp::builder()
            .maximum_size(780)
            .tightening_threshold(560)
            .child(&stack)
            .build();

        let choose_bar = gtk::Button::with_label("Choose File…");
        choose_bar.add_css_class("flat");
        let launch = gtk::Button::with_label("Launch");
        launch.add_css_class("suggested-action");
        launch.set_visible(false);
        let remove = gtk::Button::with_label("Remove");
        remove.add_css_class("destructive-action");
        remove.set_visible(false);
        let primary = gtk::Button::with_label("Install");
        primary.add_css_class("suggested-action");
        primary.set_sensitive(false);
        primary.set_receives_default(true);

        let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        let bar = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        bar.set_margin_top(8);
        bar.set_margin_bottom(8);
        bar.set_margin_start(14);
        bar.set_margin_end(14);
        bar.append(&choose_bar);
        bar.append(&spacer);
        bar.append(&launch);
        bar.append(&remove);
        bar.append(&primary);
        bar.set_visible(false);

        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&header);
        toolbar.add_top_bar(&banner);
        toolbar.set_content(Some(&clamp));
        toolbar.add_bottom_bar(&bar);

        let overlay = adw::ToastOverlay::new();
        overlay.set_child(Some(&toolbar));
        window.set_content(Some(&overlay));
        window.set_default_widget(Some(&primary));

        let rc = Rc::new(Self {
            window,
            stack,
            overlay,
            banner,
            bar,
            card,
            log,
            spinner,
            busy_label,
            primary,
            remove,
            launch,
            choose_bar,
            info: RefCell::new(None),
            plan: RefCell::new(None),
            installed: RefCell::new(None),
            action: Cell::new(apt::Action::Install),
            busy: Cell::new(false),
            pid: Cell::new(None),
            seq: Cell::new(0),
            phase: Cell::new(0),
            plan_ok: Cell::new(None),
            installed_ready: Cell::new(false),
            banner_mode: Cell::new(BannerMode::None),
            last_dir: RefCell::new(None),
        });

        Self::wire(&rc, app, choose_empty, open_button);
        rc
    }

    fn wire(
        rc: &Rc<Self>,
        app: &adw::Application,
        choose_empty: gtk::Button,
        open_button: gtk::Button,
    ) {
        {
            let this = Rc::clone(rc);
            choose_empty.connect_clicked(move |_| this.pick_file());
        }
        {
            let this = Rc::clone(rc);
            rc.choose_bar.connect_clicked(move |_| this.pick_file());
        }
        {
            let this = Rc::clone(rc);
            open_button.connect_clicked(move |_| this.pick_file());
        }
        {
            let this = Rc::clone(rc);
            rc.primary.connect_clicked(move |_| this.start_install());
        }
        {
            let this = Rc::clone(rc);
            rc.remove.connect_clicked(move |_| this.confirm_remove());
        }
        {
            let this = Rc::clone(rc);
            rc.card
                .scripts_button()
                .connect_clicked(move |_| this.show_scripts_dialog());
        }
        {
            let this = Rc::clone(rc);
            rc.card
                .homepage_button()
                .connect_clicked(move |_| this.open_homepage());
        }
        {
            let this = Rc::clone(rc);
            rc.card
                .files_button()
                .connect_clicked(move |_| this.show_files_dialog());
        }
        {
            let this = Rc::clone(rc);
            rc.launch.connect_clicked(move |_| this.launch_app());
        }
        {
            let this = Rc::clone(rc);
            rc.banner
                .connect_button_clicked(move |_| match this.banner_mode.get() {
                    BannerMode::Refresh => this.start_update(),
                    _ => this.show_details_dialog(),
                });
        }

        // Drag & drop: GDK turns text/uri-list into a FileList for us.
        let target = gtk::DropTarget::new(gdk::FileList::static_type(), gdk::DragAction::COPY);
        {
            let stack = rc.stack.clone();
            target.connect_enter(move |_, _, _| {
                stack.add_css_class("drop-hover");
                gdk::DragAction::COPY
            });
        }
        {
            let stack = rc.stack.clone();
            target.connect_leave(move |_| stack.remove_css_class("drop-hover"));
        }
        target.connect_accept(|_, drop| drop.formats().contains_type(gdk::FileList::static_type()));
        {
            let this = Rc::clone(rc);
            target.connect_drop(move |_, value, _, _| {
                let Ok(list) = value.get::<gdk::FileList>() else {
                    return false;
                };
                let files = list.files();
                if files.len() != 1 {
                    this.toast("Drop a single .deb file");
                    return false;
                }
                match files[0].path() {
                    Some(path) => {
                        this.load_path(path);
                        true
                    }
                    None => false,
                }
            });
        }
        rc.stack.add_controller(target);

        // Closing mid-transaction must be deliberate.
        {
            let this = Rc::clone(rc);
            rc.window.connect_close_request(move |_| {
                if !this.busy.get() {
                    return glib::Propagation::Proceed;
                }
                let dialog = adw::AlertDialog::builder()
                    .heading("Package operation in progress")
                    .body("Stopping now can leave dpkg half-configured. If that happens, run “sudo dpkg --configure -a” in a terminal.")
                    .build();
                dialog.add_response("stay", "Keep running");
                dialog.add_response("quit", "Quit anyway");
                dialog.set_response_appearance("quit", adw::ResponseAppearance::Destructive);
                dialog.set_default_response(Some("stay"));
                dialog.set_close_response("stay");
                let window = this.window.clone();
                let pid = this.pid.get();
                dialog.choose(Some(&this.window), gio::Cancellable::NONE, move |result| {
                    if result.as_str() == "quit" {
                        if let Some(pid) = pid {
                            runner::terminate(pid);
                        }
                        window.destroy();
                    }
                });
                glib::Propagation::Stop
            });
        }

        // Application actions.
        {
            let this = Rc::clone(rc);
            let action = gio::SimpleAction::new("open", None);
            action.connect_activate(move |_, _| this.pick_file());
            app.add_action(&action);
            app.set_accels_for_action("app.open", &["<Control>o"]);
        }
        {
            let this = Rc::clone(rc);
            let action = gio::SimpleAction::new("refresh", None);
            action.connect_activate(move |_, _| this.start_update());
            app.add_action(&action);
        }
        {
            let this = Rc::clone(rc);
            let action = gio::SimpleAction::new("show-log", None);
            action.connect_activate(move |_, _| this.show_details_dialog());
            app.add_action(&action);
        }
        {
            let this = Rc::clone(rc);
            let action = gio::SimpleAction::new("about", None);
            action.connect_activate(move |_, _| this.show_about());
            app.add_action(&action);
        }
        {
            let quitter = app.clone();
            let action = gio::SimpleAction::new("quit", None);
            action.connect_activate(move |_, _| quitter.quit());
            app.add_action(&action);
            app.set_accels_for_action("app.quit", &["<Control>q"]);
        }
    }

    pub fn present(&self) {
        self.window.present();
    }

    fn toast(&self, message: &str) {
        self.overlay.add_toast(adw::Toast::new(message));
    }

    fn set_busy_controls(&self, enabled: bool) {
        self.choose_bar.set_sensitive(enabled);
        if enabled {
            self.refresh_primary();
            let installed = self
                .installed
                .borrow()
                .as_ref()
                .map(|s| s.installed)
                .unwrap_or(false);
            self.remove.set_sensitive(installed);
            let can_launch = installed
                && self
                    .info
                    .borrow()
                    .as_ref()
                    .and_then(|i| i.desktop.as_ref())
                    .and_then(|d| d.exec.as_ref())
                    .is_some();
            self.launch.set_sensitive(can_launch);
        } else {
            self.primary.set_sensitive(false);
            self.remove.set_sensitive(false);
            self.launch.set_sensitive(false);
        }
    }

    fn refresh_primary(&self) {
        let enabled =
            !self.busy.get() && self.installed_ready.get() && self.plan_ok.get() == Some(true);
        self.primary.set_sensitive(enabled);
    }

    fn show_banner(&self, message: &str, mode: BannerMode) {
        self.banner.set_title(message);
        self.banner.set_button_label(Some(match mode {
            BannerMode::Refresh => "Refresh lists",
            _ => "Details",
        }));
        self.banner_mode.set(mode);
        self.banner.set_revealed(true);
    }

    pub fn load_path(self: &Rc<Self>, path: PathBuf) {
        if self.busy.get() {
            self.toast("Wait for the current operation to finish");
            return;
        }

        match deb::load(&path) {
            Ok(info) => {
                let package = info.package.clone();
                let version = info.version.clone();
                let deb_path = info.path.clone();
                let arch = info.architecture.clone();

                if let Some(parent) = deb_path.parent() {
                    *self.last_dir.borrow_mut() = Some(parent.to_path_buf());
                }

                self.card.show(&info);
                *self.info.borrow_mut() = Some(info);
                *self.installed.borrow_mut() = None;
                *self.plan.borrow_mut() = None;
                self.action.set(apt::Action::Install);
                self.plan_ok.set(None);
                self.installed_ready.set(false);
                self.primary.set_label("Checking…");
                self.primary.set_sensitive(false);
                self.remove.set_visible(false);
                self.launch.set_visible(false);
                self.banner.set_revealed(false);
                self.banner_mode.set(BannerMode::None);
                self.log.clear();
                self.stack.set_visible_child_name("card");
                self.bar.set_visible(true);

                if let Some(problem) = apt::architecture_problem(&arch) {
                    self.card.set_blocked(&problem);
                    self.plan_ok.set(Some(false));
                    self.show_banner(&problem, BannerMode::Details);
                    self.toast(&problem);
                    return;
                }

                let _ = package;
                let _ = version;
                self.seq.set(self.seq.get() + 1);
                let seq = self.seq.get();
                self.query_installed(seq);
                self.query_plan(seq);
            }
            Err(e) => {
                self.toast(&e.to_string());
            }
        }
    }

    fn query_installed(self: &Rc<Self>, seq: u64) {
        let Some((package, version)) = self
            .info
            .borrow()
            .as_ref()
            .map(|i| (i.package.clone(), i.version.clone()))
        else {
            return;
        };

        let (tx, rx) = async_channel::unbounded::<(apt::InstalledState, apt::Action)>();
        std::thread::spawn(move || {
            let state = apt::installed_state(&package);
            let action = apt::action_for(&version, &state);
            let _ = tx.send_blocking((state, action));
        });

        let this = Rc::clone(self);
        glib::spawn_future_local(async move {
            if let Ok((state, action)) = rx.recv().await {
                if this.seq.get() != seq {
                    return;
                }
                this.apply_installed(state, action);
            }
        });
    }

    fn query_plan(self: &Rc<Self>, seq: u64) {
        let Some(path) = self.info.borrow().as_ref().map(|i| i.path.clone()) else {
            return;
        };
        let (tx, rx) = async_channel::unbounded::<apt::Plan>();
        std::thread::spawn(move || {
            let plan = apt::simulate(&path);
            let _ = tx.send_blocking(plan);
        });
        let this = Rc::clone(self);
        glib::spawn_future_local(async move {
            if let Ok(plan) = rx.recv().await {
                if this.seq.get() != seq {
                    return;
                }
                this.apply_plan(plan);
            }
        });
    }

    fn apply_installed(&self, state: apt::InstalledState, action: apt::Action) {
        let version = self
            .info
            .borrow()
            .as_ref()
            .map(|i| i.version.clone())
            .unwrap_or_default();
        let label = match action {
            apt::Action::Install => "Install".to_string(),
            apt::Action::Reinstall => "Reinstall".to_string(),
            apt::Action::Upgrade => format!("Upgrade to {version}"),
            apt::Action::Downgrade => format!("Downgrade to {version}"),
        };
        let installed = state.installed;
        self.card.set_installed(&state, action);
        self.primary.set_label(&label);
        self.remove.set_visible(installed);
        self.remove.set_sensitive(installed && !self.busy.get());

        let can_launch = installed
            && self
                .info
                .borrow()
                .as_ref()
                .and_then(|i| i.desktop.as_ref())
                .and_then(|d| d.exec.as_ref())
                .is_some();
        self.launch.set_visible(can_launch);
        self.launch.set_sensitive(can_launch && !self.busy.get());

        self.action.set(action);
        self.installed_ready.set(true);
        *self.installed.borrow_mut() = Some(state);
        self.refresh_primary();
    }

    fn apply_plan(&self, mut plan: apt::Plan) {
        // apt lists the package itself in the transaction; the card already
        // says what is happening to it, so drop that row.
        if let Some(package) = self.info.borrow().as_ref().map(|i| i.package.clone()) {
            plan.new.retain(|n| n != &package);
            plan.upgraded.retain(|n| n != &package);
            plan.downgraded.retain(|n| n != &package);
            plan.removed.retain(|n| n != &package);
        }
        if let Some(info) = self.info.borrow().as_ref() {
            self.card.set_plan(info, &plan);
        }
        self.plan_ok.set(Some(plan.ok));
        if !plan.ok {
            let mode = if plan.stale_lists {
                BannerMode::Refresh
            } else {
                BannerMode::Details
            };
            let headline = plan.headline();
            self.show_banner(&headline, mode);
        }
        *self.plan.borrow_mut() = Some(plan);
        self.refresh_primary();
    }

    fn refresh_queries(self: &Rc<Self>) {
        if self.info.borrow().is_none() {
            return;
        }
        self.seq.set(self.seq.get() + 1);
        let seq = self.seq.get();
        self.installed_ready.set(false);
        self.plan_ok.set(None);
        self.refresh_primary();
        self.query_installed(seq);
        self.query_plan(seq);
    }

    fn start_install(self: &Rc<Self>) {
        if self.busy.get() {
            return;
        }
        if self.plan_ok.get() != Some(true) {
            self.toast("Dependencies are not resolved");
            return;
        }
        let Some((path, name, version)) = self
            .info
            .borrow()
            .as_ref()
            .map(|i| (i.path.clone(), i.package.clone(), i.version.clone()))
        else {
            return;
        };
        let action = self.action.get();
        let argv = apt::install_argv(&path, action);
        let kind = RunKind::Install {
            name,
            version,
            action,
        };
        self.run(argv, kind, "Waiting for authentication…");
    }

    fn confirm_remove(self: &Rc<Self>) {
        if self.busy.get() {
            return;
        }
        let Some((package, installed_version)) = self.info.borrow().as_ref().map(|i| {
            (
                i.package.clone(),
                self.installed
                    .borrow()
                    .as_ref()
                    .and_then(|s| s.version.clone())
                    .unwrap_or_else(|| i.version.clone()),
            )
        }) else {
            return;
        };

        let dialog = adw::AlertDialog::builder()
            .heading(format!("Remove {package}?"))
            .body(format!(
                "This removes {package} {installed_version}. Packages that depend on it may be removed too. Dependencies installed with it are kept."
            ))
            .build();
        dialog.add_response("cancel", "Cancel");
        dialog.add_response("remove", "Remove");
        dialog.set_response_appearance("remove", adw::ResponseAppearance::Destructive);
        dialog.set_default_response(Some("cancel"));
        dialog.set_close_response("cancel");

        let this = Rc::clone(self);
        dialog.choose(Some(&self.window), gio::Cancellable::NONE, move |result| {
            if result.as_str() == "remove" {
                this.start_remove(package);
            }
        });
    }

    fn start_remove(self: &Rc<Self>, package: String) {
        let argv = apt::remove_argv(&package);
        self.run(
            argv,
            RunKind::Remove { name: package },
            "Waiting for authentication…",
        );
    }

    fn start_update(self: &Rc<Self>) {
        if self.busy.get() {
            return;
        }
        self.run(
            apt::update_argv(),
            RunKind::Update,
            "Waiting for authentication…",
        );
    }

    fn run(self: &Rc<Self>, argv: Vec<OsString>, kind: RunKind, initial: &str) {
        self.busy.set(true);
        self.phase.set(0);
        self.set_busy_controls(false);
        self.log.clear();
        self.log
            .append(&format!("$ {}", runner::display_cmd(&argv)));
        self.busy_label.set_text(initial);
        self.spinner.set_visible(true);
        self.spinner.start();
        self.banner.set_revealed(false);
        self.banner_mode.set(BannerMode::None);
        self.stack.set_visible_child_name("busy");

        match runner::spawn(argv) {
            Ok(handle) => {
                let pid = handle.pid;
                self.pid.set(Some(pid));
                let rx = handle.rx;
                let this = Rc::clone(self);
                glib::spawn_future_local(async move {
                    while let Ok(event) = rx.recv().await {
                        match event {
                            runner::Event::Line { text } => this.on_line(&text),
                            runner::Event::Exited { code } => {
                                this.finish(code, &kind);
                                break;
                            }
                        }
                    }
                });
            }
            Err(message) => {
                self.busy.set(false);
                self.pid.set(None);
                self.spinner.stop();
                self.spinner.set_visible(false);
                self.toast(&message);
                self.stack.set_visible_child_name("card");
                self.set_busy_controls(true);
            }
        }
    }

    fn on_line(&self, text: &str) {
        self.log.append(text);
        if let Some(phase) = phase_index(text) {
            if phase > self.phase.get() {
                self.phase.set(phase);
                self.busy_label.set_text(phase_label(phase));
            }
        }
    }

    fn finish(self: &Rc<Self>, code: i32, kind: &RunKind) {
        self.busy.set(false);
        self.pid.set(None);
        self.spinner.stop();
        self.spinner.set_visible(false);

        let output = self.log.text();
        if code == 0 {
            match kind {
                RunKind::Install {
                    name,
                    version,
                    action,
                } => {
                    let verb = match action {
                        apt::Action::Upgrade => "Upgraded",
                        apt::Action::Downgrade => "Downgraded",
                        apt::Action::Reinstall => "Reinstalled",
                        apt::Action::Install => "Installed",
                    };
                    self.toast(&format!("{verb} {name} {version}"));
                }
                RunKind::Remove { name } => self.toast(&format!("Removed {name}")),
                RunKind::Update => self.toast("Package lists refreshed"),
            }
            self.banner.set_revealed(false);
            self.banner_mode.set(BannerMode::None);
            self.refresh_queries();
        } else {
            let (class, hint) = apt::classify(code, &output);
            if class == apt::ErrorClass::Auth {
                self.toast("Installation cancelled");
            } else {
                self.show_banner(&hint, BannerMode::Details);
            }
        }

        self.stack.set_visible_child_name("card");
        self.set_busy_controls(true);
    }

    fn pick_file(self: &Rc<Self>) {
        let dialog = gtk::FileDialog::builder()
            .title("Choose a .deb package")
            .modal(true)
            .build();

        if let Some(dir) = self.last_dir.borrow().as_ref() {
            dialog.set_initial_folder(Some(&gio::File::for_path(dir)));
        }

        let filter = gtk::FileFilter::new();
        filter.set_name(Some("Debian packages"));
        filter.add_suffix("deb");
        filter.add_pattern("*.deb");
        let filters = gio::ListStore::new::<gtk::FileFilter>();
        filters.append(&filter);
        dialog.set_filters(Some(&filters));
        dialog.set_default_filter(Some(&filter));

        let this = Rc::clone(self);
        dialog.open(Some(&self.window), gio::Cancellable::NONE, move |result| {
            if let Ok(file) = result {
                if let Some(path) = file.path() {
                    this.load_path(path);
                }
            }
        });
    }

    fn show_details_dialog(&self) {
        let text = {
            let log = self.log.text();
            if log.trim().is_empty() {
                self.plan
                    .borrow()
                    .as_ref()
                    .map(|p| p.detail.clone())
                    .unwrap_or_default()
            } else {
                log
            }
        };
        if text.trim().is_empty() {
            self.toast("Nothing to show yet");
            return;
        }

        let dialog = adw::Dialog::builder()
            .title("Last operation")
            .content_width(780)
            .content_height(540)
            .build();
        let buffer = gtk::TextBuffer::new(None);
        buffer.set_text(&text);
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
        let scrolled = gtk::ScrolledWindow::builder()
            .child(&view)
            .hexpand(true)
            .vexpand(true)
            .build();
        scrolled.add_css_class("card");
        let wrapper = gtk::Box::new(gtk::Orientation::Vertical, 0);
        wrapper.set_margin_top(10);
        wrapper.set_margin_bottom(10);
        wrapper.set_margin_start(10);
        wrapper.set_margin_end(10);
        wrapper.append(&scrolled);
        dialog.set_child(Some(&wrapper));
        dialog.present(Some(&self.window));
    }

    fn show_scripts_dialog(&self) {
        let Some((scripts, package)) = self.info.borrow().as_ref().map(|i| {
            let scripts: Vec<(String, String)> = i
                .scripts
                .iter()
                .map(|name| {
                    let text = i
                        .script_text(name)
                        .unwrap_or_else(|e| format!("Could not read {name}: {e}"));
                    (name.clone(), text)
                })
                .collect();
            (scripts, i.package.clone())
        }) else {
            return;
        };
        if scripts.is_empty() {
            return;
        }

        let dialog = adw::Dialog::builder()
            .title(format!("{package}: maintainer scripts"))
            .content_width(780)
            .content_height(560)
            .build();

        let names: Vec<&str> = scripts.iter().map(|(n, _)| n.as_str()).collect();
        let model = gtk::StringList::new(&names);
        let dropdown = gtk::DropDown::builder().model(&model).selected(0).build();

        let buffer = gtk::TextBuffer::new(None);
        buffer.set_text(&scripts[0].1);
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

        let scrolled = gtk::ScrolledWindow::builder()
            .child(&view)
            .hexpand(true)
            .vexpand(true)
            .build();
        scrolled.add_css_class("card");

        {
            let buffer = buffer.clone();
            dropdown.connect_selected_notify(move |dropdown| {
                if let Some((_, text)) = scripts.get(dropdown.selected() as usize) {
                    buffer.set_text(text);
                }
            });
        }

        let wrapper = gtk::Box::new(gtk::Orientation::Vertical, 10);
        wrapper.set_margin_top(10);
        wrapper.set_margin_bottom(10);
        wrapper.set_margin_start(10);
        wrapper.set_margin_end(10);
        let note = gtk::Label::new(Some(
            "These scripts run as root during installation. Read them before installing.",
        ));
        note.set_wrap(true);
        note.set_xalign(0.0);
        note.add_css_class("debdrop-meta");
        wrapper.append(&note);
        wrapper.append(&dropdown);
        wrapper.append(&scrolled);
        dialog.set_child(Some(&wrapper));
        dialog.present(Some(&self.window));
    }

    fn show_about(&self) {
        let about = adw::AboutDialog::builder()
            .application_name("DebDrop")
            .application_icon("debdrop")
            .developer_name("DebDrop")
            .version(env!("CARGO_PKG_VERSION"))
            .comments(
                "Inspect and install local .deb packages.\n\nA .deb runs maintainer scripts as root. DebDrop shows them and never installs without a click and a password prompt.",
            )
            .license_type(gtk::License::MitX11)
            .build();
        about.present(Some(&self.window));
    }

    fn open_homepage(&self) {
        let binding = self.info.borrow();
        let Some(info) = binding.as_ref() else {
            return;
        };
        let Some(url) = &info.homepage else {
            return;
        };
        let ctx = gio::AppLaunchContext::NONE;
        if let Err(e) = gio::AppInfo::launch_default_for_uri(url, ctx) {
            self.toast(&format!("Could not open website: {e}"));
        }
    }

    fn launch_app(&self) {
        let (exec, title) = {
            let binding = self.info.borrow();
            let Some(info) = binding.as_ref() else {
                return;
            };
            let Some(desktop) = &info.desktop else {
                return;
            };
            let Some(exec) = &desktop.exec else {
                return;
            };
            (exec.clone(), info.display_title())
        };
        let mut parts = exec.split_whitespace();
        let Some(bin) = parts.next() else {
            return;
        };
        let args: Vec<_> = parts.collect();
        match std::process::Command::new(bin).args(&args).spawn() {
            Ok(_) => self.toast(&format!("Launched {title}")),
            Err(e) => self.toast(&format!("Failed to launch {bin}: {e}")),
        }
    }

    fn show_files_dialog(&self) {
        let (files, title) = {
            let borrow = self.info.borrow();
            let Some(info) = borrow.as_ref() else {
                return;
            };
            (info.files.clone(), info.display_title())
        };

        if files.is_empty() {
            self.toast("No package file information available");
            return;
        }

        let dialog = adw::Dialog::builder()
            .title(format!("Contents: {title}"))
            .content_width(720)
            .content_height(500)
            .build();

        let header = adw::HeaderBar::new();
        header.set_show_end_title_buttons(true);

        let search_entry = gtk::SearchEntry::new();
        search_entry.set_hexpand(true);
        search_entry.set_placeholder_text(Some("Filter files…"));

        let count_label = gtk::Label::new(Some(&format!("{} files", files.len())));
        count_label.add_css_class("dim-label");
        count_label.set_margin_end(8);

        let top_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        top_box.set_margin_top(8);
        top_box.set_margin_bottom(8);
        top_box.set_margin_start(12);
        top_box.set_margin_end(12);
        top_box.append(&search_entry);
        top_box.append(&count_label);

        let buffer = gtk::TextBuffer::new(None);
        buffer.set_text(&files.join("\n"));

        let view = gtk::TextView::builder()
            .buffer(&buffer)
            .editable(false)
            .cursor_visible(false)
            .monospace(true)
            .wrap_mode(gtk::WrapMode::None)
            .top_margin(8)
            .bottom_margin(8)
            .left_margin(12)
            .right_margin(12)
            .build();
        view.add_css_class("debdrop-file-tree");

        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Automatic)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .child(&view)
            .vexpand(true)
            .hexpand(true)
            .build();
        scrolled.add_css_class("card");
        scrolled.set_margin_start(12);
        scrolled.set_margin_end(12);
        scrolled.set_margin_bottom(12);

        {
            let buffer = buffer.clone();
            let all_files = files.clone();
            let count_label = count_label.clone();
            search_entry.connect_search_changed(move |entry| {
                let query = entry.text().to_lowercase();
                if query.trim().is_empty() {
                    buffer.set_text(&all_files.join("\n"));
                    count_label.set_text(&format!("{} files", all_files.len()));
                } else {
                    let filtered: Vec<&str> = all_files
                        .iter()
                        .filter(|f| f.to_lowercase().contains(&query))
                        .map(String::as_str)
                        .collect();
                    buffer.set_text(&filtered.join("\n"));
                    count_label.set_text(&format!("{} of {} files", filtered.len(), all_files.len()));
                }
            });
        }

        let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
        content.append(&header);
        content.append(&top_box);
        content.append(&scrolled);

        dialog.set_child(Some(&content));
        dialog.present(Some(&self.window));
    }
}

fn first_scrolled_window(widget: &gtk::Widget) -> Option<gtk::ScrolledWindow> {
    if let Some(scrolled) = widget.downcast_ref::<gtk::ScrolledWindow>() {
        return Some(scrolled.clone());
    }
    let mut child = widget.first_child();
    while let Some(current) = child {
        if let Some(found) = first_scrolled_window(&current) {
            return Some(found);
        }
        child = current.next_sibling();
    }
    None
}

fn phase_index(line: &str) -> Option<u8> {
    if line.contains("Unpacking")
        || line.contains("Preparing to unpack")
        || line.contains("Selecting previously")
    {
        return Some(3);
    }
    if line.contains("Setting up") || line.contains("Processing triggers") {
        return Some(4);
    }
    if line.starts_with("Get:") || line.contains("Need to get") || line.contains("Fetching") {
        return Some(2);
    }
    if line.contains("Reading package lists") || line.contains("Building dependency tree") {
        return Some(1);
    }
    None
}

fn phase_label(phase: u8) -> &'static str {
    match phase {
        1 => "Resolving dependencies…",
        2 => "Downloading packages…",
        3 => "Unpacking…",
        4 => "Configuring…",
        _ => "Working…",
    }
}
