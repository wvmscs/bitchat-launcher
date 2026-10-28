# bitchat-launcher

Starts the BitChat map ([bitchat-geohash](https://github.com/wvmscs/bitchat-geohash)) and the
console ([bitchat-console](https://github.com/wvmscs/bitchat-console)) over it, in one command:

```sh
bitchat-launcher u09tv --teleport
```

- **Ports**: the page on 8080, or on a free port between 50000 and 60000 when 8080 is taken;
  the map on another free port of that range, reached by the console only.
- **HTTPS** (HTTP/2, no 6-stream limit of the browser): a certificate for localhost made once
  with mkcert in `~/.bitchat-launcher/tls`.
- **Tor** for the console: `$BITCHAT_TOR_SOCKS`, else the Tor daemon on 127.0.0.1:9050, else
  Arti inside bitchat-console. The map does not use Tor (read-only, it never publishes).
- Opens the page when both answer; Ctrl-C stops both; if one stops, the other is stopped.

`bitchat-launcher --help` lists the options, the most used first; `bitchat-launcher --man`
is the manual, with every option of both tools (`--console-args`, `--map-args`).

## Install

```sh
brew install mkcert && mkcert -install        # once: the browser trusts the page
brew install tor && brew services start tor   # optional: else Arti is used
cargo install --path ../bitchat-console --features arti
cargo install --path ../bitchat-geohash
cargo install --path .
```

The tools are looked for next to the launcher, then on `$PATH` (or `--console-bin`,
`--map-bin`). The manual: `man ./man/bitchat-launcher.1`, or copy it into a `man1` directory.

## Tests

```sh
cargo test     # offline: fake tools, fake SOCKS proxy, fake mkcert
```

## Documents

[PLAN.md](PLAN.md) (in French): decisions and journal. [AGENTS.md](AGENTS.md): working rules.
[CHANGELOG.md](CHANGELOG.md).

## Licence

GPL-3.0-only.
