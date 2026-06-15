//! Stable, reversible IP-address redaction map plus secret stripping.
//!
//! Faithful Rust port of `dialects/f5/bigip/redact_map.py` (the IP-remap
//! engine: [`RedactionMap`], [`CidrAssignment`], [`Allocator`],
//! [`collect_addresses`], [`build_map`], [`map_address`] / [`unmap_address`],
//! [`apply_map`]) and the `redact_secrets` half of
//! `dialects/f5/bigip/rewrite.py` ([`redact_secrets`], [`RedactReport`]).
//!
//! The map-file text format and the address->address mapping are reproduced
//! exactly so a redact -> unredact round-trip is byte-identical to the Python
//! CLI.
//!
//! # Shuffle / `--shuffle`
//!
//! The default (direct) mode preserves host bits. The `shuffle` mode permutes
//! host bits within each source CIDR via a Fisher-Yates shuffle seeded from
//! CPython's `random.Random(seed_int)`. CPython's MT19937 PRNG and its
//! `_randbelow_with_getrandbits` / `shuffle` are reproduced here exactly (see
//! [`mt19937`]) so `--shuffle` output is byte-identical too.

// IP / network maths is inherently full of width-exact casts between
// u8/u32/u128 (octets, prefix lengths, address ints) and the MT19937 port
// mirrors CPython's 32-bit word arithmetic. The casts are deliberate and
// width-checked by construction, so the cast lints are noise here.
#![allow(
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::doc_markdown,
    clippy::wrong_self_convention
)]

use std::collections::HashMap;
use std::net::{Ipv4Addr, Ipv6Addr};

use ipnet::{Ipv4Net, Ipv6Net};
use regex::Regex;
use sha2::{Digest, Sha256};

// ── Defaults ────────────────────────────────────────────────────────

/// Default RFC1918 IPv4 target pool, walked in order.
pub const DEFAULT_TARGET_CIDRS_V4: [&str; 3] = ["10.0.0.0/8", "172.16.0.0/12", "192.168.0.0/16"];
/// Default IPv6 target pool.
pub const DEFAULT_TARGET_CIDRS_V6: [&str; 1] = ["fd00::/8"];

// ── Regexes (ported byte-for-byte from redact_map.py) ───────────────

const RGXG_V4: &str = concat!(
    r"(25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9]?[0-9])",
    r"(\.(25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9]?[0-9])){3}"
);

const RGXG_V6: &str = concat!(
    r"((:(:[0-9A-Fa-f]{1,4}){1,7}|::|[0-9A-Fa-f]{1,4}(:(:[0-9A-Fa-f]{1,4}){1,6}|::|",
    r":[0-9A-Fa-f]{1,4}(:(:[0-9A-Fa-f]{1,4}){1,5}|::|:[0-9A-Fa-f]{1,4}(:(:[0-9A-Fa-f]{1,4}){1,4}|",
    r"::|:[0-9A-Fa-f]{1,4}(:(:[0-9A-Fa-f]{1,4}){1,3}|::|:[0-9A-Fa-f]{1,4}(:(:[0-9A-Fa-f]{1,4}){1,2}|",
    r"::|:[0-9A-Fa-f]{1,4}(::[0-9A-Fa-f]{1,4}|::|:[0-9A-Fa-f]{1,4}(::|:[0-9A-Fa-f]{1,4}))))))))|",
    r"(:(:[0-9A-Fa-f]{1,4}){0,5}|[0-9A-Fa-f]{1,4}(:(:[0-9A-Fa-f]{1,4}){0,4}|",
    r":[0-9A-Fa-f]{1,4}(:(:[0-9A-Fa-f]{1,4}){0,3}|:[0-9A-Fa-f]{1,4}(:(:[0-9A-Fa-f]{1,4}){0,2}|",
    r":[0-9A-Fa-f]{1,4}(:(:[0-9A-Fa-f]{1,4})?|:[0-9A-Fa-f]{1,4}(:|:[0-9A-Fa-f]{1,4}))))))",
    r":(25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9]?[0-9])",
    r"(\.(25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9]?[0-9])){3})"
);

/// IPv4 literal regex with Python's lookaround boundary guards emulated.
///
/// The `regex` crate has no lookaround, so we capture the literal in group 1
/// and manually verify the preceding/following byte is not `[\w.]` (for v4) or
/// `[A-Za-z0-9:]` (for v6), mirroring `(?<![\w.])…(?![\w.])`.
fn ipv4_re() -> &'static Regex {
    static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| Regex::new(RGXG_V4).expect("valid v4 regex"))
}

fn ipv6_re() -> &'static Regex {
    static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| Regex::new(RGXG_V6).expect("valid v6 regex"))
}

/// `\w` in Python's default (unicode) mode for ASCII is `[A-Za-z0-9_]`; the
/// inputs here are ASCII config text so an ASCII-only check matches CPython.
fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn v4_boundary_ok(bytes: &[u8], start: usize, end: usize) -> bool {
    // (?<![\w.]) before, (?![\w.]) after
    let before_ok = start == 0 || {
        let b = bytes[start - 1];
        !(is_word_byte(b) || b == b'.')
    };
    let after_ok = end >= bytes.len() || {
        let b = bytes[end];
        !(is_word_byte(b) || b == b'.')
    };
    before_ok && after_ok
}

fn v6_boundary_ok(bytes: &[u8], start: usize, end: usize) -> bool {
    // (?<![A-Za-z0-9:]) before, (?![A-Za-z0-9:]) after
    let before_ok = start == 0 || {
        let b = bytes[start - 1];
        !(b.is_ascii_alphanumeric() || b == b':')
    };
    let after_ok = end >= bytes.len() || {
        let b = bytes[end];
        !(b.is_ascii_alphanumeric() || b == b':')
    };
    before_ok && after_ok
}

/// One match of a literal: byte offsets and the matched text.
struct LitMatch {
    start: usize,
    end: usize,
    text: String,
}

/// Find every IPv4 literal honouring the lookaround boundary guards, scanning
/// left-to-right and never overlapping (mirrors `re.finditer`).
fn find_v4(text: &str) -> Vec<LitMatch> {
    find_bounded(text, ipv4_re(), v4_boundary_ok)
}

fn find_v6(text: &str) -> Vec<LitMatch> {
    find_bounded(text, ipv6_re(), v6_boundary_ok)
}

fn find_bounded(
    text: &str,
    re: &Regex,
    boundary_ok: fn(&[u8], usize, usize) -> bool,
) -> Vec<LitMatch> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut search_from = 0usize;
    // Emulate Python's lookaround: the regex engine, when a lookbehind/ahead
    // fails, tries to match starting one position later. So on a boundary
    // failure we must retry the regex from `m.start()+1`, not `m.end()`.
    while search_from <= bytes.len() {
        let Some(m) = re.find_at(text, search_from) else {
            break;
        };
        if boundary_ok(bytes, m.start(), m.end()) {
            out.push(LitMatch {
                start: m.start(),
                end: m.end(),
                text: m.as_str().to_owned(),
            });
            search_from = m.end();
        } else {
            search_from = m.start() + 1;
        }
    }
    out
}

// ── Address helpers ─────────────────────────────────────────────────

/// An IP address, mirroring the discriminated handling in `ipaddress`.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Addr {
    V4(Ipv4Addr),
    V6(Ipv6Addr),
}

impl Addr {
    fn version(&self) -> u8 {
        match self {
            Addr::V4(_) => 4,
            Addr::V6(_) => 6,
        }
    }
    fn max_prefixlen(&self) -> u32 {
        match self {
            Addr::V4(_) => 32,
            Addr::V6(_) => 128,
        }
    }
    fn to_int(&self) -> u128 {
        match self {
            Addr::V4(a) => u32::from(*a) as u128,
            Addr::V6(a) => u128::from(*a),
        }
    }
    /// `str(addr)` per Python's `ipaddress` (RFC 5952 compression for v6;
    /// Rust std uses the same compression rules).
    fn to_string_py(&self) -> String {
        match self {
            Addr::V4(a) => a.to_string(),
            Addr::V6(a) => a.to_string(),
        }
    }
}

