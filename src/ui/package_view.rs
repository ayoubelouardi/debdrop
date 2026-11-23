use std::cell::{Cell, RefCell};
use std::rc::Rc;

use adw::prelude::*;
use gtk::prelude::*;
use libadwaita as adw;

use crate::{apt, deb};

const MAX_DEP_ROWS: usize = 30;

/// AdwActionRow titles and subtitles are Pango markup, so every value that
/// comes from a package must be escaped. A maintainer address like
/// `<test@example.invalid>` is otherwise parsed as a tag and dropped.
fn markup(text: &str) -> String {
    gtk::glib::markup_escape_text(text).to_string()
}

struct DepRow {
    icon: &'static str,
    title: String,
    subtitle: String,
    warning: bool,
}

/// The dependencies group and its Show/Hide toggle. The toggle lives in the
/// group header, not in a list row: a GtkListBox in libadwaita does not pass
/// Tab on to its rows, so rows never receive keyboard focus.
struct Deps {
    group: adw::PreferencesGroup,
    button: gtk::Button,
    rows: RefCell<Vec<adw::ActionRow>>,
    data: RefCell<Vec<DepRow>>,
    expanded: Cell<bool>,
}

impl Deps {
    fn new() -> Rc<Self> {
        let group = adw::PreferencesGroup::new();
        group.set_title("Dependencies");
        group.set_description(Some("Checking dependencies…"));

        let button = gtk::Button::with_label("Show");
        button.add_css_class("flat");
        button.set_valign(gtk::Align::Center);
        button.set_visible(false);
        group.set_header_suffix(Some(&button));

        let deps = Rc::new(Self {
            group,
            button,
            rows: RefCell::new(Vec::new()),
            data: RefCell::new(Vec::new()),
            expanded: Cell::new(true),
        });

        let weak = Rc::downgrade(&deps);
        deps.button.connect_clicked(move |_| {
            if let Some(deps) = weak.upgrade() {
                deps.expanded.set(!deps.expanded.get());
                deps.render();
            }
        });
        deps
    }

    fn reset(&self, description: &str) {
        *self.data.borrow_mut() = Vec::new();
        self.render();
        self.group.set_description(Some(&markup(description)));
        self.button.set_visible(false);
    }

    fn set(&self, description: &str, data: Vec<DepRow>, expand: bool) {
        self.group.set_description(Some(&markup(description)));
        self.expanded.set(expand);
        self.button.set_visible(!data.is_empty());
        *self.data.borrow_mut() = data;
        self.render();
    }

    fn render(&self) {
        for row in self.rows.borrow().iter() {
            self.group.remove(row);
        }
        self.rows.borrow_mut().clear();

        if !self.expanded.get() {
            self.button.set_label("Show");
            return;
        }
        self.button.set_label("Hide");
        for item in self.data.borrow().iter() {
            let row = adw::ActionRow::builder()
                .title(markup(&item.title))
                .subtitle(markup(&item.subtitle))
                .build();
            row.set_title_lines(0);
            row.set_can_focus(false);
            row.set_focusable(false);
            let icon = gtk::Image::from_icon_name(item.icon);
            if item.warning {
                icon.add_css_class("debdrop-warning");
            }
            row.add_prefix(&icon);
            self.group.add(&row);
            self.rows.borrow_mut().push(row);
        }
    }
}

pub struct PackageCard {
    pub page: adw::PreferencesPage,

    // Hero Header
    app_icon: gtk::Image,
    hero_title: gtk::Label,
    hero_pkg: gtk::Label,
    version_chip: gtk::Label,
    arch_chip: gtk::Label,
    size_chip: gtk::Label,
    status_chip: gtk::Label,

    // Details rows
    file_row: adw::ActionRow,
    maintainer_row: adw::ActionRow,
    homepage_row: adw::ActionRow,
    homepage_button: gtk::Button,
    files_row: adw::ActionRow,
    files_button: gtk::Button,
    description_row: adw::ActionRow,
    debconf_row: adw::ActionRow,

    deps: Rc<Deps>,
    scripts_group: adw::PreferencesGroup,
    scripts_row: adw::ActionRow,
    scripts_icon: gtk::Image,
    scripts_button: gtk::Button,
    path: Rc<RefCell<String>>,
}

