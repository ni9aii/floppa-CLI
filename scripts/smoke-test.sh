#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

if [[ -f "$HOME/.cargo/env" ]]; then
  # shellcheck disable=SC1090
  . "$HOME/.cargo/env"
fi

echo "== fmt =="
cargo fmt --check

echo "== test =="
cargo test -p floppa-cli --locked

echo "== clippy =="
cargo clippy -p floppa-cli --locked -- -D warnings

echo "== check =="
cargo check -p floppa-cli --locked

echo "== release build =="
cargo build --release --locked -p floppa-cli

echo "== help =="
./target/release/floppa-cli --help >/dev/null

echo "== version =="
./target/release/floppa-cli --version >/dev/null

echo "== status without tunnel =="
status_out="$(mktemp)"
status_err="$(mktemp)"
if ./target/release/floppa-cli status >"$status_out" 2>"$status_err"; then
  echo "status: connected"
else
  if grep -q "not connected" "$status_err"; then
    echo "status: not connected (expected)"
  else
    cat "$status_err" >&2
    rm -f "$status_out" "$status_err"
    exit 1
  fi
fi
rm -f "$status_out" "$status_err"

if [[ "${RUN_CARGO_INSTALL:-0}" == "1" ]]; then
  echo "== cargo install smoke =="
  cargo install --path floppa-cli --locked --force
  floppa-cli --help >/dev/null
  floppa-cli --version >/dev/null
fi

echo "Smoke test passed"
