//! `IPAddress` / `FQDN` value types — the host half of a destination, plus
//! the `Address` sum type.

use super::error::ValueError;
use super::ip_class;
use super::port::py_repr;
use std::fmt;
use std::net::IpAddr;

/// An IPv4 or IPv6 address with an optional route-domain id.
///
/// F5 tmsh writes route-domain-qualified addresses as `<host>%<rd>` — e.g.
/// `10.0.0.1%10` is the IP `10.0.0.1` living on route-domain `10`. When
/// that suffix is present [`route_domain`](IPAddress::route_domain) carries
/// the numeric id; otherwise it is `None`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IPAddress {
    /// The underlying IPv4 or IPv6 address.
    pub addr: IpAddr,
    /// Optional `%RD` suffix; `None` for the default-RD case.
    pub route_domain: Option<u32>,
}

impl IPAddress {
    /// Construct directly from an [`IpAddr`] and optional route domain.
    #[must_use]
    pub fn new(addr: IpAddr, route_domain: Option<u32>) -> Self {
        Self { addr, route_domain }
    }

    /// Parse `text` as an IPv4/IPv6 host with an optional `%<rd>` suffix.
    ///
    /// Wildcard spellings (`"any"` / `"*"`) are NOT accepted here.
    ///
    /// # Errors
    /// Returns [`ValueError`] for input the stdlib IP parser rejects.
    pub fn parse(text: &str) -> Result<Self, ValueError> {
        let text = text.trim();
        let mut rd: Option<u32> = None;
        let mut host = text;
        if text.contains('%') {
            // Split on the LAST '%'.
            let idx = text.rfind('%').unwrap();
            let rd_text = &text[idx + 1..];
            // ``str.isdigit()`` is true for a non-empty run of digits.
            if !rd_text.is_empty() && rd_text.bytes().all(|b| b.is_ascii_digit()) {
                rd = rd_text.parse::<u32>().ok();
                host = &text[..idx];
            }
            // Non-numeric (`fe80::1%eth0`) — leave host intact and let the
            // IP parser reject it naturally.
        }
        let addr: IpAddr = host.parse().map_err(|_| {
            ValueError(format!(
                "{} does not appear to be an IPv4 or IPv6 address",
                py_repr(host)
            ))
        })?;
        Ok(Self {
            addr,
            route_domain: rd,
        })
    }

    /// Like [`IPAddress::parse`] but returns `None` instead of erroring.
    #[must_use]
    pub fn try_parse(text: &str) -> Option<Self> {
        Self::parse(text).ok()
    }

    /// `true` when the address is IPv4.
    #[must_use]
    pub fn is_ipv4(&self) -> bool {
        self.addr.is_ipv4()
    }

    /// `true` when the address is IPv6.
    #[must_use]
    pub fn is_ipv6(&self) -> bool {
        self.addr.is_ipv6()
    }

    /// `true` for loopback addresses.
    #[must_use]
    pub fn is_loopback(&self) -> bool {
        ip_class::is_loopback(self.addr)
    }

    /// `true` for private (not-globally-reachable) addresses.
    #[must_use]
    pub fn is_private(&self) -> bool {
        ip_class::is_private(self.addr)
    }

    /// `0.0.0.0` / `::` — F5 uses these as listen-on-any wildcards.
    #[must_use]
    pub fn is_unspecified(&self) -> bool {
        ip_class::is_unspecified(self.addr)
    }

    /// RFC 1112 (v4) / RFC 4291 (v6) multicast range.
    #[must_use]
    pub fn is_multicast(&self) -> bool {
        ip_class::is_multicast(self.addr)
    }

    /// RFC 3927 / RFC 4291 link-local range.
    #[must_use]
    pub fn is_link_local(&self) -> bool {
        ip_class::is_link_local(self.addr)
    }

    /// IANA-reserved range.
    #[must_use]
    pub fn is_reserved(&self) -> bool {
        ip_class::is_reserved(self.addr)
    }

