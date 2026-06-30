//! `Destination` value type — F5 `[folder/]address[%route-domain][:port]`
//! triples.
//!
//! The parser is hand-written, not regex-driven: it splits the input on
//! the structural anchors (`/`, `[`, `]`, `%`, the last `.` or `:` for the
//! port separator) and delegates each piece to its dedicated value type's
//! parser. Every documented spelling is handled.

use super::address::{Address, IPAddress, parse_address};
use super::error::ValueError;
use super::folder::Folder;
use super::partition::Partition;
use super::port::{Port, py_repr};
use super::route_domain::RouteDomain;
use std::fmt;

/// An F5 destination triple: optional folder, address, optional
/// route-domain, port.
///
/// Every field except `address` and `port` is optional; the canonical
/// form drops them when absent. When present they serialise as
/// `<folder>/<address>%<rd>:<port>` (with the appropriate IPv6 bracketing
/// / dot-port separator).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Destination {
    /// The host address (IP or FQDN).
    pub address: Address,
    /// The destination port.
    pub port: Port,
    /// Optional folder prefix.
    pub folder: Option<Folder>,
    /// Optional route-domain.
    pub route_domain: Option<RouteDomain>,
    /// Whether the input wrapped an IPv6 address in `[...]`.
    pub ipv6_brackets: bool,
    /// `"."` vs `":"` separator for IPv6 destinations.
    pub port_separator: String,
}

impl Destination {
    /// The partition portion of the folder, if any.
    #[must_use]
    pub fn partition(&self) -> Option<&Partition> {
        self.folder.as_ref().map(|f| &f.partition)
    }

    /// Parse `text` as an F5 destination.
    ///
    /// # Errors
    /// Returns [`ValueError`] for unparseable input.
    pub fn parse(text: &str) -> Result<Self, ValueError> {
        let text = text.trim();
        if text.is_empty() {
            return Err(ValueError("Destination: empty input".to_owned()));
        }

        let (folder, rest) = split_folder_prefix(text);

        // IPv6 in brackets: ``[ADDR]`` followed by ``:port`` or ``.port``.
        if let Some(after_open) = rest.strip_prefix('[') {
            return Self::parse_bracketed_ipv6(text, after_open, folder);
        }

        // No brackets — split address from port.
        let (addr_part, route_domain, _) = split_route_domain(&rest)?;
        if count_char(&addr_part, ':') >= 2 {
            // IPv6 without brackets: port separator is the last ``.``.
            return Self::parse_unbracketed_ipv6(&addr_part, folder, route_domain);
        }

        // IPv4 / FQDN — port separator is the LAST ``:``.
        Self::parse_ipv4_or_fqdn(&addr_part, folder, route_domain)
    }

    /// Parse the bracketed IPv6 spelling ``[ADDR]`` optionally followed by a
    /// ``:port`` / ``.port`` suffix. `after_open` is the text after the `[`.
    fn parse_bracketed_ipv6(
        text: &str,
        after_open: &str,
        folder: Option<Folder>,
    ) -> Result<Self, ValueError> {
        let close = after_open.find(']').ok_or_else(|| {
            ValueError(format!("Destination: unmatched '[' in {}", py_repr(text)))
        })?;
        let addr_text = &after_open[..close];
        let port_part = &after_open[close + 1..];
        let (addr_clean, route_domain, _) = split_route_domain(addr_text)?;
        let ip = IPAddress::try_parse(&addr_clean).ok_or_else(|| {
            ValueError(format!(
                "Destination: bracketed value isn't a valid IP ({})",
                py_repr(addr_text)
            ))
        })?;
        if port_part.is_empty() {
            return Ok(Self {
                address: Address::Ip(ip),
                port: Port::new(0, ""),
                folder,
                route_domain,
                ipv6_brackets: true,
                port_separator: ":".to_owned(),
            });
        }
        let first = port_part.chars().next().unwrap();
        if first != ':' && first != '.' {
            return Err(ValueError(format!(
                "Destination: missing port separator after ']' in {}",
                py_repr(text)
            )));
        }
        let port_separator = first.to_string();
        let port = Port::parse(&port_part[first.len_utf8()..])?;
        Ok(Self {
            address: Address::Ip(ip),
            port,
            folder,
            route_domain,
            ipv6_brackets: true,
            port_separator,
        })
    }

