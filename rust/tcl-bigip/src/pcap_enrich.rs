//! Config → capture-label engine for `f5 enrich-pcapng`.
//!
//! [`build_merged_name_index`] derives hostname-style labels for every IP an
//! object touches (virtual-server destinations, pool / SNAT members, nodes,
//! self-IPs and their subnets, GTM wide-IPs), and [`enrich_pcapng`] injects
//! those mappings into a PCAPNG as a Name Resolution Block (plus an optional
//! TLS keylog Decryption Secrets Block), round-tripping every other block
//! byte-for-byte. The `NameIndex` and direct-write PCAPNG paths are
//! deterministic; libpcap → pcapng conversion shells out to
//! `editcap` / `tshark` and is handled by the verb layer.

use std::collections::BTreeMap;
use std::net::IpAddr;

use crate::model::ModelObject;
use crate::parser::BigipConfig;
use crate::parser::helpers::{extract_blocks, parse_list_block, parse_properties};
use crate::pcap_remap::find_ip_offset;
use crate::pcapng::{self, Endian};

/// Result stats reported back to the CLI after a successful enrichment.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EnrichResult {
    /// Number of IPv4 NRB records emitted.
    pub ipv4_records: usize,
    /// Number of IPv6 NRB records emitted.
    pub ipv6_records: usize,
    /// Total label count across all records.
    pub names_total: usize,
    /// Bytes of keylog material embedded in the DSB (0 if none).
    pub keylog_bytes: usize,
    /// True if the input was converted from libpcap.
    pub converted_from_libpcap: bool,
    /// Observed IPv4 address count (0 in `--all` mode).
    pub observed_ipv4: usize,
    /// Observed IPv6 address count (0 in `--all` mode).
    pub observed_ipv6: usize,
}

// ── Name index ──────────────────────────────────────────────────────

/// Maps IP addresses (canonical `str`) to one or more annotation names, with a
/// CIDR-fallback list for self-IP subnets.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NameIndex {
    /// Exact IPv4 → labels, insertion-ordered.
    pub v4: Vec<(String, Vec<String>)>,
    /// Exact IPv6 → labels, insertion-ordered.
    pub v6: Vec<(String, Vec<String>)>,
    /// IPv4 subnet (network string) → label, insertion-ordered, deduped.
    pub v4_subnets: Vec<(IpNetwork, String)>,
    /// IPv6 subnet (network string) → label, insertion-ordered, deduped.
    pub v6_subnets: Vec<(IpNetwork, String)>,
}

/// A canonicalised IP network (address + prefix length), with host bits
/// cleared (non-strict).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IpNetwork {
    /// Network address (host bits zeroed).
    network_addr: IpAddr,
    /// Prefix length in bits.
    prefix: u8,
    /// Canonical `network/prefix` text.
    text: String,
}

impl IpNetwork {
    /// Parse `addr/prefix` non-strictly (host bits are masked off).
    fn parse(addr: &str, prefix: u8) -> Option<Self> {
        let ip: IpAddr = addr.parse().ok()?;
        let (network_addr, max) = match ip {
            IpAddr::V4(v4) => {
                if prefix > 32 {
                    return None;
                }
                let bits = u32::from(v4);
                let mask = if prefix == 0 {
                    0
                } else {
                    u32::MAX << (32 - prefix)
                };
                (IpAddr::V4((bits & mask).into()), 32u8)
            }
            IpAddr::V6(v6) => {
                if prefix > 128 {
                    return None;
                }
                let bits = u128::from(v6);
                let mask = if prefix == 0 {
                    0
                } else {
                    u128::MAX << (128 - prefix)
                };
                (IpAddr::V6((bits & mask).into()), 128u8)
            }
        };
        let _ = max;
        let text = format!("{network_addr}/{prefix}");
        Some(Self {
            network_addr,
            prefix,
            text,
        })
    }

    /// Canonical `network/prefix` text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// True if `ip` falls inside this network.
    fn contains(&self, ip: IpAddr) -> bool {
        match (self.network_addr, ip) {
            (IpAddr::V4(net), IpAddr::V4(addr)) => {
                let mask = if self.prefix == 0 {
                    0
                } else {
                    u32::MAX << (32 - self.prefix)
                };
                (u32::from(addr) & mask) == u32::from(net)
            }
            (IpAddr::V6(net), IpAddr::V6(addr)) => {
                let mask = if self.prefix == 0 {
                    0
                } else {
                    u128::MAX << (128 - self.prefix)
                };
                (u128::from(addr) & mask) == u128::from(net)
            }
            _ => false,
        }
    }

