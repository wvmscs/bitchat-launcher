// SPDX-FileCopyrightText: 2026 William VITAL, FR (wvmscs)
// SPDX-License-Identifier: GPL-3.0-or-later

//! The page's certificate: given, or made once with mkcert.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Names the certificate covers: what the browser is sent to.
pub const NAMES: &[&str] = &["localhost", "127.0.0.1", "::1"];

/// `<dir>/cert.pem` and `<dir>/key.pem`, made with `mkcert` when missing
/// (`mkcert` is the program, for the tests). The second value warns when
/// mkcert's authority is not installed, so the browser would not trust it.
pub fn ensure(dir: &Path, mkcert: &str) -> Result<((PathBuf, PathBuf), Option<String>), String> {
    let (cert, key) = (dir.join("cert.pem"), dir.join("key.pem"));
    let warning = authority_warning(mkcert);
    if cert.is_file() && key.is_file() {
        return Ok(((cert, key), warning));
    }
    private_dir(dir)?;
    let out = Command::new(mkcert)
        .arg("-cert-file")
        .arg(&cert)
        .arg("-key-file")
        .arg(&key)
        .args(NAMES)
        .output()
        .map_err(|e| {
            format!(
                "HTTPS needs a certificate and {mkcert} cannot run ({e}): \
                 brew install mkcert && mkcert -install, or give --tls-cert and --tls-key, \
                 or use --http"
            )
        })?;
    if !out.status.success() || !cert.is_file() || !key.is_file() {
        return Err(format!(
            "{mkcert} could not make the certificate: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    private_file(&key)?;
    Ok(((cert, key), warning))
}

/// mkcert's authority missing from its directory: the browser will warn.
fn authority_warning(mkcert: &str) -> Option<String> {
    let out = Command::new(mkcert).arg("-CAROOT").output().ok()?;
    let root = PathBuf::from(String::from_utf8_lossy(&out.stdout).trim());
    (!root.join("rootCA.pem").is_file()).then(|| {
        "mkcert's authority is not installed: run `mkcert -install` once, \
         or the browser will not trust the page"
            .to_owned()
    })
}

#[cfg(unix)]
pub fn private_dir(dir: &Path) -> Result<(), String> {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)
        .and_then(|()| std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700)))
        .map_err(|e| format!("{}: {e}", dir.display()))
}

#[cfg(unix)]
fn private_file(f: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(f, std::fs::Permissions::from_mode(0o600))
        .map_err(|e| format!("{}: {e}", f.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn scratch(name: &str) -> PathBuf {
        let d =
            std::env::temp_dir().join(format!("bitchat-launcher-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    /// A fake mkcert that writes its two files.
    fn fake_mkcert(dir: &Path, ca: bool) -> String {
        std::fs::create_dir_all(dir).unwrap();
        let ca_dir = dir.join("ca");
        std::fs::create_dir_all(&ca_dir).unwrap();
        if ca {
            std::fs::write(ca_dir.join("rootCA.pem"), "x").unwrap();
        }
        let p = dir.join("mkcert");
        crate::testutil::script(
            &p,
            &format!(
                "[ \"$1\" = -CAROOT ] && {{ echo {}; exit 0; }}\n\
                 echo cert > \"$2\"; echo key > \"$4\"\n",
                ca_dir.display()
            ),
        );
        p.display().to_string()
    }

    #[test]
    fn made_once_private() {
        let d = scratch("tls");
        let bin = fake_mkcert(&d.join("bin"), true);
        let tls = d.join("tls");
        let ((cert, key), warn) = ensure(&tls, &bin).unwrap();
        assert!(warn.is_none());
        assert_eq!(std::fs::read_to_string(&cert).unwrap(), "cert\n");
        let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
        assert_eq!((mode(&tls), mode(&key)), (0o700, 0o600));
        // Kept: not made again.
        std::fs::write(&cert, "mine").unwrap();
        ensure(&tls, &bin).unwrap();
        assert_eq!(std::fs::read_to_string(&cert).unwrap(), "mine");
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn warnings_and_errors() {
        let d = scratch("tls-err");
        let bin = fake_mkcert(&d.join("bin"), false);
        let (_, warn) = ensure(&d.join("tls"), &bin).unwrap();
        assert!(warn.unwrap().contains("mkcert -install"));
        let e = ensure(&d.join("tls2"), "/nonexistent/mkcert").unwrap_err();
        assert!(e.contains("--http"), "{e}");
        std::fs::remove_dir_all(&d).unwrap();
    }
}