    /// Globally routable as a *unicast* address on the public internet.
    ///
    /// Stricter than the stdlib `is_global`: also rejects multicast,
    /// link-local, loopback, unspecified, and reserved ranges.
    #[must_use]
    pub fn is_public(&self) -> bool {
        ip_class::is_global(self.addr)
            && !ip_class::is_multicast(self.addr)
            && !ip_class::is_link_local(self.addr)
            && !ip_class::is_loopback(self.addr)
            && !ip_class::is_unspecified(self.addr)
            && !ip_class::is_reserved(self.addr)
    }

    /// RFC 5737 / RFC 3849 documentation-example range.
    #[must_use]
    pub fn is_documentation(&self) -> bool {
        match self.addr {
            IpAddr::V4(a) => ip_class::v4_is_documentation(a),
            IpAddr::V6(a) => ip_class::v6_is_documentation(a),
        }
    }
}

impl fmt::Display for IPAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.route_domain {
            None => write!(f, "{}", self.addr),
            Some(rd) => write!(f, "{}%{rd}", self.addr),
        }
    }
}

/// A fully-qualified domain name used by FQDN-based pool members.
///
/// The parser is conservative: it accepts dotted labels of letters /
/// digits / hyphens, refuses leading or trailing hyphens / dots, and
/// refuses anything that looks like an IPv4 address.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FQDN {
    /// The validated domain name text.
    pub name: String,
}

impl FQDN {
    /// Parse `text` as an FQDN.
    ///
    /// # Errors
    /// Returns [`ValueError`] for empty, dotless, malformed-label, or
    /// IPv4-shaped input.
    pub fn parse(text: &str) -> Result<Self, ValueError> {
        let text = text.trim();
        if text.is_empty() {
            return Err(ValueError("FQDN: empty input".to_owned()));
        }
        if text.starts_with('.') || text.ends_with('.') {
            return Err(ValueError(format!(
                "FQDN: leading/trailing dot in {}",
                py_repr(text)
            )));
        }
        let labels: Vec<&str> = text.split('.').collect();
        if labels.len() < 2 {
            return Err(ValueError(format!(
                "FQDN: must have at least one dot ({})",
                py_repr(text)
            )));
        }
        for label in &labels {
            if label.is_empty() {
                return Err(ValueError(format!(
                    "FQDN: empty label in {}",
                    py_repr(text)
                )));
            }
            if label.len() > 63 {
                return Err(ValueError(format!(
                    "FQDN: label longer than 63 chars in {}",
                    py_repr(text)
                )));
            }
            if label.starts_with('-') || label.ends_with('-') {
                return Err(ValueError(format!(
                    "FQDN: label leading/trailing hyphen in {}",
                    py_repr(text)
                )));
            }
            if !label.chars().all(|c| is_label_alnum(c) || c == '-') {
                return Err(ValueError(format!(
                    "FQDN: invalid char in label of {}",
                    py_repr(text)
                )));
            }
        }
        // Refuse a value that's actually an IPv4 address (all-numeric
        // labels with three dots, i.e. four labels).
        if labels.len() == 4 && labels.iter().all(|l| is_ascii_digits(l)) {
            return Err(ValueError(format!(
                "FQDN: looks like an IPv4 address ({})",
                py_repr(text)
            )));
        }
        Ok(Self {
            name: text.to_owned(),
        })
    }

    /// Like [`FQDN::parse`] but returns `None` instead of erroring.
    #[must_use]
    pub fn try_parse(text: &str) -> Option<Self> {
        Self::parse(text).ok()
    }
}

impl fmt::Display for FQDN {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

/// F5 FQDN labels are ASCII in practice, but this permits the full
/// permissive Unicode-alphanumeric set so label classification stays
/// lenient.
fn is_label_alnum(c: char) -> bool {
    c.is_alphanumeric() || c.is_numeric()
}

/// `true` for a non-empty ASCII digit run, used by the IPv4-shape guard
/// (all labels being ASCII digit runs).
fn is_ascii_digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

/// The host part of a pool member or other address-bearing field: either
/// an [`IPAddress`] or an [`FQDN`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Address {
    /// An IP address host.
    Ip(IPAddress),
    /// A fully-qualified domain name host.
    Fqdn(FQDN),
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Address::Ip(ip) => write!(f, "{ip}"),
            Address::Fqdn(fqdn) => write!(f, "{fqdn}"),
        }
    }
}

