//! Reading and validating Debian binary packages.
//!
//! Every field comes from `dpkg-deb`; the file name is never trusted for
//! metadata. Loading is unprivileged and must stay that way.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

/// Maintainer scripts we are willing to show. Anything else in the control
/// archive is ignored, including members with hostile names.
const KNOWN_SCRIPTS: [&str; 4] = ["preinst", "postinst", "prerm", "postrm"];

const MAX_SCRIPT_BYTES: u64 = 256 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dep {
    pub name: String,
    pub constraint: Option<String>,
    pub alternatives: Vec<String>,
}

impl Dep {
    pub fn label(&self) -> String {
        match &self.constraint {
            Some(c) => format!("{} ({})", self.name, c),
            None => self.name.clone(),
        }
    }

    pub fn alternatives_label(&self) -> Option<String> {
        if self.alternatives.is_empty() {
            None
        } else {
            Some(self.alternatives.join(" | "))
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DesktopInfo {
    pub display_name: Option<String>,
    pub generic_name: Option<String>,
    pub icon_name: Option<String>,
    pub icon_path: Option<PathBuf>,
    pub exec: Option<String>,
}

pub struct DebInfo {
    pub path: PathBuf,
    pub package: String,
    pub version: String,
    pub architecture: String,
    pub section: Option<String>,
    pub priority: Option<String>,
    pub maintainer: Option<String>,
    pub homepage: Option<String>,
    pub installed_size_kb: Option<u64>,
    pub description: String,
    pub description_long: Option<String>,
    pub depends: Vec<Dep>,
    pub recommends: Vec<Dep>,
    pub scripts: Vec<String>,
    pub has_debconf: bool,
    pub desktop: Option<DesktopInfo>,
    pub files: Vec<String>,
    ctrl: TempDir,
}

impl DebInfo {
    /// Read one of the maintainer scripts. The name must be one of the four
    /// known scripts; the path is built from the extracted control directory.
    pub fn script_text(&self, name: &str) -> Result<String, String> {
        if !KNOWN_SCRIPTS.contains(&name) {
            return Err(format!("unknown script {name}"));
        }
        let path = self.ctrl.path().join(name);
        let meta = fs::metadata(&path).map_err(|e| format!("{name}: {e}"))?;
        let mut bytes = fs::read(&path).map_err(|e| format!("{name}: {e}"))?;
        if meta.len() > MAX_SCRIPT_BYTES {
            bytes.truncate(MAX_SCRIPT_BYTES as usize);
            let mut text = String::from_utf8_lossy(&bytes).into_owned();
            text.push_str("\n\n[truncated by DebDrop]\n");
            return Ok(text);
        }
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    pub fn display_size(&self) -> Option<String> {
        self.installed_size_kb.map(|kb| {
            if kb >= 1024 {
                format!("{:.1} MiB", kb as f64 / 1024.0)
            } else {
                format!("{kb} KiB")
            }
        })
    }

    /// Primary display title: desktop Name if available, otherwise package name.
    pub fn display_title(&self) -> String {
        self.desktop
            .as_ref()
            .and_then(|d| d.display_name.clone())
            .unwrap_or_else(|| self.package.clone())
    }

    /// Intelligent fallback symbolic icon based on package section.
    pub fn fallback_icon(&self) -> &'static str {
        match self.section.as_deref().unwrap_or("").trim() {
            "admin" | "utils" | "shells" => "utilities-terminal-symbolic",
            "devel" | "libdevel" | "libs" => "applications-development-symbolic",
            "net" | "web" => "applications-internet-symbolic",
            "graphics" => "applications-graphics-symbolic",
            "sound" | "video" => "applications-multimedia-symbolic",
            "games" => "applications-games-symbolic",
            "editors" => "accessories-text-editor-symbolic",
            "doc" => "help-browser-symbolic",
            _ => "package-x-generic",
        }
    }
}

#[derive(Debug)]
pub enum DebError {
    Io(String),
    NoDpkg,
    NotADeb(String),
    Invalid(String),
}

impl fmt::Display for DebError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DebError::Io(m) => write!(f, "{m}"),
            DebError::NoDpkg => write!(f, "dpkg-deb was not found; is this a Debian system?"),
            DebError::NotADeb(m) => {
                if m.trim().is_empty() {
                    write!(f, "This file is not a valid .deb package")
                } else {
                    write!(f, "This file is not a valid .deb package: {m}")
                }
            }
            DebError::Invalid(m) => write!(f, "Invalid package: {m}"),
        }
    }
}

