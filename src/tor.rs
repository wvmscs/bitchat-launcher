// SPDX-FileCopyrightText: 2026 William VITAL, FR (wvmscs)
// SPDX-License-Identifier: GPL-3.0-or-later

//! Which Tor the console gets: `$BITCHAT_TOR_SOCKS`, then the daemon's
//! default port, then Arti.

use std::net::SocketAddr;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::args::{Tor, TorChoice};

/// The variable naming a Tor daemon's SOCKS port.
pub const ENV: &str = "BITCHAT_TOR_SOCKS";
/// The Tor daemon's default SOCKS port.
pub const DEFAULT_SOCKS: SocketAddr =
    SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), 9050);
/// Reached through the proxy to know that Tor has circuits, not only a port.
pub const PROBE: (&str, u16) = ("check.torproject.org", 443);

/// The SOCKS proxy at `addr` answers, with a user name like the console's
/// (one circuit per name), and opens a stream to `target`.
pub async fn check(addr: SocketAddr, target: (&str, u16), timeout: Duration) -> Result<(), String> {
    tokio::time::timeout(timeout, handshake(addr, target))
        .await
        .map_err(|_| format!("no answer within {} s", timeout.as_secs()))?
}

async fn handshake(addr: SocketAddr, (host, port): (&str, u16)) -> Result<(), String> {
    let mut s = TcpStream::connect(addr)
        .await
        .map_err(|e| format!("nothing listens ({e})"))?;
    let io = |e: std::io::Error| format!("connection lost ({e})");
    // Greeting: user name and password, the console's method.
    s.write_all(&[5, 1, 2]).await.map_err(io)?;
    let mut r = [0u8; 2];
    s.read_exact(&mut r).await.map_err(io)?;
    if r != [5, 2] {
        return Err("not a SOCKS5 proxy taking user names".into());
    }
    let user = b"bitchat-launcher/check";
    let mut auth = vec![1, user.len() as u8];
    auth.extend_from_slice(user);
    auth.extend_from_slice(&[1, b'x']);
    s.write_all(&auth).await.map_err(io)?;
    s.read_exact(&mut r).await.map_err(io)?;
    if r != [1, 0] {
        return Err("user name refused".into());
    }
    let mut req = vec![5, 1, 0, 3, host.len() as u8];
    req.extend_from_slice(host.as_bytes());
    req.extend_from_slice(&port.to_be_bytes());
    s.write_all(&req).await.map_err(io)?;
    let mut rep = [0u8; 4];
    s.read_exact(&mut rep).await.map_err(io)?;
    if rep[1] != 0 {
        return Err(format!(
            "cannot reach {host} through it (SOCKS reply {})",
            rep[1]
        ));
    }
    Ok(())
}

/// Where a daemon is looked for, in order: `env`, then `default`.
pub fn candidates(env: Option<&str>, default: SocketAddr) -> (Vec<SocketAddr>, Option<String>) {
    let mut v = Vec::new();
    let mut note = None;
    if let Some(e) = env.map(str::trim).filter(|e| !e.is_empty()) {
        match e.parse() {
            Ok(a) => v.push(a),
            Err(_) => note = Some(format!("{ENV}={e:?} is not address:port, ignored")),
        }
    }
    if !v.contains(&default) {
        v.push(default);
    }
    (v, note)
}