    /// True if this is an IPv4 network.
    #[must_use]
    pub fn is_v4(&self) -> bool {
        matches!(self.network_addr, IpAddr::V4(_))
    }
}

/// Canonicalise an address to its standard compressed text form.
fn canonical_ip(address: &str) -> Option<(IpAddr, String)> {
    let ip: IpAddr = address.parse().ok()?;
    Some((ip, ip.to_string()))
}

impl NameIndex {
    /// Add an exact-IP label (deduped per address).
    pub fn add(&mut self, address: &str, name: &str) {
        if address.is_empty() || name.is_empty() {
            return;
        }
        let Some((ip, canon)) = canonical_ip(address) else {
            return;
        };
        let bucket = if ip.is_ipv4() {
            &mut self.v4
        } else {
            &mut self.v6
        };
        if let Some(slot) = bucket.iter_mut().find(|(a, _)| *a == canon) {
            if !slot.1.iter().any(|n| n == name) {
                slot.1.push(name.to_owned());
            }
        } else {
            bucket.push((canon, vec![name.to_owned()]));
        }
    }

    /// Add a CIDR-fallback subnet label (deduped by `(network, name)`).
    pub fn add_subnet(&mut self, network: IpNetwork, name: &str) {
        if name.is_empty() {
            return;
        }
        let bucket = if network.is_v4() {
            &mut self.v4_subnets
        } else {
            &mut self.v6_subnets
        };
        let entry = (network, name.to_owned());
        if !bucket.contains(&entry) {
            bucket.push(entry);
        }
    }

    /// Merge `other` into `self`.
    fn update(&mut self, other: NameIndex) {
        for (addr, names) in &other.v4 {
            for name in names {
                self.add(addr, name);
            }
        }
        for (addr, names) in &other.v6 {
            for name in names {
                self.add(addr, name);
            }
        }
        for (net, name) in other.v4_subnets {
            self.add_subnet(net, &name);
        }
        for (net, name) in other.v6_subnets {
            self.add_subnet(net, &name);
        }
    }

    /// Every label that applies to `address` (exact + subnet).
    #[must_use]
    pub fn lookup(&self, address: &str) -> Vec<String> {
        let Some((ip, canon)) = canonical_ip(address) else {
            return Vec::new();
        };
        let (exact, subnets) = if ip.is_ipv4() {
            (&self.v4, &self.v4_subnets)
        } else {
            (&self.v6, &self.v6_subnets)
        };
        let mut names: Vec<String> = exact
            .iter()
            .find(|(a, _)| *a == canon)
            .map(|(_, n)| n.clone())
            .unwrap_or_default();
        for (net, label) in subnets {
            if net.contains(ip) && !names.iter().any(|n| n == label) {
                names.push(label.clone());
            }
        }
        names
    }

    /// Total label count across exact + subnet entries.
    #[must_use]
    pub fn total_names(&self) -> usize {
        let exact: usize = self.v4.iter().map(|(_, n)| n.len()).sum::<usize>()
            + self.v6.iter().map(|(_, n)| n.len()).sum::<usize>();
        exact + self.v4_subnets.len() + self.v6_subnets.len()
    }
}

// ── Naming helpers ──────────────────────────────────────────────────

/// Lowercase `text`, replacing runs of non-`[a-z0-9-]` with a single `-`,
/// stripping leading/trailing `-`.
#[must_use]
pub fn slug(text: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for ch in text.chars() {
        for lc in ch.to_lowercase() {
            if lc.is_ascii_lowercase() || lc.is_ascii_digit() || lc == '-' {
                out.push(lc);
                prev_dash = lc == '-';
            } else if !prev_dash {
                out.push('-');
                prev_dash = true;
            }
        }
    }
    out.trim_matches('-').to_owned()
}

/// Build a `purpose-objname` label, lowercased with `-` separators.
#[must_use]
pub fn label(purpose: &str, parts: &[&str]) -> String {
    let mut pieces = vec![purpose.to_owned()];
    for part in parts {
        let s = slug(part);
        if !s.is_empty() {
            pieces.push(s);
        }
    }
    pieces.join("-")
}

/// Extract the address portion of a `[/Common/]ADDR[:port]` destination.
#[must_use]
pub fn split_destination(dest: &str) -> String {
    if dest.is_empty() {
        return String::new();
    }
    let base = dest.rsplit('/').next().unwrap_or(dest);
    for sep in [':', '.'] {
        // BIG-IP uses `.port` for IPv6 destinations, `:port` for IPv4.
        if let Some(idx) = base.rfind(sep) {
            let host = &base[..idx];
            let port = &base[idx + sep.len_utf8()..];
            if !host.is_empty() && resolve_port(port).is_some() && host.parse::<IpAddr>().is_ok() {
                return host.to_owned();
            }
        }
    }
    if base.parse::<IpAddr>().is_ok() {
        base.to_owned()
    } else {
        String::new()
    }
}

