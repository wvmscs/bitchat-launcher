//! The two tools as child processes: found, started, watched, stopped.

use std::path::{Path, PathBuf};
use std::process::{ExitStatus, Stdio};
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::{Child, Command};

pub const CONSOLE: &str = "bitchat-console";
pub const MAP: &str = "bitchat-geohash";

/// The binary `name`: `given`, else next to the launcher (`beside`), else
/// on `path` (`$PATH`).
pub fn find(
    name: &str,
    given: Option<&Path>,
    beside: Option<&Path>,
    path: Option<&std::ffi::OsStr>,
) -> Result<PathBuf, String> {
    if let Some(g) = given {
        return if g.is_file() {
            Ok(g.to_owned())
        } else {
            Err(format!("{}: no such file", g.display()))
        };
    }
    beside
        .map(|d| d.join(name))
        .into_iter()
        .chain(
            path.into_iter()
                .flat_map(std::env::split_paths)
                .map(|d| d.join(name)),
        )
        .find(|p| p.is_file())
        .ok_or_else(|| {
            let features = if name == CONSOLE {
                " --features arti"
            } else {
                ""
            };
            format!(
                "{name} not found next to the launcher nor on $PATH: \
                 cargo install --path <{name} sources>{features}, or give its path \
                 (--{}-bin)",
                if name == CONSOLE { "console" } else { "map" }
            )
        })
}

/// A running tool, whose output is printed with its name in front.
pub struct Tool {
    pub name: &'static str,
    pub label: &'static str,
    child: Child,
}

impl Tool {
    /// Starts `bin` in its own process group, so that the terminal's Ctrl-C
    /// reaches the launcher only, which stops the tools in order.
    pub fn spawn(
        name: &'static str,
        label: &'static str,
        bin: &Path,
        args: &[String],
    ) -> Result<Tool, String> {
        let mut child = Command::new(bin)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0)
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| format!("cannot start {}: {e}", bin.display()))?;
        if let Some(out) = child.stdout.take() {
            tokio::spawn(forward(label, out));
        }
        if let Some(err) = child.stderr.take() {
            tokio::spawn(forward(label, err));
        }
        Ok(Tool { name, label, child })
    }

    /// Its exit status, once it has stopped.
    pub fn exited(&mut self) -> Option<ExitStatus> {
        self.child.try_wait().ok().flatten()
    }

    pub async fn wait(&mut self) -> Option<ExitStatus> {
        self.child.wait().await.ok()
    }

    /// SIGINT (as Ctrl-C would), then SIGKILL after `grace`.
    pub async fn stop(&mut self, grace: Duration) {
        if self.exited().is_some() {
            return;
        }
        if let Some(pid) = self.child.id() {
            // SAFETY: a plain signal to our own child's pid.
            unsafe {
                libc::kill(pid as libc::pid_t, libc::SIGINT);
            }
        }
        if tokio::time::timeout(grace, self.child.wait())
            .await
            .is_err()
        {
            eprintln!(
                "bitchat-launcher: {} did not stop within {} s, killed",
                self.name,
                grace.as_secs()
            );
            let _ = self.child.kill().await;
        }
    }
}

async fn forward(label: &'static str, r: impl AsyncRead + Unpin) {
    let mut lines = BufReader::new(r).lines();
    while let Ok(Some(l)) = lines.next_line().await {
        eprintln!("[{label}] {l}");
    }
}

/// Something listens on the loopback port.
pub async fn listening(port: u16) -> bool {
    tokio::time::timeout(
        Duration::from_millis(500),
        tokio::net::TcpStream::connect(("127.0.0.1", port)),
    )
    .await
    .is_ok_and(|r| r.is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn scratch(name: &str) -> PathBuf {
        let d =
            std::env::temp_dir().join(format!("bitchat-launcher-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn finding() {
        let d = scratch("find");
        let (beside, on_path) = (d.join("a"), d.join("b"));
        for dir in [&beside, &on_path] {
            std::fs::create_dir_all(dir).unwrap();
        }
        std::fs::write(on_path.join(MAP), "").unwrap();
        let path = std::env::join_paths([&beside, &on_path]).unwrap();
        assert_eq!(
            find(MAP, None, Some(&beside), Some(&path)).unwrap(),
            on_path.join(MAP)
        );
        std::fs::write(beside.join(MAP), "").unwrap();
        assert_eq!(
            find(MAP, None, Some(&beside), Some(&path)).unwrap(),
            beside.join(MAP)
        );
        let given = d.join("given");
        assert!(
            find(MAP, Some(&given), Some(&beside), None)
                .unwrap_err()
                .contains("no such file")
        );
        std::fs::write(&given, "").unwrap();
        assert_eq!(find(MAP, Some(&given), None, None).unwrap(), given);
        let e = find(CONSOLE, None, None, None).unwrap_err();
        assert!(
            e.contains("--features arti") && e.contains("--console-bin"),
            "{e}"
        );
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[tokio::test]
    async fn stopped_by_sigint() {
        let d = scratch("sigint");
        let script = d.join("tool");
        // Exits 0 on SIGINT; says when its trap is set.
        let ready = d.join("ready");
        std::fs::write(
            &script,
            format!(
                "#!/bin/sh\ntrap 'exit 0' INT\ntouch {}\nwhile :; do sleep 0.1; done\n",
                ready.display()
            ),
        )
        .unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        let mut t = Tool::spawn("tool", "t", &script, &[]).unwrap();
        while !ready.exists() {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert!(t.exited().is_none());
        t.stop(Duration::from_secs(5)).await;
        let s = t.exited();
        assert!(s.is_some_and(|s| s.success()), "{s:?}");
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[tokio::test]
    async fn killed_when_deaf() {
        let d = scratch("deaf");
        let script = d.join("tool");
        std::fs::write(
            &script,
            "#!/bin/sh\ntrap '' INT\nwhile :; do sleep 0.1; done\n",
        )
        .unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        let mut t = Tool::spawn("tool", "t", &script, &[]).unwrap();
        tokio::time::sleep(Duration::from_millis(300)).await;
        t.stop(Duration::from_millis(500)).await;
        assert!(!t.wait().await.unwrap().success());
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[tokio::test]
    async fn port_check() {
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let p = l.local_addr().unwrap().port();
        assert!(listening(p).await);
        // A port bound but not listening: refused, and no other test can
        // take it meanwhile.
        let held = tokio::net::TcpSocket::new_v4().unwrap();
        held.bind("127.0.0.1:0".parse().unwrap()).unwrap();
        assert!(!listening(held.local_addr().unwrap().port()).await);
        drop(l);
    }
}
