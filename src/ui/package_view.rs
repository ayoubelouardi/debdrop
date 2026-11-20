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
    title_row: adw::ActionRow,
    file_row: adw::ActionRow,
    maintainer_row: adw::ActionRow,
    description_row: adw::ActionRow,
    status_row: adw::ActionRow,
    debconf_row: adw::ActionRow,
    deps: Rc<Deps>,
    scripts_group: adw::PreferencesGroup,
    scripts_row: adw::ActionRow,
    scripts_button: gtk::Button,
    path: Rc<RefCell<String>>,
}

impl PackageCard {
    pub fn new() -> Self {
        let page = adw::PreferencesPage::new();

        let group = adw::PreferencesGroup::new();
        group.set_title("Package");

        let title_row = adw::ActionRow::builder().title("…").build();
        title_row.set_title_lines(1);
        title_row.set_subtitle_lines(1);

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
        group.set_header_suffix(Some(&copy_button));

        let maintainer_row = adw::ActionRow::builder().title("Maintainer").build();
        maintainer_row.set_subtitle_lines(2);

        let description_row = adw::ActionRow::builder().title("Description").build();
        description_row.set_subtitle_lines(6);

        let status_row = adw::ActionRow::builder()
            .title("Status")
            .subtitle("Checking installed state…")
            .build();

        let debconf_row = adw::ActionRow::builder()
            .title("Configuration")
            .subtitle("Asks configuration questions; defaults will be used")
            .build();
        debconf_row.set_subtitle_lines(2);

        group.add(&title_row);
        group.add(&file_row);
        group.add(&maintainer_row);
        group.add(&description_row);
        group.add(&status_row);
        group.add(&debconf_row);

        for row in [
            &title_row,
            &file_row,
            &maintainer_row,
            &description_row,
            &status_row,
            &debconf_row,
        ] {
            row.set_can_focus(false);
            row.set_focusable(false);
        }

        let deps = Deps::new();

        let scripts_group = adw::PreferencesGroup::new();
        scripts_group.set_title("Maintainer scripts");
        let scripts_row = adw::ActionRow::builder()
            .title("Runs as root during installation")
            .build();
        scripts_row.set_subtitle_lines(2);
        scripts_row.set_can_focus(false);
        scripts_row.set_focusable(false);
        let warning = gtk::Image::from_icon_name("dialog-warning-symbolic");
        warning.add_css_class("debdrop-warning");
        scripts_row.add_prefix(&warning);
        scripts_group.add(&scripts_row);
        let scripts_button = gtk::Button::with_label("View…");
        scripts_button.add_css_class("flat");
        scripts_button.set_valign(gtk::Align::Center);
        scripts_button.set_tooltip_text(Some("Show the scripts that will run as root"));
        scripts_group.set_header_suffix(Some(&scripts_button));
        scripts_group.set_visible(false);

        page.add(&group);
        page.add(&deps.group);
        page.add(&scripts_group);

        Self {
            page,
            title_row,
            file_row,
            maintainer_row,
            description_row,
            status_row,
            debconf_row,
            deps,
            scripts_group,
            scripts_row,
            scripts_button,
            path,
        }
    }

    pub fn scripts_button(&self) -> gtk::Button {
        self.scripts_button.clone()
    }

    pub fn show(&self, info: &deb::DebInfo) {
        self.title_row.set_title(&markup(&info.package));
        let mut meta = vec![markup(&info.architecture)];
        if let Some(section) = &info.section {
            meta.push(markup(section));
        }
        if let Some(priority) = &info.priority {
            meta.push(markup(&format!("{priority} priority")));
        }
        if let Some(size) = info.display_size() {
            meta.push(markup(&format!("{size} installed")));
        }
        self.title_row.set_subtitle(&meta.join(" · "));
        self.title_row
            .set_tooltip_text(Some(&info.path.display().to_string()));

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

        if info.scripts.is_empty() {
            self.scripts_group.set_visible(false);
        } else {
            self.scripts_row
                .set_title(&markup(&info.scripts.join(", ")));
            self.scripts_row
                .set_subtitle("Runs with root privileges during installation");
            self.scripts_group.set_visible(true);
        }
        self.debconf_row.set_visible(info.has_debconf);

        self.status_row.set_subtitle("Checking installed state…");
        self.deps.reset("Checking dependencies…");
    }

    pub fn set_installed(&self, state: &apt::InstalledState) {
        let text = if state.installed {
            format!(
                "Installed {}",
                state.version.as_deref().unwrap_or("(unknown version)")
            )
        } else if let Some(note) = &state.note {
            format!("Not installed ({note})")
        } else {
            "Not installed".to_string()
        };
        self.status_row.set_subtitle(&markup(&text));
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
        self.deps.reset(message);
    }
}

impl Default for PackageCard {
    fn default() -> Self {
        Self::new()
    }
}
