use crate::paths;
use anyhow::{Result, anyhow, bail};

#[derive(Debug, Clone)]
pub struct NetworkState {
    pub interface: String,
    pub endpoint_route: Option<String>,
    pub endpoint_gateway: Option<String>,
    pub added_routes: Vec<String>,
}

pub fn run_ip(args: &[&str]) -> Result<()> {
    let output = paths::command("ip").args(args).output()?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(anyhow!("ip {} failed: {}", args.join(" "), stderr.trim()))
    }
}

pub fn run_ip_quiet(args: &[&str]) -> bool {
    paths::command("ip")
        .args(args)
        .output()
        .is_ok_and(|output| output.status.success())
}

pub fn route_exists(args: &[&str]) -> bool {
    paths::command("ip")
        .args(args)
        .output()
        .is_ok_and(|output| output.status.success() && !output.stdout.is_empty())
}

pub fn get_default_gateway() -> Result<Option<String>> {
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

pub fn cleanup_networking(state: &NetworkState) -> Result<()> {
    if let (Some(route), Some(gateway)) = (&state.endpoint_route, &state.endpoint_gateway) {
        run_ip_quiet(&["route", "del", route, "via", gateway]);
    }
    for route in &state.added_routes {
        run_ip_quiet(&["route", "del", route, "dev", &state.interface]);
    }
    run_ip_quiet(&["link", "del", &state.interface]);
    Ok(())
}

pub fn verify_networking(state: &NetworkState) -> Result<()> {
    if !route_exists(&["link", "show", &state.interface]) {
        bail!("VPN interface {} is not up", state.interface);
    }
    if let (Some(route), Some(gateway)) = (&state.endpoint_route, &state.endpoint_gateway)
        && !route_exists(&["route", "show", route])
    {
        bail!("Endpoint route {route} via {gateway} is missing");
    }
    for route in &state.added_routes {
        if !route_exists(&["route", "show", route]) {
            bail!("VPN route {route} is missing on {}", state.interface);
        }
    }
    Ok(())
}