/// Resolve a port token (decimal or BIG-IP service name).
#[must_use]
pub fn resolve_port(value: &str) -> Option<i64> {
    if value.is_empty() {
        return None;
    }
    if value.bytes().all(|b| b.is_ascii_digit()) {
        let num: i64 = value.parse().ok()?;
        return (0..=65535).contains(&num).then_some(num);
    }
    crate::model::port_names::name_to_port(&value.to_lowercase())
}

// ── BigipConfig accessors ──

fn typed_objects<'a>(
    cfg: &'a BigipConfig,
    table: &'static str,
) -> impl Iterator<Item = (&'a str, &'a ModelObject)> {
    cfg.objects
        .iter()
        .filter(move |p| p.table_name == table)
        .map(|p| (p.full_path.as_str(), &p.object))
}

fn find_object<'a>(
    cfg: &'a BigipConfig,
    table: &'static str,
    full_path: &str,
) -> Option<&'a ModelObject> {
    cfg.objects
        .iter()
        .find(|p| p.table_name == table && p.full_path == full_path)
        .map(|p| &p.object)
}

/// `BigipConfig.resolve_name`: exact, partition-qualified, then suffix match.
fn resolve_name(cfg: &BigipConfig, name: &str, table: &'static str) -> Option<String> {
    if find_object(cfg, table, name).is_some() {
        return Some(name.to_owned());
    }
    if !name.starts_with('/') {
        let partition = {
            let p = if cfg.default_partition.is_empty() {
                "Common"
            } else {
                cfg.default_partition.as_str()
            };
            p.trim_matches('/').to_owned()
        };
        if !partition.is_empty() {
            let candidate = format!("/{partition}/{name}");
            if find_object(cfg, table, &candidate).is_some() {
                return Some(candidate);
            }
        }
        if partition != "Common" {
            let candidate = format!("/Common/{name}");
            if find_object(cfg, table, &candidate).is_some() {
                return Some(candidate);
            }
        }
    }
    let suffix = format!("/{name}");
    typed_objects(cfg, table)
        .map(|(p, _)| p)
        .find(|key| key.ends_with(&suffix))
        .map(ToOwned::to_owned)
}

/// `BigipConfig.resolve_rule`.
fn resolve_rule(cfg: &BigipConfig, name: &str) -> Option<String> {
    resolve_name(cfg, name, "rules")
}

/// `BigipConfig.resolve_name(..., self.nodes)` returning the node address.
fn resolve_node_address(cfg: &BigipConfig, candidate: &str) -> Option<String> {
    let resolved = resolve_name(cfg, candidate, "nodes")?;
    if let Some(ModelObject::Node(node)) = find_object(cfg, "nodes", &resolved) {
        return node.address.as_ref().map(ToString::to_string);
    }
    None
}

/// Resolve a pool member's IP from its name, falling back to the node table.
fn resolve_pool_member_address(member_name: &str, cfg: &BigipConfig) -> String {
    let addr = split_destination(member_name);
    if !addr.is_empty() {
        return addr;
    }
    let base = member_name.rsplit('/').next().unwrap_or(member_name);
    // Python: base.split(":",1)[0].split(".",1)[0]
    let short_no_port = {
        let s = base.split_once(':').map_or(base, |(h, _)| h);
        s.split_once('.').map_or(s, |(h, _)| h)
    };
    let full_no_port = {
        let s = member_name.split_once(':').map_or(member_name, |(h, _)| h);
        s.split_once('.').map_or(s, |(h, _)| h)
    };
    for candidate in [full_no_port, member_name, short_no_port] {
        if let Some(addr) = resolve_node_address(cfg, candidate) {
            return addr;
        }
    }
    String::new()
}

// ── net self extraction ─────────────────────────────────────────────

pub(crate) struct SelfIp {
    pub(crate) full_path: String,
    pub(crate) address: String,
    pub(crate) network: Option<IpNetwork>,
}