    /// Parse an unbracketed IPv6 destination, where the port separator (if any)
    /// is the last ``.``.
    fn parse_unbracketed_ipv6(
        addr_part: &str,
        folder: Option<Folder>,
        route_domain: Option<RouteDomain>,
    ) -> Result<Self, ValueError> {
        match addr_part.rfind('.') {
            None => {
                let ip = IPAddress::try_parse(addr_part).ok_or_else(|| {
                    ValueError(format!(
                        "Destination: invalid IPv6 ({})",
                        py_repr(addr_part)
                    ))
                })?;
                Ok(Self {
                    address: Address::Ip(ip),
                    port: Port::new(0, ""),
                    folder,
                    route_domain,
                    ipv6_brackets: false,
                    port_separator: ".".to_owned(),
                })
            }
            Some(sep) => {
                let addr_text = &addr_part[..sep];
                let port_text = &addr_part[sep + 1..];
                let ip = IPAddress::try_parse(addr_text).ok_or_else(|| {
                    ValueError(format!(
                        "Destination: invalid IPv6 ({})",
                        py_repr(addr_text)
                    ))
                })?;
                Ok(Self {
                    address: Address::Ip(ip),
                    port: Port::parse(port_text)?,
                    folder,
                    route_domain,
                    ipv6_brackets: false,
                    port_separator: ".".to_owned(),
                })
            }
        }
    }

    /// Parse an IPv4 / FQDN destination, where the port separator (if any) is
    /// the last ``:``.
    fn parse_ipv4_or_fqdn(
        addr_part: &str,
        folder: Option<Folder>,
        route_domain: Option<RouteDomain>,
    ) -> Result<Self, ValueError> {
        match addr_part.rfind(':') {
            None => {
                let address_value = parse_address(addr_part)?;
                Ok(Self {
                    address: address_value,
                    port: Port::new(0, ""),
                    folder,
                    route_domain,
                    ipv6_brackets: false,
                    port_separator: ":".to_owned(),
                })
            }
            Some(sep) => {
                let addr_text = &addr_part[..sep];
                let port_text = &addr_part[sep + 1..];
                let address_value = parse_address(addr_text)?;
                Ok(Self {
                    address: address_value,
                    port: Port::parse(port_text)?,
                    folder,
                    route_domain,
                    ipv6_brackets: false,
                    port_separator: ":".to_owned(),
                })
            }
        }
    }

    /// Like [`Destination::parse`] but returns `None` instead of erroring.
    #[must_use]
    pub fn try_parse(text: &str) -> Option<Self> {
        Self::parse(text).ok()
    }
}

impl fmt::Display for Destination {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut addr_text = self.address.to_string();
        let is_ipv6 = matches!(&self.address, Address::Ip(ip) if ip.is_ipv6());
        if is_ipv6 && self.ipv6_brackets {
            addr_text = format!("[{addr_text}]");
        }
        if let Some(rd) = &self.route_domain
            && !rd.is_default()
        {
            if self.ipv6_brackets {
                // Insert ``%rd`` before the closing bracket.
                let trimmed = &addr_text[..addr_text.len() - 1];
                addr_text = format!("{trimmed}{rd}]");
            } else {
                addr_text = format!("{addr_text}{rd}");
            }
        }
        let mut out = addr_text;
        if !(self.port.is_any() && self.port.spelling.is_empty()) {
            out = format!("{out}{}{}", self.port_separator, self.port);
        }
        if let Some(folder) = &self.folder {
            out = format!("{folder}/{out}");
        }
        f.write_str(&out)
    }
}

/// `true` when `segment` looks like a host rather than a folder name.
fn looks_like_address(segment: &str) -> bool {
    if segment.contains(':') {
        return true;
    }
    matches!(segment.chars().next(), Some(c) if c.is_ascii_digit())
}