/// The Tor to give the console, and what was tried, for the user;
/// `default` is the daemon's port ([`DEFAULT_SOCKS`]).
pub async fn resolve(
    choice: &TorChoice,
    env: Option<&str>,
    default: SocketAddr,
    target: (&str, u16),
    timeout: Duration,
) -> Result<(Tor, Vec<String>), String> {
    match choice {
        TorChoice::Arti => Ok((Tor::Arti, vec!["Arti, as asked".into()])),
        TorChoice::Socks(addr) => match check(*addr, target, timeout).await {
            Ok(()) => Ok((Tor::Socks(*addr), vec![format!("Tor daemon at {addr}")])),
            Err(e) => Err(format!("no working Tor at {addr} (--tor-socks): {e}")),
        },
        TorChoice::Auto => {
            let (addrs, note) = candidates(env, default);
            let mut tried: Vec<String> = note.into_iter().collect();
            for addr in addrs {
                match check(addr, target, timeout).await {
                    Ok(()) => {
                        tried.push(format!("Tor daemon at {addr}"));
                        return Ok((Tor::Socks(addr), tried));
                    }
                    Err(e) => tried.push(format!("no Tor daemon at {addr}: {e}")),
                }
            }
            tried.push("falling back to Arti, inside bitchat-console".into());
            Ok((Tor::Arti, tried))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    /// A SOCKS5 proxy that accepts `ok_host` only.
    async fn fake_socks(ok_host: &'static str) -> SocketAddr {
        let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = l.local_addr().unwrap();
        tokio::spawn(async move {
            while let Ok((mut s, _)) = l.accept().await {
                tokio::spawn(async move {
                    let mut b = [0u8; 3];
                    s.read_exact(&mut b).await.unwrap();
                    assert_eq!(b, [5, 1, 2]);
                    s.write_all(&[5, 2]).await.unwrap();
                    let mut h = [0u8; 2];
                    s.read_exact(&mut h).await.unwrap();
                    let mut user = vec![0u8; h[1] as usize];
                    s.read_exact(&mut user).await.unwrap();
                    assert_eq!(user, b"bitchat-launcher/check");
                    let mut p = [0u8; 1];
                    s.read_exact(&mut p).await.unwrap();
                    let mut pass = vec![0u8; p[0] as usize];
                    s.read_exact(&mut pass).await.unwrap();
                    s.write_all(&[1, 0]).await.unwrap();
                    let mut req = [0u8; 5];
                    s.read_exact(&mut req).await.unwrap();
                    let mut host = vec![0u8; req[4] as usize + 2];
                    s.read_exact(&mut host).await.unwrap();
                    let ok = &host[..host.len() - 2] == ok_host.as_bytes();
                    let code = if ok { 0 } else { 4 };
                    s.write_all(&[5, code, 0, 1, 0, 0, 0, 0, 0, 0])
                        .await
                        .unwrap();
                });
            }
        });
        addr
    }

    const T: Duration = Duration::from_secs(5);

    #[tokio::test]
    async fn working_proxy() {
        let a = fake_socks("example.org").await;
        check(a, ("example.org", 443), T).await.unwrap();
        let e = check(a, ("other.org", 443), T).await.unwrap_err();
        assert!(e.contains("cannot reach other.org"), "{e}");
    }

    #[tokio::test]
    async fn nothing_or_silence() {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let closed = l.local_addr().unwrap();
        drop(l);
        assert!(
            check(closed, PROBE, T)
                .await
                .unwrap_err()
                .contains("nothing listens")
        );
        let silent = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let e = check(
            silent.local_addr().unwrap(),
            PROBE,
            Duration::from_millis(300),
        )
        .await
        .unwrap_err();
        assert!(e.contains("no answer"), "{e}");
    }

    #[test]
    fn candidate_order() {
        let d = DEFAULT_SOCKS;
        assert_eq!(candidates(None, d).0, [d]);
        let (v, n) = candidates(Some("127.0.0.1:9150"), d);
        assert_eq!(v, ["127.0.0.1:9150".parse().unwrap(), d]);
        assert!(n.is_none());
        assert_eq!(candidates(Some("127.0.0.1:9050"), d).0, [d]);
        let (v, n) = candidates(Some("localhost"), d);
        assert_eq!(v, [d]);
        assert!(n.unwrap().contains("ignored"));
    }

    #[tokio::test]
    async fn resolution() {
        let good = fake_socks("example.org").await;
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let dead = l.local_addr().unwrap();
        drop(l);
        let t = ("example.org", 443);
        let auto = |env: Option<String>, default| async move {
            resolve(&TorChoice::Auto, env.as_deref(), default, t, T)
                .await
                .unwrap()
        };
        // The variable first.
        let (tor, tried) = auto(Some(good.to_string()), dead).await;
        assert_eq!((tor, tried.len()), (Tor::Socks(good), 1));
        // A dead variable: the default port.
        let (tor, tried) = auto(Some(dead.to_string()), good).await;
        assert_eq!(tor, Tor::Socks(good));
        assert!(tried[0].contains("no Tor daemon"), "{tried:?}");
        // Neither: Arti.
        let (tor, tried) = auto(None, dead).await;
        assert_eq!(tor, Tor::Arti);
        assert!(tried.last().unwrap().contains("Arti"));
        // A proxy that answers but reaches nothing is not Tor.
        let (tor, _) = resolve(&TorChoice::Auto, None, good, ("other.org", 443), T)
            .await
            .unwrap();
        assert_eq!(tor, Tor::Arti);
        // Explicit choices: no fallback.
        assert!(
            resolve(&TorChoice::Socks(dead), None, good, t, T)
                .await
                .is_err()
        );
        assert_eq!(
            resolve(&TorChoice::Arti, None, good, t, T).await.unwrap().0,
            Tor::Arti
        );
    }
}
