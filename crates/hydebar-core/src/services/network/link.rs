//! Facts about the live link, read from the kernel's own tools.
//!
//! The D-Bus backends know the connection by name and strength; the hover
//! wants what waybar shows — interface, address, gateway, netmask, frequency,
//! signal in dBm. Those facts live with the routing table and the wireless
//! stack, and reading them through `ip -j` and `iw` answers identically
//! whether `NetworkManager` or iwd manages the link.

use serde::Deserialize;
use tokio::process::Command;

use super::LinkDetails;

/// One default route as `ip -j route show default` spells it.
#[derive(Debug, Deserialize)]
struct Route {
    dev:     Option<String>,
    gateway: Option<String>
}

/// One interface as `ip -j addr show dev` spells it.
#[derive(Debug, Deserialize)]
struct Interface {
    #[serde(default)]
    addr_info: Vec<AddrInfo>
}

/// One address of an interface.
#[derive(Debug, Deserialize)]
struct AddrInfo {
    family:    String,
    local:     String,
    prefixlen: u8
}

/// Reads the link the default route rides on.
///
/// Every part degrades on its own: a machine without a default route answers
/// with nothing, a wired link simply has no wireless facts, an absent tool
/// leaves its fields empty rather than failing the rest.
pub(super) async fn read() -> LinkDetails {
    let mut details = LinkDetails::default();

    let Some(route) = default_route().await else {
        return details;
    };

    details.gateway = route.gateway;

    let Some(interface) = route.dev else {
        return details;
    };

    if let Some((address, netmask)) = first_address(&interface).await {
        details.address = Some(address);
        details.netmask = Some(netmask);
    }

    if let Some((dbm, mhz)) = wireless_link(&interface).await {
        details.signal_dbm = dbm;
        details.frequency_mhz = mhz;
    }

    details.interface = Some(interface);

    details
}

/// The first default route, if the machine has one.
async fn default_route() -> Option<Route> {
    let output = Command::new("ip")
        .args(["-j", "route", "show", "default"])
        .output()
        .await
        .ok()?;

    first_route(&output.stdout)
}

/// The first default route in an `ip -j route` report.
fn first_route(report: &[u8]) -> Option<Route> {
    serde_json::from_slice::<Vec<Route>>(report)
        .ok()?
        .into_iter()
        .next()
}

/// The first IPv4 address of `interface`, with its netmask spelled out.
async fn first_address(interface: &str) -> Option<(String, String)> {
    let output = Command::new("ip")
        .args(["-j", "addr", "show", "dev", interface])
        .output()
        .await
        .ok()?;

    first_inet(&output.stdout)
}

/// The first IPv4 address in an `ip -j addr` report, with its netmask.
///
/// IPv4 on purpose: the report usually lists link-local IPv6 first, and the
/// hover states the address the way the router hands it out.
fn first_inet(report: &[u8]) -> Option<(String, String)> {
    serde_json::from_slice::<Vec<Interface>>(report)
        .ok()?
        .into_iter()
        .flat_map(|interface| interface.addr_info)
        .find(|info| info.family == "inet")
        .map(|info| {
            (
                format!("{}/{}", info.local, info.prefixlen),
                netmask(info.prefixlen)
            )
        })
}

/// Signal and frequency of a wireless `interface`, when it is one.
///
/// Answers [`None`] for a wired interface — the kernel has no wireless
/// directory for it — so the caller never runs the wireless tool against
/// copper.
async fn wireless_link(interface: &str) -> Option<(Option<i32>, Option<u32>)> {
    if !std::path::Path::new(&format!("/sys/class/net/{interface}/wireless")).exists() {
        return None;
    }

    let output = Command::new("iw")
        .args(["dev", interface, "link"])
        .output()
        .await
        .ok()?;

    let report = String::from_utf8_lossy(&output.stdout).into_owned();

    Some(parse_wireless(&report))
}

/// Pulls the signal and frequency out of an `iw dev <if> link` report.
fn parse_wireless(report: &str) -> (Option<i32>, Option<u32>) {
    let mut dbm = None;
    let mut mhz = None;

    for line in report.lines() {
        let line = line.trim();

        if let Some(rest) = line.strip_prefix("signal:") {
            dbm = rest
                .split_whitespace()
                .next()
                .and_then(|value| value.parse::<i32>().ok());
        } else if let Some(rest) = line.strip_prefix("freq:") {
            mhz = rest
                .trim()
                .parse::<f32>()
                .ok()
                .map(|value| value.round() as u32);
        }
    }

    (dbm, mhz)
}

/// The dotted netmask of a prefix length.
fn netmask(prefix: u8) -> String {
    let bits = u32::MAX
        .checked_shl(u32::from(32 - prefix.min(32)))
        .unwrap_or(0);
    let octets = bits.to_be_bytes();

    format!("{}.{}.{}.{}", octets[0], octets[1], octets[2], octets[3])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_prefix_length_spells_its_dotted_netmask() {
        assert_eq!(netmask(24), "255.255.255.0");
        assert_eq!(netmask(16), "255.255.0.0");
        assert_eq!(netmask(32), "255.255.255.255");
        assert_eq!(netmask(0), "0.0.0.0");
        assert_eq!(netmask(22), "255.255.252.0");
    }

    #[test]
    fn a_wireless_report_yields_its_signal_and_frequency() {
        let report = "Connected to ea:43:68:5c:14:1d (on wlan0)\n\tSSID: Home\n\tfreq: \
                      5320.0\n\tsignal: -27 dBm\n";

        assert_eq!(parse_wireless(report), (Some(-27), Some(5320)));
    }

    #[test]
    fn a_report_without_the_lines_yields_nothing() {
        assert_eq!(parse_wireless("Not connected."), (None, None));
    }

    #[test]
    fn the_first_default_route_wins() {
        let report = br#"[{"dst":"default","gateway":"192.168.2.253","dev":"wlan0"},
                          {"dst":"default","gateway":"10.0.0.1","dev":"eth0"}]"#;

        let route = first_route(report).expect("route");

        assert_eq!(route.dev.as_deref(), Some("wlan0"));
        assert_eq!(route.gateway.as_deref(), Some("192.168.2.253"));
    }

    #[test]
    fn no_route_at_all_is_an_honest_none() {
        assert!(first_route(b"[]").is_none());
        assert!(first_route(b"not json").is_none());
    }

    #[test]
    fn the_inet_address_is_picked_over_the_ipv6_one() {
        let report = br#"[{"ifname":"wlan0","addr_info":[
            {"family":"inet6","local":"fe80::1","prefixlen":64},
            {"family":"inet","local":"192.168.2.19","prefixlen":24}
        ]}]"#;

        assert_eq!(
            first_inet(report),
            Some(("192.168.2.19/24".to_owned(), "255.255.255.0".to_owned()))
        );
    }

    #[test]
    fn an_interface_without_ipv4_yields_nothing() {
        let report = br#"[{"ifname":"wlan0","addr_info":[
            {"family":"inet6","local":"fe80::1","prefixlen":64}
        ]}]"#;

        assert!(first_inet(report).is_none());
    }
}
