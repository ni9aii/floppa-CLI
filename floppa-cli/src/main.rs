mod api;
mod auth;
mod dns;
mod tunnel;
mod vless;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
#[cfg(unix)]
use tokio::signal::unix::SignalKind;

const DEFAULT_API_URL: &str = "https://floppa.okhsunrog.dev/api";

#[derive(Parser)]
#[command(name = "floppa-cli", about = "CLI client for Floppa VPN")]
struct Cli {
    /// Write debug logs to a file (e.g. /tmp/floppa-cli.log)
    #[arg(long, global = true)]
    log_file: Option<String>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Log in via Telegram (opens browser)
    Login {
        #[arg(long, env = "FLOPPA_API_URL", default_value = DEFAULT_API_URL)]
        api_url: String,
    },
    /// Connect to VPN (auto-detects WireGuard/AmneziaWG .conf or VLESS URI)
    Connect {
        /// Config file (.conf) or VLESS URI file
        #[arg(long)]
        config: Option<String>,
        /// Protocol: wireguard (default), amneziawg, or vless
        #[arg(long, default_value = "wireguard")]
        protocol: String,
        /// TUN interface name
        #[arg(long, default_value = tunnel::DEFAULT_INTERFACE_NAME)]
        interface: String,
        /// Skip DNS configuration
        #[arg(long)]
        no_dns: bool,
        #[arg(long, env = "FLOPPA_API_URL", default_value = DEFAULT_API_URL)]
        api_url: String,
    },
    /// List your peers
    Peers {
        #[arg(long, env = "FLOPPA_API_URL", default_value = DEFAULT_API_URL)]
        api_url: String,
    },
    /// Manage peers: delete stale or device-specific peers
    Peer {
        #[command(subcommand)]
        command: PeerCommand,
    },
    /// Manage VLESS config
    Vless {
        #[command(subcommand)]
        command: VlessCommand,
    },
    /// Manage local CLI device identity
    Device {
        #[command(subcommand)]
        command: DeviceCommand,
    },
    /// Fetch and print config (WireGuard/AmneziaWG .conf or VLESS URI)
    Config {
        /// Protocol: wireguard (default), amneziawg, or vless
        #[arg(long, default_value = "wireguard")]
        protocol: String,
        /// Peer ID (WireGuard/AmneziaWG only; uses first active peer of that protocol if omitted)
        #[arg(long)]
        peer_id: Option<i64>,
        #[arg(long, env = "FLOPPA_API_URL", default_value = DEFAULT_API_URL)]
        api_url: String,
    },
    /// Show local tunnel status without contacting the API
    Status {
        /// TUN interface name
        #[arg(long, default_value = tunnel::DEFAULT_INTERFACE_NAME)]
        interface: String,
    },
    /// Remove saved login token
    Logout,
}

#[derive(Subcommand)]
enum PeerCommand {
    /// Delete one peer, all peers for this device/protocol, or all peers
    Delete {
        /// Exact peer ID to delete
        #[arg(long)]
        peer_id: Option<i64>,
        /// Delete all active peers for this protocol and this CLI device
        #[arg(long)]
        protocol: Option<String>,
        /// Delete all peers for the current account. Use with care.
        #[arg(long)]
        all: bool,
        #[arg(long, env = "FLOPPA_API_URL", default_value = DEFAULT_API_URL)]
        api_url: String,
    },
}

#[derive(Subcommand)]
enum VlessCommand {
    /// Regenerate VLESS UUID and print the new URI
    Regenerate {
        #[arg(long, env = "FLOPPA_API_URL", default_value = DEFAULT_API_URL)]
        api_url: String,
    },
}

#[derive(Subcommand)]
enum DeviceCommand {
    /// Print local device_id/device_name
    Show,
    /// Generate a new local device identity
    Reset,
}

