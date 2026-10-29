// SPDX-FileCopyrightText: 2026 William VITAL, FR (wvmscs)
// SPDX-License-Identifier: GPL-3.0-or-later

//! The command line: the launcher's own options, and what goes to each tool.

use std::net::SocketAddr;
use std::path::PathBuf;

/// `--help`: the most usual options first.
pub const HELP: &str = "\
bitchat-launcher: the BitChat map (bitchat-geohash) with the console (bitchat-console) over it

usage: bitchat-launcher <geohash> [options]

  <geohash>              cell the console follows, with its 8 neighbours (e.g. u09tv)
  --teleport             you are not there: messages tagged teleported, no presence
  --map-geohash <gh>     area of the map (default: <geohash> without its last letter)
  --depth <n>            channel levels the map watches below its area (map's default: 1)
  --no-open              do not open the browser
  --port <n>             port of the page (default: 8080, else a free one in 50000-60000)

Tor (the console's relays; the map connects directly):
  --tor-socks <addr:port>  Tor daemon to use, no fallback
  --arti                 Tor inside bitchat-console (built with --features arti)
                         default: $BITCHAT_TOR_SOCKS, then 127.0.0.1:9050, then Arti

HTTPS (default; certificate made with mkcert in <data-dir>/tls):
  --tls-cert <pem> --tls-key <pem>   own certificate
  --http                 clear HTTP on the loopback (the browser then keeps 6 live streams)
  --behind-proxy <https://name>  a TLS proxy on this machine serves the page at that name
                         (Cloudflare Tunnel, NGINX, Caddy) to the port of --port (default
                         8080); the console checks at start that the name leads to it
  --no-proxy-check       with --behind-proxy: start without that check

Tools:
  --console-args '<args>'  more options for bitchat-console (see --man)
  --map-args '<args>'      more options for bitchat-geohash (see --man)
  --console-bin <path>   bitchat-console binary (default: next to this one, else $PATH)
  --map-bin <path>       bitchat-geohash binary (default: next to this one, else $PATH)
  --data-dir <dir>       certificate and map database (default: ~/.bitchat-launcher)
  --verbose              both tools log in detail

  --help                 this text
  --man                  the manual, with every option of both tools
  --version              version

Ctrl-C stops both tools. Exit code 2 for a wrong command line, 1 when a tool cannot start
or stops on its own.";

/// Options the launcher sets itself: not accepted in `--console-args`.
const CONSOLE_MANAGED: &[&str] = &[
    "--listen",
    "--allow-remote",
    "--tls-cert",
    "--tls-key",
    "--behind-proxy",
    "--proxy-from",
    "--no-proxy-check",
    "--map",
    "--tor-socks",
    "--arti",
    "--open",
];

/// Options the launcher sets itself: not accepted in `--map-args`.
const MAP_MANAGED: &[&str] = &[
    "--listen",
    "--allow-remote",
    "--insecure-remote",
    "--tls-cert",
    "--tls-key",
    "--behind-proxy",
    "--proxy-from",
    "--no-proxy-check",
    "--open",
];

/// Which Tor the console uses.
#[derive(Debug, Clone, PartialEq)]
pub enum TorChoice {
    /// `$BITCHAT_TOR_SOCKS`, then the daemon's default port, then Arti.
    Auto,
    /// This daemon, or nothing.
    Socks(SocketAddr),
    Arti,
}

/// How the page is served.
#[derive(Debug, Clone, PartialEq)]
pub enum Https {
    /// A mkcert certificate in `<data-dir>/tls`, made if missing.
    Auto,
    Files {
        cert: PathBuf,
        key: PathBuf,
    },
    Off,
    /// Clear HTTP on the loopback, to a TLS proxy of this machine that serves
    /// `public` (L12); `check`: the console checks that it leads to it.
    Proxy {
        public: String,
        check: bool,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Args {
    pub geohash: String,
    pub teleport: bool,
    pub map_geohash: String,
    pub depth: Option<u32>,
    pub open: bool,
    pub port: Option<u16>,
    pub tor: TorChoice,
    pub https: Https,
    pub data_dir: PathBuf,
    pub console_bin: Option<PathBuf>,
    pub map_bin: Option<PathBuf>,
    pub console_args: Vec<String>,
    pub map_args: Vec<String>,
    pub verbose: bool,
}

#[derive(Debug, PartialEq)]
pub enum Command {
    Run(Box<Args>),
    Help,
    Man,
    Version,
}

const BASE32: &str = "0123456789bcdefghjkmnpqrstuvwxyz";

fn geohash(flag: &str, v: String) -> Result<String, String> {
    let g = v.to_ascii_lowercase();
    if g.is_empty() || g.len() > 12 || !g.chars().all(|c| BASE32.contains(c)) {
        return Err(format!("{flag}: geohash expected, got {v:?}"));
    }
    Ok(g)
}

/// Reads the arguments (without the program name); `home` gives the
/// default data directory.
pub fn parse(
    args: impl IntoIterator<Item = String>,
    home: Option<PathBuf>,
) -> Result<Command, String> {
    let mut it = args.into_iter();
    let mut geo = None;
    let mut a = Args {
        geohash: String::new(),
        teleport: false,
        map_geohash: String::new(),
        depth: None,
        open: true,
        port: None,
        tor: TorChoice::Auto,
        https: Https::Auto,
        data_dir: home
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".bitchat-launcher"),
        console_bin: None,
        map_bin: None,
        console_args: Vec::new(),
        map_args: Vec::new(),
        verbose: false,
    };
    let mut map_geohash = None;
    let (mut cert, mut key, mut http) = (None::<PathBuf>, None::<PathBuf>, false);
    let (mut public, mut no_check) = (None::<String>, false);
    while let Some(arg) = it.next() {
        let mut value = || it.next().ok_or_else(|| format!("{arg} needs a value"));
        match arg.as_str() {
            "--help" | "-h" => return Ok(Command::Help),
            "--man" => return Ok(Command::Man),
            "--version" | "-V" => return Ok(Command::Version),
            "--teleport" => a.teleport = true,
            "--map-geohash" => map_geohash = Some(geohash(&arg, value()?)?),
            "--depth" => {
                let v = value()?;
                a.depth = Some(
                    v.parse()
                        .map_err(|_| format!("--depth: number of levels expected, got {v:?}"))?,
                )
            }
            "--no-open" => a.open = false,
            "--port" => {
                let v = value()?;
                a.port = Some(
                    v.parse()
                        .ok()
                        .filter(|p| *p != 0)
                        .ok_or(format!("--port: 1 to 65535, got {v:?}"))?,
                )
            }
            "--tor-socks" => {
                let v = value()?;
                a.tor = TorChoice::Socks(
                    v.parse()
                        .map_err(|_| format!("--tor-socks: address:port expected, got {v:?}"))?,
                )
            }
            "--arti" => a.tor = TorChoice::Arti,
            "--tls-cert" => cert = Some(value()?.into()),
            "--tls-key" => key = Some(value()?.into()),
            "--http" => http = true,
            "--behind-proxy" => {
                let v = value()?;
                if !v.starts_with("https://") {
                    return Err(format!(
                        "--behind-proxy: the public address, https://name, got {v:?}"
                    ));
                }
                public = Some(v)
            }
            "--no-proxy-check" => no_check = true,
            "--console-args" => a.console_args.extend(split(&value()?)?),
            "--map-args" => a.map_args.extend(split(&value()?)?),
            "--console-bin" => a.console_bin = Some(value()?.into()),
            "--map-bin" => a.map_bin = Some(value()?.into()),
            "--data-dir" => a.data_dir = value()?.into(),
            "--verbose" => a.verbose = true,
            s if s.starts_with('-') => {
                return Err(format!(
                    "unknown option {s} (options of the tools go in --console-args or --map-args)"
                ));
            }
            _ if geo.is_none() => geo = Some(geohash("the geohash", arg)?),
            _ => return Err(format!("one geohash only, got {arg:?} too")),
        }
    }
    a.geohash = geo.ok_or("the geohash to follow is missing")?;
    a.map_geohash = map_geohash.unwrap_or_else(|| {
        let g = &a.geohash;
        g[..g.len().saturating_sub(1).max(1)].to_owned()
    });
    a.https = match (cert, key, http) {
        (None, None, false) if public.is_some() => Https::Proxy {
            public: public.take().unwrap_or_default(),
            check: !no_check,
        },
        _ if public.is_some() => {
            return Err(
                "--behind-proxy: the proxy does the TLS (no --http, --tls-cert nor \
                 --tls-key)"
                    .into(),
            );
        }
        _ if no_check => return Err("--no-proxy-check goes with --behind-proxy".into()),
        (None, None, false) => Https::Auto,
        (None, None, true) => Https::Off,
        (Some(cert), Some(key), false) => Https::Files { cert, key },
        (_, _, true) => return Err("--http and --tls-cert/--tls-key exclude each other".into()),
        _ => return Err("--tls-cert and --tls-key go together".into()),
    };
    managed("--console-args", &a.console_args, CONSOLE_MANAGED)?;
    managed("--map-args", &a.map_args, MAP_MANAGED)?;
    Ok(Command::Run(Box::new(a)))
}

fn managed(what: &str, args: &[String], set: &[&str]) -> Result<(), String> {
    match args.iter().find(|a| set.contains(&a.as_str())) {
        Some(a) => Err(format!("{what}: {a} is set by the launcher")),
        None => Ok(()),
    }
}

/// Splits `--console-args` and `--map-args` like a shell: blanks separate,
/// quotes group, a backslash escapes.
pub fn split(s: &str) -> Result<Vec<String>, String> {
    let (mut out, mut cur, mut any) = (Vec::new(), String::new(), false);
    let mut quote = None;
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        match (quote, c) {
            (None, c) if c.is_whitespace() => {
                if any {
                    out.push(std::mem::take(&mut cur));
                    any = false;
                }
            }
            (None, '\'' | '"') => (quote, any) = (Some(c), true),
            (Some(q), c) if c == q => quote = None,
            (Some('\''), c) => cur.push(c),
            (_, '\\') => {
                cur.push(chars.next().ok_or("a backslash ends the arguments")?);
                any = true;
            }
            (_, c) => {
                cur.push(c);
                any = true;
            }
        }
    }
    if quote.is_some() {
        return Err(format!("unclosed quote in {s:?}"));
    }
    if any {
        out.push(cur);
    }
    Ok(out)
}

/// Arguments of bitchat-geohash, serving the map on the loopback port `port`.
pub fn map_command(a: &Args, port: u16) -> Vec<String> {
    let mut v = vec![
        a.map_geohash.clone(),
        "--listen".into(),
        format!("127.0.0.1:{port}"),
        "--db".into(),
        a.data_dir
            .join("bitchat-geohash.sqlite")
            .display()
            .to_string(),
    ];
    if let Some(d) = a.depth {
        v.extend(["--depth".into(), d.to_string()]);
    }
    if a.verbose {
        v.push("--verbose".into());
    }
    // Last, so that the user's --db wins.
    v.extend(a.map_args.iter().cloned());
    v
}

/// The Tor the console is given.
#[derive(Debug, Clone, PartialEq)]
pub enum Tor {
    Socks(SocketAddr),
    Arti,
}

/// Arguments of bitchat-console, serving the page on `port` over the map on
/// `map_port`. Behind a proxy, only the console is: the map stays behind it.
pub fn console_command(
    a: &Args,
    port: u16,
    map_port: u16,
    tor: &Tor,
    tls: Option<(&PathBuf, &PathBuf)>,
) -> Vec<String> {
    let mut v = vec![
        a.geohash.clone(),
        "--listen".into(),
        format!("127.0.0.1:{port}"),
        "--map".into(),
        format!("http://127.0.0.1:{map_port}"),
    ];
    if a.teleport {
        v.push("--teleport".into());
    }
    match tor {
        Tor::Socks(addr) => v.extend(["--tor-socks".into(), addr.to_string()]),
        Tor::Arti => v.push("--arti".into()),
    }
    if let Some((cert, key)) = tls {
        v.extend([
            "--tls-cert".into(),
            cert.display().to_string(),
            "--tls-key".into(),
            key.display().to_string(),
        ]);
    }
    if let Https::Proxy { public, check } = &a.https {
        v.extend(["--behind-proxy".into(), public.clone()]);
        if !check {
            v.push("--no-proxy-check".into());
        }
    }
    if a.verbose {
        v.push("--verbose".into());
    }
    v.extend(a.console_args.iter().cloned());
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(s: &[&str]) -> Result<Args, String> {
        match parse(
            s.iter().map(|s| s.to_string()),
            Some(PathBuf::from("/home/u")),
        )? {
            Command::Run(a) => Ok(*a),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn defaults() {
        let a = run(&["U09TV"]).unwrap();
        assert_eq!(
            (a.geohash.as_str(), a.map_geohash.as_str()),
            ("u09tv", "u09t")
        );
        assert!(a.open && !a.teleport && !a.verbose);
        assert_eq!(
            (a.tor, a.https, a.port),
            (TorChoice::Auto, Https::Auto, None)
        );
        assert_eq!(a.data_dir, PathBuf::from("/home/u/.bitchat-launcher"));
        assert_eq!(run(&["u"]).unwrap().map_geohash, "u");
    }

    #[test]
    fn options() {
        let a = run(&[
            "u09tv",
            "--teleport",
            "--map-geohash",
            "u0",
            "--depth",
            "2",
            "--no-open",
            "--port",
            "9000",
            "--tor-socks",
            "127.0.0.1:9150",
            "--tls-cert",
            "c.pem",
            "--tls-key",
            "k.pem",
            "--console-args",
            "--pow-bits 12 --no-presence",
            "--map-args",
            "--tiles https://t/{z}/{x}/{y}.png --tiles-attribution 'tiles: me, you'",
            "--console-bin",
            "/b/c",
            "--map-bin",
            "/b/m",
            "--data-dir",
            "/d",
            "--verbose",
        ])
        .unwrap();
        assert!(a.teleport && !a.open && a.verbose);
        assert_eq!(
            (a.map_geohash.as_str(), a.depth, a.port),
            ("u0", Some(2), Some(9000))
        );
        assert_eq!(a.tor, TorChoice::Socks("127.0.0.1:9150".parse().unwrap()));
        assert_eq!(
            a.https,
            Https::Files {
                cert: "c.pem".into(),
                key: "k.pem".into()
            }
        );
        assert_eq!(a.console_args, ["--pow-bits", "12", "--no-presence"]);
        assert_eq!(a.map_args[3], "tiles: me, you");
        assert_eq!(a.console_bin, Some(PathBuf::from("/b/c")));
        assert_eq!(run(&["u09", "--arti", "--http"]).unwrap().https, Https::Off);
    }

    #[test]
    fn behind_a_proxy() {
        let a = run(&[
            "u09tv",
            "--behind-proxy",
            "https://b.example.org",
            "--port",
            "8090",
        ])
        .unwrap();
        assert_eq!(
            a.https,
            Https::Proxy {
                public: "https://b.example.org".into(),
                check: true
            }
        );
        let v = console_command(&a, 8090, 51000, &Tor::Arti, None);
        let at = v.iter().position(|s| s == "--behind-proxy").unwrap();
        assert_eq!(v[at + 1], "https://b.example.org");
        assert_eq!(v[1..3], ["--listen", "127.0.0.1:8090"]);
        assert!(!v.contains(&"--no-proxy-check".to_string()));
        let a = run(&[
            "u09tv",
            "--behind-proxy",
            "https://b.example.org",
            "--no-proxy-check",
        ])
        .unwrap();
        assert!(
            console_command(&a, 8080, 51000, &Tor::Arti, None)
                .contains(&"--no-proxy-check".to_string())
        );
        assert!(!map_command(&a, 51000).contains(&"--behind-proxy".to_string()));

        let err = |s: &[&str]| run(s).unwrap_err();
        assert!(err(&["u09", "--behind-proxy", "http://b"]).contains("https://name"));
        assert!(
            err(&["u09", "--behind-proxy", "https://b", "--http"])
                .contains("the proxy does the TLS")
        );
        assert!(err(&["u09", "--no-proxy-check"]).contains("goes with --behind-proxy"));
        assert!(
            err(&["u09", "--console-args", "--proxy-from 10.0.0.1"])
                .contains("set by the launcher")
        );
    }

    #[test]
    fn errors() {
        let err = |s: &[&str]| run(s).unwrap_err();
        assert_eq!(err(&["--teleport"]), "the geohash to follow is missing");
        assert!(err(&["u09", "--pow-bits", "3"]).contains("--console-args"));
        assert!(err(&["u0a"]).contains("geohash expected"));
        assert!(err(&["u09", "u0"]).contains("one geohash only"));
        assert!(err(&["u09", "--port", "0"]).contains("1 to 65535"));
        assert!(err(&["u09", "--tls-cert", "c"]).contains("go together"));
        assert!(err(&["u09", "--http", "--tls-cert", "c", "--tls-key", "k"]).contains("exclude"));
        assert_eq!(
            err(&["u09", "--console-args", "--listen 0.0.0.0:1"]),
            "--console-args: --listen is set by the launcher"
        );
        assert!(err(&["u09", "--map-args", "--open"]).contains("set by the launcher"));
        assert!(err(&["u09", "--map-args", "'x"]).contains("unclosed quote"));
        assert_eq!(err(&["u09", "--depth"]), "--depth needs a value");
    }

    #[test]
    fn help_man_version() {
        let p = |s: &str| parse([s.to_string()], None).unwrap();
        assert_eq!(p("--help"), Command::Help);
        assert_eq!(p("-h"), Command::Help);
        assert_eq!(p("--man"), Command::Man);
        assert_eq!(p("--version"), Command::Version);
    }

    #[test]
    fn splitting() {
        assert_eq!(
            split("  a  b\\ c \"d e\" '' x'y'z ").unwrap(),
            ["a", "b c", "d e", "", "xyz"]
        );
        assert!(split("").unwrap().is_empty());
        assert_eq!(split(r#""a\"b""#).unwrap(), [r#"a"b"#]);
    }

    #[test]
    fn commands() {
        let mut a = run(&["u09tv", "--teleport", "--depth", "2", "--verbose"]).unwrap();
        a.map_args = vec!["--db".into(), "/x.sqlite".into()];
        a.console_args = vec!["--no-presence".into()];
        assert_eq!(
            map_command(&a, 51000).join(" "),
            "u09t --listen 127.0.0.1:51000 --db /home/u/.bitchat-launcher/bitchat-geohash.sqlite \
             --depth 2 --verbose --db /x.sqlite"
        );
        let (c, k) = (PathBuf::from("/t/c.pem"), PathBuf::from("/t/k.pem"));
        assert_eq!(
            console_command(
                &a,
                8080,
                51000,
                &Tor::Socks("127.0.0.1:9050".parse().unwrap()),
                Some((&c, &k))
            )
            .join(" "),
            "u09tv --listen 127.0.0.1:8080 --map http://127.0.0.1:51000 --teleport \
             --tor-socks 127.0.0.1:9050 --tls-cert /t/c.pem --tls-key /t/k.pem --verbose --no-presence"
        );
        assert_eq!(
            console_command(&a, 8080, 51000, &Tor::Arti, None)[5..7],
            ["--teleport", "--arti"]
        );
    }
}