impl PackageCard {
    pub fn new() -> Self {
        let page = adw::PreferencesPage::new();

        // 1. Hero Header Group
        let hero_group = adw::PreferencesGroup::new();
        let hero_box = gtk::Box::new(gtk::Orientation::Horizontal, 16);
        hero_box.add_css_class("debdrop-hero");

        let app_icon = gtk::Image::new();
        app_icon.set_pixel_size(64);
        app_icon.add_css_class("debdrop-app-icon");
        hero_box.append(&app_icon);

        let hero_text_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
        hero_text_box.set_hexpand(true);
        hero_text_box.set_valign(gtk::Align::Center);

        let hero_title = gtk::Label::new(None);
        hero_title.add_css_class("title-1");
        hero_title.set_halign(gtk::Align::Start);
        hero_title.set_wrap(true);
        hero_text_box.append(&hero_title);

        let hero_pkg = gtk::Label::new(None);
        hero_pkg.add_css_class("dim-label");
        hero_pkg.set_halign(gtk::Align::Start);
        hero_text_box.append(&hero_pkg);

        let chips_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        chips_box.set_margin_top(4);

        let version_chip = gtk::Label::new(None);
        version_chip.add_css_class("debdrop-chip");

        let arch_chip = gtk::Label::new(None);
        arch_chip.add_css_class("debdrop-chip");

        let size_chip = gtk::Label::new(None);
        size_chip.add_css_class("debdrop-chip");

        let status_chip = gtk::Label::new(None);
        status_chip.add_css_class("debdrop-chip");
        status_chip.add_css_class("debdrop-status-ready");

        chips_box.append(&version_chip);
        chips_box.append(&arch_chip);
        chips_box.append(&size_chip);
        chips_box.append(&status_chip);

        hero_text_box.append(&chips_box);
        hero_box.append(&hero_text_box);
        hero_group.add(&hero_box);

        // 2. Details Group
        let group = adw::PreferencesGroup::new();
        group.set_title("Package Details");

        let file_row = adw::ActionRow::builder().title("File").build();
        file_row.set_subtitle_lines(2);
        file_row.add_css_class("debdrop-path");

        let path = Rc::new(RefCell::new(String::new()));
        let copy_button = gtk::Button::from_icon_name("edit-copy-symbolic");
        copy_button.add_css_class("flat");
        copy_button.set_tooltip_text(Some("Copy file path"));
        copy_button.set_valign(gtk::Align::Center);
        copy_button.update_property(&[gtk::accessible::Property::Label("Copy file path")]);
        {
            let path = Rc::clone(&path);
            let button = copy_button.clone();
            copy_button.connect_clicked(move |_| {
                if let Some(display) = gtk::gdk::Display::default() {
                    display.clipboard().set_text(&path.borrow());
                }
                button.set_tooltip_text(Some("Copied"));
            });
        }
        file_row.add_suffix(&copy_button);

        let maintainer_row = adw::ActionRow::builder().title("Maintainer").build();
        maintainer_row.set_subtitle_lines(2);

        let homepage_row = adw::ActionRow::builder().title("Homepage").build();
        homepage_row.set_subtitle_lines(1);
        let homepage_button = gtk::Button::from_icon_name("web-browser-symbolic");
        homepage_button.add_css_class("flat");
        homepage_button.set_valign(gtk::Align::Center);
        homepage_button.set_tooltip_text(Some("Open homepage in browser"));
        homepage_row.add_suffix(&homepage_button);

        let files_row = adw::ActionRow::builder().title("Contents").build();
        files_row.set_subtitle_lines(1);
        let files_button = gtk::Button::with_label("Inspect…");
        files_button.add_css_class("flat");
        files_button.set_valign(gtk::Align::Center);
        files_button.set_tooltip_text(Some("View files installed by this package"));
        files_row.add_suffix(&files_button);

        let description_row = adw::ActionRow::builder().title("Description").build();
        description_row.set_subtitle_lines(8);

        let debconf_row = adw::ActionRow::builder()
            .title("Configuration")
            .subtitle("Asks configuration questions; defaults will be used")
            .build();
        debconf_row.set_subtitle_lines(2);

        group.add(&file_row);
        group.add(&maintainer_row);
        group.add(&homepage_row);
        group.add(&files_row);
        group.add(&description_row);
        group.add(&debconf_row);

        for row in [
            &file_row,
            &maintainer_row,
            &homepage_row,
            &files_row,
            &description_row,
            &debconf_row,
        ] {
            row.set_can_focus(false);
            row.set_focusable(false);
        }

        let deps = Deps::new();

        // 3. Security & Scripts Group
        let scripts_group = adw::PreferencesGroup::new();
        scripts_group.set_title("Security & Scripts");
        let scripts_row = adw::ActionRow::builder().title("Maintainer scripts").build();
        scripts_row.set_subtitle_lines(2);
        scripts_row.set_can_focus(false);
        scripts_row.set_focusable(false);

        let scripts_icon = gtk::Image::from_icon_name("dialog-warning-symbolic");
        scripts_row.add_prefix(&scripts_icon);

        let scripts_button = gtk::Button::with_label("View…");
        scripts_button.add_css_class("flat");
        scripts_button.set_valign(gtk::Align::Center);
        scripts_button.set_tooltip_text(Some("Show the scripts that will run as root"));
        scripts_group.set_header_suffix(Some(&scripts_button));

        scripts_group.add(&scripts_row);

        page.add(&hero_group);
        page.add(&group);
        page.add(&deps.group);
        page.add(&scripts_group);

        Self {
            page,
            app_icon,
            hero_title,
            hero_pkg,
            version_chip,
            arch_chip,
            size_chip,
            status_chip,
            file_row,
            maintainer_row,
            homepage_row,
            homepage_button,
            files_row,
            files_button,
            description_row,
            debconf_row,
            deps,
            scripts_group,
            scripts_row,
            scripts_icon,
            scripts_button,
            path,
        }
    }

