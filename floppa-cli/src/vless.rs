use crate::paths;
use anyhow::{Result, anyhow, bail};
use ipnetwork::IpNetwork;
use shoes_lite::api::{VlessConfig, VlessTunnel};

#[derive(Debug, Clone)]
pub struct NetworkState {
    pub interface: String,
    pub endpoint_route: Option<String>,
    pub endpoint_gateway: Option<String>,
}

/// Parse a VLESS URI and create a VlessConfig with VPN defaults.
pub fn parse_uri(uri: &str) -> Result<VlessConfig> {
    let mut config = VlessConfig::from_uri(uri).map_err(|e| anyhow!("{e}"))?;

    // Set VPN defaults if not specified in URI
    if config.address.is_none() {
        config.address = Some("10.0.0.2".to_string());
    }
    if config.dns.is_none() {
        config.dns = Some("1.1.1.1".to_string());
    }
    if config.mtu.is_none() {
        config.mtu = Some(1500);
    }
    if config.allowed_ips.is_none() {
        config.allowed_ips = Some("0.0.0.0/0, ::/0".to_string());
    }

    Ok(config)
}

/// Create and start a VLESS+REALITY tunnel.
pub async fn create_tunnel(config: &VlessConfig, interface: &str) -> Result<VlessTunnel> {
    VlessTunnel::new(config, interface)
        .await
        .map_err(|e| anyhow!("{e}"))
}

fn run_ip(args: &[&str]) -> Result<()> {
    let output = paths::command("ip").args(args).output()?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(anyhow!("ip {} failed: {}", args.join(" "), stderr.trim()))
    }
}

fn get_default_gateway() -> Result<Option<String>> {
    let output = paths::command("ip")
        .args(["route", "show", "default"])
        .output()?;
    let route_output = String::from_utf8_lossy(&output.stdout);
    Ok(route_output
        .split_whitespace()
        .skip_while(|&w| w != "via")
        .nth(1)
        .map(|s| s.to_string()))
}

/// Configure routes for the VLESS tunnel (endpoint bypass + allowed IPs).
pub async fn configure_networking(config: &VlessConfig, interface: &str) -> Result<NetworkState> {
    // Add host route for VLESS endpoint via default gateway to prevent routing loop
    let endpoint_host = config
        .server_addr
        .split(':')
        .next()
        .unwrap_or(&config.server_addr);
    let endpoint_ip: std::net::IpAddr = match endpoint_host.parse() {
        Ok(ip) => ip,
        Err(_) => {
            // Resolve hostname
            tokio::net::lookup_host(&config.server_addr)
                .await?
                .next()
                .ok_or_else(|| anyhow!("Cannot resolve {}", config.server_addr))?
                .ip()
        }
    };

    let endpoint_route = get_default_gateway()?
        .map(|gateway| {
            let route = format!("{endpoint_ip}/32");
            run_ip(&["route", "replace", &route, "via", &gateway])?;
            eprintln!("Endpoint route: {route} via {gateway}");
            Ok::<_, anyhow::Error>((route, gateway))
        })
        .transpose()?;

    // Parse allowed IPs and add routes through TUN. Use `replace` for idempotent restarts.
    let allowed_ips_str = config.allowed_ips.as_deref().unwrap_or("0.0.0.0/0, ::/0");
    let networks: Vec<IpNetwork> = allowed_ips_str
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();

    for network in &networks {
        if network.prefix() == 0 {
            if network.is_ipv4() {
                run_ip(&["route", "replace", "0.0.0.0/1", "dev", interface])?;
                run_ip(&["route", "replace", "128.0.0.0/1", "dev", interface])?;
            } else {
                if let Err(e) = run_ip(&["route", "replace", "::/1", "dev", interface]) {
                    eprintln!("Skipping IPv6 VPN route ::/1: {e}");
                }
                if let Err(e) = run_ip(&["route", "replace", "8000::/1", "dev", interface]) {
                    eprintln!("Skipping IPv6 VPN route 8000::/1: {e}");
                }
            }
        } else {
            run_ip(&["route", "replace", &network.to_string(), "dev", interface])?;
        }
    }

    let addr = config.address.as_deref().unwrap_or("unknown");
    eprintln!("VPN IP: {addr}");
    eprintln!("Endpoint: {}", config.server_addr);

    Ok(NetworkState {
        interface: interface.to_string(),
        endpoint_route: endpoint_route.as_ref().map(|(route, _)| route.clone()),
        endpoint_gateway: endpoint_route.as_ref().map(|(_, gateway)| gateway.clone()),
    })
}

pub fn cleanup_networking(state: &NetworkState) -> Result<()> {
    if let (Some(route), Some(gateway)) = (&state.endpoint_route, &state.endpoint_gateway) {
        run_ip_quiet(&["route", "del", route, "via", gateway]);
    }

    for route in ["0.0.0.0/1", "128.0.0.0/1", "::/1", "8000::/1"] {
        run_ip_quiet(&["route", "del", route, "dev", &state.interface]);
    }
    run_ip_quiet(&["link", "del", &state.interface]);
    Ok(())
}

pub fn verify_networking(state: &NetworkState) -> Result<()> {
    if !route_exists(&["link", "show", &state.interface]) {
        bail!("VPN interface {} is not up", state.interface);
    }
    if let (Some(route), Some(gateway)) = (&state.endpoint_route, &state.endpoint_gateway) {
        if !route_exists(&["route", "show", route]) {
            bail!("Endpoint route {route} via {gateway} is missing");
        }
    }
    if !route_exists(&["route", "show", "0.0.0.0/1"])
        || !route_exists(&["route", "show", "128.0.0.0/1"])
    {
        bail!(
            "Default VPN split routes are missing on {}",
            state.interface
        );
    }
    Ok(())
}

fn run_ip_quiet(args: &[&str]) -> bool {
    paths::command("ip")
        .args(args)
        .output()
        .is_ok_and(|output| output.status.success())
}

fn route_exists(args: &[&str]) -> bool {
    paths::command("ip")
        .args(args)
        .output()
        .is_ok_and(|output| output.status.success() && !output.stdout.is_empty())
}