impl std::error::Error for DebError {}

pub fn load(path: &Path) -> Result<DebInfo, DebError> {
    let path =
        fs::canonicalize(path).map_err(|e| DebError::Io(format!("{}: {e}", path.display())))?;

    let meta = fs::metadata(&path).map_err(|e| DebError::Io(format!("{}: {e}", path.display())))?;
    if !meta.is_file() {
        return Err(DebError::Invalid("not a regular file".into()));
    }
    let is_deb = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("deb"))
        .unwrap_or(false);
    if !is_deb {
        return Err(DebError::NotADeb("only .deb files are supported".into()));
    }

    // Validity gate: if dpkg-deb cannot read the archive, nothing else runs.
    let info = run_dpkg_deb(&["--info"], &path)?;
    if !info.status.success() {
        return Err(DebError::NotADeb(
            String::from_utf8_lossy(&info.stderr).trim().to_string(),
        ));
    }

    let ctrl_out = run_dpkg_deb(&["-f"], &path)?;
    if !ctrl_out.status.success() {
        return Err(DebError::Invalid(
            String::from_utf8_lossy(&ctrl_out.stderr).trim().to_string(),
        ));
    }
    let fields = parse_control(&String::from_utf8_lossy(&ctrl_out.stdout));

    let package = require(&fields, "package")?;
    if !valid_package_name(&package) {
        return Err(DebError::Invalid(format!("bad package name {package:?}")));
    }
    let version = require(&fields, "version")?;
    let architecture = fields
        .get("architecture")
        .cloned()
        .unwrap_or_else(|| "unknown".to_string());

    let description_raw = fields.get("description").cloned().unwrap_or_default();
    let (description, description_long) = split_description(&description_raw);

    let ctrl = TempDir::new().map_err(|e| DebError::Io(format!("temp dir: {e}")))?;
    let extracted = Command::new("dpkg-deb")
        .env("LC_ALL", "C")
        .arg("-e")
        .arg(&path)
        .arg(ctrl.path())
        .output()
        .map_err(|e| DebError::Io(format!("dpkg-deb -e: {e}")))?;
    if !extracted.status.success() {
        return Err(DebError::Invalid(
            String::from_utf8_lossy(&extracted.stderr)
                .trim()
                .to_string(),
        ));
    }

    let mut scripts = Vec::new();
    if let Ok(entries) = fs::read_dir(ctrl.path()) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if KNOWN_SCRIPTS.contains(&name.as_str()) {
                scripts.push(name);
            }
        }
    }
    scripts.sort();
    let has_debconf = ctrl.path().join("config").exists() || ctrl.path().join("templates").exists();

    // Inspect data payload file list and desktop integration
    let raw_files = list_archive_members(&path);
    let files = clean_member_paths(&raw_files);
    let desktop = extract_desktop_and_icon(&path, &raw_files, ctrl.path());

    Ok(DebInfo {
        path,
        package,
        version,
        architecture,
        section: fields.get("section").cloned(),
        priority: fields.get("priority").cloned(),
        maintainer: fields.get("maintainer").cloned(),
        homepage: fields.get("homepage").cloned(),
        installed_size_kb: fields
            .get("installed-size")
            .and_then(|v| v.trim().parse().ok()),
        description,
        description_long,
        depends: parse_dep_field(fields.get("depends").map(String::as_str).unwrap_or("")),
        recommends: parse_dep_field(fields.get("recommends").map(String::as_str).unwrap_or("")),
        scripts,
        has_debconf,
        desktop,
        files,
        ctrl,
    })
}

fn run_dpkg_deb(args: &[&str], path: &Path) -> Result<std::process::Output, DebError> {
    let mut cmd = Command::new("dpkg-deb");
    cmd.env("LC_ALL", "C").args(args).arg(path);
    cmd.output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            DebError::NoDpkg
        } else {
            DebError::Io(format!("dpkg-deb: {e}"))
        }
    })
}

