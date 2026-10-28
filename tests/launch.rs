// SPDX-FileCopyrightText: 2026 William VITAL, FR (wvmscs)
// SPDX-License-Identifier: GPL-3.0-or-later

//! The launcher's binary with two fake tools (Python scripts that listen
//! where they are told and write down their arguments).

use std::io::{BufRead, BufReader};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// Listens on `--listen`, writes its arguments to `<dir>/<name>.args`,
/// exits 0 on SIGINT; with `FAIL`, exits 1 at once.
const FAKE: &str = r#"#!/usr/bin/env python3
import os, signal, socket, sys
name = os.path.basename(sys.argv[0])
here = os.path.dirname(os.path.abspath(sys.argv[0]))
with open(os.path.join(here, name + ".args"), "w") as f:
    f.write("\n".join(sys.argv[1:]))
if os.path.exists(os.path.join(here, name + ".fail")):
    sys.exit(1)
signal.signal(signal.SIGINT, lambda *_: sys.exit(0))
addr = sys.argv[sys.argv.index("--listen") + 1]
host, port = addr.rsplit(":", 1)
s = socket.socket()
s.bind((host, int(port)))
s.listen()
while True:
    c, _ = s.accept()
    c.close()
"#;

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("bitchat-launcher-it-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    for tool in ["bitchat-console", "bitchat-geohash"] {
        let p = d.join(tool);
        std::fs::write(&p, FAKE).unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    d
}

/// Tests run side by side: each page gets its own free port, not 8080.
fn launcher(dir: &Path, extra: &[&str]) -> Child {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    Command::new(env!("CARGO_BIN_EXE_bitchat-launcher"))
        .args(["u09tv", "--teleport", "--arti", "--http", "--no-open"])
        .args(["--port", &port.to_string()])
        .arg("--console-bin")
        .arg(dir.join("bitchat-console"))
        .arg("--map-bin")
        .arg(dir.join("bitchat-geohash"))
        .arg("--data-dir")
        .arg(dir.join("data"))
        .args(extra)
        .stderr(Stdio::piped())
        .spawn()
        .unwrap()
}

/// Starts the launcher and reads its stderr until a line contains `what`.
/// The free port a test picks can be taken by another test before the
/// launcher binds it: then it starts again on another one.
fn start(dir: &Path, extra: &[&str], what: &str) -> (Child, Vec<String>) {
    for _ in 0..5 {
        let mut l = launcher(dir, extra);
        match until(&mut l, what) {
            Ok(lines) => return (l, lines),
            Err(lines) if lines.iter().any(|x| x.contains("already in use")) => {
                let _ = l.wait();
            }
            Err(lines) => panic!("no {what:?} in {lines:#?}"),
        }
    }
    panic!("no free port");
}

/// Lines of the launcher's stderr until one contains `what`; `Err` with
/// them if it ends or takes too long.
fn until(child: &mut Child, what: &str) -> Result<Vec<String>, Vec<String>> {
    let err = child.stderr.take().unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        for l in BufReader::new(err).lines().map_while(Result::ok) {
            let _ = tx.send(l);
        }
    });
    let mut seen = Vec::new();
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(20) {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(l) => {
                let done = l.contains(what);
                seen.push(l);
                if done {
                    return Ok(seen);
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return Err(seen),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
        }
    }
    Err(seen)
}

fn args_of(dir: &Path, tool: &str) -> Vec<String> {
    std::fs::read_to_string(dir.join(format!("{tool}.args")))
        .unwrap()
        .lines()
        .map(str::to_owned)
        .collect()
}

fn wait(child: &mut Child) -> i32 {
    let start = Instant::now();
    loop {
        if let Some(s) = child.try_wait().unwrap() {
            return s.code().unwrap_or(-1);
        }
        assert!(
            start.elapsed() < Duration::from_secs(30),
            "launcher still running"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[test]
fn starts_both_then_stops_both() {
    let d = scratch("run");
    let (mut l, lines) = start(
        &d,
        &["--console-args", "--no-presence"],
        "Ctrl-C stops both",
    );
    let url = lines.last().unwrap();
    assert!(url.contains("http://localhost:"), "{url}");

    let console = args_of(&d, "bitchat-console");
    let map = args_of(&d, "bitchat-geohash");
    assert_eq!(map[0], "u09t");
    assert_eq!(console[0], "u09tv");
    let at = |v: &[String], f: &str| v[v.iter().position(|a| a == f).unwrap() + 1].clone();
    let map_listen = at(&map, "--listen");
    assert_eq!(at(&console, "--map"), format!("http://{map_listen}"));
    let page = at(&console, "--listen");
    assert!(url.contains(page.rsplit(':').next().unwrap()));
    assert!(console.contains(&"--arti".to_owned()) && console.contains(&"--teleport".to_owned()));
    assert_eq!(console.last().unwrap(), "--no-presence");
    assert!(!console.contains(&"--tls-cert".to_owned()));
    assert!(at(&map, "--db").ends_with("data/bitchat-geohash.sqlite"));

    // Ctrl-C: both stop, the launcher exits 0.
    unsafe {
        libc::kill(l.id() as i32, libc::SIGINT);
    }
    assert_eq!(wait(&mut l), 0);
    for addr in [page, map_listen] {
        assert!(
            std::net::TcpStream::connect(&addr).is_err(),
            "{addr} still served"
        );
    }
    std::fs::remove_dir_all(&d).unwrap();
}

#[test]
fn one_failing_stops_the_other() {
    let d = scratch("fail");
    std::fs::write(d.join("bitchat-console.fail"), "").unwrap();
    let (mut l, lines) = start(&d, &[], "stopped while starting");
    assert!(
        lines.iter().any(|l| l.contains("bitchat-console")),
        "{lines:?}"
    );
    assert_eq!(wait(&mut l), 1);
    // The map was stopped too: it no longer listens (or was stopped before
    // it even wrote its arguments down).
    if d.join("bitchat-geohash.args").exists() {
        let map = args_of(&d, "bitchat-geohash");
        let map_listen = map[map.iter().position(|a| a == "--listen").unwrap() + 1].clone();
        assert!(std::net::TcpStream::connect(&map_listen).is_err());
    }
    std::fs::remove_dir_all(&d).unwrap();
}

#[test]
fn wrong_command_line() {
    let out = Command::new(env!("CARGO_BIN_EXE_bitchat-launcher"))
        .arg("--nope")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("--help"));
    let help = Command::new(env!("CARGO_BIN_EXE_bitchat-launcher"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(help.status.success());
    let text = String::from_utf8_lossy(&help.stdout);
    // The usual options come first.
    assert!(text.find("--teleport").unwrap() < text.find("--console-args").unwrap());
}
