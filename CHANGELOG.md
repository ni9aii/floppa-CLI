# Changelog

All notable changes to the `floppa-cli` crate are documented here. The format
is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this
project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html)
for the CLI crate.

## [0.3.0-cli] - 2026-07-18

First stable release of the CLI-only connector (no `-cli-alpha` suffix).

### Added
- **`floppa-cli service`** (fork-only): install/uninstall the connector as a
  systemd unit. `service install` renders `/etc/systemd/system/floppa-cli.service`
  pointing at the current binary + chosen connect args, then `daemon-reload` +
  `enable --now` (use `--no-start` to enable without starting). `service uninstall`
  disables, stops and removes it; `service print` dumps the unit to stdout.
  Requires root for install/uninstall.
- **Auto-reconnect** (`reconnect.rs`):
  - Background watchdog that health-checks the tunnel every 30 s
    (WireGuard handshake age / VLESS TCP reachability) and rebuilds it on
    drop.
  - **Instant wake on system resume**: subscribes to systemd-logind
    `PrepareForSleep` over D-Bus (Linux) so the tunnel is rebuilt the moment
    the machine wakes from sleep — no waiting for the next watchdog tick.
  - Exponential backoff (2 s → 60 s cap) with retryable vs. fatal error
    classification; fatal errors surface so systemd `Restart=on-failure` kicks
    in.
  - `docs/RECONNECT.md` describing the mechanism and tuning knobs.
- **Docs rewritten for the CLI-only fork**: `README.md` now describes only the
  CLI client (not the upstream daemon/bot/admin/Tauri stack), plus a new
  rewritten README install/usage section.
- `systemd/floppa-cli.service` — example unit with `Restart=on-failure` so a
  fatal CLI exit is recovered by systemd (the in-process reconnect loop covers
  transient drops on its own).
- Unit tests for the reconnect loop (`run`): initial connect, rebuild-on-
  unhealthy, plus the existing backoff / signal / config coverage.
- `CONTRIBUTING.md`, `SECURITY.md`, `CHANGELOG.md` for repo hygiene.

### Changed
- `connect_wireguard` / `connect_vless` now drive the reconnect loop instead
  of blocking on a one-shot `wait_for_shutdown`. Shutdown (Ctrl+C / SIGTERM)
  still tears the tunnel down cleanly.

### Fixed
- Committed `Cargo.lock` so the `rpassword` dependency addition is
  reproducible.

## [0.2.0-cli-alpha] - 2026-07-10

### Added
- `rpassword`-based password prompt (with `FLOPPA_PASSWORD` env fallback) for
  token retrieval.
- `just` task runner targets (`build`, `lint`, `test`, `run`).

### Changed
- CLI split into the `floppa-cli` crate inside the `floppa-CLI` workspace.