fn parse_v4(token: &str) -> Option<Ipv4Addr> {
    // Python's IPv4Address rejects leading zeros / non-canonical octets.
    // `Ipv4Addr::from_str` is equally strict on octet count and range, but
    // Rust *accepts* nothing extra here that Python rejects for the tokens the
    // RGXG regex emits (no leading zeros possible beyond a single "0").
    token.parse::<Ipv4Addr>().ok()
}

fn parse_v6(token: &str) -> Option<Ipv6Addr> {
    token.parse::<Ipv6Addr>().ok()
}

// ── is_private / is_skip predicates (faithful to ipaddress) ─────────

const V4_PRIVATE_NETS: [&str; 14] = [
    "0.0.0.0/8",
    "10.0.0.0/8",
    "127.0.0.0/8",
    "169.254.0.0/16",
    "172.16.0.0/12",
    "192.0.0.0/24",
    "192.0.0.170/31",
    "192.0.2.0/24",
    "192.168.0.0/16",
    "198.18.0.0/15",
    "198.51.100.0/24",
    "203.0.113.0/24",
    "240.0.0.0/4",
    "255.255.255.255/32",
];
const V4_PRIVATE_EXCEPTIONS: [&str; 2] = ["192.0.0.9/32", "192.0.0.10/32"];

const V6_PRIVATE_NETS: [&str; 11] = [
    "::1/128",
    "::/128",
    "::ffff:0.0.0.0/96",
    "64:ff9b:1::/48",
    "100::/64",
    "2001::/23",
    "2001:db8::/32",
    "2002::/16",
    "3fff::/20",
    "fc00::/7",
    "fe80::/10",
];
const V6_PRIVATE_EXCEPTIONS: [&str; 6] = [
    "2001:1::1/128",
    "2001:1::2/128",
    "2001:3::/32",
    "2001:4:112::/48",
    "2001:20::/28",
    "2001:30::/28",
];

fn v4_in(nets: &[&str], a: Ipv4Addr) -> bool {
    nets.iter().any(|n| {
        let net: Ipv4Net = n.parse().expect("const net");
        net.contains(&a)
    })
}

fn v6_in(nets: &[&str], a: Ipv6Addr) -> bool {
    nets.iter().any(|n| {
        let net: Ipv6Net = n.parse().expect("const net");
        net.contains(&a)
    })
}

/// `ipv4_mapped` per Python: an `::ffff:a.b.c.d` address yields the embedded v4.
fn ipv4_mapped(a: Ipv6Addr) -> Option<Ipv4Addr> {
    let ip = u128::from(a);
    if (ip >> 32) != 0xFFFF {
        return None;
    }
    Some(Ipv4Addr::from((ip & 0xFFFF_FFFF) as u32))
}

fn is_loopback(addr: Addr) -> bool {
    match addr {
        Addr::V4(a) => {
            let net: Ipv4Net = "127.0.0.0/8".parse().unwrap();
            net.contains(&a)
        }
        Addr::V6(a) => {
            if let Some(m) = ipv4_mapped(a) {
                return is_loopback(Addr::V4(m));
            }
            u128::from(a) == 1
        }
    }
}

fn is_link_local(addr: Addr) -> bool {
    match addr {
        Addr::V4(a) => {
            let net: Ipv4Net = "169.254.0.0/16".parse().unwrap();
            net.contains(&a)
        }
        Addr::V6(a) => {
            if let Some(m) = ipv4_mapped(a) {
                return is_link_local(Addr::V4(m));
            }
            let net: Ipv6Net = "fe80::/10".parse().unwrap();
            net.contains(&a)
        }
    }
}

fn is_multicast(addr: Addr) -> bool {
    match addr {
        Addr::V4(a) => {
            let net: Ipv4Net = "224.0.0.0/4".parse().unwrap();
            net.contains(&a)
        }
        Addr::V6(a) => {
            if let Some(m) = ipv4_mapped(a) {
                return is_multicast(Addr::V4(m));
            }
            let net: Ipv6Net = "ff00::/8".parse().unwrap();
            net.contains(&a)
        }
    }
}

fn is_unspecified(addr: Addr) -> bool {
    match addr {
        Addr::V4(a) => u32::from(a) == 0,
        Addr::V6(a) => {
            if let Some(m) = ipv4_mapped(a) {
                return is_unspecified(Addr::V4(m));
            }
            u128::from(a) == 0
        }
    }
}

fn is_private(addr: Addr) -> bool {
    match addr {
        Addr::V4(a) => v4_in(&V4_PRIVATE_NETS, a) && !v4_in(&V4_PRIVATE_EXCEPTIONS, a),
        Addr::V6(a) => {
            if let Some(m) = ipv4_mapped(a) {
                return is_private(Addr::V4(m));
            }
            v6_in(&V6_PRIVATE_NETS, a) && !v6_in(&V6_PRIVATE_EXCEPTIONS, a)
        }
    }
}

fn should_skip(addr: Addr, remap_private: bool) -> bool {
    if is_loopback(addr) || is_link_local(addr) || is_multicast(addr) || is_unspecified(addr) {
        return true;
    }
    if !remap_private && is_private(addr) {
        return true;
    }
    false
}

// ── Network helpers ─────────────────────────────────────────────────

/// `ip_network(f"{addr}/{prefix}", strict=False)` — host bits zeroed.
fn enclosing_cidr(addr: Addr, prefix: u32) -> Net {
    match addr {
        Addr::V4(a) => {
            let net = Ipv4Net::new(a, prefix as u8).unwrap().trunc();
            Net::V4(net)
        }
        Addr::V6(a) => {
            let net = Ipv6Net::new(a, prefix as u8).unwrap().trunc();
            Net::V6(net)
        }
    }
}

/// A network, mirroring `IPv4Network | IPv6Network`.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Net {
    V4(Ipv4Net),
    V6(Ipv6Net),
}

impl Net {
    fn prefixlen(&self) -> u32 {
        match self {
            Net::V4(n) => n.prefix_len() as u32,
            Net::V6(n) => n.prefix_len() as u32,
        }
    }
    fn network_int(&self) -> u128 {
        match self {
            Net::V4(n) => u32::from(n.network()) as u128,
            Net::V6(n) => u128::from(n.network()),
        }
    }
    fn num_addresses(&self) -> u128 {
        match self {
            Net::V4(n) => 1u128 << (32 - n.prefix_len() as u32),
            Net::V6(n) => 1u128 << (128 - n.prefix_len() as u32),
        }
    }
    fn contains_addr(&self, addr: Addr) -> bool {
        match (self, addr) {
            (Net::V4(n), Addr::V4(a)) => n.contains(&a),
            (Net::V6(n), Addr::V6(a)) => n.contains(&a),
            _ => false,
        }
    }
    /// `str(network)` per Python: `network_address/prefixlen`.
    fn to_string_py(&self) -> String {
        match self {
            Net::V4(n) => format!("{}/{}", n.network(), n.prefix_len()),
            Net::V6(n) => format!("{}/{}", n.network(), n.prefix_len()),
        }
    }
    fn subnet_of(&self, pool: &Net) -> bool {
        match (self, pool) {
            (Net::V4(a), Net::V4(b)) => {
                b.contains(&a.network()) && a.prefix_len() >= b.prefix_len()
            }
            (Net::V6(a), Net::V6(b)) => {
                b.contains(&a.network()) && a.prefix_len() >= b.prefix_len()
            }
            _ => false,
        }
    }
}

