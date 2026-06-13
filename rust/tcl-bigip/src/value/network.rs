//! `Network` value type — IPv4 or IPv6 CIDR. Rust port of `_network.py`.
//!
//! Hand-rolled against `std::net` to match Python `ipaddress.ip_network`
//! semantics exactly (host-bit masking, dotted-quad netmasks, the
//! `default` keyword) without pulling in a CIDR crate.

use super::address::IPAddress;
use super::error::ValueError;
use super::ip_class;
use super::port::py_repr;
use std::fmt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// The masked network address plus its prefix length, for either family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cidr {
    /// An IPv4 network: masked network address + prefix length (0..=32).
    V4 {
        /// Masked network address.
        network: Ipv4Addr,
        /// Prefix length in bits.
        prefix: u8,
    },
    /// An IPv6 network: masked network address + prefix length (0..=128).
    V6 {
        /// Masked network address.
        network: Ipv6Addr,
        /// Prefix length in bits.
        prefix: u8,
    },
}

impl Cidr {
    fn prefix_len(self) -> u8 {
        match self {
            Cidr::V4 { prefix, .. } | Cidr::V6 { prefix, .. } => prefix,
        }
    }

    fn with_prefixlen(self) -> String {
        match self {
            Cidr::V4 { network, prefix } => format!("{network}/{prefix}"),
            Cidr::V6 { network, prefix } => format!("{network}/{prefix}"),
        }
    }
}

/// An IPv4 or IPv6 network in CIDR notation.
///
/// The stored value lives in canonical form (host bits masked out); the
/// original textual host-bit spelling is preserved on `original` so things
/// like `net self` addresses (`203.0.113.5/24`) round-trip without losing
/// the interface address. The `default` keyword is preserved as-is.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Network {
    /// The canonical (host-bits-masked) network.
    pub network: Cidr,
    /// Original textual spelling (or the literal keyword `default`).
    pub original: String,
}

impl Network {
    /// Parse `text` as a CIDR network with `strict = false`.
    ///
    /// # Errors
    /// Returns [`ValueError`] for input the network parser rejects.
    pub fn parse(text: &str) -> Result<Self, ValueError> {
        Self::parse_strict(text, false)
    }

    /// Parse `text` as a CIDR network.
    ///
    /// Accepts integer CIDR (`10.0.0.0/24`), dotted-quad netmask
    /// (`10.0.0.0/255.255.255.0` or space-separated `10.0.0.0
    /// 255.255.255.0`), and the `default` / `default-inet6` keywords.
    /// `strict = true` rejects input with host bits set.
    ///
    /// # Errors
    /// Returns [`ValueError`] for malformed input or, when `strict`, for
    /// host bits set.
    pub fn parse_strict(text: &str, strict: bool) -> Result<Self, ValueError> {
        let original = text.trim().to_owned();
        let mut work = original.clone();
        if work == "default" {
            return Ok(Self {
                network: default_v4(),
                original: "default".to_owned(),
            });
        }
        if work == "default-inet6" {
            return Ok(Self {
                network: default_v6(),
                original: "default-inet6".to_owned(),
            });
        }
        // Space-separated ``ADDR MASK`` form.
        if work.contains(' ') && !work.contains('/') {
            let (addr_part, mask_part) = work.split_once(' ').unwrap_or((&work, ""));
            work = format!("{}/{}", addr_part.trim(), mask_part.trim());
        }
        let network = parse_network(&work, strict)?;
        Ok(Self { network, original })
    }

    /// Like [`Network::parse`] but returns `None` instead of erroring.
    #[must_use]
    pub fn try_parse(text: &str) -> Option<Self> {
        Self::parse(text).ok()
    }

    /// Like [`Network::parse_strict`] but returns `None` instead of erroring.
    #[must_use]
    pub fn try_parse_strict(text: &str, strict: bool) -> Option<Self> {
        Self::parse_strict(text, strict).ok()
    }

    /// `true` when the network is IPv4.
    #[must_use]
    pub fn is_ipv4(&self) -> bool {
        matches!(self.network, Cidr::V4 { .. })
    }