fn list_archive_members(path: &Path) -> Vec<String> {
    let Ok(out) = run_dpkg_deb(&["-c"], path) else {
        return Vec::new();
    };
    if !out.status.success() {
        return Vec::new();
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut members = Vec::new();
    for line in text.lines() {
        if let Some(m) = parse_archive_line_path(line) {
            members.push(m);
        }
    }
    members
}

fn parse_archive_line_path(line: &str) -> Option<String> {
    let mut parts = line.split_whitespace();
    parts.next()?; // permissions
    parts.next()?; // owner/group
    parts.next()?; // size
    parts.next()?; // date
    parts.next()?; // time
    let path = parts.collect::<Vec<_>>().join(" ");
    if path.is_empty() {
        None
    } else {
        Some(path)
    }
}

fn clean_member_paths(raw: &[String]) -> Vec<String> {
    raw.iter()
        .map(|p| {
            let s = p.strip_prefix("./").unwrap_or(p);
            format!("/{s}")
        })
        .collect()
}

fn extract_archive_member(deb_path: &Path, member: &str, dest_dir: &Path) -> Option<PathBuf> {
    let mut cmd1 = Command::new("dpkg-deb");
    cmd1.env("LC_ALL", "C").args(["--fsys-tarfile"]).arg(deb_path);
    cmd1.stdout(std::process::Stdio::piped());
    let mut child1 = cmd1.spawn().ok()?;

    let mut cmd2 = Command::new("tar");
    cmd2.env("LC_ALL", "C")
        .args(["-C"])
        .arg(dest_dir)
        .args(["-xf", "-"])
        .arg(member);

    if let Some(stdout) = child1.stdout.take() {
        cmd2.stdin(stdout);
    }

    let child2 = cmd2.output().ok()?;
    let _ = child1.wait();
    if !child2.status.success() {
        return None;
    }

    let rel = member.strip_prefix("./").unwrap_or(member);
    let extracted = dest_dir.join(rel);
    if extracted.exists() {
        Some(extracted)
    } else {
        None
    }
}

fn extract_desktop_and_icon(
    deb_path: &Path,
    raw_files: &[String],
    dest_dir: &Path,
) -> Option<DesktopInfo> {
    // 1. Locate .desktop file
    let desktop_member = raw_files.iter().find(|p| {
        (p.contains("usr/share/applications/") || p.contains("/applications/"))
            && p.ends_with(".desktop")
    })?;

    let desktop_file = extract_archive_member(deb_path, desktop_member, dest_dir)?;
    let content = fs::read_to_string(&desktop_file).ok()?;
    let mut desktop = parse_desktop_text(&content);

    // 2. Search for bundled icon if specified
    if let Some(icon_name) = &desktop.icon_name {
        let is_abs_icon = icon_name.starts_with('/');
        let icon_member = raw_files.iter().find(|p| {
            if is_abs_icon {
                p.ends_with(icon_name.as_str())
            } else {
                let stem_match = p.contains(&format!("/{icon_name}."))
                    || p.ends_with(&format!("/{icon_name}.png"))
                    || p.ends_with(&format!("/{icon_name}.svg"))
                    || p.ends_with(&format!("/{icon_name}.xpm"));
                stem_match && (p.contains("pixmaps") || p.contains("icons"))
            }
        });

        if let Some(icon_member) = icon_member {
            if let Some(icon_file) = extract_archive_member(deb_path, icon_member, dest_dir) {
                desktop.icon_path = Some(icon_file);
            }
        }
    }

    Some(desktop)
}

pub fn parse_desktop_text(text: &str) -> DesktopInfo {
    let mut in_desktop_entry = false;
    let mut name = None;
    let mut generic_name = None;
    let mut icon_name = None;
    let mut exec = None;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed == "[Desktop Entry]" {
            in_desktop_entry = true;
            continue;
        } else if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_desktop_entry = false;
            continue;
        }
        if !in_desktop_entry {
            continue;
        }
        if let Some((k, v)) = trimmed.split_once('=') {
            let key = k.trim();
            let val = v.trim();
            match key {
                "Name" if name.is_none() => name = Some(val.to_string()),
                "GenericName" if generic_name.is_none() => generic_name = Some(val.to_string()),
                "Icon" if icon_name.is_none() => icon_name = Some(val.to_string()),
                "Exec" if exec.is_none() => {
                    let clean = val
                        .split_whitespace()
                        .filter(|part| !part.starts_with('%'))
                        .collect::<Vec<_>>()
                        .join(" ");
                    if !clean.is_empty() {
                        exec = Some(clean);
                    }
                }
                _ => {}
            }
        }
    }

    DesktopInfo {
        display_name: name,
        generic_name,
        icon_name,
        icon_path: None,
        exec,
    }
}

