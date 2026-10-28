// SPDX-FileCopyrightText: 2026 William VITAL, FR (wvmscs)
// SPDX-License-Identifier: GPL-3.0-or-later

//! `bitchat-launcher <geohash> [options]`: see `--help` and `--man`.
//!
//! Exit code 2 for a wrong command line, 1 when a tool cannot start or
//! stops on its own.

use bitchat_launcher::args::{Command, HELP, parse};

/// Shows the manual with `man`, else prints its source.
fn man() {
    let path = std::env::temp_dir().join(format!("bitchat-launcher-{}.1", std::process::id()));
    let shown = std::fs::write(&path, bitchat_launcher::MAN).is_ok()
        && std::process::Command::new("man")
            .arg(&path)
            .status()
            .is_ok_and(|s| s.success());
    let _ = std::fs::remove_file(&path);
    if !shown {
        print!("{}", bitchat_launcher::MAN);
    }
}

fn main() {
    let home = std::env::var_os("HOME").map(Into::into);
    let args = match parse(std::env::args().skip(1), home) {
        Ok(Command::Run(a)) => a,
        Ok(Command::Help) => return println!("{HELP}"),
        Ok(Command::Man) => return man(),
        Ok(Command::Version) => return println!("bitchat-launcher {}", env!("CARGO_PKG_VERSION")),
        Err(e) => {
            eprintln!("bitchat-launcher: {e}\n(bitchat-launcher --help for the options)");
            std::process::exit(2)
        }
    };
    let code = tokio::runtime::Runtime::new()
        .expect("tokio runtime")
        .block_on(bitchat_launcher::launch(*args));
    std::process::exit(code)
}