    pub fn scripts_button(&self) -> gtk::Button {
        self.scripts_button.clone()
    }

    pub fn homepage_button(&self) -> gtk::Button {
        self.homepage_button.clone()
    }

    pub fn files_button(&self) -> gtk::Button {
        self.files_button.clone()
    }

    pub fn show(&self, info: &deb::DebInfo) {
        // App Icon
        if let Some(desktop) = &info.desktop {
            if let Some(path) = &desktop.icon_path {
                self.app_icon.set_from_file(Some(path));
            } else if let Some(name) = &desktop.icon_name {
                self.app_icon.set_icon_name(Some(name));
            } else {
                self.app_icon
                    .set_icon_name(Some(info.fallback_icon()));
            }
        } else {
            self.app_icon
                .set_icon_name(Some(info.fallback_icon()));
        }

        // Hero Titles
        let title = info.display_title();
        self.hero_title.set_text(&title);
        let mut meta_sub = vec![info.architecture.clone()];
        if let Some(section) = &info.section {
            meta_sub.push(section.clone());
        }
        if let Some(priority) = &info.priority {
            meta_sub.push(priority.clone());
        }
        if title != info.package {
            self.hero_pkg
                .set_text(&format!("{} · {}", info.package, meta_sub.join(" · ")));
        } else {
            self.hero_pkg.set_text(&meta_sub.join(" · "));
        }

        // Badges
        self.version_chip.set_text(&format!("v{}", info.version));
        self.arch_chip.set_text(&info.architecture);
        if let Some(size) = info.display_size() {
            self.size_chip.set_text(&size);
            self.size_chip.set_visible(true);
        } else {
            self.size_chip.set_visible(false);
        }

        self.status_chip.set_text("Checking…");
        self.status_chip
            .set_css_classes(&["debdrop-chip", "debdrop-status-ready"]);

        // Details rows
        self.file_row
            .set_subtitle(&markup(&info.path.display().to_string()));
        *self.path.borrow_mut() = info.path.display().to_string();

        match &info.maintainer {
            Some(maintainer) => {
                self.maintainer_row.set_subtitle(&markup(maintainer));
                self.maintainer_row.set_visible(true);
            }
            None => self.maintainer_row.set_visible(false),
        }

        match &info.homepage {
            Some(url) => {
                self.homepage_row.set_subtitle(&markup(url));
                self.homepage_row.set_visible(true);
            }
            None => self.homepage_row.set_visible(false),
        }

        let file_count = info.files.len();
        self.files_row
            .set_subtitle(&format!("{file_count} files installed in system"));

        let description = info
            .description_long
            .as_deref()
            .unwrap_or(&info.description)
            .trim()
            .to_string();
        if description.is_empty() {
            self.description_row.set_visible(false);
        } else {
            self.description_row.set_subtitle(&markup(&description));
            self.description_row.set_visible(true);
        }

        // Security indicator
        if info.scripts.is_empty() {
            self.scripts_icon
                .set_icon_name(Some("emblem-ok-symbolic"));
            self.scripts_icon
                .set_css_classes(&["debdrop-security-clean"]);
            self.scripts_row
                .set_css_classes(&["debdrop-security-clean-box"]);
            self.scripts_row.set_title("Clean package");
            self.scripts_row
                .set_subtitle("No root maintainer scripts detected");
            self.scripts_button.set_visible(false);
            self.scripts_group.set_visible(true);
        } else {
            self.scripts_icon
                .set_icon_name(Some("dialog-warning-symbolic"));
            self.scripts_icon.set_css_classes(&["debdrop-warning"]);
            self.scripts_row.set_css_classes(&["debdrop-security-box"]);
            self.scripts_row
                .set_title(&markup(&info.scripts.join(", ")));
            self.scripts_row
                .set_subtitle("Runs with root privileges during installation");
            self.scripts_button.set_visible(true);
            self.scripts_group.set_visible(true);
        }

        self.debconf_row.set_visible(info.has_debconf);
        self.deps.reset("Checking dependencies…");
    }

