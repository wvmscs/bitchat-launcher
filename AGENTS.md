# AGENTS.md

Guidance for coding agents (and humans) working in this crate.

## What this is

A launcher: it starts `bitchat-geohash` (the map) and `bitchat-console` (over it) as two child
processes, with free ports, HTTPS and a working Tor, and stops them together. It does not link
them: they need two SQLite versions that cannot be linked together (bitchat-console D26).

## Constraints

### Do NOT
- Modify `bitchat-geohash`, `bitchat-envelope`, `nip90-dvm`, the workspace `Nostr/Cargo.toml`
  or `src/` (read-only). `bitchat-console` and `geohash-relays` follow their own `AGENTS.md`.
- Give the console a way to reach relays without Tor: the launcher passes `--tor-socks` or
  `--arti`, never nothing.
- Serve anything beyond the loopback.
- Commit certificates, keys, `.env` or database files.

### DO
- Keep `PLAN.md` (French) up to date: decisions, dated journal line; `CHANGELOG.md`.
- Keep `--help`, the manual (`man/bitchat-launcher.1`) and the README in step with the options,
  and the manual's tool sections in step with the tools' options.
- Run the commands below before each commit.

### Others
- Own `[workspace]`, own `Cargo.lock`, GPL-3.0-only.
- Commits only when the user asks. Never force-push.

## Commands

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

## Style

Code, comments, `--help`, the manual and messages in English; `PLAN.md` in French. Messages
start with `bitchat-launcher:`; the tools' lines with `[map]` or `[console]`.
