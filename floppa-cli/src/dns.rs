use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use crate::tunnel::WgConfig;

const RESOLV_CONF: &str = "/etc/resolv.conf";
const RESOLV_BACKUP: &str = "/etc/resolv.conf.floppa-backup";

pub fn set_dns(config: &WgConfig, interface: &str) -> Result<()> {
    let servers = config.dns_servers();
    if servers.is_empty() {
        return Ok(());
    }
    write_dns(&servers, interface)
}

/// Write DNS servers to /etc/resolv.conf, backing up the original.
pub fn write_dns(servers: &[String], interface: &str) -> Result<()> {
    if servers.is_empty() {
        return Ok(());
    }

    // Check for systemd-resolved symlink before any operations
    let resolv_conf = Path::new(RESOLV_CONF);
    let is_symlink = std::fs::symlink_metadata(resolv_conf)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false);

    if is_symlink {
        // On systemd-resolved systems, use resolvectl instead of overwriting the stub
        if resolvectl_available() {
            return write_dns_resolvectl(servers, interface);
        }
    }

    // Backup current resolv.conf (atomic copy with metadata preservation)
    let backup_path = Path::new(RESOLV_BACKUP);
    if resolv_conf.exists() && !backup_path.exists() {
        // Atomic copy preserves metadata, but we still have the race between exists() and copy()
        // This is acceptable for single-user systems - the backup file is just a safety net
        fs::copy(resolv_conf, backup_path).context("Failed to backup /etc/resolv.conf")?;
    }

    let content: String = servers
        .iter()
        .map(|s| format!("nameserver {s}"))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";

    fs::write(RESOLV_CONF, content).context("Failed to write /etc/resolv.conf")?;
    eprintln!("DNS: {}", servers.join(", "));

    Ok(())
}

/// Check if resolvectl is available for systemd-resolved integration.
fn resolvectl_available() -> bool {
    std::process::Command::new("resolvectl")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Write DNS servers via systemd-resolved's resolvectl.
fn write_dns_resolvectl(servers: &[String], interface: &str) -> Result<()> {
    for server in servers {
        let output = std::process::Command::new("resolvectl")
            .args(["dns", interface, server])
            .output()
            .context("Failed to run resolvectl dns")?;
        if !output.status.success() {
            eprintln!(
                "resolvectl dns warning: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
    eprintln!("DNS (systemd-resolved): {}", servers.join(", "));
    Ok(())
}

pub fn restore_dns() -> Result<()> {
    let backup_path = Path::new(RESOLV_BACKUP);
    if !backup_path.exists() {
        return Ok(());
    }

    // Check if we're dealing with a systemd-resolved symlink scenario
    // (resolv.conf is currently a symlink, not our backed-up content)
    let resolv_conf = Path::new(RESOLV_CONF);
    let is_symlink = std::fs::symlink_metadata(resolv_conf)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false);

    if is_symlink {
        // Don't overwrite symlinks - let systemd-resolved manage it
        // Just clean up our backup since we never actually modified the file
        let _ = fs::remove_file(backup_path);
        eprintln!("DNS: left to systemd-resolved management");
        return Ok(());
    }

    // Standard file restore
    fs::copy(backup_path, resolv_conf).context("Failed to restore /etc/resolv.conf")?;
    let _ = fs::remove_file(backup_path);
    eprintln!("DNS restored.");
    Ok(())
}