    /// `true` when the network is IPv6.
    #[must_use]
    pub fn is_ipv6(&self) -> bool {
        matches!(self.network, Cidr::V6 { .. })
    }

    /// The CIDR prefix length.
    #[must_use]
    pub fn prefix_length(&self) -> u8 {
        self.network.prefix_len()
    }

    /// `true` when the original spelling was a `default` keyword.
    #[must_use]
    pub fn is_default(&self) -> bool {
        self.original == "default" || self.original == "default-inet6"
    }

    fn network_addr(&self) -> IpAddr {
        match self.network {
            Cidr::V4 { network, .. } => IpAddr::V4(network),
            Cidr::V6 { network, .. } => IpAddr::V6(network),
        }
    }

    fn broadcast_addr(&self) -> IpAddr {
        match self.network {
            Cidr::V4 { network, prefix } => {
                let base = network.to_bits();
                let last = if prefix == 0 {
                    u32::MAX
                } else {
                    base | (u32::MAX >> u32::from(prefix))
                };
                IpAddr::V4(Ipv4Addr::from_bits(last))
            }
            Cidr::V6 { network, prefix } => {
                let base = network.to_bits();
                let last = if prefix == 0 {
                    u128::MAX
                } else {
                    base | (u128::MAX >> u32::from(prefix))
                };
                IpAddr::V6(Ipv6Addr::from_bits(last))
            }
        }
    }

    /// RFC 1918 / RFC 4193 / link-local / reserved private range.
    #[must_use]
    pub fn is_private(&self) -> bool {
        match self.network {
            Cidr::V4 { network, .. } => {
                let last = match self.broadcast_addr() {
                    IpAddr::V4(a) => a,
                    IpAddr::V6(_) => unreachable!(),
                };
                ip_class::v4_net_is_private(network.to_bits(), last.to_bits())
            }
            Cidr::V6 { network, .. } => {
                let last = match self.broadcast_addr() {
                    IpAddr::V6(a) => a,
                    IpAddr::V4(_) => unreachable!(),
                };
                ip_class::v6_net_is_private(network.to_bits(), last.to_bits())
            }
        }
    }

    /// Globally routable *unicast* network on the public internet.
    #[must_use]
    pub fn is_public(&self) -> bool {
        // Network-level ``is_global`` is ``not is_private``.
        !self.is_private()
            && !self.is_multicast()
            && !self.is_link_local()
            && !self.is_loopback()
            && !self.is_unspecified()
            && !self.is_reserved()
    }

    /// `true` when both endpoints are loopback.
    #[must_use]
    pub fn is_loopback(&self) -> bool {
        ip_class::is_loopback(self.network_addr()) && ip_class::is_loopback(self.broadcast_addr())
    }

    /// `true` when both endpoints are link-local.
    #[must_use]
    pub fn is_link_local(&self) -> bool {
        ip_class::is_link_local(self.network_addr())
            && ip_class::is_link_local(self.broadcast_addr())
    }

    /// `true` when both endpoints are multicast.
    #[must_use]
    pub fn is_multicast(&self) -> bool {
        ip_class::is_multicast(self.network_addr()) && ip_class::is_multicast(self.broadcast_addr())
    }

    /// `true` when both endpoints are reserved.
    #[must_use]
    pub fn is_reserved(&self) -> bool {
        ip_class::is_reserved(self.network_addr()) && ip_class::is_reserved(self.broadcast_addr())
    }

    /// `true` when both endpoints are unspecified.
    #[must_use]
    pub fn is_unspecified(&self) -> bool {
        ip_class::is_unspecified(self.network_addr())
            && ip_class::is_unspecified(self.broadcast_addr())
    }

    /// `true` when `addr` is a member of this network.
    #[must_use]
    pub fn contains_addr(&self, addr: IpAddr) -> bool {
        match (self.network, addr) {
            (Cidr::V4 { network, prefix }, IpAddr::V4(a)) => {
                masked_eq_v4(a.to_bits(), network.to_bits(), prefix)
            }
            (Cidr::V6 { network, prefix }, IpAddr::V6(a)) => {
                masked_eq_v6(a.to_bits(), network.to_bits(), prefix)
            }
            _ => false,
        }
    }

