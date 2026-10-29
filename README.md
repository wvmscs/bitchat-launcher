# bitchat-launcher

Starts the BitChat map ([bitchat-geohash](https://github.com/wvmscs/bitchat-geohash)) and the
console ([bitchat-console](https://github.com/wvmscs/bitchat-console)) over it, in one command:

```sh
bitchat-launcher u09tv --teleport
```

The browser then opens `https://localhost:8080/`: the map of the area, with the console
floating over it. Ctrl-C stops both tools.

`bitchat-launcher --help` gives the usual options; `bitchat-launcher --man` is the manual:
every option of the launcher and of both tools, ports, Tor, HTTPS, behind a proxy.

## Install

Two ways: the **binaries** (to use the tools), or a **development environment** (to build
and change them).

### Binaries

Each release on the [releases page](https://github.com/wvmscs/bitchat-launcher/releases)
(tags `bitchat-web-YYYY.MM.DD`) holds the three tools built together: `bitchat-launcher`,
`bitchat-console` (with Tor inside, Arti), `bitchat-geohash`, plus `bitchat-history` (the
console's message history) and the manual. Its `VERSIONS` file gives the version of each.

| System | File |
|---|---|
| macOS, Apple silicon | `bitchat-web-<version>-macos-arm64.pkg` (or `.tar.gz`) |
| macOS, Intel | `bitchat-web-<version>-macos-amd64.pkg` (or `.tar.gz`) |
| Linux x86-64 | `bitchat-web-<version>-linux-amd64.tar.gz` |
| Linux arm64 (Raspberry Pi OS 64-bit…) | `bitchat-web-<version>-linux-arm64.tar.gz` |
| Docker, linux/amd64 and linux/arm64 | `ghcr.io/wvmscs/bitchat-web` |

The macOS binaries are signed with a Developer ID and notarized by Apple; every file is
signed with GPG ([check them](#check-the-files)). The Linux binaries are static: nothing
else to install. There is no 32-bit build (`uname -m` must say `arm64`, `aarch64` or
`x86_64`).

The latest version, for the commands below:

```sh
VER=$(curl -s https://api.github.com/repos/wvmscs/bitchat-launcher/releases/latest |
      sed -n 's/.*"tag_name": *"bitchat-web-\([^"]*\)".*/v\1/p')
URL=https://github.com/wvmscs/bitchat-launcher/releases/download/bitchat-web-${VER#v}
echo "$VER"
```

#### macOS (arm64, amd64)

The package installs into `/usr/local/bin` and `/usr/local/share/man`:

```sh
ARCH=$(uname -m | sed 's/x86_64/amd64/')           # arm64 or amd64
curl -LO "$URL/bitchat-web-$VER-macos-$ARCH.pkg"
sudo installer -pkg "bitchat-web-$VER-macos-$ARCH.pkg" -target /
```

(or open the `.pkg` in the Finder). Without administrator rights, the archive instead:

```sh
curl -LO "$URL/bitchat-web-$VER-macos-$ARCH.tar.gz"
tar xzf "bitchat-web-$VER-macos-$ARCH.tar.gz"
PREFIX=~/.local sh "bitchat-web-$VER-macos-$ARCH/install.sh"   # then ~/.local/bin in $PATH
```

Then, once:

```sh
brew install mkcert && mkcert -install        # the browser trusts the page (HTTPS)
brew install tor && brew services start tor   # optional: else Arti, inside the console
```

#### Linux (amd64, arm64)

```sh
ARCH=$(uname -m | sed 's/x86_64/amd64/; s/aarch64/arm64/')   # amd64 or arm64
curl -LO "$URL/bitchat-web-$VER-linux-$ARCH.tar.gz"
tar xzf "bitchat-web-$VER-linux-$ARCH.tar.gz"
sudo sh "bitchat-web-$VER-linux-$ARCH/install.sh"            # or PREFIX=~/.local sh …
```

Then, once (Debian, Ubuntu, Raspberry Pi OS):

```sh
sudo apt install mkcert libnss3-tools && mkcert -install     # HTTPS; or run with --http
sudo apt install tor                                         # optional: else Arti
```

On a machine without a screen (a server, a Raspberry Pi), the page is served on its loopback
only: start `bitchat-launcher u09tv --http --no-open --port 8080` there, then
`ssh -L 8080:127.0.0.1:8080 <machine>` from yours and open `http://localhost:8080/`. For a
public name, see `--behind-proxy` above.

#### Docker (linux/amd64, linux/arm64)

One image for both architectures; Docker takes the right one. It starts the launcher in clear
HTTP (`--http`, no mkcert inside), Tor by Arti, its data in the volume `/data`:

```sh
docker pull ghcr.io/wvmscs/bitchat-web:latest
docker run --rm -it --network host -v bitchat:/data ghcr.io/wvmscs/bitchat-web u09tv --port 8080
```

The page is on the host's loopback: `http://localhost:8080/` there, or through an SSH tunnel
as above. `--network host` is needed because the page listens on the loopback only; on
Docker Desktop (macOS, Windows), turn on host networking first (*Settings → Resources →
Network*), or use the native binaries. Tagged versions: `ghcr.io/wvmscs/bitchat-web:<version>`
without the `v` (`2026.10.29`).

#### Check the files

`SHA256SUMS` lists every file of a release; it and each file have a GPG signature (`.asc`),
by the key `signing-key.asc` of the release, fingerprint
`9934 918F AC07 7C3D 02BF FF1D 4053 54B4 8708 C96F`:

```sh
curl -LO "$URL/SHA256SUMS" -LO "$URL/SHA256SUMS.asc" -LO "$URL/signing-key.asc"
gpg --import signing-key.asc
gpg --verify SHA256SUMS.asc SHA256SUMS
shasum -a 256 -c --ignore-missing SHA256SUMS                 # sha256sum on Linux
```

On macOS, `spctl -a -vv -t install bitchat-web-$VER-macos-$ARCH.pkg` must say
`source=Notarized Developer ID`.

#### Uninstall

Remove `bitchat-launcher`, `bitchat-console`, `bitchat-geohash` and `bitchat-history` from the
`bin` directory, and `bitchat-launcher.1` from `share/man/man1` (after the `.pkg`, also
`sudo pkgutil --forget fr.wvital.bitchat-web`). Your data stays in `~/.bitchat-console` and
`~/.bitchat-launcher` (Docker: the volume `bitchat`).

### Development environment

With Rust (`rustup`), from the sources of the three repositories:

```sh
brew install mkcert && mkcert -install        # once: the browser trusts the page
brew install tor && brew services start tor   # optional: else Arti is used
cargo install --git https://github.com/wvmscs/bitchat-console --features arti
cargo install --git https://github.com/wvmscs/bitchat-geohash
cargo install --git https://github.com/wvmscs/bitchat-launcher
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

GPL-3.0-or-later. Every file says so with its SPDX tags ([REUSE](https://reuse.software)): a header, or `REUSE.toml`;
the licence texts are in `LICENSES/`.