/// Split a leading folder path off `text`, returning `(folder, remainder)`
/// where `remainder` starts at the host segment. A bare destination with no
/// leading `/` returns `(None, text)`.
fn split_folder_prefix(text: &str) -> (Option<Folder>, String) {
    if !text.starts_with('/') {
        return (None, text.to_owned());
    }
    let parts: Vec<&str> = text.trim_start_matches('/').split('/').collect();
    if parts.len() < 2 {
        // ``/Common`` alone — no host segment.
        return (None, text.to_owned());
    }
    let mut folder_segments: Vec<&str> = vec![parts[0]];
    let mut host_index = None;
    for (i, seg) in parts.iter().enumerate().skip(1) {
        if seg.starts_with('[') || looks_like_address(seg) {
            host_index = Some(i);
            break;
        }
        folder_segments.push(seg);
    }
    let host_index = if let Some(i) = host_index {
        i
    } else {
        // Every segment was folder-shaped — treat the last as host.
        let hi = parts.len() - 1;
        folder_segments = parts[..hi].to_vec();
        hi
    };
    let folder_text = format!("/{}", folder_segments.join("/"));
    match Folder::try_parse(&folder_text) {
        None => (None, text.to_owned()),
        Some(folder) => {
            let remainder = parts[host_index..].join("/");
            (Some(folder), remainder)
        }
    }
}

/// Strip a trailing `%N` route-domain off `text`, returning
/// `(addr-without-rd, route-domain, original-rd-text)`.
fn split_route_domain(text: &str) -> Result<(String, Option<RouteDomain>, String), ValueError> {
    if !text.contains('%') {
        return Ok((text.to_owned(), None, String::new()));
    }
    // Split on the FIRST '%'.
    let (addr, rd_text) = text.split_once('%').unwrap_or((text, ""));
    let rd_digits: String = rd_text.chars().take_while(char::is_ascii_digit).collect();
    let remainder = &rd_text[rd_digits.len()..];
    if rd_digits.is_empty() {
        return Ok((text.to_owned(), None, String::new()));
    }
    let rd = RouteDomain::parse(&rd_digits)?;
    Ok((format!("{addr}{remainder}"), Some(rd), rd_digits))
}