/// Parse `text` as either an [`IPAddress`] or [`FQDN`].
///
/// IP addresses are tried first; anything that doesn't parse as an IP
/// falls through to FQDN.
///
/// # Errors
/// Returns [`ValueError`] if the input is neither an IP nor a valid FQDN.
pub fn parse_address(text: &str) -> Result<Address, ValueError> {
    let text = text.trim();
    if let Some(ip) = IPAddress::try_parse(text) {
        return Ok(Address::Ip(ip));
    }
    FQDN::parse(text).map(Address::Fqdn)
}

/// Like [`parse_address`] but returns `None` on failure.
#[must_use]
pub fn try_parse_address(text: &str) -> Option<Address> {
    parse_address(text).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ipv4_and_ipv6() {
        let a = IPAddress::parse("10.0.0.1").unwrap();
        assert!(a.is_ipv4());
        assert!(a.is_private());
        assert_eq!(a.to_string(), "10.0.0.1");

        let a = IPAddress::parse("2001:db8::1").unwrap();
        assert!(a.is_ipv6());
        assert_eq!(a.to_string(), "2001:db8::1");
        assert!(a.is_documentation());
    }

    #[test]
    fn route_domain_round_trips() {
        let a = IPAddress::parse("10.0.0.1%10").unwrap();
        assert_eq!(a.route_domain, Some(10));
        assert_eq!(a.to_string(), "10.0.0.1%10");
    }

    #[test]
    fn ipv6_zone_id_rejected() {
        // ``fe80::1%eth0`` — non-numeric suffix is left intact and the IP
        // parser rejects it.
        assert!(IPAddress::try_parse("fe80::1%eth0").is_none());
    }

    #[test]
    fn public_predicate() {
        assert!(IPAddress::parse("8.8.8.8").unwrap().is_public());
        assert!(!IPAddress::parse("224.0.0.1").unwrap().is_public());
        assert!(!IPAddress::parse("10.0.0.1").unwrap().is_public());
    }

    #[test]
    fn fqdn_parse() {
        let f = FQDN::parse("host.example.com").unwrap();
        assert_eq!(f.to_string(), "host.example.com");
        assert!(FQDN::try_parse("nodot").is_none());
        assert!(FQDN::try_parse(".leading").is_none());
        assert!(FQDN::try_parse("1.2.3.4").is_none());
        assert!(FQDN::try_parse("-bad.example.com").is_none());
    }

    #[test]
    fn address_sum_type() {
        match parse_address("10.0.0.1").unwrap() {
            Address::Ip(_) => {}
            Address::Fqdn(_) => panic!("expected IP"),
        }
        match parse_address("host.example.com").unwrap() {
            Address::Fqdn(f) => assert_eq!(f.name, "host.example.com"),
            Address::Ip(_) => panic!("expected FQDN"),
        }
        assert!(try_parse_address("!!!").is_none());
    }

    /// Differential parity fixtures: `str(parse_address(input))`.
    #[test]
    fn parity_against_python() {
        let cases: &[(&str, &str)] = &[
            ("10.0.0.1%10", "10.0.0.1%10"),
            ("2001:db8::1", "2001:db8::1"),
            ("::ffff:8.8.8.8", "::ffff:8.8.8.8"),
            ("fe80::1", "fe80::1"),
            ("host.example.com", "host.example.com"),
            ("sub.domain.co.uk", "sub.domain.co.uk"),
            ("10.0.0.1", "10.0.0.1"),
            ("255.255.255.255", "255.255.255.255"),
        ];
        for (input, expected) in cases {
            assert_eq!(
                &parse_address(input).unwrap().to_string(),
                expected,
                "input {input:?}"
            );
        }
        // IPv4-mapped IPv6 takes its global/private classification from the
        // embedded v4 address (per the IPv4-mapped rule).
        assert!(IPAddress::parse("::ffff:8.8.8.8").unwrap().is_public());
        assert!(IPAddress::parse("::ffff:10.0.0.1").unwrap().is_private());
    }
}
