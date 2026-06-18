# Changelog

All notable changes to `floppa-cli` are documented in this file.

## 0.1.0-cli-alpha - 2026-06-18

### Added

- `floppa-cli --version` for release/version checks.
- `scripts/smoke-test.sh` for local and CI release-prep validation.
- GitHub Actions CI based on the shared smoke-test script.
- GitHub Actions release workflow for Linux, Windows, and macOS CLI binaries.
- GitHub Release draft generation with Linux, Windows, and macOS binaries plus `SHA256SUMS.txt`.
- CLI-only fork with `floppa-cli`.
- Stable local `device_id` stored under the user config directory.
- Peer reuse by `device_id + protocol` via the API.
- Peer lifecycle commands:
  - `peer delete --peer-id <id>`
  - `peer delete --protocol amneziawg`
  - `peer delete --all`
  - `vless regenerate`
  - `device show`
  - `device reset`
- `status` command without API dependency.
- `stop` command for graceful tunnel shutdown.
- Built-in PATH self-configuration for child commands.
- Linux CLI networking code for WireGuard/AmneziaWG and VLESS+REALITY tunnels.
- Telegram login/auth flow used by the CLI.

### Changed

- CI now uses the shared smoke-test script instead of duplicating command lists.
- README now includes release workflow and verification notes.
- Removed dependency on the `floppa-vpn-manage` wrapper.
- Kept the fork CLI-only: no Tauri desktop client, web UI, server, daemon, migrations, or mobile platform glue.
- Privileged commands are documented through absolute-path usage because `sudo secure_path` may not include `~/.local/bin`.

### Fixed

- Removed user-specific absolute paths from code/tests.
- Fixed clippy `collapsible_if` warnings in CLI cleanup and network verification paths.
- Idempotent Linux route handling with `ip route replace`.
- Cleanup guard for DNS and routes on Ctrl+C/SIGTERM/error paths.
- Basic route/interface verification after tunnel setup.
- Windows release build by gating Unix-only signal handling behind `#[cfg(unix)]`.
- macOS release runner by using the current GitHub-hosted macOS runner.
- README logo rendering by using a solid PNG instead of an alpha-only viewer-dependent PNG.