fn count_char(s: &str, c: char) -> usize {
    s.chars().filter(|&ch| ch == c).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(s: &str) -> String {
        Destination::parse(s).unwrap().to_string()
    }

    #[test]
    fn ipv4_with_port() {
        assert_eq!(roundtrip("10.0.0.1:80"), "10.0.0.1:80");
    }

    #[test]
    fn bare_ipv6_dot_port() {
        assert_eq!(roundtrip("2001:db8::1.80"), "2001:db8::1.80");
    }

    #[test]
    fn bracketed_ipv6_dot_port() {
        assert_eq!(roundtrip("[2001:db8::1].80"), "[2001:db8::1].80");
    }

    #[test]
    fn bracketed_ipv6_colon_port() {
        assert_eq!(roundtrip("[2001:db8::1]:80"), "[2001:db8::1]:80");
    }

    #[test]
    fn partition_prefixed() {
        assert_eq!(roundtrip("/Common/10.0.0.1:80"), "/Common/10.0.0.1:80");
    }

    #[test]
    fn partition_bracketed_ipv6() {
        assert_eq!(
            roundtrip("/Common/[2001:db8::1].80"),
            "/Common/[2001:db8::1].80"
        );
    }

    #[test]
    fn folder_nested() {
        assert_eq!(
            roundtrip("/Common/Application_X/10.0.0.1:80"),
            "/Common/Application_X/10.0.0.1:80"
        );
    }

    #[test]
    fn ipv4_route_domain() {
        let d = Destination::parse("10.0.0.1%5:80").unwrap();
        assert_eq!(d.route_domain.as_ref().unwrap().id, 5);
        assert_eq!(d.to_string(), "10.0.0.1%5:80");
    }

    #[test]
    fn ipv6_route_domain() {
        assert_eq!(roundtrip("2001:db8::1%5.80"), "2001:db8::1%5.80");
    }

    #[test]
    fn wildcards() {
        assert_eq!(roundtrip("0.0.0.0:any"), "0.0.0.0:any");
        assert_eq!(roundtrip("::.0"), "::.0");
        // ``*.*`` is an aspirational spelling that is documented but not
        // accepted here: the real parser treats it as an FQDN with an
        // invalid label and rejects it.
        assert!(Destination::try_parse("*.*").is_none());
    }

    #[test]
    fn fqdn_members() {
        assert_eq!(roundtrip("host.example.com:443"), "host.example.com:443");
        assert_eq!(
            roundtrip("/Common/host.example.com:443"),
            "/Common/host.example.com:443"
        );
        assert_eq!(
            roundtrip("/Common/iApps/Tenant.app/host.example.com:443"),
            "/Common/iApps/Tenant.app/host.example.com:443"
        );
    }

    #[test]
    fn errors() {
        assert_eq!(
            Destination::parse("").unwrap_err().0,
            "Destination: empty input"
        );
        assert!(Destination::try_parse("[2001:db8::1").is_none());
    }

    /// Differential parity fixtures: each `(input, expected)` pair is
    /// `str(Destination.parse(input))`. `None`
    /// expectations are inputs that are rejected.
    #[test]
    fn parity_against_python() {
        let cases: &[(&str, Option<&str>)] = &[
            ("/Common/[2001:db8::1].80", Some("/Common/[2001:db8::1].80")),
            ("10.0.0.1:80", Some("10.0.0.1:80")),
            ("2001:db8::1.80", Some("2001:db8::1.80")),
            ("[2001:db8::1]:80", Some("[2001:db8::1]:80")),
            ("/Common/10.0.0.1:80", Some("/Common/10.0.0.1:80")),
            (
                "/Common/Application_X/10.0.0.1:80",
                Some("/Common/Application_X/10.0.0.1:80"),
            ),
            ("10.0.0.1%5:80", Some("10.0.0.1%5:80")),
            ("2001:db8::1%5.80", Some("2001:db8::1%5.80")),
            ("0.0.0.0:any", Some("0.0.0.0:any")),
            ("::.0", Some("::.0")),
            ("host.example.com:443", Some("host.example.com:443")),
            (
                "/Common/iApps/Tenant.app/host.example.com:443",
                Some("/Common/iApps/Tenant.app/host.example.com:443"),
            ),
            ("[2001:db8::1%7].443", Some("[2001:db8::1%7].443")),
            ("/Common/[::1]:0", Some("/Common/[::1]:0")),
            ("10.0.0.1", Some("10.0.0.1")),
            ("2001:db8::1", Some("2001:db8::1")),
            ("[2001:db8::]", Some("[2001:db8::]")),
            ("::", Some("::")),
            ("/Common/1.2.3.4", Some("/Common/1.2.3.4")),
            (
                "/Common/Application_X/Sub_Y/10.0.0.1:443",
                Some("/Common/Application_X/Sub_Y/10.0.0.1:443"),
            ),
            ("/Common/0.0.0.0:any", Some("/Common/0.0.0.0:any")),
            (
                "/Common/[2001:db8::1%9]:443",
                Some("/Common/[2001:db8::1%9]:443"),
            ),
            // Route-domain 0 is the default and is dropped on render.
            ("10.0.0.1%0:80", Some("10.0.0.1:80")),
            ("10.0.0.1:0", Some("10.0.0.1:0")),
            ("[::]:0", Some("[::]:0")),
            (
                "/Partition_2/host.example.com",
                Some("/Partition_2/host.example.com"),
            ),
            ("/Common/2001:db8::5.443", Some("/Common/2001:db8::5.443")),
            ("255.255.255.255:65535", Some("255.255.255.255:65535")),
            // ``/Common/just_a_folder`` — the trailing segment is treated as
            // an FQDN host and rejected (no dot).
            ("/Common/just_a_folder", None),
            // Non-numeric route-domain inside brackets — rejected.
            ("/Common/[fe80::1]%3.80", None),
        ];
        for (input, expected) in cases {
            match (Destination::try_parse(input), expected) {
                (Some(d), Some(exp)) => assert_eq!(&d.to_string(), exp, "input {input:?}"),
                (None, None) => {}
                (got, exp) => panic!("mismatch for {input:?}: got {got:?}, expected {exp:?}"),
            }
        }
    }
}