/// `ip_network(s, strict=False)` — accepts both `addr` and `addr/prefix`.
fn parse_net(s: &str) -> Option<Net> {
    if s.contains(':') {
        if let Some((addr, pfx)) = split_net(s, 128) {
            let a: Ipv6Addr = addr.parse().ok()?;
            let net = Ipv6Net::new(a, pfx as u8).ok()?.trunc();
            return Some(Net::V6(net));
        }
        None
    } else if let Some((addr, pfx)) = split_net(s, 32) {
        let a: Ipv4Addr = addr.parse().ok()?;
        let net = Ipv4Net::new(a, pfx as u8).ok()?.trunc();
        Some(Net::V4(net))
    } else {
        None
    }
}

fn split_net(s: &str, default_pfx: u32) -> Option<(&str, u32)> {
    match s.split_once('/') {
        Some((a, p)) => Some((a, p.parse().ok()?)),
        None => Some((s, default_pfx)),
    }
}

/// Build an address from a network base int OR'd with host bits.
fn direct_host(host_bits: u128, _source: Net, target: Net) -> Addr {
    match target {
        Net::V4(_) => Addr::V4(Ipv4Addr::from((target.network_int() | host_bits) as u32)),
        Net::V6(_) => Addr::V6(Ipv6Addr::from(target.network_int() | host_bits)),
    }
}

// ── CidrAssignment / RedactionMap ───────────────────────────────────

/// One source CIDR mapped to one target CIDR of equal prefix length.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CidrAssignment {
    /// Source CIDR (e.g. `93.184.216.0/24`).
    pub source: String,
    /// Target CIDR (e.g. `10.0.0.0/24`).
    pub target: String,
    /// Per-CIDR shuffle key (hex), or `None` for a direct mapping.
    pub shuffle_key: Option<String>,
}

/// Bidirectional record of every real<->redacted IP encountered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RedactionMap {
    /// Source->target CIDR assignments, in first-seen order.
    pub cidr_assignments: Vec<CidrAssignment>,
    /// Forward (real -> fake) IP cache.
    pub forward: HashMap<String, String>,
    /// Reverse (fake -> real) IP cache.
    pub reverse: HashMap<String, String>,
    /// IPv4 target pool.
    pub target_pool_v4: Vec<String>,
    /// IPv6 target pool.
    pub target_pool_v6: Vec<String>,
    /// `"direct"` or `"shuffle"`.
    pub mode: String,
    /// Shuffle seed (string; recorded in the map).
    pub seed: String,
}

impl Default for RedactionMap {
    fn default() -> Self {
        RedactionMap {
            cidr_assignments: Vec::new(),
            forward: HashMap::new(),
            reverse: HashMap::new(),
            target_pool_v4: DEFAULT_TARGET_CIDRS_V4
                .iter()
                .map(|s| (*s).to_owned())
                .collect(),
            target_pool_v6: DEFAULT_TARGET_CIDRS_V6
                .iter()
                .map(|s| (*s).to_owned())
                .collect(),
            mode: "direct".to_owned(),
            seed: String::new(),
        }
    }
}