/// Parse `{ name1 { body1 } name2 { body2 } ... }` into `[(name, body), …]`.
pub(crate) fn parse_named_subblocks(braced: &str) -> Vec<(String, String)> {
    let mut inner = braced.trim();
    inner = inner.strip_prefix('{').unwrap_or(inner);
    inner = inner.strip_suffix('}').unwrap_or(inner);
    let bytes = inner.as_bytes();
    let n = inner.len();
    let mut out = Vec::new();
    let mut pos = 0;
    while pos < n {
        while pos < n && matches!(bytes[pos], b' ' | b'\t' | b'\n' | b'\r') {
            pos += 1;
        }
        if pos >= n {
            break;
        }
        let name_start = pos;
        while pos < n && !matches!(bytes[pos], b' ' | b'\t' | b'\n' | b'\r' | b'{') {
            pos += 1;
        }
        let name = inner[name_start..pos].to_owned();
        while pos < n && matches!(bytes[pos], b' ' | b'\t') {
            pos += 1;
        }
        let mut body = String::new();
        if pos < n && bytes[pos] == b'{' {
            let body_start = pos + 1;
            pos += 1;
            let mut depth = 1;
            while pos < n && depth > 0 {
                match bytes[pos] {
                    b'{' => depth += 1,
                    b'}' => depth -= 1,
                    _ => {}
                }
                pos += 1;
            }
            inner[body_start..pos - 1].clone_into(&mut body);
        }
        if !name.is_empty() {
            out.push((name, body));
        }
    }
    out
}

fn property(props: &[(String, String)], key: &str) -> Option<String> {
    props.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
}

/// Pull every `net self` block's address (and CIDR, if present).
pub(crate) fn extract_self_ips(source: &str) -> Vec<SelfIp> {
    let mut found = Vec::new();
    for block in extract_blocks(source) {
        let parts: Vec<&str> = block.header.split_whitespace().collect();
        if parts.len() < 3 || parts[0] != "net" || parts[1] != "self" {
            continue;
        }
        let full_path = parts[2].to_owned();
        let props = parse_properties(&block.body);
        let raw = property(&props, "address").unwrap_or_default();
        let raw = raw.trim();
        if raw.is_empty() {
            continue;
        }
        let (addr_part, cidr) = match raw.split_once('/') {
            Some((a, c)) => (a, c),
            None => (raw, ""),
        };
        let addr_no_rd = addr_part.split_once('%').map_or(addr_part, |(a, _)| a);
        if addr_no_rd.parse::<IpAddr>().is_err() {
            continue;
        }
        let network = if cidr.is_empty() {
            None
        } else {
            cidr.parse::<u8>()
                .ok()
                .and_then(|p| IpNetwork::parse(addr_no_rd, p))
        };
        found.push(SelfIp {
            full_path,
            address: addr_no_rd.to_owned(),
            network,
        });
    }
    found
}

// ── GTM extraction ──────────────────────────────────────────────────

struct GtmServer {
    addresses: Vec<String>,
    virtual_servers: BTreeMap<String, String>,
}

/// Strip a `:port` / `.port` suffix and return just the IP literal.
fn addr_only(text: &str) -> String {
    let s = split_destination(text);
    if s.is_empty() { text.to_owned() } else { s }
}

fn extract_gtm_servers(source: &str) -> BTreeMap<String, GtmServer> {
    let mut servers: BTreeMap<String, GtmServer> = BTreeMap::new();
    let mut order: Vec<String> = Vec::new();
    let _ = &mut order;
    for block in extract_blocks(source) {
        let parts: Vec<&str> = block.header.split_whitespace().collect();
        if parts.len() < 3 || parts[0] != "gtm" || parts[1] != "server" {
            continue;
        }
        let full_path = parts[2].to_owned();
        let props = parse_properties(&block.body);

        let mut addresses = Vec::new();
        if let Some(addresses_block) = property(&props, "addresses") {
            for (raw, _body) in parse_named_subblocks(&addresses_block) {
                let addr = addr_only(&raw);
                if addr.parse::<IpAddr>().is_ok() {
                    addresses.push(addr);
                }
            }
        }

        let mut vs_addrs = BTreeMap::new();
        if let Some(vs_block) = property(&props, "virtual-servers") {
            for (vs_name, body) in parse_named_subblocks(&vs_block) {
                let inner = parse_properties(&body);
                let dest = property(&inner, "destination").unwrap_or_default();
                let addr = addr_only(&dest);
                if addr.parse::<IpAddr>().is_ok() {
                    vs_addrs.insert(vs_name, addr);
                }
            }
        }

        servers.insert(
            full_path,
            GtmServer {
                addresses,
                virtual_servers: vs_addrs,
            },
        );
    }
    servers
}

