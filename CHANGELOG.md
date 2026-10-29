# Changelog

## [Unreleased]

## [0.1.1] - 2026-10-29

### Added
- README: installing the binaries (macOS and Linux, arm64 and amd64, Docker), checking them
  (GPG, SHA-256, Gatekeeper) and uninstalling, apart from the development environment. The
  signed packages are published in this repository's releases too (tags `bitchat-web-*`).
- `--behind-proxy https://<name>` and `--no-proxy-check` (L12): a TLS proxy on this machine
  serves the page at that name; the console runs behind it (its D34), the map behind the
  console, no mkcert; the page's port is `--port` or 8080, never another one; the browser
  opens at the public name. `--behind-proxy`, `--proxy-from` and `--no-proxy-check` are refused
  in `--console-args` and `--map-args`.

### Added
- Opening to the public: `SECURITY.md`, `CONTRIBUTING.md`, issue and pull request templates, CI (fmt, clippy, tests, `cargo deny`), `deny.toml`, Dependabot; actions pinned by commit.

## [0.1.0] - 2026-10-29

### Added
- Manual: the map follows the console's cell (bitchat-geohash 0.2.0).
- `bitchat-launcher <geohash>`: starts bitchat-geohash and bitchat-console over it, on 8080 or
  a free port of 50000–60000 (the map on another one, loopback only), in HTTPS with a mkcert
  certificate made once (`--tls-cert`/`--tls-key`, `--http`), with the first working Tor of
  `$BITCHAT_TOR_SOCKS`, 127.0.0.1:9050 and Arti (`--tor-socks`, `--arti`); opens the page when
  both answer; Ctrl-C stops both (SIGINT, 20 s, then SIGKILL); one stopping stops the other.
- `--console-args`, `--map-args` (split like a shell; the options the launcher sets are
  refused), `--console-bin`, `--map-bin`, `--data-dir`, `--map-geohash`, `--depth`,
  `--teleport`, `--no-open`, `--port`, `--verbose`.
- `--help` (most used options first), `--man` (manual `man/bitchat-launcher.1`, with every
  option of both tools), `--version`.
- Tests: arguments, ports, SOCKS check against a fake proxy, mkcert with a fake one, child
  processes (SIGINT, SIGKILL), the binary with two fake tools.
