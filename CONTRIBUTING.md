# Contributing

Thanks for looking at floppa-cli! This document covers the bits that are
specific to this crate. For the wider project see the parent `floppa-vpn`
workspace.

## Repository layout

```
floppa-CLI/                 workspace root (this repo)
├── floppa-cli/             the CLI binary crate (where the logic lives)
│   ├── src/
│   │   ├── main.rs         CLI, config resolution, connect_* entry points
│   │   ├── tunnel.rs       WireGuard / AmneziaWG setup (gotatun)
│   │   ├── vless.rs        VLESS+REALITY setup (shoes-lite)
│   │   ├── reconnect.rs    auto-reconnect loop + DBus sleep/resume watcher
│   │   ├── net.rs          policy routing / interface plumbing
│   │   ├── dns.rs          resolv.conf management
│   │   ├── service.rs      systemd unit rendering
│   │   └── ...
│   └── Cargo.toml
├── docs/                   design + ops docs (incl. RECONNECT.md)
├── systemd/                example unit files
└── justfile                common dev tasks
```

## Getting started

```bash
# Build (needs network plumbing deps; runs as root to actually connect)
cargo build -p floppa-cli

# Lint + test
cargo clippy -p floppa-cli -- -D warnings
cargo test -p floppa-cli
```

# Run against a local server (supply a .conf or vless:// file)
sudo cargo run -p floppa-cli -- connect --config <config> --no-dns
```

`cargo` is the build tool — the commands below mirror what CI runs.

## Before you open a PR

- `cargo clippy -- -D warnings` is clean (CI enforces this).
- `cargo test` passes.
- `Cargo.lock` is committed — keep builds reproducible. If you changed
  dependencies, commit the refreshed lockfile.
- New behaviour in `reconnect.rs` should come with a unit test where it makes
  sense (backoff math, signal plumbing, retryability).
- Keep credentials/secrets out of the tree. Configs are passed at runtime,
  never committed.

## Commit / PR style

- Small, focused commits. One logical change per commit.
- Write the *why*, not the *what* — the diff already shows the what.
- PRs target `main`, unless you know what you're doing.

## Code notes

- The reconnect loop owns the tunnel lifecycle. `connect_wireguard` /
  `connect_vless` build `rebuild` / `health` closures and hand them to
  `reconnect::run`. Don't block the loop on a `RefCell` borrow across an
  `.await` (clippy `await_holding_refcell_ref`) — take the value out of the
  shared cell first.
- `gotatun::Device` and `shoes_lite::VlessTunnel` are **not** `Clone` and are
  torn down via `stop(self)`. Rebuild creates a fresh one each time.
