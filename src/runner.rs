//! Process plumbing: streaming output for transactions, captured output for
//! one-shot queries. No shell, ever.

use std::ffi::OsString;
use std::io::Read;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use async_channel::{unbounded, Sender};

const MAX_LINE_BYTES: usize = 4096;

#[derive(Debug)]
pub enum Event {
    Line { text: String },
    Exited { code: i32 },
}

pub struct Handle {
    pub pid: i32,
    pub rx: async_channel::Receiver<Event>,
}

/// Spawn a command in its own process group so it can be terminated together
/// with the children it starts (apt runs dpkg, dpkg runs maintainer scripts).
pub fn spawn(argv: Vec<OsString>) -> Result<Handle, String> {
    let program = argv.first().ok_or("empty command")?;
    let mut cmd = Command::new(program);
    cmd.args(&argv[1..])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0);

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("cannot run {}: {e}", program.to_string_lossy()))?;
    let pid = child.id() as i32;
    let (tx, rx) = unbounded();

    let out = child.stdout.take().ok_or("no stdout pipe")?;
    let err = child.stderr.take().ok_or("no stderr pipe")?;
    let tx_out = tx.clone();
    let h_out = thread::spawn(move || pump(out, tx_out));
    let tx_err = tx.clone();
    let h_err = thread::spawn(move || pump(err, tx_err));

    // The reaper waits for both readers so `Exited` is always the last event.
    thread::spawn(move || {
        let code = child
            .wait()
            .map(|status| status.code().unwrap_or(-1))
            .unwrap_or(-1);
        let _ = h_out.join();
        let _ = h_err.join();
        let _ = tx.send_blocking(Event::Exited { code });
    });

    Ok(Handle { pid, rx })
}

fn pump<R: Read>(mut reader: R, tx: Sender<Event>) {
    let mut chunk = [0u8; 8192];
    let mut acc: Vec<u8> = Vec::new();
    loop {
        match reader.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                acc.extend_from_slice(&chunk[..n]);
                // apt and dpkg use both \n and \r for progress lines.
                while let Some(pos) = acc.iter().position(|&b| b == b'\n' || b == b'\r') {
                    let line: Vec<u8> = acc.drain(..=pos).collect();
                    emit(&line[..line.len().saturating_sub(1)], &tx);
                }
                if acc.len() > 64 * 1024 {
                    emit(&acc, &tx);
                    acc.clear();
                }
            }
            Err(_) => break,
        }
    }
    if !acc.is_empty() {
        emit(&acc, &tx);
    }
}

fn emit(raw: &[u8], tx: &Sender<Event>) {
    let text = clean(raw);
    if text.is_empty() {
        return;
    }
    let _ = tx.send_blocking(Event::Line { text });
}

fn clean(raw: &[u8]) -> String {
    let lossy = String::from_utf8_lossy(raw);
    let mut out = String::with_capacity(lossy.len());
    let mut chars = lossy.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            // Drop CSI sequences (colours, cursor moves) and OSC titles.
            match chars.peek() {
                Some('[') => {
                    chars.next();
                    for c in chars.by_ref() {
                        if ('@'..='~').contains(&c) {
                            break;
                        }
                    }
                }
                Some(']') => {
                    chars.next();
                    for c in chars.by_ref() {
                        if c == '\u{7}' {
                            break;
                        }
                    }
                }
                _ => {}
            }
            continue;
        }
        if c.is_control() && c != '\t' {
            continue;
        }
        out.push(c);
    }
    let mut out = out.trim_end().to_string();
    if out.len() > MAX_LINE_BYTES {
        out.truncate(MAX_LINE_BYTES);
    }
    out.trim().is_empty().then(String::new).unwrap_or(out)
}

/// Terminate a command group politely. Never SIGKILL dpkg.
pub fn terminate(group_leader: i32) {
    if group_leader > 0 {
        unsafe {
            libc::kill(-group_leader, libc::SIGTERM);
        }
    }
}

pub struct Captured {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
    pub timed_out: bool,
}

/// Run a command to completion with a timeout, capturing both streams.
pub fn capture(
    argv: &[OsString],
    env: &[(&str, &str)],
    timeout: Duration,
) -> Result<Captured, String> {
    let program = argv.first().ok_or("empty command")?;
    let mut cmd = Command::new(program);
    cmd.args(&argv[1..])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0);
    for (key, value) in env {
        cmd.env(key, value);
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("cannot run {}: {e}", program.to_string_lossy()))?;
    let pid = child.id() as i32;
    let out = child.stdout.take().ok_or("no stdout pipe")?;
    let err = child.stderr.take().ok_or("no stderr pipe")?;
    let h_out = thread::spawn(move || read_all(out));
    let h_err = thread::spawn(move || read_all(err));

    let start = Instant::now();
    let mut timed_out = false;
    let code = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.code().unwrap_or(-1),
            Ok(None) => {
                if start.elapsed() >= timeout {
                    timed_out = true;
                    unsafe {
                        libc::kill(-pid, libc::SIGTERM);
                    }
                    thread::sleep(Duration::from_millis(200));
                    let _ = child.kill();
                    let _ = child.wait();
                    break -1;
                }
                thread::sleep(Duration::from_millis(20));
            }
            Err(_) => break -1,
        }
    };

    let stdout = h_out.join().unwrap_or_default();
    let stderr = h_err.join().unwrap_or_default();
    Ok(Captured {
        code,
        stdout,
        stderr,
        timed_out,
    })
}

fn read_all<R: Read>(mut reader: R) -> String {
    let mut buf = Vec::new();
    let _ = reader.read_to_end(&mut buf);
    String::from_utf8_lossy(&buf).into_owned()
}

pub fn display_cmd(argv: &[OsString]) -> String {
    argv.iter()
        .map(|a| {
            let s = a.to_string_lossy();
            if s.contains(' ') {
                format!("{s:?}")
            } else {
                s.into_owned()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