fn is_vless(config_str: &str) -> bool {
    config_str.trim().starts_with("vless://")
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    // _guard must live until main() returns to flush the file appender
    let _guard = if let Some(ref log_path) = cli.log_file {
        let path = std::path::Path::new(log_path);
        let dir = path.parent().unwrap_or(std::path::Path::new("."));
        let filename = path
            .file_name()
            .context("Invalid log file path")?
            .to_str()
            .context("Invalid log file name")?;
        let file_appender = tracing_appender::rolling::never(dir, filename);
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
        tracing_subscriber::fmt()
            .with_writer(non_blocking)
            .with_env_filter(env_filter)
            .init();
        Some(guard)
    } else {
        tracing_subscriber::fmt()
            .with_writer(std::io::stderr)
            .with_env_filter(env_filter)
            .init();
        None
    };
    tracing_log::LogTracer::init().ok();

    match cli.command {
        Command::Login { api_url } => {
            auth::login(&api_url).await?;
        }
        Command::Connect {
            config,
            protocol,
            interface,
            no_dns,
            api_url,
        } => {
            let config_str = match config {
                Some(path) => std::fs::read_to_string(&path)
                    .with_context(|| format!("Failed to read config file: {path}"))?,
                None => {
                    let token = auth::load_token()?
                        .context("Not logged in. Run `floppa-cli login` first.")?;
                    let client = api::ApiClient::new(&api_url, &token);
                    let me = client.get_me().await?;
                    if let Some(ref sub) = me.subscription {
                        eprintln!(
                            "Plan: {} (speed limit: {})",
                            sub.plan_name,
                            sub.speed_limit_mbps
                                .map(|s| format!("{s} Mbps"))
                                .unwrap_or_else(|| "unlimited".into())
                        );
                    } else {
                        bail!("No active subscription");
                    }
                    if protocol == "vless" {
                        client.get_vless_config().await?
                    } else {
                        client.find_or_create_peer(&protocol).await?
                    }
                }
            };

            if is_vless(&config_str) {
                connect_vless(&config_str, &interface, no_dns).await?;
            } else {
                connect_wireguard(&config_str, &interface, no_dns).await?;
            }
        }
        Command::Peers { api_url } => {
            let token =
                auth::load_token()?.context("Not logged in. Run `floppa-cli login` first.")?;
            let client = api::ApiClient::new(&api_url, &token);
            let peers = client.list_peers().await?;
            if peers.is_empty() {
                eprintln!("No peers found.");
            } else {
                println!(
                    "{:<6} {:<18} {:<14} {:<32} Device",
                    "ID", "IP", "Status", "Device ID"
                );
                for p in &peers {
                    println!(
                        "{:<6} {:<18} {:<14} {:<32} {}",
                        p.id,
                        p.assigned_ip,
                        p.sync_status,
                        p.device_id.as_deref().unwrap_or("-"),
                        p.device_name.as_deref().unwrap_or("-")
                    );
                }
            }
        }
        Command::Peer {
            command:
                PeerCommand::Delete {
                    peer_id,
                    protocol,
                    all,
                    api_url,
                },
        } => {
            let token =
                auth::load_token()?.context("Not logged in. Run `floppa-cli login` first.")?;
            let client = api::ApiClient::new(&api_url, &token);

            let identity = if protocol.is_some() || all {
                Some(api::get_or_create_device_identity()?)
            } else {
                None
            };
            let peers = if protocol.is_some() || all {
                Some(client.list_peers().await?)
            } else {
                None
            };

            let mut ids = Vec::new();
            if let Some(id) = peer_id {
                ids.push(id);
            } else if let Some(protocol) = protocol {
                let identity = identity.as_ref().expect("identity loaded above");
                ids.extend(
                    peers
                        .as_ref()
                        .expect("peers loaded above")
                        .iter()
                        .filter(|p| {
                            p.protocol == protocol
                                && p.device_id.as_deref() == Some(identity.device_id.as_str())
                        })
                        .map(|p| p.id),
                );
            } else if all {
                ids.extend(
                    peers
                        .as_ref()
                        .expect("peers loaded above")
                        .iter()
                        .map(|p| p.id),
                );
            } else {
                bail!("Provide --peer-id, --protocol, or --all");
            }

            if ids.is_empty() {
                eprintln!("No matching peers found.");
            }
            for id in ids {
                client.delete_peer(id).await?;
                println!("Deleted peer {id}.");
            }
        }
        Command::Vless {
            command: VlessCommand::Regenerate { api_url },
        } => {
            let token =
                auth::load_token()?.context("Not logged in. Run `floppa-cli login` first.")?;
            let client = api::ApiClient::new(&api_url, &token);
            let uri = client.regenerate_vless_config().await?;
            println!("{uri}");
        }
        Command::Device {
            command: DeviceCommand::Show,
        } => {
            let identity = api::get_or_create_device_identity()?;
            println!("{}", serde_json::to_string_pretty(&identity)?);
        }
        Command::Device {
            command: DeviceCommand::Reset,
        } => {
            let identity = api::reset_device_identity()?;
            println!("{}", serde_json::to_string_pretty(&identity)?);
        }
        Command::Config {
            protocol,
            peer_id,
            api_url,
        } => {
            let token =
                auth::load_token()?.context("Not logged in. Run `floppa-cli login` first.")?;
            let client = api::ApiClient::new(&api_url, &token);
            let config = if protocol == "vless" {
                client.get_vless_config().await?
            } else {
                match peer_id {
                    Some(id) => client.get_peer_config(id).await?,
                    None => client.find_or_create_peer(&protocol).await?,
                }
            };
            print!("{config}");
        }
        Command::Status { interface } => {
            tunnel::status(&interface)?;
        }
        Command::Logout => {
            auth::logout()?;
            eprintln!("Logged out.");
        }
    }

    Ok(())
}

