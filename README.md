<p align="center">
  <img src="branding/logo-transparent.png" width="200" alt="Floppa CLI" />
</p>

<h1 align="center">floppa-cli</h1>

<p align="center">Headless CLI client for Floppa VPN — WireGuard / AmneziaWG / VLESS+REALITY, with auto-reconnect and a systemd unit.</p>

[![CI](https://github.com/ni9aii/floppa-CLI/actions/workflows/ci.yml/badge.svg)](https://github.com/ni9aii/floppa-CLI/actions/workflows/ci.yml)
[![License: GPL-3.0](https://img.shields.io/badge/license-GPL--3.0-blue.svg)](LICENSE)

## What this is

`floppa-cli` is a **fork of [okhsunrog/floppa-vpn](https://github.com/okhsunrog/floppa-vpn)** that keeps only the
standalone CLI connector. The upstream project also ships a server daemon, a Telegram bot, an admin panel and a
Tauri desktop app — none of that is part of this repo. Here you get the command-line client and its supporting
tooling (auto-reconnect, systemd integration, docs).

Use it to:

- Connect a headless box or server to Floppa VPN from the terminal.
- Run the tunnel as a `systemd` service that survives reboots and hard crashes.
- Auto-recover the tunnel after sleep/resume, Wi-Fi roaming, or a transient outage — no manual
  intervention.

## Features

- **Three protocols**: WireGuard, AmneziaWG (DPI-resistant WireGuard), and VLESS+REALITY — the same
  ones the upstream server supports. Protocol is selected at connect time (`--protocol`).
- **Config from a file or the API**: pass a `.conf` (WireGuard/AmneziaWG) or a `vless://` URI file, or
  let the client fetch a config from your account via `floppa-cli login` + `floppa-cli connect`.
- **Auto-reconnect** — a background watchdog health-checks the tunnel every 30 s and rebuilds it on
  drop; on Linux + systemd it also wakes instantly when the machine resumes from sleep. See
  [docs/RECONNECT.md](docs/RECONNECT.md).
- **systemd service** — `floppa-cli service install` renders a unit that starts the connector on boot
  and restarts it if it exits with a fatal error. Requires root.
- **DNS handling** — rewrites `/etc/resolv.conf` while connected and restores the previous contents on
  disconnect (`--no-dns` to opt out).

## Install

Prebuilt `floppa-cli-linux-x86_64` binaries are attached to each
[release](https://github.com/ni9aii/floppa-CLI/releases). Copy the binary somewhere on your `PATH`:

```bash
curl -L -o floppa-cli https://github.com/ni9aii/floppa-CLI/releases/latest/download/floppa-cli-linux-x86_64
chmod +x floppa-cli
sudo mv floppa-cli /usr/local/bin/
```

Or build from source (needs the Rust toolchain and the usual network-plumbing tools — `ip`, `wg`/`awg`,
`resolvectl`/`resolvconf`):

```bash
cargo build --release -p floppa-cli
# binary lands in target/release/floppa-cli
```

Connecting actually brings up an interface, so run it as root (or via `sudo`).

## Usage

```bash
# 1. Log in (opens a browser for Telegram auth), saves a token under ~/.config/floppa-cli/
floppa-cli login

# 2. Connect with an auto-fetched config (WireGuard by default)
sudo floppa-cli connect

# Or connect from a saved config file / VLESS URI file
sudo floppa-cli connect --config /etc/floppa-cli/client.conf --protocol amneziawg

# Inspect your account / peers
floppa-cli peers
floppa-cli config --protocol vless        # print a VLESS URI to stdout

# Drop the saved login token
floppa-cli logout
```

### Run as a systemd service

```bash
# Install + enable + start a unit that connects on boot (needs root)
sudo floppa-cli service install --config /etc/floppa-cli/client.conf --protocol wireguard

# Just print the unit that would be written, without touching the system
floppa-cli service print --config /etc/floppa-cli/client.conf

# Stop, disable and remove it
sudo floppa-cli service uninstall
```

See [systemd/floppa-cli.service](systemd/floppa-cli.service) for the hand-written example unit, and
[docs/RECONNECT.md](docs/RECONNECT.md) for how the in-process reconnect loop and the unit's
`Restart=on-failure` interact.

## Commands

| Command | Description |
|---------|-------------|
| `login` | Log in via Telegram (opens browser), saves a token. |
| `connect` | Connect to the VPN. Auto-detects a `.conf` or `vless://` file, or fetches a config from your account. |
| `peers` | List your peers. |
| `config` | Fetch and print a config (WireGuard/AmneziaWG `.conf` or a VLESS URI). |
| `logout` | Remove the saved login token. |
| `service` | Manage the systemd unit (`install` / `uninstall` / `print`). |

Global flag: `--log-file <path>` writes debug logs to a file instead of stderr.

## Development

```bash
# Build
cargo build -p floppa-cli

# Lint (CI enforces -D warnings)
cargo clippy -p floppa-cli -- -D warnings

# Test
cargo test -p floppa-cli
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for the workflow and code notes. The reconnect loop owns the
tunnel lifecycle — `connect_wireguard` / `connect_vless` build `rebuild` / `health` closures and hand
them to `reconnect::run`.

## Relationship to upstream

This repo tracks upstream `okhsunrog/floppa-vpn` for the client code but deliberately drops the
server/desktop stack. If you need the full product (bot, admin panel, Tauri app, Ansible deployment),
use the upstream repository.

## License

[GPL-3.0](LICENSE) — inherited from the upstream project.