    /// `true` when the given [`IPAddress`] is a member of this network.
    #[must_use]
    pub fn contains_ip(&self, ip: &IPAddress) -> bool {
        self.contains_addr(ip.addr)
    }

    /// `true` when `other` is a subnet of `self` (membership of both
    /// endpoints with a prefix at least as long).
    #[must_use]
    pub fn contains_network(&self, other: &Network) -> bool {
        if self.is_ipv4() != other.is_ipv4() {
            return false;
        }
        if other.prefix_length() < self.prefix_length() {
            return false;
        }
        self.contains_addr(other.network_addr()) && self.contains_addr(other.broadcast_addr())
    }
}

impl fmt::Display for Network {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.original.is_empty() {
            return f.write_str(&self.original);
        }
        f.write_str(&self.network.with_prefixlen())
    }
}

fn default_v4() -> Cidr {
    Cidr::V4 {
        network: Ipv4Addr::UNSPECIFIED,
        prefix: 0,
    }
}

fn default_v6() -> Cidr {
    Cidr::V6 {
        network: Ipv6Addr::UNSPECIFIED,
        prefix: 0,
    }
}

fn masked_eq_v4(a: u32, net: u32, prefix: u8) -> bool {
    if prefix == 0 {
        return true;
    }
    let mask = u32::MAX << (32 - u32::from(prefix));
    (a & mask) == (net & mask)
}

fn masked_eq_v6(a: u128, net: u128, prefix: u8) -> bool {
    if prefix == 0 {
        return true;
    }
    let mask = u128::MAX << (128 - u32::from(prefix));
    (a & mask) == (net & mask)
}

/// Parse `ADDR/PREFIX` or `ADDR/NETMASK`, mirroring
/// `ipaddress.ip_network`. The error message text matches `CPython`'s
/// "does not appear to be an IPv4 or IPv6 network" / "has host bits set".
fn parse_network(text: &str, strict: bool) -> Result<Cidr, ValueError> {
    let reject = || {
        ValueError(format!(
            "{} does not appear to be an IPv4 or IPv6 network",
            py_repr(text)
        ))
    };
    let (addr_str, prefix_str) = match text.split_once('/') {
        Some((a, p)) => (a, Some(p)),
        None => (text, None),
    };
    let addr: IpAddr = addr_str.parse().map_err(|_| reject())?;
    match addr {
        IpAddr::V4(a) => {
            let prefix = match prefix_str {
                None => 32u8,
                Some(p) => parse_v4_prefix(p).ok_or_else(reject)?,
            };
            let bits = a.to_bits();
            let mask = if prefix == 0 {
                0
            } else {
                u32::MAX << (32 - u32::from(prefix))
            };
            let masked = bits & mask;
            if strict && masked != bits {
                return Err(ValueError(format!("{text} has host bits set")));
            }
            Ok(Cidr::V4 {
                network: Ipv4Addr::from_bits(masked),
                prefix,
            })
        }
        IpAddr::V6(a) => {
            let prefix = match prefix_str {
                None => 128u8,
                Some(p) => p
                    .parse::<u8>()
                    .ok()
                    .filter(|&n| n <= 128)
                    .ok_or_else(reject)?,
            };
            let bits = a.to_bits();
            let mask = if prefix == 0 {
                0
            } else {
                u128::MAX << (128 - u32::from(prefix))
            };
            let masked = bits & mask;
            if strict && masked != bits {
                return Err(ValueError(format!("{text} has host bits set")));
            }
            Ok(Cidr::V6 {
                network: Ipv6Addr::from_bits(masked),
                prefix,
            })
        }
    }
}

