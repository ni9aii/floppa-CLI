-- Add AmneziaWG as a third protocol, alongside WireGuard and VLESS.
--
-- AmneziaWG is structurally WireGuard + interface-wide obfuscation params, so it
-- reuses the peers table (own keypair, own IP, own server interface/subnet) with a
-- protocol discriminator — the same model VLESS briefly used in 0002 before it was
-- moved to a per-user UUID in 0003.

-- Protocol discriminator: 'wireguard' or 'amneziawg'.
-- Backfill: every existing peer is WireGuard. New peers default to AmneziaWG
-- (AWG is the default protocol for clients).
ALTER TABLE peers ADD COLUMN IF NOT EXISTS protocol TEXT NOT NULL DEFAULT 'wireguard';
ALTER TABLE peers ALTER COLUMN protocol SET DEFAULT 'amneziawg';

CREATE INDEX IF NOT EXISTS idx_peers_protocol ON peers(protocol);

-- Per-device uniqueness ("one active peer per device per protocol") is enforced
-- app-side in find_peer_by_device_id (device identity moved to app_installations in
-- 0005, which left no DB-level unique index — we keep that approach).