fn require(fields: &BTreeMap<String, String>, key: &str) -> Result<String, DebError> {
    match fields.get(key) {
        Some(v) if !v.trim().is_empty() => Ok(v.trim().to_string()),
        _ => Err(DebError::Invalid(format!("missing {key} field"))),
    }
}

/// Parse a debian/control style block into lowercase field names.
/// Continuation lines start with a space or tab.
pub fn parse_control(text: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    let mut current: Option<String> = None;
    for raw in text.lines() {
        if raw.starts_with(' ') || raw.starts_with('\t') {
            if let Some(key) = &current {
                let entry = map.entry(key.clone()).or_insert_with(String::new);
                let cont = raw
                    .strip_prefix(' ')
                    .or_else(|| raw.strip_prefix('\t'))
                    .unwrap_or(raw);
                if !entry.is_empty() {
                    entry.push('\n');
                }
                entry.push_str(cont);
            }
            continue;
        }
        if raw.trim().is_empty() {
            continue;
        }
        if let Some((key, value)) = raw.split_once(':') {
            let key = key.trim().to_ascii_lowercase();
            map.insert(key.clone(), value.trim().to_string());
            current = Some(key);
        }
    }
    map
}

fn split_description(raw: &str) -> (String, Option<String>) {
    let mut lines = raw.lines();
    let short = lines.next().unwrap_or("").trim().to_string();
    let rest: Vec<&str> = lines.collect();
    if rest.is_empty() {
        (short, None)
    } else {
        let mut long = rest.join("\n").trim().to_string();
        if long.is_empty() {
            (short, None)
        } else {
            long.push('\n');
            (short, Some(long))
        }
    }
}

/// Debian package names: lowercase alphanumerics plus `+`, `-`, `.`,
/// starting with an alphanumeric.
pub fn valid_package_name(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() || c.is_ascii_digit() => {}
        _ => return false,
    }
    if name.len() > 200 {
        return false;
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '+' | '-' | '.'))
}

pub fn parse_dep_field(value: &str) -> Vec<Dep> {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(|item| {
            let mut alternatives = item.split('|').map(str::trim);
            let first = alternatives.next().unwrap_or("");
            let (name, constraint) = split_dep(first);
            let rest: Vec<String> = alternatives.map(|alt| split_dep(alt).0).collect();
            Dep {
                name,
                constraint,
                alternatives: rest,
            }
        })
        .collect()
}