/// `pool_full_path -> [(server_full_path, vs_name), …]`.
fn extract_gtm_pools(source: &str) -> Vec<(String, Vec<(String, String)>)> {
    let mut pools: Vec<(String, Vec<(String, String)>)> = Vec::new();
    for block in extract_blocks(source) {
        let parts: Vec<&str> = block.header.split_whitespace().collect();
        if parts.len() < 3 || parts[0] != "gtm" || parts[1] != "pool" {
            continue;
        }
        let full_path = parts[parts.len() - 1].to_owned();
        let props = parse_properties(&block.body);
        let Some(members_block) = property(&props, "members") else {
            continue;
        };
        let bucket = if let Some(slot) = pools.iter_mut().find(|(p, _)| *p == full_path) {
            &mut slot.1
        } else {
            pools.push((full_path.clone(), Vec::new()));
            &mut pools.last_mut().unwrap().1
        };
        for member_name in parse_list_block(&members_block) {
            // `<server_full_path>:<vs_name>` — split on the last `:`.
            let Some(idx) = member_name.rfind(':') else {
                continue;
            };
            let head = &member_name[..idx];
            let vs_name = &member_name[idx + 1..];
            if head.is_empty() || vs_name.is_empty() {
                continue;
            }
            bucket.push((head.to_owned(), vs_name.to_owned()));
        }
    }
    pools
}

/// `wideip_full_path -> [pool_full_path, …]`.
fn extract_gtm_wideips(source: &str) -> Vec<(String, Vec<String>)> {
    let mut wideips = Vec::new();
    for block in extract_blocks(source) {
        let parts: Vec<&str> = block.header.split_whitespace().collect();
        if parts.len() < 3 || parts[0] != "gtm" || parts[1] != "wideip" {
            continue;
        }
        let full_path = parts[parts.len() - 1].to_owned();
        let props = parse_properties(&block.body);
        let Some(pools_block) = property(&props, "pools") else {
            continue;
        };
        wideips.push((full_path, parse_list_block(&pools_block)));
    }
    wideips
}

/// Resolve a GTM reference: exact, `/Common/<name>`, then suffix match.
fn resolve_gtm_path<'a, T>(name: &str, pool: &'a [(String, T)]) -> Option<&'a str> {
    if pool.iter().any(|(k, _)| k == name) {
        return Some(pool.iter().find(|(k, _)| k == name).unwrap().0.as_str());
    }
    let candidate = format!("/Common/{name}");
    if let Some((k, _)) = pool.iter().find(|(k, _)| *k == candidate) {
        return Some(k.as_str());
    }
    let suffix = format!("/{name}");
    pool.iter()
        .find(|(k, _)| k.ends_with(&suffix))
        .map(|(k, _)| k.as_str())
}

fn resolve_gtm_server<'a>(name: &str, servers: &'a BTreeMap<String, GtmServer>) -> Option<&'a str> {
    if servers.contains_key(name) {
        return servers
            .keys()
            .find(|k| k.as_str() == name)
            .map(String::as_str);
    }
    let candidate = format!("/Common/{name}");
    if let Some(k) = servers.keys().find(|k| **k == candidate) {
        return Some(k.as_str());
    }
    let suffix = format!("/{name}");
    servers
        .keys()
        .find(|k| k.ends_with(&suffix))
        .map(String::as_str)
}

/// For each wide-IP, the set of IPs its pool chain resolves to (insertion
/// order preserved per wide-IP).
fn wideip_addresses(
    wideips: &[(String, Vec<String>)],
    gtm_pools: &[(String, Vec<(String, String)>)],
    servers: &BTreeMap<String, GtmServer>,
) -> Vec<(String, Vec<String>)> {
    let mut out = Vec::new();
    for (wideip_path, pool_refs) in wideips {
        let mut addrs: Vec<String> = Vec::new();
        for pool_ref in pool_refs {
            let Some(resolved_pool) = resolve_gtm_path(pool_ref, gtm_pools) else {
                continue;
            };
            let members = &gtm_pools
                .iter()
                .find(|(p, _)| p == resolved_pool)
                .unwrap()
                .1;
            for (server_ref, vs_name) in members {
                let Some(resolved_server) = resolve_gtm_server(server_ref, servers) else {
                    continue;
                };
                let server = &servers[resolved_server];
                if let Some(addr) = server.virtual_servers.get(vs_name) {
                    if !addrs.contains(addr) {
                        addrs.push(addr.clone());
                    }
                } else {
                    for addr in &server.addresses {
                        if !addrs.contains(addr) {
                            addrs.push(addr.clone());
                        }
                    }
                }
            }
        }
        if !addrs.is_empty() {
            out.push((wideip_path.clone(), addrs));
        }
    }
    out
}

// ── Index builders ──────────────────────────────────────────────────

