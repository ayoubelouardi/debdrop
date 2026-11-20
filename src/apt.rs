//! Everything that talks to apt/dpkg: reads run as the user, writes run as
//! root through `pkexec`. No shell, no interpolation.

use std::cmp::Ordering;
use std::ffi::OsString;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

use crate::runner;

pub const APT_GET: &str = "/usr/bin/apt-get";
pub const PKEXEC: &str = "/usr/bin/pkexec";
pub const ENV: &str = "/usr/bin/env";
const SIMULATE_TIMEOUT: Duration = Duration::from_secs(45);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Install,
    Reinstall,
    Upgrade,
    Downgrade,
}

#[derive(Debug, Clone, Default)]
pub struct InstalledState {
    pub installed: bool,
    pub version: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Plan {
    pub new: Vec<String>,
    pub upgraded: Vec<String>,
    pub downgraded: Vec<String>,
    pub removed: Vec<String>,
    pub unresolved: Vec<String>,
    pub summary: Option<String>,
    pub detail: String,
    pub timed_out: bool,
    pub code: i32,
    pub ok: bool,
    pub stale_lists: bool,
}

impl Plan {
    pub fn headline(&self) -> String {
        if self.timed_out {
            return "Could not resolve dependencies (timed out)".into();
        }
        if self.stale_lists {
            return "Package lists may be out of date".into();
        }
        if !self.unresolved.is_empty() {
            return format!("Cannot resolve: {}", self.unresolved.join(", "));
        }
        if !self.ok {
            return format!("Could not resolve packages (apt exit {})", self.code);
        }
        if self.new.is_empty() && self.upgraded.is_empty() && self.downgraded.is_empty() {
            "All dependencies are available".into()
        } else {
            let count = self.new.len() + self.upgraded.len() + self.downgraded.len();
            format!("{count} package(s) will be installed with it")
        }
    }

