#!/usr/bin/env bash
# Systemd integration tests for floppa-cli service lifecycle.
# Requires: sudo access, tun kernel module, ubuntu-style systemd as PID 1.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

BINARY="$(pwd)/target/release/floppa-cli"
ITG_IFACE="floppa-itg0"
ITG_SVC="floppa-itg"
ITG_USER="floppa-itg"
ITG_HOME="/tmp/floppa-itg-home"
ITG_LOG="/tmp/floppa-itg.log"
ITG_CONF_DIR="/tmp/floppa-itg-conf"
ITG_CONF="$ITG_CONF_DIR/peer.conf"

# ─── Helpers ────────────────────────────────────────────────────────────────

die() { echo "FAIL: $*" >&2; exit 1; }

# Poll until <cmd> succeeds; fail after <timeout> seconds.
wait_for() {
  local desc="$1" timeout="$2"
  shift 2
  local end=$((SECONDS + timeout))
  until "$@" &>/dev/null; do
    if (( SECONDS >= end )); then
      sudo journalctl -u "$ITG_SVC" -n 30 --no-pager 2>/dev/null || true
      die "$desc (timeout ${timeout}s)"
    fi
    sleep 0.5
  done
}

# Poll until <cmd> fails; fail (timeout) if it never does.
wait_until_gone() {
  local desc="$1" timeout="$2"
  shift 2
  local end=$((SECONDS + timeout))
  while "$@" &>/dev/null; do
    if (( SECONDS >= end )); then
      die "$desc still present (timeout ${timeout}s)"
    fi
    sleep 0.5
  done
}

substate() {
  systemctl show "$ITG_SVC" --property=SubState --value 2>/dev/null || echo unknown
}

# ─── Cleanup ────────────────────────────────────────────────────────────────

cleanup() {
  echo "-- cleanup --"
  sudo systemctl stop   "$ITG_SVC" 2>/dev/null || true
  sudo systemctl reset-failed "$ITG_SVC" 2>/dev/null || true
  sudo "$BINARY" service --scope system --name "$ITG_SVC" uninstall 2>/dev/null || true
  sudo systemctl daemon-reload 2>/dev/null || true
  sudo ip link del "$ITG_IFACE" 2>/dev/null || true
  sudo userdel "$ITG_USER"  2>/dev/null || true
  sudo rm -rf "$ITG_CONF_DIR" "$ITG_HOME" "$ITG_LOG"
}
trap cleanup EXIT

# ─── Preflight ──────────────────────────────────────────────────────────────

echo "=== Setup ==="
[ -f "$BINARY" ] || die "Binary not found at $BINARY — run 'cargo build --release -p floppa-cli' first"

sudo modprobe tun 2>/dev/null || true

sudo useradd -r -M -s /sbin/nologin -d "$ITG_HOME" "$ITG_USER" 2>/dev/null || true
sudo mkdir -p "$ITG_HOME" "$ITG_CONF_DIR"

# Generate a minimal WireGuard config with random but correctly-sized keys.
# Endpoint 192.0.2.1 is RFC 5737 TEST-NET: non-routable IP, no DNS lookup.
PRIV_KEY="$(openssl rand -base64 32)"
PUB_KEY="$(openssl rand -base64 32)"
sudo tee "$ITG_CONF" > /dev/null <<EOF
[Interface]
PrivateKey = $PRIV_KEY
Address = 10.99.0.2/32

[Peer]
PublicKey = $PUB_KEY
Endpoint = 192.0.2.1:51820
AllowedIPs = 0.0.0.0/0
PersistentKeepalive = 25
EOF

install_svc() {
  local cfg="$1"
  sudo "$BINARY" service --scope system --name "$ITG_SVC" install \
    --binary   "$BINARY"    \
    --protocol wireguard    \
    --interface "$ITG_IFACE" \
    --user     "$ITG_USER"  \
    --home     "$ITG_HOME"  \
    --log-file "$ITG_LOG"   \
    --config   "$cfg"
  sudo systemctl daemon-reload
}

install_svc "$ITG_CONF"
echo "Setup complete."

# ─── Test A: VPN routes are added when service starts ───────────────────────

echo ""
echo "=== Test A: routes on start ==="

sudo systemctl start "$ITG_SVC"

# Wait for the TUN interface to appear (gotatun creates it during tunnel setup)
wait_for "$ITG_IFACE interface" 15 ip link show "$ITG_IFACE"

ip route show | grep -q "0.0.0.0/1 dev $ITG_IFACE"   || die "0.0.0.0/1 route missing"
ip route show | grep -q "128.0.0.0/1 dev $ITG_IFACE"  || die "128.0.0.0/1 route missing"

echo "PASS: VPN split routes exist on $ITG_IFACE"

# ─── Test B: interface cleaned up after SIGKILL + stop ──────────────────────

echo ""
echo "=== Test B: cleanup after SIGKILL ==="

# SIGKILL bypasses the signal handler; ExecStopPost is the safety net.
# systemctl stop cancels the pending auto-restart and waits for the unit
# to reach inactive — so by the time it returns, ExecStopPost has run.
sudo systemctl kill --signal=SIGKILL "$ITG_SVC" 2>/dev/null || true
sudo systemctl stop "$ITG_SVC" 2>/dev/null || true

wait_until_gone "$ITG_IFACE interface" 10 ip link show "$ITG_IFACE"

if ip route show 2>/dev/null | grep -q "dev $ITG_IFACE"; then
  die "Routes for $ITG_IFACE still present after stop"
fi

echo "PASS: interface and routes removed after SIGKILL + stop"

# ─── Test C: StartLimitBurst caps the crash-loop ────────────────────────────

echo ""
echo "=== Test C: StartLimitBurst caps crash-loop ==="

sudo systemctl reset-failed "$ITG_SVC" 2>/dev/null || true

# Reinstall with a non-existent config so the service exits immediately
# on every start, triggering repeated fast failures.
install_svc /nonexistent/floppa-itg.conf

sudo systemctl start "$ITG_SVC" 2>/dev/null || true

# With StartLimitBurst=10 and default RestartSec=100ms, the limit fires in ~1s.
# Allow 90s for slow CI environments.
echo -n "Waiting for start-limit-hit..."
end=$((SECONDS + 90))
until [ "$(substate)" = "start-limit-hit" ]; do
  if (( SECONDS >= end )); then
    echo ""
    echo "Last SubState: $(substate)"
    sudo journalctl -u "$ITG_SVC" -n 30 --no-pager 2>/dev/null || true
    die "start-limit-hit not reached (timeout 90s)"
  fi
  echo -n "."
  sleep 1
done
echo ""

echo "PASS: StartLimitBurst prevented infinite crash-loop"

# ─── Done ───────────────────────────────────────────────────────────────────

echo ""
echo "All integration tests passed."