/// Build a name index from a parsed [`BigipConfig`]. When `source` is `Some`,
/// `net self` and GTM blocks are also scanned.
#[must_use]
pub fn build_name_index(cfg: &BigipConfig, source: Option<&str>) -> NameIndex {
    let mut index = NameIndex::default();

    for (full_path, obj) in typed_objects(cfg, "virtual_servers") {
        let ModelObject::VirtualServer(vs) = obj else {
            continue;
        };
        let dest_text = vs
            .destination
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_default();
        let addr = split_destination(&dest_text);
        if addr.is_empty() {
            continue;
        }
        index.add(&addr, &label("vs", &[full_path]));
        for rule_ref in vs.rules.paths() {
            let resolved = resolve_rule(cfg, &rule_ref).unwrap_or_else(|| rule_ref.clone());
            index.add(&addr, &label("irule", &[&resolved]));
        }
    }

    for (_path, obj) in typed_objects(cfg, "pools") {
        let ModelObject::Pool(pool) = obj else {
            continue;
        };
        for item in &pool.members.items {
            let crate::value::ListItemValue::PoolMember(member) = &item.value else {
                continue;
            };
            let member_addr_text = member
                .address
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default();
            let addr = if member_addr_text.is_empty() {
                resolve_pool_member_address(&member.name, cfg)
            } else {
                member_addr_text
            };
            if addr.is_empty() {
                continue;
            }
            let name_part = if member.name.is_empty() {
                addr.clone()
            } else {
                member.name.clone()
            };
            index.add(&addr, &label("pool", &[&name_part]));
        }
    }

    for (full_path, obj) in typed_objects(cfg, "snat_pools") {
        let ModelObject::SnatPool(snat) = obj else {
            continue;
        };
        for raw in snat.members.paths() {
            let addr = split_destination(&raw);
            if addr.is_empty() {
                continue;
            }
            index.add(&addr, &label("snat", &[full_path]));
        }
    }

    for (full_path, obj) in typed_objects(cfg, "nodes") {
        let ModelObject::Node(node) = obj else {
            continue;
        };
        if let Some(addr) = &node.address {
            index.add(&addr.to_string(), &label("node", &[full_path]));
        }
    }

    if let Some(source) = source {
        for self_ip in extract_self_ips(source) {
            index.add(&self_ip.address, &label("self", &[&self_ip.full_path]));
            if let Some(network) = self_ip.network {
                let lbl = label("net", &[&self_ip.full_path]);
                index.add_subnet(network, &lbl);
            }
        }

        let servers = extract_gtm_servers(source);
        let gtm_pools = extract_gtm_pools(source);
        let wideips = extract_gtm_wideips(source);
        for (wideip_path, addrs) in wideip_addresses(&wideips, &gtm_pools, &servers) {
            let lbl = label("wideip", &[&wideip_path]);
            for addr in addrs {
                index.add(&addr, &lbl);
            }
        }
    }

    index
}