/// Parse an IPv4 prefix specifier: either an integer prefix length or a
/// dotted-quad netmask (contiguous-mask only, matching `ipaddress`).
fn parse_v4_prefix(spec: &str) -> Option<u8> {
    if let Ok(n) = spec.parse::<u8>() {
        if n <= 32 {
            return Some(n);
        }
        return None;
    }
    // Dotted-quad netmask: must be a contiguous run of leading ones.
    let mask: Ipv4Addr = spec.parse().ok()?;
    let bits = mask.to_bits();
    let ones = bits.leading_ones();
    let expected = if ones == 0 {
        0
    } else {
        u32::MAX << (32 - ones)
    };
    if bits == expected {
        #[allow(clippy::cast_possible_truncation)]
        return Some(ones as u8);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_integer_cidr() {
        let n = Network::parse("10.0.0.0/24").unwrap();
        assert!(n.is_ipv4());
        assert_eq!(n.prefix_length(), 24);
        assert_eq!(n.to_string(), "10.0.0.0/24");
        assert!(n.is_private());

        let n = Network::parse("2001:db8::/32").unwrap();
        assert!(n.is_ipv6());
        assert_eq!(n.to_string(), "2001:db8::/32");
    }

    #[test]
    fn preserves_host_bits_in_original() {
        let n = Network::parse("203.0.113.5/24").unwrap();
        assert_eq!(n.to_string(), "203.0.113.5/24");
        assert_eq!(
            n.network,
            Cidr::V4 {
                network: "203.0.113.0".parse().unwrap(),
                prefix: 24
            }
        );
    }

    #[test]
    fn dotted_quad_and_space_forms() {
        let n = Network::parse("10.0.0.0/255.255.255.0").unwrap();
        assert_eq!(n.prefix_length(), 24);
        let n = Network::parse("10.0.0.0 255.255.255.0").unwrap();
        assert_eq!(n.prefix_length(), 24);
        assert_eq!(n.to_string(), "10.0.0.0 255.255.255.0");
    }

    #[test]
    fn default_keywords() {
        let n = Network::parse("default").unwrap();
        assert!(n.is_default());
        assert_eq!(n.to_string(), "default");
        assert_eq!(n.prefix_length(), 0);
        let n = Network::parse("default-inet6").unwrap();
        assert!(n.is_default());
        assert!(n.is_ipv6());
    }

    #[test]
    fn strict_rejects_host_bits() {
        assert!(Network::parse_strict("10.0.0.5/24", true).is_err());
        assert!(Network::parse_strict("10.0.0.0/24", true).is_ok());
    }

    #[test]
    fn rejects_noncontiguous_mask() {
        assert!(Network::try_parse("10.0.0.0/255.255.0.255").is_none());
    }

    #[test]
    fn membership() {
        let n = Network::parse("10.0.0.0/24").unwrap();
        assert!(n.contains_ip(&IPAddress::parse("10.0.0.5").unwrap()));
        assert!(!n.contains_ip(&IPAddress::parse("10.0.1.5").unwrap()));
        let sub = Network::parse("10.0.0.0/28").unwrap();
        assert!(n.contains_network(&sub));
        assert!(!sub.contains_network(&n));
    }

    /// Differential parity: `str(Network.parse(input))` from Python.
    #[test]
    fn parity_against_python() {
        let cases: &[(&str, &str)] = &[
            ("10.0.0.0/24", "10.0.0.0/24"),
            ("10.0.0.5/24", "10.0.0.5/24"),
            ("2001:db8::/32", "2001:db8::/32"),
            ("10.0.0.0/255.255.255.0", "10.0.0.0/255.255.255.0"),
            ("10.0.0.0 255.255.255.0", "10.0.0.0 255.255.255.0"),
            ("default", "default"),
            ("default-inet6", "default-inet6"),
            ("0.0.0.0/0", "0.0.0.0/0"),
            ("::/0", "::/0"),
            ("192.168.1.128/25", "192.168.1.128/25"),
            ("100.64.0.0/10", "100.64.0.0/10"),
        ];
        for (input, expected) in cases {
            assert_eq!(
                &Network::parse(input).unwrap().to_string(),
                expected,
                "input {input:?}"
            );
        }
        // 100.64.0.0/10 is neither private nor (network-level) public.
        let cgnat = Network::parse("100.64.0.0/10").unwrap();
        assert!(!cgnat.is_private());
    }
}
