// SPDX-FileCopyrightText: 2026 William VITAL, FR (wvmscs)
// SPDX-License-Identifier: GPL-3.0-or-later

//! `bitchat-launcher`: starts bitchat-geohash (the map) and bitchat-console
//! (over it) together, on free ports, in HTTPS, with a working Tor.
//!
//! The two tools stay separate programs (they need two versions of SQLite
//! that cannot be linked together): the launcher picks the ports and the
//! Tor, makes the certificate, starts both, opens the page when both
//! answer, and stops both on Ctrl-C or when one of them stops.

pub mod args;
pub mod ports;
pub mod tls;
pub mod tools;
pub mod tor;

use std::time::{Duration, Instant};

use args::{Args, Https, Tor, console_command, map_command};
use tools::{CONSOLE, MAP, Tool, listening};

/// The manual (`--man`).
pub const MAN: &str = include_str!("../man/bitchat-launcher.1");

/// Longest wait for both tools to answer: Arti's first start and the relay
/// lists through Tor can take a few minutes.
const STARTUP: Duration = Duration::from_secs(300);
/// Time a tool gets to stop cleanly (the console sends what it still has).
const GRACE: Duration = Duration::from_secs(20);

fn say(s: impl std::fmt::Display) {
    eprintln!("bitchat-launcher: {s}");
}

/// Ctrl-C or SIGTERM.
async fn stop_signal() {
    let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .expect("SIGTERM handler");
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = term.recv() => {}
    }
}

fn open_browser(url: &str) {
    let program = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    if let Err(e) = std::process::Command::new(program).arg(url).spawn() {
        say(format!("cannot open the browser ({program}): {e}"));
    }
}

/// Runs both tools until Ctrl-C or until one stops; the process exit code.
pub async fn launch(a: Args) -> i32 {
    match prepare(&a).await {
        Ok(p) => supervise(&a, p).await,
        Err(e) => {
            say(e);
            1
        }
    }
}

struct Prepared {
    port: u16,
    map_port: u16,
    tor: Tor,
    arti_fallback: bool,
    https: bool,
    console: Tool,
    map: Tool,
}

async fn prepare(a: &Args) -> Result<Prepared, String> {
    let tls = match &a.https {
        Https::Off => None,
        Https::Files { cert, key } => {
            for f in [cert, key] {
                if !f.is_file() {
                    return Err(format!("{}: no such file", f.display()));
                }
            }
            Some((cert.clone(), key.clone()))
        }
        Https::Auto => {
            let (files, warning) = tls::ensure(&a.data_dir.join("tls"), "mkcert")?;
            if let Some(w) = warning {
                say(w);
            }
            Some(files)
        }
    };

    let seed = ports::seed();
    let port = match a.port {
        Some(p) if ports::is_free(p) => p,
        Some(p) => return Err(format!("port {p} (--port) is already in use")),
        None => ports::pick(
            Some(ports::PREFERRED),
            ports::RANGE,
            &[],
            seed,
            ports::is_free,
        )
        .ok_or("no free port for the page")?,
    };
    if a.port.is_none() && port != ports::PREFERRED {
        say(format!(
            "port {} is in use, the page goes to {port}",
            ports::PREFERRED
        ));
    }
    let map_port = ports::pick(
        None,
        ports::RANGE,
        &[port],
        seed.rotate_left(7),
        ports::is_free,
    )
    .ok_or("no free port for the map")?;

    let exe = std::env::current_exe().ok();
    let beside = exe.as_deref().and_then(|e| e.parent());
    let path = std::env::var_os("PATH");
    let console_bin = tools::find(CONSOLE, a.console_bin.as_deref(), beside, path.as_deref())?;
    let map_bin = tools::find(MAP, a.map_bin.as_deref(), beside, path.as_deref())?;

    let env = std::env::var(tor::ENV).ok();
    let (tor, tried) = tor::resolve(
        &a.tor,
        env.as_deref(),
        tor::DEFAULT_SOCKS,
        tor::PROBE,
        Duration::from_secs(30),
    )
    .await?;
    for t in &tried {
        say(format!("Tor: {t}"));
    }
    let arti_fallback = tor == Tor::Arti && a.tor != args::TorChoice::Arti;

    tls::private_dir(&a.data_dir)?;
    let map = Tool::spawn(MAP, "map", &map_bin, &map_command(a, map_port))?;
    let console = Tool::spawn(
        CONSOLE,
        "console",
        &console_bin,
        &console_command(a, port, map_port, &tor, tls.as_ref().map(|(c, k)| (c, k))),
    )?;
    Ok(Prepared {
        port,
        map_port,
        tor,
        arti_fallback,
        https: tls.is_some(),
        console,
        map,
    })
}