impl RedactionMap {
    /// Serialise to the sidecar map-file TOML (byte-identical to Python's
    /// `RedactionMap.to_toml`).
    #[must_use]
    pub fn to_toml(&self) -> String {
        let mut lines: Vec<String> = Vec::new();
        lines.push("# f5 redact / unredact map file".to_owned());
        lines.push("# Generated automatically — do not edit by hand unless you".to_owned());
        lines.push("# accept that round-tripping may then produce different output.".to_owned());
        lines.push(String::new());
        lines.push(r#"schema = "f5-redact-map/v1""#.to_owned());
        lines.push(format!(r#"mode = "{}""#, self.mode));
        lines.push(format!(r#"seed = "{}""#, self.seed));
        lines.push(format!(
            "target_pool_v4 = [{}]",
            self.target_pool_v4
                .iter()
                .map(|c| format!("\"{c}\""))
                .collect::<Vec<_>>()
                .join(", ")
        ));
        lines.push(format!(
            "target_pool_v6 = [{}]",
            self.target_pool_v6
                .iter()
                .map(|c| format!("\"{c}\""))
                .collect::<Vec<_>>()
                .join(", ")
        ));
        lines.push(String::new());
        for a in &self.cidr_assignments {
            lines.push("[[cidr]]".to_owned());
            lines.push(format!(r#"source = "{}""#, a.source));
            lines.push(format!(r#"target = "{}""#, a.target));
            if let Some(k) = &a.shuffle_key {
                lines.push(format!(r#"shuffle_key = "{k}""#));
            }
            lines.push(String::new());
        }
        if !self.forward.is_empty() {
            lines.push("[ips]".to_owned());
            // Python: sorted(self.forward.items()) — lexicographic by key.
            let mut items: Vec<(&String, &String)> = self.forward.iter().collect();
            items.sort_by(|a, b| a.0.cmp(b.0));
            for (real, fake) in items {
                lines.push(format!(r#""{real}" = "{fake}""#));
            }
        }
        let mut out = lines.join("\n");
        // Python: "\n".join(lines).rstrip() + "\n"
        let trimmed = out.trim_end();
        out.truncate(trimmed.len());
        out.push('\n');
        out
    }

    /// Parse a sidecar map-file TOML (mirrors `RedactionMap.from_toml`).
    ///
    /// # Errors
    /// Returns an error string on an unknown schema or malformed structure.
    pub fn from_toml(text: &str) -> Result<RedactionMap, String> {
        let parsed = toml_lite::parse(text)?;
        let schema = parsed.top.get("schema").map(String::as_str);
        if schema != Some("f5-redact-map/v1") {
            return Err(format!(
                "unknown map schema: {}",
                schema.map_or_else(|| "None".to_owned(), |s| format!("'{s}'"))
            ));
        }
        let mut cidr_assignments = Vec::new();
        for item in &parsed.cidr {
            let source = item
                .get("source")
                .ok_or("missing source in [[cidr]]")?
                .clone();
            let target = item
                .get("target")
                .ok_or("missing target in [[cidr]]")?
                .clone();
            let shuffle_key = item.get("shuffle_key").cloned();
            cidr_assignments.push(CidrAssignment {
                source,
                target,
                shuffle_key,
            });
        }
        let target_pool_v4 = parsed.target_pool_v4.unwrap_or_else(|| {
            DEFAULT_TARGET_CIDRS_V4
                .iter()
                .map(|s| (*s).to_owned())
                .collect()
        });
        let target_pool_v6 = parsed.target_pool_v6.unwrap_or_else(|| {
            DEFAULT_TARGET_CIDRS_V6
                .iter()
                .map(|s| (*s).to_owned())
                .collect()
        });
        let mode = parsed
            .top
            .get("mode")
            .cloned()
            .unwrap_or_else(|| "direct".to_owned());
        let seed = parsed.top.get("seed").cloned().unwrap_or_default();

        let mut forward = HashMap::new();
        let mut reverse = HashMap::new();
        for (k, v) in &parsed.ips {
            forward.insert(k.clone(), v.clone());
            reverse.insert(v.clone(), k.clone());
        }
        Ok(RedactionMap {
            cidr_assignments,
            forward,
            reverse,
            target_pool_v4,
            target_pool_v6,
            mode,
            seed,
        })
    }
}

// ── Allocator ───────────────────────────────────────────────────────

/// Error raised when no non-overlapping target CIDR can be allocated.
#[derive(Debug)]
pub struct TargetCollisionError(pub String);

struct Allocator {
    pools_v4: Vec<Ipv4Net>,
    pools_v6: Vec<Ipv6Net>,
    used_v4: Vec<u128>,
    used_v6: Vec<u128>,
    forbidden_v4: Vec<(u128, u128)>,
    forbidden_v6: Vec<(u128, u128)>,
}

impl Allocator {
    fn new(pools_v4: &[String], pools_v6: &[String]) -> Allocator {
        let pools_v4: Vec<Ipv4Net> = pools_v4
            .iter()
            .map(|p| p.parse::<Ipv4Net>().expect("valid v4 pool").trunc())
            .collect();
        let pools_v6: Vec<Ipv6Net> = pools_v6
            .iter()
            .map(|p| p.parse::<Ipv6Net>().expect("valid v6 pool").trunc())
            .collect();
        let used_v4 = vec![0u128; pools_v4.len()];
        let used_v6 = vec![0u128; pools_v6.len()];
        Allocator {
            pools_v4,
            pools_v6,
            used_v4,
            used_v6,
            forbidden_v4: Vec::new(),
            forbidden_v6: Vec::new(),
        }
    }

    fn forbid(&mut self, network: Net) {
        let low = network.network_int();
        let high = low + network.num_addresses();
        match network {
            Net::V4(_) => {
                self.forbidden_v4.push((low, high));
                self.forbidden_v4.sort_unstable();
            }
            Net::V6(_) => {
                self.forbidden_v6.push((low, high));
                self.forbidden_v6.sort_unstable();
            }
        }
    }

    fn next_safe_offset(
        pool_base: u128,
        mut offset: u128,
        block_size: u128,
        forbidden: &[(u128, u128)],
        pool_capacity: u128,
    ) -> Option<u128> {
        while offset + block_size <= pool_capacity {
            let low = pool_base + offset;
            let high = low + block_size;
            let mut collided = false;
            for &(forb_low, forb_high) in forbidden {
                if forb_low >= high {
                    break;
                }
                if forb_high <= low {
                    continue;
                }
                let mut new_low = (forb_high - pool_base).div_ceil(block_size) * block_size;
                if new_low <= offset {
                    new_low = offset + block_size;
                }
                offset = new_low;
                collided = true;
                break;
            }
            if !collided {
                return Some(offset);
            }
        }
        None
    }

    fn allocate(&mut self, prefix: u32, version: u8) -> Result<Net, TargetCollisionError> {
        if version == 4 {
            self.allocate_v(prefix, 4)
        } else {
            self.allocate_v(prefix, 6)
        }
    }

    fn allocate_v(&mut self, prefix: u32, version: u8) -> Result<Net, TargetCollisionError> {
        let (pools_len, is_v4) = if version == 4 {
            (self.pools_v4.len(), true)
        } else {
            (self.pools_v6.len(), false)
        };
        for i in 0..pools_len {
            let (pool_prefix, max_prefix, base, capacity) = if is_v4 {
                let p = self.pools_v4[i];
                (
                    p.prefix_len() as u32,
                    32u32,
                    u32::from(p.network()) as u128,
                    1u128 << (32 - p.prefix_len() as u32),
                )
            } else {
                let p = self.pools_v6[i];
                (
                    p.prefix_len() as u32,
                    128u32,
                    u128::from(p.network()),
                    1u128 << (128 - p.prefix_len() as u32),
                )
            };
            if prefix < pool_prefix {
                continue;
            }
            let block_size = 1u128 << (max_prefix - prefix);
            let used = if is_v4 {
                self.used_v4[i]
            } else {
                self.used_v6[i]
            };
            let forbidden = if is_v4 {
                &self.forbidden_v4
            } else {
                &self.forbidden_v6
            };
            let Some(safe) = Self::next_safe_offset(base, used, block_size, forbidden, capacity)
            else {
                continue;
            };
            let target = if is_v4 {
                self.used_v4[i] = safe + block_size;
                Net::V4(
                    Ipv4Net::new(((base + safe) as u32).into(), prefix as u8)
                        .unwrap()
                        .trunc(),
                )
            } else {
                self.used_v6[i] = safe + block_size;
                Net::V6(
                    Ipv6Net::new((base + safe).into(), prefix as u8)
                        .unwrap()
                        .trunc(),
                )
            };
            return Ok(target);
        }
        if is_v4 {
            Err(TargetCollisionError(format!(
                "no non-overlapping target available for v4 /{prefix} \
                 (every pool exhausted or fully covered by forbidden ranges)"
            )))
        } else {
            Err(TargetCollisionError(format!(
                "no non-overlapping target available for v6 /{prefix}"
            )))
        }
    }

    fn consume(&mut self, target: Net) {
        match target {
            Net::V4(_) => {
                for i in 0..self.pools_v4.len() {
                    let pool = Net::V4(self.pools_v4[i]);
                    if target.subnet_of(&pool) {
                        let high = target.network_int() + target.num_addresses();
                        let used_offset = high - pool.network_int();
                        if used_offset > self.used_v4[i] {
                            self.used_v4[i] = used_offset;
                        }
                        return;
                    }
                }
            }
            Net::V6(_) => {
                for i in 0..self.pools_v6.len() {
                    let pool = Net::V6(self.pools_v6[i]);
                    if target.subnet_of(&pool) {
                        let high = target.network_int() + target.num_addresses();
                        let used_offset = high - pool.network_int();
                        if used_offset > self.used_v6[i] {
                            self.used_v6[i] = used_offset;
                        }
                        return;
                    }
                }
            }
        }
    }
}

// ── Permutation (shuffle mode) ──────────────────────────────────────

fn derive_key(seed: &str, source_cidr: &str) -> Vec<u8> {
    let mut h = Sha256::new();
    h.update(format!("{seed}::{source_cidr}").as_bytes());
    h.finalize().to_vec()
}

const SHUFFLE_MAX_WIDTH: u32 = 20;

/// Return `(forward, inverse)` permutations for `host_width` bits, matching
/// `_build_permutation` (CPython `random.Random(seed_int).shuffle`).
fn build_permutation(host_width: u32, key: &[u8]) -> (Vec<u32>, Vec<u32>) {
    let size = 1usize << host_width;
    let digest = {
        let mut h = Sha256::new();
        h.update(key);
        h.finalize()
    };
    // seed_int = int.from_bytes(digest[:16], "big")
    let mut seed_int: u128 = 0;
    for &b in &digest[..16] {
        seed_int = (seed_int << 8) | u128::from(b);
    }
    let mut rng = mt19937::Random::from_int(seed_int);
    let mut forward: Vec<u32> = (0..size as u32).collect();
    rng.shuffle(&mut forward);
    let mut inverse = vec![0u32; size];
    for (i, &j) in forward.iter().enumerate() {
        inverse[j as usize] = i as u32;
    }
    (forward, inverse)
}

fn shuffle_host(addr: Addr, source: Net, target: Net, key: &[u8], invert: bool) -> Addr {
    let host_width = addr.max_prefixlen() - source.prefixlen();
    if host_width == 0 {
        return match target {
            Net::V4(_) => Addr::V4(Ipv4Addr::from(target.network_int() as u32)),
            Net::V6(_) => Addr::V6(Ipv6Addr::from(target.network_int())),
        };
    }
    let mask = if host_width >= 128 {
        u128::MAX
    } else {
        (1u128 << host_width) - 1
    };
    if host_width > SHUFFLE_MAX_WIDTH {
        let host_bits = addr.to_int() & mask;
        return direct_host(host_bits, source, target);
    }
    let (forward, inverse) = build_permutation(host_width, key);
    let table = if invert { &inverse } else { &forward };
    let host_bits = (addr.to_int() & mask) as usize;
    direct_host(u128::from(table[host_bits]), source, target)
}

// ── Map operations ──────────────────────────────────────────────────

fn normalise_token(token: &str) -> Option<Addr> {
    if let Some(a) = parse_v4(token) {
        Some(Addr::V4(a))
    } else {
        parse_v6(token).map(Addr::V6)
    }
}

/// Find every IPv4 / IPv6 literal in `text` in first-appearance order
/// (mirrors `collect_addresses`).
fn collect_addresses(text: &str, remap_private: bool) -> (Vec<Ipv4Addr>, Vec<Ipv6Addr>) {
    let mut seen_v4: Vec<Ipv4Addr> = Vec::new();
    let mut seen_v4_set: std::collections::HashSet<Ipv4Addr> = std::collections::HashSet::new();
    let v4_matches = find_v4(text);
    for m in &v4_matches {
        if let Some(a) = parse_v4(&m.text) {
            if should_skip(Addr::V4(a), remap_private) {
                continue;
            }
            if seen_v4_set.insert(a) {
                seen_v4.push(a);
            }
        }
    }

    let mut seen_v6: Vec<Ipv6Addr> = Vec::new();
    let mut seen_v6_set: std::collections::HashSet<Ipv6Addr> = std::collections::HashSet::new();
    let v6_matches = find_v6(text);
    let v4_starts: std::collections::HashSet<usize> = v4_matches.iter().map(|m| m.start).collect();
    let mut consumed_v4_at: std::collections::HashSet<usize> = std::collections::HashSet::new();
    for m in &v6_matches {
        if let Some(a) = parse_v6(&m.text) {
            if should_skip(Addr::V6(a), remap_private) {
                continue;
            }
            if seen_v6_set.insert(a) {
                seen_v6.push(a);
            }
            for &s in &v4_starts {
                if m.start <= s && s < m.end {
                    consumed_v4_at.insert(s);
                }
            }
        }
    }

    if !consumed_v4_at.is_empty() {
        let mut v4_again: Vec<Ipv4Addr> = Vec::new();
        let mut v4_again_set: std::collections::HashSet<Ipv4Addr> =
            std::collections::HashSet::new();
        for m in &v4_matches {
            if consumed_v4_at.contains(&m.start) {
                continue;
            }
            if let Some(a) = parse_v4(&m.text) {
                if should_skip(Addr::V4(a), remap_private) {
                    continue;
                }
                if v4_again_set.insert(a) {
                    v4_again.push(a);
                }
            }
        }
        seen_v4 = v4_again;
    }

    (seen_v4, seen_v6)
}

/// Options for [`build_map`].
pub struct BuildMapOptions<'a> {
    /// Source text to scan for IP literals.
    pub text: &'a str,
    /// IPv4 target pool.
    pub target_pool_v4: Vec<String>,
    /// IPv6 target pool.
    pub target_pool_v6: Vec<String>,
    /// `"direct"` or `"shuffle"`.
    pub mode: String,
    /// Shuffle seed.
    pub seed: String,
    /// Inferred source-CIDR prefix for IPv4 (default 24).
    pub source_cidr_v4_prefix: u32,
    /// Inferred source-CIDR prefix for IPv6 (default 64).
    pub source_cidr_v6_prefix: u32,
    /// Explicit source CIDRs to honour before inference.
    pub explicit_source_cidrs: Vec<String>,
    /// An existing map to extend (its settings are inherited).
    pub existing: Option<RedactionMap>,
    /// Also remap RFC1918 / fc00::/7 private addresses.
    pub remap_private: bool,
}

/// Build (or extend) a [`RedactionMap`] covering every public IP in the text.
///
/// # Errors
/// Returns [`TargetCollisionError`] when a target CIDR cannot be allocated, or
/// an error string on an unknown mode / malformed CIDR.
#[allow(clippy::too_many_lines)]
pub fn build_map(opts: BuildMapOptions) -> Result<RedactionMap, String> {
    let BuildMapOptions {
        text,
        target_pool_v4,
        target_pool_v6,
        mut mode,
        mut seed,
        source_cidr_v4_prefix,
        source_cidr_v6_prefix,
        explicit_source_cidrs,
        existing,
        remap_private,
    } = opts;

    if mode != "direct" && mode != "shuffle" {
        return Err(format!("unknown mode '{mode}'"));
    }

    let (mut rm, pool_v4, pool_v6) = if let Some(ex) = existing {
        // Inherit every setting from the prior map (the existing map's settings
        // win over any conflicting arguments).
        mode.clone_from(&ex.mode);
        seed.clone_from(&ex.seed);
        let pool_v4 = ex.target_pool_v4.clone();
        let pool_v6 = ex.target_pool_v6.clone();
        (ex, pool_v4, pool_v6)
    } else {
        let rm = RedactionMap {
            cidr_assignments: Vec::new(),
            forward: HashMap::new(),
            reverse: HashMap::new(),
            target_pool_v4: target_pool_v4.clone(),
            target_pool_v6: target_pool_v6.clone(),
            mode: mode.clone(),
            seed: seed.clone(),
        };
        (rm, target_pool_v4, target_pool_v6)
    };

    let mut allocator = Allocator::new(&pool_v4, &pool_v6);

    // Replay prior assignments through the allocator.
    let mut cidr_for: HashMap<Net, Net> = HashMap::new();
    for a in &rm.cidr_assignments {
        let src = parse_net(&a.source).ok_or("bad source CIDR")?;
        let tgt = parse_net(&a.target).ok_or("bad target CIDR")?;
        cidr_for.insert(src, tgt);
        allocator.consume(tgt);
    }

    let explicit: Vec<Net> = explicit_source_cidrs
        .iter()
        .map(|c| parse_net(c).ok_or_else(|| format!("bad source CIDR: {c}")))
        .collect::<Result<_, _>>()?;

    let source_cidr_for = |addr: Addr| -> Net {
        for net in &explicit {
            match (net, addr) {
                (Net::V4(_), Addr::V4(_)) | (Net::V6(_), Addr::V6(_))
                    if net.contains_addr(addr) =>
                {
                    return *net;
                }
                _ => {}
            }
        }
        let prefix = if addr.version() == 4 {
            source_cidr_v4_prefix
        } else {
            source_cidr_v6_prefix
        };
        enclosing_cidr(addr, prefix)
    };

    let (addrs_v4, addrs_v6) = collect_addresses(text, remap_private);
    let all_addrs: Vec<Addr> = addrs_v4
        .iter()
        .map(|a| Addr::V4(*a))
        .chain(addrs_v6.iter().map(|a| Addr::V6(*a)))
        .collect();

    // Pre-scan: forbid every source CIDR as a target.
    for &addr in &all_addrs {
        allocator.forbid(source_cidr_for(addr));
    }
    for a in &rm.cidr_assignments {
        allocator.forbid(parse_net(&a.source).ok_or("bad source CIDR")?);
    }
    for net in &explicit {
        allocator.forbid(*net);
    }

    for &addr in &all_addrs {
        let source_net = source_cidr_for(addr);
        if let std::collections::hash_map::Entry::Vacant(slot) = cidr_for.entry(source_net) {
            let target_net = allocator
                .allocate(source_net.prefixlen(), addr.version())
                .map_err(|e| e.0)?;
            let shuffle_key = if mode == "shuffle" {
                Some(hex_encode(&derive_key(&seed, &source_net.to_string_py())))
            } else {
                None
            };
            rm.cidr_assignments.push(CidrAssignment {
                source: source_net.to_string_py(),
                target: target_net.to_string_py(),
                shuffle_key,
            });
            slot.insert(target_net);
        }

        let target_net = cidr_for[&source_net];
        let mapped = if mode == "shuffle" {
            shuffle_host(
                addr,
                source_net,
                target_net,
                &derive_key(&seed, &source_net.to_string_py()),
                false,
            )
        } else {
            let host_width = addr.max_prefixlen() - source_net.prefixlen();
            let mask = if host_width >= 128 {
                u128::MAX
            } else {
                (1u128 << host_width) - 1
            };
            let host_bits = addr.to_int() & mask;
            direct_host(host_bits, source_net, target_net)
        };

        rm.forward
            .insert(addr.to_string_py(), mapped.to_string_py());
        rm.reverse
            .insert(mapped.to_string_py(), addr.to_string_py());
    }

    Ok(rm)
}

fn hex_encode(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(s, "{b:02x}");
    }
    s
}

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

/// Return the redacted form of `real`, or `real` unchanged if no rule applies.
fn map_address(rm: &mut RedactionMap, real: &str) -> String {
    if let Some(v) = rm.forward.get(real) {
        return v.clone();
    }
    let Some(addr) = normalise_token(real) else {
        return real.to_owned();
    };
    if should_skip(addr, false) {
        // `_is_already_private` == should_skip(remap_private=False)
        return real.to_owned();
    }
    let assignments = rm.cidr_assignments.clone();
    for a in &assignments {
        let Some(source) = parse_net(&a.source) else {
            continue;
        };
        match (source, addr) {
            (Net::V4(_), Addr::V4(_)) | (Net::V6(_), Addr::V6(_)) => {}
            _ => continue,
        }
        if source.contains_addr(addr) {
            let target = parse_net(&a.target).expect("valid target");
            let mapped = if let Some(k) = &a.shuffle_key {
                if k.is_empty() {
                    direct_map(addr, source, target)
                } else {
                    shuffle_host(
                        addr,
                        source,
                        target,
                        &hex_decode(k).unwrap_or_default(),
                        false,
                    )
                }
            } else {
                direct_map(addr, source, target)
            };
            rm.forward.insert(real.to_owned(), mapped.to_string_py());
            rm.reverse.insert(mapped.to_string_py(), real.to_owned());
            return mapped.to_string_py();
        }
    }
    real.to_owned()
}

fn direct_map(addr: Addr, source: Net, target: Net) -> Addr {
    let host_width = addr.max_prefixlen() - source.prefixlen();
    let mask = if host_width >= 128 {
        u128::MAX
    } else {
        (1u128 << host_width) - 1
    };
    let host_bits = addr.to_int() & mask;
    direct_host(host_bits, source, target)
}

/// Return the real form of `fake`, or `fake` unchanged if no rule applies.
fn unmap_address(rm: &mut RedactionMap, fake: &str) -> String {
    if let Some(v) = rm.reverse.get(fake) {
        return v.clone();
    }
    let Some(addr) = normalise_token(fake) else {
        return fake.to_owned();
    };
    let assignments = rm.cidr_assignments.clone();
    for a in &assignments {
        let Some(target) = parse_net(&a.target) else {
            continue;
        };
        match (target, addr) {
            (Net::V4(_), Addr::V4(_)) | (Net::V6(_), Addr::V6(_)) => {}
            _ => continue,
        }
        if target.contains_addr(addr) {
            let source = parse_net(&a.source).expect("valid source");
            let real = if let Some(k) = &a.shuffle_key {
                if k.is_empty() {
                    direct_map(addr, target, source)
                } else {
                    shuffle_host(
                        addr,
                        target,
                        source,
                        &hex_decode(k).unwrap_or_default(),
                        true,
                    )
                }
            } else {
                direct_map(addr, target, source)
            };
            rm.reverse.insert(fake.to_owned(), real.to_string_py());
            rm.forward.insert(real.to_string_py(), fake.to_owned());
            return real.to_string_py();
        }
    }
    fake.to_owned()
}

/// Substitute every IPv4 / IPv6 literal in `text` via `rm`. Returns
/// `(text, count)` where `count` is the number of literals that changed.
///
/// Mirrors `apply_map`: the v4 regex is applied first, then the v6 regex over
/// the result.
#[must_use]
pub fn apply_map(rm: &mut RedactionMap, text: &str, reverse: bool) -> (String, usize) {
    let mut count = 0usize;
    let after_v4 = substitute(text, find_v4, |tok| {
        let repl = if reverse {
            unmap_address(rm, tok)
        } else {
            map_address(rm, tok)
        };
        if repl != tok {
            count += 1;
        }
        repl
    });
    let after_v6 = substitute(&after_v4, find_v6, |tok| {
        let repl = if reverse {
            unmap_address(rm, tok)
        } else {
            map_address(rm, tok)
        };
        if repl != tok {
            count += 1;
        }
        repl
    });
    (after_v6, count)
}

/// Apply a replacement closure over every bounded literal match (mirrors
/// `re.sub`: non-overlapping, left-to-right).
fn substitute(
    text: &str,
    finder: fn(&str) -> Vec<LitMatch>,
    mut repl: impl FnMut(&str) -> String,
) -> String {
    let matches = finder(text);
    let mut out = String::with_capacity(text.len());
    let mut last = 0usize;
    for m in &matches {
        out.push_str(&text[last..m.start]);
        out.push_str(&repl(&m.text));
        last = m.end;
    }
    out.push_str(&text[last..]);
    out
}

// ── Secret stripping (rewrite.py redact half) ───────────────────────

const SECRET_KEYS: [&str; 9] = [
    "passphrase",
    "password",
    "encrypted-password",
    "secret",
    "shared-secret",
    "community",
    "auth-password",
    "priv-password",
    "rcv",
];

/// Report from [`redact_secrets`].
pub struct RedactReport {
    /// Number of secret-bearing property-value redactions performed.
    pub redactions_count: usize,
    /// Number of PEM blocks replaced.
    pub pem_blocks_replaced: usize,
    /// Number of IP literals remapped.
    pub ips_remapped: usize,
    /// Rewritten source text.
    pub new_source: String,
    /// The (possibly extended) redaction map, or `None` when IPs were kept.
    pub redaction_map: Option<RedactionMap>,
}

fn redact_property_values(source: &str) -> (String, usize) {
    let mut count = 0usize;
    let mut out = source.to_owned();
    for key in SECRET_KEYS {
        // (?<![\w-])(KEY)\s+("(?:[^"\\]|\\.)*"|[^\s{}]+)
        let pat = format!(
            r#"({})\s+("(?:[^"\\]|\\.)*"|[^\s{{}}]+)"#,
            regex::escape(key)
        );
        let re = Regex::new(&pat).expect("valid secret regex");
        let bytes = out.as_bytes();
        let mut result = String::with_capacity(out.len());
        let mut last = 0usize;
        let mut search_from = 0usize;
        while search_from <= bytes.len() {
            let Some(m) = re.find_at(&out, search_from) else {
                break;
            };
            // (?<![\w-]) — preceding byte must not be word char or '-'.
            let before_ok = m.start() == 0 || {
                let b = bytes[m.start() - 1];
                !(is_word_byte(b) || b == b'-')
            };
            if before_ok {
                // Need the captured group 1 (the key) to rebuild.
                let caps = re.captures_at(&out, m.start()).expect("match has caps");
                let g1 = caps.get(1).expect("group 1");
                result.push_str(&out[last..m.start()]);
                result.push_str(g1.as_str());
                result.push_str(" <REDACTED>");
                count += 1;
                last = m.end();
                search_from = m.end();
            } else {
                search_from = m.start() + 1;
            }
        }
        result.push_str(&out[last..]);
        out = result;
    }
    (out, count)
}

fn redact_pem_blocks(source: &str) -> (String, usize) {
    // -----BEGIN [A-Z ]+-----.*?-----END [A-Z ]+----- with DOTALL.
    let pem_re = Regex::new(r"(?s)-----BEGIN [A-Z ]+-----.*?-----END [A-Z ]+-----")
        .expect("valid PEM regex");
    let hdr_re = Regex::new(r"-----BEGIN [A-Z ]+-----").expect("valid hdr regex");
    let ftr_re = Regex::new(r"-----END [A-Z ]+-----").expect("valid ftr regex");
    let mut count = 0usize;
    let out = pem_re.replace_all(source, |caps: &regex::Captures| {
        count += 1;
        let whole = &caps[0];
        let header = hdr_re
            .find(whole)
            .map_or("-----BEGIN BLOCK-----", |m| m.as_str());
        let footer = ftr_re
            .find(whole)
            .map_or("-----END BLOCK-----", |m| m.as_str());
        format!("{header}<REDACTED>{footer}")
    });
    (out.into_owned(), count)
}

/// Options for [`redact_secrets`].
pub struct RedactOptions {
    /// Whether to remap public IPs (false == `--keep-ips`).
    pub remap_ips: bool,
    /// IPv4 target pool (defaults applied when empty).
    pub target_pool_v4: Vec<String>,
    /// IPv6 target pool (defaults applied when empty).
    pub target_pool_v6: Vec<String>,
    /// `"direct"` or `"shuffle"`.
    pub mode: String,
    /// Shuffle seed.
    pub seed: String,
    /// Explicit source CIDRs.
    pub explicit_source_cidrs: Vec<String>,
    /// An existing map to extend.
    pub existing_map: Option<RedactionMap>,
    /// Also remap private addresses.
    pub remap_private: bool,
}

/// Redact secrets in `source`; optionally remap public IPs (mirrors
/// `redact_secrets`).
///
/// # Errors
/// Propagates [`build_map`] errors (target collision / bad CIDR / unknown mode).
pub fn redact_secrets(source: &str, opts: RedactOptions) -> Result<RedactReport, String> {
    let (out, secrets) = redact_property_values(source);
    let (out, pem) = redact_pem_blocks(&out);
    if opts.remap_ips {
        let mut rm = build_map(BuildMapOptions {
            text: &out,
            target_pool_v4: if opts.target_pool_v4.is_empty() {
                DEFAULT_TARGET_CIDRS_V4
                    .iter()
                    .map(|s| (*s).to_owned())
                    .collect()
            } else {
                opts.target_pool_v4
            },
            target_pool_v6: if opts.target_pool_v6.is_empty() {
                DEFAULT_TARGET_CIDRS_V6
                    .iter()
                    .map(|s| (*s).to_owned())
                    .collect()
            } else {
                opts.target_pool_v6
            },
            mode: opts.mode,
            seed: opts.seed,
            source_cidr_v4_prefix: 24,
            source_cidr_v6_prefix: 64,
            explicit_source_cidrs: opts.explicit_source_cidrs,
            existing: opts.existing_map,
            remap_private: opts.remap_private,
        })?;
        let (out, ips) = apply_map(&mut rm, &out, false);
        Ok(RedactReport {
            redactions_count: secrets,
            pem_blocks_replaced: pem,
            ips_remapped: ips,
            new_source: out,
            redaction_map: Some(rm),
        })
    } else {
        Ok(RedactReport {
            redactions_count: secrets,
            pem_blocks_replaced: pem,
            ips_remapped: 0,
            new_source: out,
            redaction_map: Some(RedactionMap::default()),
        })
    }
}

// ── CPython random.Random (MT19937) port ────────────────────────────

/// A from-scratch port of CPython's `random.Random` sufficient for
/// `shuffle` byte-parity: `init_by_array` integer seeding, `getrandbits`,
/// `_randbelow_with_getrandbits`, and the reverse Fisher-Yates `shuffle`.
mod mt19937 {
    const N: usize = 624;
    const M: usize = 397;
    const MATRIX_A: u32 = 0x9908_b0df;
    const UPPER_MASK: u32 = 0x8000_0000;
    const LOWER_MASK: u32 = 0x7fff_ffff;

    /// Mersenne-Twister state + the helpers CPython layers on top.
    pub struct Random {
        mt: [u32; N],
        mti: usize,
    }

    impl Random {
        fn init_genrand(seed: u32) -> Random {
            let mut mt = [0u32; N];
            mt[0] = seed;
            for i in 1..N {
                mt[i] = 1_812_433_253u32
                    .wrapping_mul(mt[i - 1] ^ (mt[i - 1] >> 30))
                    .wrapping_add(i as u32);
            }
            Random { mt, mti: N }
        }

        fn init_by_array(init_key: &[u32]) -> Random {
            let mut r = Random::init_genrand(19_650_218);
            let mut i = 1usize;
            let mut j = 0usize;
            let key_length = init_key.len();
            let mut k = N.max(key_length);
            while k > 0 {
                r.mt[i] = (r.mt[i] ^ ((r.mt[i - 1] ^ (r.mt[i - 1] >> 30)).wrapping_mul(1_664_525)))
                    .wrapping_add(init_key[j])
                    .wrapping_add(j as u32);
                i += 1;
                j += 1;
                if i >= N {
                    r.mt[0] = r.mt[N - 1];
                    i = 1;
                }
                if j >= key_length {
                    j = 0;
                }
                k -= 1;
            }
            k = N - 1;
            while k > 0 {
                r.mt[i] = (r.mt[i]
                    ^ ((r.mt[i - 1] ^ (r.mt[i - 1] >> 30)).wrapping_mul(1_566_083_941)))
                .wrapping_sub(i as u32);
                i += 1;
                if i >= N {
                    r.mt[0] = r.mt[N - 1];
                    i = 1;
                }
                k -= 1;
            }
            r.mt[0] = 0x8000_0000;
            r
        }

        /// CPython `random_seed` for an integer seed: take the absolute value,
        /// split into 32-bit little-endian words, then `init_by_array`.
        #[must_use]
        pub fn from_int(seed: u128) -> Random {
            // CPython operates on abs(n); our seeds are always non-negative.
            let mut key: Vec<u32> = Vec::new();
            let mut n = seed;
            if n == 0 {
                key.push(0);
            } else {
                while n > 0 {
                    key.push((n & 0xffff_ffff) as u32);
                    n >>= 32;
                }
            }
            Random::init_by_array(&key)
        }

        fn genrand_uint32(&mut self) -> u32 {
            if self.mti >= N {
                for kk in 0..(N - M) {
                    let y = (self.mt[kk] & UPPER_MASK) | (self.mt[kk + 1] & LOWER_MASK);
                    self.mt[kk] = self.mt[kk + M] ^ (y >> 1) ^ ((y & 1) * MATRIX_A);
                }
                for kk in (N - M)..(N - 1) {
                    let y = (self.mt[kk] & UPPER_MASK) | (self.mt[kk + 1] & LOWER_MASK);
                    self.mt[kk] = self.mt[kk + M - N] ^ (y >> 1) ^ ((y & 1) * MATRIX_A);
                }
                let y = (self.mt[N - 1] & UPPER_MASK) | (self.mt[0] & LOWER_MASK);
                self.mt[N - 1] = self.mt[M - 1] ^ (y >> 1) ^ ((y & 1) * MATRIX_A);
                self.mti = 0;
            }
            let mut y = self.mt[self.mti];
            self.mti += 1;
            y ^= y >> 11;
            y ^= (y << 7) & 0x9d2c_5680;
            y ^= (y << 15) & 0xefc6_0000;
            y ^= y >> 18;
            y
        }

        /// CPython `getrandbits(k)` for `k <= 32` (host widths here are
        /// `<= SHUFFLE_MAX_WIDTH == 20`).
        fn getrandbits(&mut self, k: u32) -> u32 {
            debug_assert!(k <= 32);
            if k == 0 {
                return 0;
            }
            self.genrand_uint32() >> (32 - k)
        }

        /// CPython `_randbelow_with_getrandbits(n)`.
        fn randbelow(&mut self, n: u32) -> u32 {
            if n == 0 {
                return 0;
            }
            let k = 32 - n.leading_zeros(); // n.bit_length()
            let mut r = self.getrandbits(k);
            while r >= n {
                r = self.getrandbits(k);
            }
            r
        }

        /// CPython `Random.shuffle` (reverse Fisher-Yates).
        pub fn shuffle(&mut self, x: &mut [u32]) {
            let len = x.len();
            if len <= 1 {
                return;
            }
            for i in (1..len).rev() {
                let j = self.randbelow((i + 1) as u32) as usize;
                x.swap(i, j);
            }
        }
    }
}

// ── Minimal TOML reader for the map file ────────────────────────────

/// A tiny TOML reader tailored to the redact map-file shape (top-level
/// string keys + string arrays, repeated `[[cidr]]` tables, and an `[ips]`
/// table of quoted-key string values). Not a general TOML parser.
mod toml_lite {
    use std::collections::BTreeMap;

    pub struct Parsed {
        pub top: BTreeMap<String, String>,
        pub target_pool_v4: Option<Vec<String>>,
        pub target_pool_v6: Option<Vec<String>>,
        pub cidr: Vec<BTreeMap<String, String>>,
        pub ips: Vec<(String, String)>,
    }

    #[derive(PartialEq)]
    enum Section {
        Top,
        Cidr,
        Ips,
    }

    pub fn parse(text: &str) -> Result<Parsed, String> {
        let mut top = BTreeMap::new();
        let mut target_pool_v4 = None;
        let mut target_pool_v6 = None;
        let mut cidr: Vec<BTreeMap<String, String>> = Vec::new();
        let mut ips: Vec<(String, String)> = Vec::new();
        let mut section = Section::Top;

        for raw in text.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if line == "[[cidr]]" {
                section = Section::Cidr;
                cidr.push(BTreeMap::new());
                continue;
            }
            if line == "[ips]" {
                section = Section::Ips;
                continue;
            }
            if line.starts_with('[') {
                // Unknown table — treat as top to stay permissive.
                section = Section::Top;
                continue;
            }
            let Some((key_raw, val_raw)) = line.split_once('=') else {
                return Err(format!("malformed line: {line}"));
            };
            let key = key_raw.trim();
            let val = val_raw.trim();
            match section {
                Section::Top => {
                    if key == "target_pool_v4" {
                        target_pool_v4 = Some(parse_array(val)?);
                    } else if key == "target_pool_v6" {
                        target_pool_v6 = Some(parse_array(val)?);
                    } else {
                        top.insert(key.to_owned(), unquote(val).unwrap_or_default());
                    }
                }
                Section::Cidr => {
                    let table = cidr.last_mut().expect("cidr table present");
                    table.insert(key.to_owned(), unquote(val).unwrap_or_default());
                }
                Section::Ips => {
                    let k = unquote(key).ok_or("ips key not quoted")?;
                    let v = unquote(val).unwrap_or_default();
                    ips.push((k, v));
                }
            }
        }
        Ok(Parsed {
            top,
            target_pool_v4,
            target_pool_v6,
            cidr,
            ips,
        })
    }

    /// Strip surrounding double quotes. Returns `Some(s)` when quoted,
    /// `None` when the token wasn't a quoted string.
    fn unquote(s: &str) -> Option<String> {
        let s = s.trim();
        if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
            Some(s[1..s.len() - 1].to_owned())
        } else {
            None
        }
    }

    fn parse_array(s: &str) -> Result<Vec<String>, String> {
        let s = s.trim();
        let inner = s
            .strip_prefix('[')
            .and_then(|s| s.strip_suffix(']'))
            .ok_or_else(|| format!("malformed array: {s}"))?;
        let mut out = Vec::new();
        for part in inner.split(',') {
            let p = part.trim();
            if p.is_empty() {
                continue;
            }
            out.push(unquote(p).ok_or_else(|| format!("array element not quoted: {p}"))?);
        }
        Ok(out)
    }
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mt19937_shuffle_matches_cpython() {
        // random.Random(N).shuffle(list(range(16)))
        let cases: [(u128, [u32; 16]); 4] = [
            (0, [10, 14, 5, 1, 9, 2, 3, 11, 13, 7, 8, 4, 0, 6, 15, 12]),
            (1, [2, 10, 0, 14, 6, 5, 3, 8, 7, 11, 15, 1, 12, 13, 9, 4]),
            (42, [7, 9, 5, 6, 14, 10, 12, 8, 1, 2, 13, 15, 4, 11, 0, 3]),
            (
                34_056_686_661_485_269_175_879_051_380_952_572_020,
                [1, 15, 3, 8, 7, 5, 9, 2, 4, 0, 10, 6, 14, 11, 13, 12],
            ),
        ];
        for (seed, expected_head) in cases {
            let mut rng = mt19937::Random::from_int(seed);
            let mut x: Vec<u32> = (0..16).collect();
            rng.shuffle(&mut x);
            assert_eq!(&x[..], &expected_head[..], "seed {seed}");
        }
    }

    #[test]
    fn mt19937_perm256_matches_cpython() {
        // The first source-CIDR shuffle key for `--shuffle --seed 42`.
        let key = derive_key("42", "93.184.216.0/24");
        let (fwd, _inv) = build_permutation(8, &key);
        assert_eq!(fwd[34], 172);
        assert_eq!(fwd[35], 63);
    }

    #[test]
    fn default_redact_map_byte_identical() {
        let src = "address 93.184.216.34\naddress 8.8.8.8\naddress 1.1.1.1\n\
                   address 2001:4860:4860::8888\naddress 10.0.1.10\naddress 93.184.216.35\n";
        let report = redact_secrets(
            src,
            RedactOptions {
                remap_ips: true,
                target_pool_v4: vec![],
                target_pool_v6: vec![],
                mode: "direct".to_owned(),
                seed: String::new(),
                explicit_source_cidrs: vec![],
                existing_map: None,
                remap_private: false,
            },
        )
        .unwrap();
        // 10.0.1.10 is private -> untouched; the rest remap into 10.0.0.0/8.
        assert!(report.new_source.contains("address 10.0.0.34"));
        assert!(report.new_source.contains("address 10.0.1.8"));
        assert!(report.new_source.contains("address 10.0.2.1"));
        assert!(report.new_source.contains("address fd00::8888"));
        assert!(report.new_source.contains("address 10.0.1.10")); // unchanged
        assert_eq!(report.ips_remapped, 5);
        let toml = report.redaction_map.unwrap().to_toml();
        assert!(toml.contains(r#"source = "93.184.216.0/24""#));
        assert!(toml.contains(r#"target = "10.0.0.0/24""#));
        assert!(toml.contains(r#""1.1.1.1" = "10.0.2.1""#));
        assert!(toml.ends_with('\n'));
    }

    #[test]
    fn roundtrip_restores_original() {
        let src = "a 93.184.216.34 b 8.8.8.8 c 2001:4860:4860::8888\n";
        let report = redact_secrets(
            src,
            RedactOptions {
                remap_ips: true,
                target_pool_v4: vec![],
                target_pool_v6: vec![],
                mode: "direct".to_owned(),
                seed: String::new(),
                explicit_source_cidrs: vec![],
                existing_map: None,
                remap_private: false,
            },
        )
        .unwrap();
        let toml = report.redaction_map.unwrap().to_toml();
        let mut rm = RedactionMap::from_toml(&toml).unwrap();
        let (back, _count) = apply_map(&mut rm, &report.new_source, true);
        assert_eq!(back, src);
    }

    #[test]
    fn secret_and_pem_redaction() {
        let src = "password s3cr3tPassw0rd\nrecv \"200 OK\"\n\
                   key \"-----BEGIN RSA PRIVATE KEY-----\nABC\n-----END RSA PRIVATE KEY-----\"\n";
        let (out, n) = redact_property_values(src);
        assert_eq!(n, 1); // password only; `recv` != `rcv`
        assert!(out.contains("password <REDACTED>"));
        assert!(out.contains("recv \"200 OK\""));
        let (out2, pem) = redact_pem_blocks(&out);
        assert_eq!(pem, 1);
        assert!(
            out2.contains("-----BEGIN RSA PRIVATE KEY-----<REDACTED>-----END RSA PRIVATE KEY-----")
        );
    }
}