    pub fn set_installed(&self, state: &apt::InstalledState, action: apt::Action) {
        let (badge_text, badge_class) = if state.installed {
            match action {
                apt::Action::Upgrade => ("Upgrade Available".to_string(), "debdrop-status-upgrade"),
                apt::Action::Downgrade => {
                    ("Downgrade Available".to_string(), "debdrop-status-downgrade")
                }
                _ => ("Installed".to_string(), "debdrop-status-installed"),
            }
        } else if let Some(note) = &state.note {
            (format!("Not installed ({note})"), "debdrop-status-ready")
        } else {
            ("Ready to Install".to_string(), "debdrop-status-ready")
        };

        self.status_chip.set_text(&badge_text);
        self.status_chip
            .set_css_classes(&["debdrop-chip", badge_class]);
    }

    pub fn set_plan(&self, info: &deb::DebInfo, plan: &apt::Plan) {
        let mut data = Vec::new();
        for dep in info.depends.iter().take(MAX_DEP_ROWS) {
            let subtitle = match dep.alternatives_label() {
                Some(alts) => format!("or {alts}"),
                None => "required".to_string(),
            };
            data.push(DepRow {
                icon: "application-x-addon-symbolic",
                title: dep.label(),
                subtitle,
                warning: false,
            });
        }
        if info.depends.len() > MAX_DEP_ROWS {
            let more = info.depends.len() - MAX_DEP_ROWS;
            data.push(DepRow {
                icon: "view-more-symbolic",
                title: format!("{more} more required"),
                subtitle: "not listed here".to_string(),
                warning: false,
            });
        }
        for dep in info.recommends.iter().take(5) {
            data.push(DepRow {
                icon: "emblem-ok-symbolic",
                title: dep.label(),
                subtitle: "recommended (not installed by default)".to_string(),
                warning: false,
            });
        }
        for name in &plan.unresolved {
            data.push(DepRow {
                icon: "dialog-warning-symbolic",
                title: name.clone(),
                subtitle: "not found".to_string(),
                warning: true,
            });
        }
        for name in &plan.new {
            data.push(DepRow {
                icon: "list-add-symbolic",
                title: name.clone(),
                subtitle: "will be installed".to_string(),
                warning: false,
            });
        }
        for name in &plan.upgraded {
            data.push(DepRow {
                icon: "software-update-available-symbolic",
                title: name.clone(),
                subtitle: "will be upgraded".to_string(),
                warning: false,
            });
        }
        for name in &plan.downgraded {
            data.push(DepRow {
                icon: "software-update-available-symbolic",
                title: name.clone(),
                subtitle: "will be downgraded".to_string(),
                warning: false,
            });
        }
        for name in &plan.removed {
            data.push(DepRow {
                icon: "list-remove-symbolic",
                title: name.clone(),
                subtitle: "will be removed".to_string(),
                warning: true,
            });
        }

        let expand = plan.has_rows() || !info.depends.is_empty();
        self.deps.set(&plan.headline(), data, expand);
    }

    pub fn set_blocked(&self, message: &str) {
        self.status_chip.set_text("Blocked");
        self.status_chip
            .set_css_classes(&["debdrop-chip", "debdrop-status-blocked"]);
        self.deps.reset(message);
    }
}

impl Default for PackageCard {
    fn default() -> Self {
        Self::new()
    }
}
