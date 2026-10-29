// SPDX-FileCopyrightText: 2026 William VITAL, FR (wvmscs)
// SPDX-License-Identifier: GPL-3.0-or-later

//! Test helpers.

use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

/// The first line of a fake tool's body: `--ready` exits at once.
pub(crate) const READY: &str = "[ \"$1\" = --ready ] && exit 0\n";

/// Writes an executable shell script (`#!/bin/sh`, [`READY`], `body`), then
/// waits until it can run. On Linux, a test that starts a process at the
/// same time can keep the file open for writing a moment (its child inherits
/// the descriptor until its own exec): running it then fails with "text file
/// busy" (`ETXTBSY`).
pub(crate) fn script(path: &Path, body: &str) {
    std::fs::write(path, format!("#!/bin/sh\n{READY}{body}")).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    for _ in 0..500 {
        match Command::new(path).arg("--ready").status() {
            Err(e) if e.kind() == std::io::ErrorKind::ExecutableFileBusy => {
                std::thread::sleep(Duration::from_millis(10))
            }
            status => {
                assert!(status.unwrap().success());
                return;
            }
        }
    }
    panic!("{} still busy", path.display());
}
