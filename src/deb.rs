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

pub struct DebInfo {
    pub path: PathBuf,
    pub package: String,
    pub version: String,
    pub architecture: String,
    pub section: Option<String>,
    pub priority: Option<String>,
    pub maintainer: Option<String>,
    pub installed_size_kb: Option<u64>,
    pub description: String,
    pub description_long: Option<String>,
    pub depends: Vec<Dep>,
    pub recommends: Vec<Dep>,
    pub scripts: Vec<String>,
    pub has_debconf: bool,
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

    Ok(DebInfo {
        path,
        package,
        version,
        architecture,
        section: fields.get("section").cloned(),
        priority: fields.get("priority").cloned(),
        maintainer: fields.get("maintainer").cloned(),
        installed_size_kb: fields
            .get("installed-size")
            .and_then(|v| v.trim().parse().ok()),
        description,
        description_long,
        depends: parse_dep_field(fields.get("depends").map(String::as_str).unwrap_or("")),
        recommends: parse_dep_field(fields.get("recommends").map(String::as_str).unwrap_or("")),
        scripts,
        has_debconf,
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
    }
}
