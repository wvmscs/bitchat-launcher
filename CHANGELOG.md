# Changelog

## [Unreleased]

### Added
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