/// What to say when the console stops while starting.
fn console_hint(p: &Prepared) -> Option<&'static str> {
    p.arti_fallback.then_some(
        "no Tor daemon answered and bitchat-console may be built without Arti: \
         start tor (brew services start tor), or reinstall bitchat-console with --features arti",
    )
}

async fn supervise(a: &Args, mut p: Prepared) -> i32 {
    let started = Instant::now();
    let stop = stop_signal();
    tokio::pin!(stop);
    let (mut map_up, mut console_up) = (false, false);
    // Startup: both answer, or one stops, or Ctrl-C.
    while !(map_up && console_up) {
        let exited = [
            p.map.exited().map(|s| (MAP, s)),
            p.console.exited().map(|s| (CONSOLE, s)),
        ];
        if let Some((name, status)) = exited.into_iter().flatten().next() {
            say(format!("{name} stopped while starting ({status})"));
            if name == CONSOLE
                && let Some(h) = console_hint(&p)
            {
                say(h);
            }
            p.map.stop(GRACE).await;
            p.console.stop(GRACE).await;
            return 1;
        }
        if !map_up && listening(p.map_port).await {
            map_up = true;
            say(format!("map up ({:.1} s)", started.elapsed().as_secs_f64()));
        }
        if !console_up && listening(p.port).await {
            console_up = true;
            say(format!(
                "console up ({:.1} s)",
                started.elapsed().as_secs_f64()
            ));
        }
        if started.elapsed() > STARTUP {
            say(format!("not ready after {} s, stopping", STARTUP.as_secs()));
            p.map.stop(GRACE).await;
            p.console.stop(GRACE).await;
            return 1;
        }
        tokio::select! {
            _ = &mut stop => {
                say("stopping");
                p.console.stop(GRACE).await;
                p.map.stop(GRACE).await;
                return 0;
            }
            _ = tokio::time::sleep(Duration::from_millis(250)) => {}
        }
    }
    let scheme = if p.https { "https" } else { "http" };
    let url = format!("{scheme}://localhost:{}/", p.port);
    let tor = match p.tor {
        Tor::Socks(addr) => format!("Tor daemon {addr}"),
        Tor::Arti => "Arti".into(),
    };
    say(format!(
        "{url}  (map on 127.0.0.1:{}, {tor}; Ctrl-C stops both)",
        p.map_port
    ));
    if a.open {
        open_browser(&url);
    }
    // Running: until Ctrl-C, or one stops.
    let code = tokio::select! {
        _ = &mut stop => { say("stopping"); 0 }
        s = p.map.wait() => { say(format!("{MAP} stopped ({})", show(s))); 1 }
        s = p.console.wait() => { say(format!("{CONSOLE} stopped ({})", show(s))); 1 }
    };
    // The console first: it sends what it still has while the map runs.
    p.console.stop(GRACE).await;
    p.map.stop(GRACE).await;
    code
}

fn show(s: Option<std::process::ExitStatus>) -> String {
    s.map_or("unknown status".into(), |s| s.to_string())
}