    pub fn has_rows(&self) -> bool {
        !(self.new.is_empty()
            && self.upgraded.is_empty()
            && self.downgraded.is_empty()
            && self.removed.is_empty()
            && self.unresolved.is_empty())
    }
}

pub fn installed_state(package: &str) -> InstalledState {
    let out = Command::new("dpkg-query")
        .args(["-W", "-f=${db:Status-Abbrev}\t${Version}\n"])
        .arg(package)
        .env("LC_ALL", "C")
        .output();

    let Ok(out) = out else {
        return InstalledState::default();
    };
    if !out.status.success() {
        return InstalledState::default();
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.lines().next().unwrap_or("");
    let mut parts = line.splitn(2, '\t');
    let status = parts.next().unwrap_or("").trim();
    let version = parts
        .next()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    let installed = status == "ii" && version.is_some();
    let note = match status {
        "ii" => None,
        "rc" => Some("removed, configuration files remain".to_string()),
        "" | "un" => None,
        other => Some(format!("dpkg status {other}")),
    };
    InstalledState {
        installed,
        version,
        note,
    }
}

/// Debian version ordering, delegated to dpkg because `1.0~rc1 < 1.0`.
pub fn compare(a: &str, b: &str) -> Ordering {
    if compare_versions(a, "eq", b) {
        Ordering::Equal
    } else if compare_versions(a, "lt", b) {
        Ordering::Less
    } else {
        Ordering::Greater
    }
}

fn compare_versions(a: &str, op: &str, b: &str) -> bool {
    Command::new("dpkg")
        .args(["--compare-versions", a, op, b])
        .env("LC_ALL", "C")
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn action_for(deb_version: &str, state: &InstalledState) -> Action {
    match &state.version {
        Some(installed) if state.installed => match compare(deb_version, installed) {
            Ordering::Equal => Action::Reinstall,
            Ordering::Less => Action::Downgrade,
            Ordering::Greater => Action::Upgrade,
        },
        _ => Action::Install,
    }
}

/// Returns a user-facing message when the package's architecture cannot be
/// installed on this system.
pub fn architecture_problem(deb_arch: &str) -> Option<String> {
    if deb_arch == "all" || deb_arch.is_empty() {
        return None;
    }
    let host = command_line("dpkg", &["--print-architecture"]).unwrap_or_default();
    let foreign = command_line("dpkg", &["--print-foreign-architectures"]).unwrap_or_default();
    if host.is_empty() || deb_arch == host {
        return None;
    }
    if foreign.split_whitespace().any(|a| a == deb_arch) {
        return None;
    }
    Some(format!(
        "Package is built for {deb_arch}, this system is {host}"
    ))
}

fn command_line(program: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(program).args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

pub fn simulate(path: &Path) -> Plan {
    let argv = vec![
        OsString::from("apt-get"),
        OsString::from("-s"),
        OsString::from("install"),
        OsString::from("--no-install-recommends"),
        path.as_os_str().to_os_string(),
    ];
    let env = [("LC_ALL", "C")];
    let captured = match runner::capture(&argv, &env, SIMULATE_TIMEOUT) {
        Ok(c) => c,
        Err(e) => {
            return Plan {
                detail: e,
                ..Default::default()
            }
        }
    };
    parse_simulate(
        &captured.stdout,
        &captured.stderr,
        captured.code,
        captured.timed_out,
    )
}

pub fn parse_simulate(stdout: &str, stderr: &str, code: i32, timed_out: bool) -> Plan {
    let mut plan = Plan {
        code,
        timed_out,
        ..Default::default()
    };
    plan.detail = format!("{stdout}\n{stderr}").trim().to_string();

    #[derive(Clone, Copy, PartialEq)]
    enum Section {
        None,
        New,
        Upgraded,
        Downgraded,
        Removed,
    }
    let mut section = Section::None;

    for raw in stdout.lines() {
        let line = raw.trim_end();
        let lower = line.to_lowercase();

        if lower.starts_with("the following new packages will be installed") {
            section = Section::New;
            continue;
        }
        if lower.starts_with("the following packages will be upgraded") {
            section = Section::Upgraded;
            continue;
        }
        if lower.starts_with("the following packages will be downgraded") {
            section = Section::Downgraded;
            continue;
        }
        if lower.starts_with("the following packages will be removed")
            || lower.starts_with("the following packages will be removed:")
        {
            section = Section::Removed;
            continue;
        }
        if lower.starts_with("the following packages have unmet dependencies") {
            section = Section::None;
            continue;
        }
        if line.contains("upgraded,") && line.contains("newly installed") {
            plan.summary = Some(line.trim().to_string());
        }

        // Package lists are indented and wrapped; the header above tells us
        // which list we are in. A non-indented line ends the section.
        if section != Section::None && line.starts_with(' ') {
            for token in line.split_whitespace() {
                let name = token.trim_end_matches(',');
                if name.is_empty() || !name.chars().any(|c| c.is_ascii_alphanumeric()) {
                    continue;
                }
                let list = match section {
                    Section::New => &mut plan.new,
                    Section::Upgraded => &mut plan.upgraded,
                    Section::Downgraded => &mut plan.downgraded,
                    Section::Removed => &mut plan.removed,
                    Section::None => continue,
                };
                if !list.iter().any(|n| n == name) {
                    list.push(name.to_string());
                }
            }
            continue;
        }
        section = Section::None;

        if let Some(rest) = line.strip_prefix("E: Unable to locate package ") {
            push_unresolved(&mut plan, rest.split_whitespace().next().unwrap_or(""));
            plan.stale_lists = true;
        } else if let Some(rest) = line.strip_prefix("E: ") {
            push_unresolved(&mut plan, rest);
        } else if line.starts_with(' ') {
            if let Some((_, deps)) = line.split_once(": Depends: ") {
                let dep = deps
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .trim_end_matches(',');
                if lower.contains("not installable")
                    || lower.contains("no installation candidate")
                    || lower.contains("but it is not going to be installed")
                {
                    push_unresolved(&mut plan, dep);
                    if lower.contains("no installation candidate") {
                        plan.stale_lists = true;
                    }
                }
            }
        }
    }

    if plan
        .detail
        .to_lowercase()
        .contains("unable to locate package")
    {
        plan.stale_lists = true;
    }

    plan.ok = code == 0 && !timed_out && plan.unresolved.is_empty();
    plan
}

fn push_unresolved(plan: &mut Plan, name: &str) {
    let name = name.trim().trim_end_matches(',').trim_end_matches(':');
    if name.is_empty() {
        return;
    }
    if !plan.unresolved.iter().any(|n| n == name) {
        plan.unresolved.push(name.to_string());
    }
}

fn root_args(subcommand: &[&str], tail: &[OsString], env: bool) -> Vec<OsString> {
    let mut argv: Vec<OsString> = vec![
        OsString::from(PKEXEC),
        OsString::from(ENV),
        OsString::from("DEBIAN_FRONTEND=noninteractive"),
        OsString::from("APT_LISTCHANGES_FRONTEND=none"),
        OsString::from("LC_ALL=C"),
    ];
    let _ = env;
    argv.push(OsString::from(APT_GET));
    argv.extend(subcommand.iter().map(OsString::from));
    argv.extend(tail.iter().cloned());
    argv
}

pub fn install_argv(path: &Path, action: Action) -> Vec<OsString> {
    let mut flags: Vec<&str> = vec!["install", "-y", "--no-install-recommends"];
    if action == Action::Reinstall {
        flags.push("--reinstall");
    }
    if action == Action::Downgrade {
        flags.push("--allow-downgrades");
    }
    let tail = vec![path.as_os_str().to_os_string()];
    root_args(&flags, &tail, true)
}

pub fn remove_argv(package: &str) -> Vec<OsString> {
    let tail = vec![OsString::from(package)];
    root_args(&["remove", "-y"], &tail, true)
}

pub fn update_argv() -> Vec<OsString> {
    root_args(&["update"], &[], true)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorClass {
    Auth,
    Locked,
    StaleLists,
    MissingDeps,
    DiskFull,
    BadArchive,
    Unknown,
}

pub fn is_auth(code: i32) -> bool {
    code == 126 || code == 127
}

pub fn classify(code: i32, output: &str) -> (ErrorClass, String) {
    let text = output.to_lowercase();
    if is_auth(code) {
        return (ErrorClass::Auth, "Installation cancelled".into());
    }
    if text.contains("could not get lock")
        || text.contains("unable to acquire the dpkg frontend lock")
    {
        return (ErrorClass::Locked, "Another package manager is busy".into());
    }
    if text.contains("no space left on device") {
        return (ErrorClass::DiskFull, "Not enough free disk space".into());
    }
    if text.contains("unable to locate package") || text.contains("has no installation candidate") {
        return (
            ErrorClass::StaleLists,
            "Package lists are out of date".into(),
        );
    }
    if text.contains("unmet dependencies") || text.contains("is not installable") {
        return (
            ErrorClass::MissingDeps,
            "Dependencies cannot be satisfied".into(),
        );
    }
    if text.contains("is not a debian format archive") || text.contains("error in archive") {
        return (
            ErrorClass::BadArchive,
            "This file is not a valid .deb package".into(),
        );
    }
    (
        ErrorClass::Unknown,
        format!("Installation failed (exit code {code})"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "Reading package lists...\nBuilding dependency tree...\n\
The following NEW packages will be installed:\n  libfoo3\n  libbar1\n\
0 upgraded, 2 newly installed, 0 to remove and 0 not upgraded.\n\
Need to get 1,234 kB of archives.\nAfter this operation, 4,567 kB of additional disk space will be used.\n";

    #[test]
    fn parses_new_packages_and_summary() {
        let plan = parse_simulate(SAMPLE, "", 0, false);
        assert!(plan.ok);
        assert_eq!(plan.new, vec!["libfoo3", "libbar1"]);
        assert!(plan.summary.unwrap().contains("2 newly installed"));
    }

    #[test]
    fn detects_unresolvable() {
        let out = "The following packages have unmet dependencies:\n hello : Depends: nope (>= 1) but it is not installable\n";
        let plan = parse_simulate(out, "", 100, false);
        assert!(!plan.ok);
        assert_eq!(plan.unresolved, vec!["nope"]);
    }

    #[test]
    fn detects_stale_lists() {
        let plan = parse_simulate("", "E: Unable to locate package foo\n", 100, false);
        assert!(plan.stale_lists);
        assert!(!plan.ok);
    }

    #[test]
    fn auth_codes_are_cancelled() {
        let (class, hint) = classify(126, "anything");
        assert_eq!(class, ErrorClass::Auth);
        assert_eq!(hint, "Installation cancelled");
    }

    #[test]
    fn install_argv_only_downgrades_deliberately() {
        let path = Path::new("/tmp/x.deb");
        let plain = install_argv(path, Action::Install);
        assert!(!plain.iter().any(|a| a == "--allow-downgrades"));
        let down = install_argv(path, Action::Downgrade);
        assert!(down.iter().any(|a| a == "--allow-downgrades"));
        let re = install_argv(path, Action::Reinstall);
        assert!(re.iter().any(|a| a == "--reinstall"));
        assert_eq!(plain[0], PKEXEC);
        assert_eq!(plain.last().unwrap(), "/tmp/x.deb");
    }
}