struct CleanupKind {
    dns: bool,
    tunnel: CleanupTunnel,
}

enum CleanupTunnel {
    WireGuard(tunnel::NetworkState),
    Vless(vless::NetworkState),
}

impl CleanupKind {
    fn wireguard(state: tunnel::NetworkState, dns: bool) -> Self {
        Self {
            dns,
            tunnel: CleanupTunnel::WireGuard(state),
        }
    }

    fn vless(state: vless::NetworkState, dns: bool) -> Self {
        Self {
            dns,
            tunnel: CleanupTunnel::Vless(state),
        }
    }

    fn cleanup(&mut self) {
        if self.dns {
            if let Err(e) = dns::restore_dns() {
                eprintln!("DNS restore failed: {e}");
            }
        }

        match &self.tunnel {
            CleanupTunnel::WireGuard(state) => {
                if let Err(e) = tunnel::cleanup_networking(state) {
                    eprintln!("Tunnel cleanup failed: {e}");
                }
            }
            CleanupTunnel::Vless(state) => {
                if let Err(e) = vless::cleanup_networking(state) {
                    eprintln!("VLESS cleanup failed: {e}");
                }
            }
        }
    }
}

async fn connect_wireguard(config_str: &str, interface: &str, no_dns: bool) -> Result<()> {
    let wg_config = tunnel::WgConfig::from_config_str(config_str)?;
    eprintln!("Creating WireGuard tunnel on {interface}...");
    let device = tunnel::create_tunnel(&wg_config, interface).await?;
    eprintln!("Configuring networking...");
    let network_state = tunnel::configure_networking(&wg_config, interface).await?;
    tunnel::verify_networking(&network_state)?;

    let mut cleanup = CleanupKind::wireguard(network_state, !no_dns);
    if !no_dns {
        dns::set_dns(&wg_config)?;
    }

    println!("READY");
    eprintln!("Connected! Press Ctrl+C or send SIGTERM to disconnect.");
    wait_for_shutdown().await?;

    eprintln!("\nDisconnecting...");
    cleanup.cleanup();
    device.stop().await;
    eprintln!("Disconnected.");
    Ok(())
}

async fn connect_vless(config_str: &str, interface: &str, no_dns: bool) -> Result<()> {
    let config = vless::parse_uri(config_str.trim())?;

    eprintln!("Creating VLESS+REALITY tunnel on {interface}...");
    eprintln!("Server: {}", config.server_addr);
    eprintln!("SNI: {}", config.server_name);

    let tunnel = vless::create_tunnel(&config, interface).await?;

    eprintln!("Configuring networking...");
    let network_state = vless::configure_networking(&config, interface).await?;
    vless::verify_networking(&network_state)?;

    let mut cleanup = CleanupKind::vless(network_state, !no_dns);
    if !no_dns {
        // Write DNS servers from config
        if let Some(ref dns) = config.dns {
            let servers: Vec<String> = dns.split(',').map(|s| s.trim().to_string()).collect();
            if !servers.is_empty() {
                dns::write_dns(&servers)?;
            }
        }
    }

    println!("READY");
    eprintln!("Connected! Press Ctrl+C or send SIGTERM to disconnect.");
    wait_for_shutdown().await?;

    eprintln!("\nDisconnecting...");
    cleanup.cleanup();
    tunnel.stop().await.map_err(|e| anyhow::anyhow!("{e}"))?;
    eprintln!("Disconnected.");
    Ok(())
}

async fn wait_for_shutdown() -> Result<()> {
    #[cfg(unix)]
    {
        let mut terminate = tokio::signal::unix::signal(SignalKind::terminate())?;
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {},
            _ = terminate.recv() => {},
        }
    }

    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c().await?;
    }

    Ok(())
}