fn split_dep(text: &str) -> (String, Option<String>) {
    match text.find('(') {
        Some(idx) => {
            let name = text[..idx].trim().to_string();
            let constraint = text[idx + 1..]
                .split(')')
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
            (
                name,
                if constraint.is_empty() {
                    None
                } else {
                    Some(constraint)
                },
            )
        }
        None => (text.trim().to_string(), None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_folded_description() {
        let text = "Package: foo\nVersion: 1.0\nDescription: short synopsis\n Long line one.\n .\n Long line two.\n";
        let fields = parse_control(text);
        let (short, long) = split_description(fields.get("description").unwrap());
        assert_eq!(short, "short synopsis");
        let long = long.unwrap();
        assert!(long.contains("Long line one."));
        assert!(long.contains("Long line two."));
    }

    #[test]
    fn parses_dependencies_and_alternatives() {
        let deps = parse_dep_field("libc6 (>= 2.34), libfoo3 | libfoo2, bash");
        assert_eq!(deps.len(), 3);
        assert_eq!(deps[0].name, "libc6");
        assert_eq!(deps[0].constraint.as_deref(), Some(">= 2.34"));
        assert_eq!(deps[1].name, "libfoo3");
        assert_eq!(deps[1].alternatives, vec!["libfoo2"]);
        assert_eq!(deps[2].label(), "bash");
    }

    #[test]
    fn rejects_bad_names() {
        assert!(valid_package_name("hello-debdrop"));
        assert!(valid_package_name("libc6"));
        assert!(!valid_package_name("Hello"));
        assert!(!valid_package_name("-bad"));
        assert!(!valid_package_name("bad;rm -rf"));
        assert!(!valid_package_name(""));
    }

    #[test]
    fn dep_labels_include_constraint() {
        let d = parse_dep_field("libc6 (>= 2.34)").pop().unwrap();
        assert_eq!(d.label(), "libc6 (>= 2.34)");
        assert!(d.alternatives_label().is_none());
    }

    #[test]
    fn parses_desktop_entry() {
        let text = r#"
[Desktop Entry]
Type=Application
Name=Test Suite App
GenericName=Code Editor
Icon=test-suite-icon
Exec=test-app %F --verbose
Categories=Development;IDE;
"#;
        let d = parse_desktop_text(text);
        assert_eq!(d.display_name.as_deref(), Some("Test Suite App"));
        assert_eq!(d.generic_name.as_deref(), Some("Code Editor"));
        assert_eq!(d.icon_name.as_deref(), Some("test-suite-icon"));
        assert_eq!(d.exec.as_deref(), Some("test-app --verbose"));
    }

    #[test]
    fn selects_fallback_icons() {
        let info = DebInfo {
            path: PathBuf::new(),
            package: "foo".into(),
            version: "1.0".into(),
            architecture: "amd64".into(),
            section: Some("utils".into()),
            priority: None,
            maintainer: None,
            homepage: None,
            installed_size_kb: None,
            description: "".into(),
            description_long: None,
            depends: vec![],
            recommends: vec![],
            scripts: vec![],
            has_debconf: false,
            desktop: None,
            files: vec![],
            ctrl: TempDir::new().unwrap(),
        };
        assert_eq!(info.fallback_icon(), "utilities-terminal-symbolic");
    }

    #[test]
    fn loads_a_real_fixture_when_present() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/build/hello-debdrop_1.0.0_all.deb");
        if !path.exists() {
            // Fixtures are built by fixtures/build.sh; skip when absent.
            return;
        }
        let info = load(&path).expect("fixture should load");
        assert_eq!(info.package, "hello-debdrop");
        assert_eq!(info.version, "1.0.0");
        assert_eq!(info.architecture, "all");
        assert!(info
            .maintainer
            .as_deref()
            .unwrap_or("")
            .contains("DebDrop Test"));
        assert!(info
            .description
            .to_lowercase()
            .contains("debdrop test fixture"));
        assert!(info.description_long.is_some());
        assert!(info.scripts.contains(&"postinst".to_string()));
        assert!(info.script_text("postinst").unwrap().contains("#!/bin/sh"));
        assert!(!info.has_debconf);
        assert!(!info.files.is_empty());
    }

    #[test]
    fn loads_gui_fixture_with_desktop_and_icon() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/build/hello-debdrop-gui_1.0.0_all.deb");
        if !path.exists() {
            return;
        }
        let info = load(&path).expect("gui fixture should load");
        assert_eq!(info.package, "hello-debdrop-gui");
        assert_eq!(info.homepage.as_deref(), Some("https://debdrop.example.com"));
        let desktop = info.desktop.as_ref().expect("desktop info should be parsed");
        assert_eq!(desktop.display_name.as_deref(), Some("Hello DebDrop GUI"));
        assert_eq!(desktop.generic_name.as_deref(), Some("Sample Graphical Utility"));
        assert_eq!(desktop.icon_name.as_deref(), Some("hello-debdrop-icon"));
        assert!(desktop.icon_path.is_some(), "icon should be extracted to tempdir");
        assert!(desktop.icon_path.as_ref().unwrap().exists());
        assert_eq!(info.display_title(), "Hello DebDrop GUI");
    }
}