/// Build a merged [`NameIndex`] from multiple `(config, source)` pairs.
#[must_use]
pub fn build_merged_name_index(configs_with_sources: &[(BigipConfig, String)]) -> NameIndex {
    if configs_with_sources.is_empty() {
        return NameIndex::default();
    }
    let combined_source = configs_with_sources
        .iter()
        .map(|(_, s)| s.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let mut merged = NameIndex::default();
    for (cfg, _src) in configs_with_sources {
        merged.update(build_name_index(cfg, Some(&combined_source)));
    }
    merged
}

// ── Pcapng walking — observed IPs ───────────────────────────────────

fn scan_packet_ips(packet: &[u8], linktype: u16, v4: &mut Vec<String>, v6: &mut Vec<String>) {
    let Some((ip_off, is_v6)) = find_ip_offset(packet, linktype) else {
        return;
    };
    if is_v6 {
        if ip_off + 40 > packet.len() {
            return;
        }
        let src: [u8; 16] = packet[ip_off + 8..ip_off + 24].try_into().unwrap();
        let dst: [u8; 16] = packet[ip_off + 24..ip_off + 40].try_into().unwrap();
        push_unique(v6, std::net::Ipv6Addr::from(src).to_string());
        push_unique(v6, std::net::Ipv6Addr::from(dst).to_string());
    } else {
        if ip_off + 20 > packet.len() {
            return;
        }
        let src: [u8; 4] = packet[ip_off + 12..ip_off + 16].try_into().unwrap();
        let dst: [u8; 4] = packet[ip_off + 16..ip_off + 20].try_into().unwrap();
        push_unique(v4, std::net::Ipv4Addr::from(src).to_string());
        push_unique(v4, std::net::Ipv4Addr::from(dst).to_string());
    }
}

fn push_unique(set: &mut Vec<String>, value: String) {
    if !set.contains(&value) {
        set.push(value);
    }
}

/// Walk every EPB and return the observed `(ipv4, ipv6)` address sets.
fn collect_observed_ips(blocks: &[pcapng::PcapngBlock]) -> (Vec<String>, Vec<String>) {
    let mut v4 = Vec::new();
    let mut v6 = Vec::new();
    let mut interface_linktypes: Vec<u16> = Vec::new();
    for block in blocks {
        if block.block_type == pcapng::BLOCK_TYPE_SHB {
            interface_linktypes.clear();
        } else if block.block_type == pcapng::BLOCK_TYPE_IDB {
            interface_linktypes.push(block.linktype.unwrap_or(0));
        } else if block.block_type == pcapng::BLOCK_TYPE_EPB
            && let Some(packet) = &block.packet_data
        {
            let iface = block.interface_id.unwrap_or(0) as usize;
            if iface < interface_linktypes.len() {
                scan_packet_ips(packet, interface_linktypes[iface], &mut v4, &mut v6);
            }
        }
    }
    (v4, v6)
}

// ── PCAPNG enrichment driver ────────────────────────────────────────

fn packed_v4(addr: &str) -> [u8; 4] {
    addr.parse::<std::net::Ipv4Addr>().unwrap().octets()
}

fn packed_v6(addr: &str) -> [u8; 16] {
    addr.parse::<std::net::Ipv6Addr>().unwrap().octets()
}

type PackedRecords = (Vec<([u8; 4], Vec<String>)>, Vec<([u8; 16], Vec<String>)>);

/// Resolve labels for every observed IP (or every inventory IP when both
/// `observed` sets are `None`). Records are sorted by packed-address bytes for
/// determinism.
fn build_packed_records(
    index: &NameIndex,
    observed_v4: Option<&[String]>,
    observed_v6: Option<&[String]>,
) -> PackedRecords {
    let mut v4_packed: Vec<([u8; 4], Vec<String>)> = Vec::new();
    let mut v6_packed: Vec<([u8; 16], Vec<String>)> = Vec::new();

    if observed_v4.is_some() || observed_v6.is_some() {
        let mut v4: Vec<&String> = observed_v4.unwrap_or(&[]).iter().collect();
        v4.sort_by_key(|a| packed_v4(a));
        for addr in v4 {
            let names = index.lookup(addr);
            if !names.is_empty() {
                v4_packed.push((packed_v4(addr), names));
            }
        }
        let mut v6: Vec<&String> = observed_v6.unwrap_or(&[]).iter().collect();
        v6.sort_by_key(|a| packed_v6(a));
        for addr in v6 {
            let names = index.lookup(addr);
            if !names.is_empty() {
                v6_packed.push((packed_v6(addr), names));
            }
        }
        return (v4_packed, v6_packed);
    }

    let mut v4: Vec<&(String, Vec<String>)> = index.v4.iter().collect();
    v4.sort_by_key(|(a, _)| packed_v4(a));
    for (addr, names) in v4 {
        if !names.is_empty() {
            v4_packed.push((packed_v4(addr), names.clone()));
        }
    }
    let mut v6: Vec<&(String, Vec<String>)> = index.v6.iter().collect();
    v6.sort_by_key(|(a, _)| packed_v6(a));
    for (addr, names) in v6 {
        if !names.is_empty() {
            v6_packed.push((packed_v6(addr), names.clone()));
        }
    }
    (v4_packed, v6_packed)
}

/// Insert NRB (and optional DSB) blocks derived from `name_index` into `input`.
///
/// The NRB/DSB are inserted right after the
/// first IDB of the first section; every other block round-trips byte-for-byte.
///
/// # Errors
/// Returns an error string when the input is not PCAPNG or malformed.
pub fn enrich_pcapng_with_index(
    input: &[u8],
    name_index: &NameIndex,
    keylog_text: Option<&[u8]>,
    include_unobserved: bool,
) -> Result<(Vec<u8>, EnrichResult), String> {
    let keylog_blob: Option<&[u8]> = match keylog_text {
        Some(k) if !k.is_empty() => Some(k),
        _ => None,
    };

    if input.len() < 4 || !pcapng::is_pcapng_magic(&input[..4]) {
        return Err(
            "enrich requires PCAPNG input (libpcap can't carry NRB/DSB blocks); \
             install wireshark/editcap or convert manually first."
                .to_owned(),
        );
    }

    let blocks = pcapng::read_blocks(input)?;

    let (observed_v4, observed_v6) = if include_unobserved {
        (None, None)
    } else {
        let (v4, v6) = collect_observed_ips(&blocks);
        (Some(v4), Some(v6))
    };

    let (v4_packed, v6_packed) =
        build_packed_records(name_index, observed_v4.as_deref(), observed_v6.as_deref());

    let mut out: Vec<u8> = Vec::with_capacity(input.len());
    let mut inserted = false;
    let mut pending_section_endian: Option<Endian> = None;

    for block in &blocks {
        pcapng::write_block(&mut out, block)?;
        if !inserted && block.block_type == pcapng::BLOCK_TYPE_SHB {
            pending_section_endian = Some(block.endian);
        } else if !inserted
            && pending_section_endian.is_some()
            && block.block_type == pcapng::BLOCK_TYPE_IDB
        {
            let endian = block.endian;
            if !v4_packed.is_empty() || !v6_packed.is_empty() {
                let nrb = pcapng::build_nrb_block(endian, &v4_packed, &v6_packed);
                pcapng::write_block(&mut out, &nrb)?;
            }
            if let Some(keylog) = keylog_blob {
                let dsb = pcapng::build_dsb_block(endian, pcapng::DSB_SECRETS_TYPE_TLS, keylog);
                pcapng::write_block(&mut out, &dsb)?;
            }
            inserted = true;
        }
    }

    if !inserted && (!v4_packed.is_empty() || !v6_packed.is_empty() || keylog_blob.is_some()) {
        let endian = pending_section_endian.unwrap_or(Endian::Little);
        if !v4_packed.is_empty() || !v6_packed.is_empty() {
            let nrb = pcapng::build_nrb_block(endian, &v4_packed, &v6_packed);
            pcapng::write_block(&mut out, &nrb)?;
        }
        if let Some(keylog) = keylog_blob {
            let dsb = pcapng::build_dsb_block(endian, pcapng::DSB_SECRETS_TYPE_TLS, keylog);
            pcapng::write_block(&mut out, &dsb)?;
        }
    }

    let names_total: usize = v4_packed.iter().map(|(_, n)| n.len()).sum::<usize>()
        + v6_packed.iter().map(|(_, n)| n.len()).sum::<usize>();

    let result = EnrichResult {
        ipv4_records: v4_packed.len(),
        ipv6_records: v6_packed.len(),
        names_total,
        keylog_bytes: keylog_blob.map_or(0, <[u8]>::len),
        converted_from_libpcap: false,
        observed_ipv4: observed_v4.as_ref().map_or(0, Vec::len),
        observed_ipv6: observed_v6.as_ref().map_or(0, Vec::len),
    };
    Ok((out, result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_bigip_conf;

    #[test]
    fn slug_normalises_like_python() {
        assert_eq!(slug("/Common/My_VS"), "common-my-vs");
        assert_eq!(slug("10.0.0.1"), "10-0-0-1");
        assert_eq!(slug("--a--b--"), "a--b");
        assert_eq!(slug(""), "");
    }

    #[test]
    fn label_joins_purpose_and_parts() {
        assert_eq!(label("vs", &["/Common/www_vs"]), "vs-common-www-vs");
        assert_eq!(label("pool", &["/Common/web1:80"]), "pool-common-web1-80");
        assert_eq!(label("self", &["", "x"]), "self-x");
    }

    #[test]
    fn split_destination_handles_named_and_dotted_ports() {
        assert_eq!(split_destination("/Common/10.0.0.1:https"), "10.0.0.1");
        assert_eq!(split_destination("/Common/2001:db8::1.443"), "2001:db8::1");
        assert_eq!(split_destination("10.0.0.5"), "10.0.0.5");
        assert_eq!(split_destination("not-an-ip:80"), "");
    }

    #[test]
    fn name_index_dedupes_and_looks_up_subnets() {
        let mut idx = NameIndex::default();
        idx.add("10.0.0.1", "vs-a");
        idx.add("10.0.0.1", "vs-a"); // dup, ignored
        idx.add("10.0.0.1", "irule-b");
        let net = IpNetwork::parse("10.0.1.0", 24).unwrap();
        idx.add_subnet(net, "net-x");
        assert_eq!(idx.lookup("10.0.0.1"), vec!["vs-a", "irule-b"]);
        assert_eq!(idx.lookup("10.0.1.99"), vec!["net-x"]);
        assert_eq!(idx.total_names(), 3);
    }

    #[test]
    fn merged_index_resolves_gtm_wideip_chain() {
        let src = "\
gtm server /Common/dc1 {
    virtual-servers {
        vs1 { destination 10.0.0.9:443 }
    }
}
gtm pool a /Common/gp { members { /Common/dc1:vs1 } }
gtm wideip a /Common/w.example.com { pools { /Common/gp } }
";
        let cfg = parse_bigip_conf(src, "Common");
        let idx = build_merged_name_index(&[(cfg, src.to_owned())]);
        assert_eq!(idx.lookup("10.0.0.9"), vec!["wideip-common-w-example-com"]);
    }
}
