//! Generate a Wireshark profile directory from a BIG-IP config.
//!
//! Powers `f5 enrich-wireshark`. A pure text generator: walk one or more
//! `(config, source)` pairs, derive `hosts` / `subnets` / `vlans` /
//! `dfilters` / `services` / `ethers` / `colorfilters` / `preferences`
//! files plus a `README.md`, and render each.

use std::net::IpAddr;

use crate::model::{ModelObject, ProfileType};
use crate::parser::BigipConfig;
use crate::parser::helpers::{extract_blocks, parse_properties};
use crate::pcap_enrich::{
    NameIndex, build_merged_name_index, extract_self_ips, label, parse_named_subblocks,
    resolve_port, split_destination,
};

/// One coloring rule with 16-bit RGB foreground/background.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColorRule {
    /// Rule name (shown in View → Coloring Rules).
    pub name: String,
    /// Display-filter expression.
    pub filter: String,
    /// Background RGB (16-bit channels).
    pub bg: (u32, u32, u32),
    /// Foreground RGB (16-bit channels).
    pub fg: (u32, u32, u32),
    /// When true, written with a leading `!` (disabled).
    pub disabled: bool,
}

impl ColorRule {
    fn new(name: &str, filter: &str, bg: (u32, u32, u32)) -> Self {
        Self {
            name: name.to_owned(),
            filter: filter.to_owned(),
            bg,
            fg: (0, 0, 0),
            disabled: false,
        }
    }
}

/// In-memory model of every file the profile directory will contain.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WiresharkProfile {
    /// `(addr, name)` rows for `hosts`.
    pub hosts: Vec<(String, String)>,
    /// `(cidr, name)` rows for `subnets`.
    pub subnets: Vec<(String, String)>,
    /// `(tag, name)` rows for `vlans`.
    pub vlans: Vec<(i64, String)>,
    /// `(label, filter)` rows for `dfilters`.
    pub dfilters: Vec<(String, String)>,
    /// `(name, port, proto)` rows for `services`.
    pub services: Vec<(String, i64, String)>,
    /// `(mac, name)` rows for `ethers`.
    pub ethers: Vec<(String, String)>,
    /// Coloring rules for `colorfilters`.
    pub colorfilters: Vec<ColorRule>,
    /// Verbatim `preferences` lines.
    pub preferences: Vec<String>,
}

impl WiresharkProfile {
    /// Drop duplicate rows while preserving insertion order. Color rules dedupe
    /// by name.
    fn deduplicate(&mut self) {
        dedup_preserve(&mut self.hosts);
        dedup_preserve(&mut self.subnets);
        dedup_preserve(&mut self.vlans);
        dedup_preserve(&mut self.dfilters);
        dedup_preserve(&mut self.services);
        dedup_preserve(&mut self.ethers);
        let mut seen: Vec<String> = Vec::new();
        let mut unique = Vec::new();
        for rule in self.colorfilters.drain(..) {
            if seen.contains(&rule.name) {
                continue;
            }
            seen.push(rule.name.clone());
            unique.push(rule);
        }
        self.colorfilters = unique;
        dedup_preserve(&mut self.preferences);
    }

    /// Render the profile directory's files as `(filename, content)` pairs in
    /// the same order `WiresharkProfile.write_to` writes them, including the
    /// trailing `README.md`.
    #[must_use]
    pub fn render_files(&self, profile_name: &str) -> Vec<(String, String)> {
        let mut out: Vec<(String, String)> = Vec::new();

        if !self.hosts.is_empty() {
            out.push(("hosts".to_owned(), format_hosts(&self.hosts)));
        }
        if !self.subnets.is_empty() {
            out.push(("subnets".to_owned(), format_subnets(&self.subnets)));
        }
        if !self.vlans.is_empty() {
            out.push(("vlans".to_owned(), format_vlans(&self.vlans)));
        }
        if !self.dfilters.is_empty() {
            out.push(("dfilters".to_owned(), format_dfilters(&self.dfilters)));
        }
        if !self.services.is_empty() {
            out.push(("services".to_owned(), format_services(&self.services)));
        }
        if !self.ethers.is_empty() {
            out.push(("ethers".to_owned(), format_ethers(&self.ethers)));
        }
        if !self.colorfilters.is_empty() {
            out.push((
                "colorfilters".to_owned(),
                format_colorfilters(&self.colorfilters),
            ));
        }
        if !self.preferences.is_empty() {
            out.push((
                "preferences".to_owned(),
                format_preferences(&self.preferences),
            ));
        }

        let files: Vec<String> = out.iter().map(|(n, _)| n.clone()).collect();
        let readme = format_readme(profile_name, &files);
        out.push(("README.md".to_owned(), readme));
        out
    }
}

fn dedup_preserve<T: Clone + PartialEq>(items: &mut Vec<T>) {
    let mut seen: Vec<T> = Vec::new();
    for it in items.iter() {
        if !seen.contains(it) {
            seen.push(it.clone());
        }
    }
    *items = seen;
}

// ── Formatters ──────────────────────────────────────────────────────

fn format_hosts(entries: &[(String, String)]) -> String {
    let mut lines =
        vec!["# Generated by f5 enrich-wireshark — IP -> BIG-IP object label".to_owned()];
    for (addr, name) in entries {
        lines.push(format!("{addr}\t{name}"));
    }
    lines.join("\n") + "\n"
}

fn format_subnets(entries: &[(String, String)]) -> String {
    let mut lines = vec![
        "# Generated by f5 enrich-wireshark — CIDR -> BIG-IP self-IP label".to_owned(),
        "# Hosts inside these networks pick up the corresponding name.".to_owned(),
    ];
    for (cidr, name) in entries {
        lines.push(format!("{cidr}\t{name}"));
    }
    lines.join("\n") + "\n"
}

fn format_vlans(entries: &[(i64, String)]) -> String {
    let mut lines =
        vec!["# Generated by f5 enrich-wireshark — VLAN tag -> BIG-IP vlan name".to_owned()];
    for (tag, name) in entries {
        lines.push(format!("{tag}\t{name}"));
    }
    lines.join("\n") + "\n"
}

fn format_dfilters(entries: &[(String, String)]) -> String {
    let mut lines = vec!["# Generated by f5 enrich-wireshark — saved display filters".to_owned()];
    for (label, expr) in entries {
        let safe_label = label.replace('"', "'");
        lines.push(format!("\"{safe_label}\" {expr}"));
    }
    lines.join("\n") + "\n"
}

fn format_services(entries: &[(String, i64, String)]) -> String {
    let mut lines = vec![
        "# Generated by f5 enrich-wireshark — well-known BIG-IP service ports".to_owned(),
        "# Static F5 control-plane mapping; safe to edit/extend by hand.".to_owned(),
    ];
    for (name, port, proto) in entries {
        lines.push(format!("{name}\t{port}/{proto}"));
    }
    lines.join("\n") + "\n"
}

fn format_ethers(entries: &[(String, String)]) -> String {
    let mut lines =
        vec!["# Generated by f5 enrich-wireshark — MAC -> BIG-IP object label".to_owned()];
    for (mac, name) in entries {
        lines.push(format!("{mac}\t{name}"));
    }
    lines.join("\n") + "\n"
}

fn format_colorfilters(rules: &[ColorRule]) -> String {
    let mut lines = vec![
        "# Generated by f5 enrich-wireshark — front-end / back-end / role coloring".to_owned(),
        "# Rules are applied top-down and the first match wins.  Order is".to_owned(),
        "# server-side -> client-side -> SNAT -> wide-IP -> self so the data-plane".to_owned(),
        "# proxy-side labels beat the administrative ones (a typical BIG-IP -> pool".to_owned(),
        "# packet has self-IP src + pool-member dst — server-side is the actionable".to_owned(),
        "# colour, not self).".to_owned(),
    ];
    for rule in rules {
        let prefix = if rule.disabled { "!" } else { "" };
        let bg = format!("{},{},{}", rule.bg.0, rule.bg.1, rule.bg.2);
        let fg = format!("{},{},{}", rule.fg.0, rule.fg.1, rule.fg.2);
        lines.push(format!(
            "{prefix}@{}@{}@[{bg}][{fg}]",
            rule.name, rule.filter
        ));
    }
    lines.join("\n") + "\n"
}

fn format_preferences(lines_in: &[String]) -> String {
    let mut out = vec!["# Generated by f5 enrich-wireshark — preferences".to_owned()];
    out.extend(lines_in.iter().cloned());
    out.join("\n") + "\n"
}

fn format_readme(profile_name: &str, files: &[String]) -> String {
    let file_list = files
        .iter()
        .map(|f| format!("- `{f}`"))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "# `{profile_name}` Wireshark profile\n\nGenerated by `f5 enrich-wireshark`.  \
Drop this directory into your Wireshark profiles folder and select it from \
**Edit → Configuration Profiles**.\n\n## Install\n\n- **Linux/macOS:** copy to \
`~/.config/wireshark/profiles/{profile_name}/`\n- **Windows:** copy to \
`%APPDATA%\\Wireshark\\profiles\\{profile_name}\\`\n\n## Files\n\n{file_list}\n"
    )
}

// ── Source-text extractors ──────────────────────────────────────────

fn property(props: &[(String, String)], key: &str) -> Option<String> {
    props.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
}

/// Pull `net arp` static-ARP entries: `[(mac, ip, full_path), …]`.
fn extract_arp_entries(source: &str) -> Vec<(String, String, String)> {
    let mut entries = Vec::new();
    for block in extract_blocks(source) {
        let parts: Vec<&str> = block.header.split_whitespace().collect();
        if parts.len() < 3 || parts[0] != "net" || parts[1] != "arp" {
            continue;
        }
        let full_path = parts[2].to_owned();
        let props = parse_properties(&block.body);
        let ip_value = property(&props, "ip-address");
        let mac_value = property(&props, "mac-address");
        let (Some(ip_value), Some(mac_value)) = (ip_value, mac_value) else {
            continue;
        };
        if ip_value.is_empty() || mac_value.is_empty() {
            continue;
        }
        let ip_str = ip_value
            .split_once('%')
            .map_or(ip_value.as_str(), |(a, _)| a);
        entries.push((mac_value.to_lowercase(), ip_str.to_owned(), full_path));
    }
    entries
}

/// Return `[(full_path, tag), …]` for every `net vlan` block.
fn extract_vlans(source: &str) -> Vec<(String, i64)> {
    let mut out = Vec::new();
    for block in extract_blocks(source) {
        let parts: Vec<&str> = block.header.split_whitespace().collect();
        if parts.len() < 3 || parts[0] != "net" || parts[1] != "vlan" {
            continue;
        }
        let full_path = parts[2].to_owned();
        let Some(tag_value) = property(&parse_properties(&block.body), "tag") else {
            continue;
        };
        let Ok(tag) = tag_value.parse::<i64>() else {
            continue;
        };
        out.push((full_path, tag));
    }
    out
}

/// Static BIG-IP control-plane services (`_BIGIP_CONTROL_SERVICES`).
const BIGIP_CONTROL_SERVICES: &[(&str, i64, &str)] = &[
    ("f5-iquery", 4353, "tcp"),
    ("f5-iquery", 4353, "udp"),
    ("f5-cmi", 6699, "tcp"),
    ("f5-cmi-conf-replicator", 6700, "tcp"),
    ("f5-cmi-config", 6701, "tcp"),
    ("f5-cmi-mcpd", 6702, "tcp"),
    ("f5-cmi-statedb", 6703, "tcp"),
    ("f5-icontrol-rest", 8443, "tcp"),
    ("f5-tmsh", 22, "tcp"),
    ("f5-snmp", 161, "udp"),
    ("f5-snmp-trap", 162, "udp"),
];

// ── BigipConfig accessors ───────────────────────────────────────────

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

fn profile_types_for_virtual(
    cfg: &BigipConfig,
    vs: &crate::model::BigipVirtualServer,
) -> Vec<ProfileType> {
    let mut out = Vec::new();
    for pref in vs.profiles.paths() {
        if let Some(resolved) = resolve_name(cfg, &pref, "profiles")
            && let Some(ModelObject::Profile(p)) = find_object(cfg, "profiles", &resolved)
            && !out.contains(&p.profile_type)
        {
            out.push(p.profile_type);
        }
    }
    out
}

// ── Index → file rows ───────────────────────────────────────────────

fn hosts_from_index(index: &NameIndex) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    for (addr, names) in &index.v4 {
        for name in names {
            pairs.push((addr.clone(), name.clone()));
        }
    }
    for (addr, names) in &index.v6 {
        for name in names {
            pairs.push((addr.clone(), name.clone()));
        }
    }
    pairs
}

fn subnets_from_index(index: &NameIndex) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    for (net, name) in &index.v4_subnets {
        pairs.push((net.text().to_owned(), name.clone()));
    }
    for (net, name) in &index.v6_subnets {
        pairs.push((net.text().to_owned(), name.clone()));
    }
    pairs
}

fn dfilters_for_virtual_servers(config: &BigipConfig) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (full_path, obj) in typed_objects(config, "virtual_servers") {
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
        let Ok(ip) = addr.parse::<IpAddr>() else {
            continue;
        };
        let field_name = if ip.is_ipv6() { "ipv6.addr" } else { "ip.addr" };
        out.push((label("vs", &[full_path]), format!("{field_name} == {addr}")));
    }
    out
}

fn dfilters_for_self_ips(source: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for self_ip in extract_self_ips(source) {
        let Some(net) = self_ip.network else {
            continue;
        };
        let field_name = if net.is_v4() { "ip.addr" } else { "ipv6.addr" };
        out.push((
            label("net", &[&self_ip.full_path]),
            format!("{field_name} == {}", net.text()),
        ));
    }
    out
}

/// Extract `(port, "tcp")` from a BIG-IP destination string.
fn vs_port_proto(dest: &str) -> Option<(i64, &'static str)> {
    if dest.is_empty() {
        return None;
    }
    let base = dest.rsplit('/').next().unwrap_or(dest);
    for sep in [':', '.'] {
        if let Some(idx) = base.rfind(sep) {
            let port_str = &base[idx + sep.len_utf8()..];
            if let Some(port) = resolve_port(port_str)
                && 0 < port
                && port < 65536
            {
                return Some((port, "tcp"));
            }
        }
    }
    None
}

fn virtual_server_protocol(config: &BigipConfig, vs: &crate::model::BigipVirtualServer) -> String {
    let types = profile_types_for_virtual(config, vs);
    if types.contains(&ProfileType::Udp) {
        "udp".to_owned()
    } else {
        "tcp".to_owned()
    }
}

fn services_from_virtuals(config: &BigipConfig) -> Vec<(String, i64, String)> {
    let mut out = Vec::new();
    for (full_path, obj) in typed_objects(config, "virtual_servers") {
        let ModelObject::VirtualServer(vs) = obj else {
            continue;
        };
        let dest_text = vs
            .destination
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_default();
        let Some((port, _default_proto)) = vs_port_proto(&dest_text) else {
            continue;
        };
        let proto = virtual_server_protocol(config, vs);
        out.push((label("vs", &[full_path]), port, proto));
    }
    out
}

/// Match a UDP-hint token in a monitor type:
/// `\b(udp|dns|radius|sip-udp|snmp|wmi)\b`.
fn monitor_is_udp(monitor_type: &str) -> bool {
    const HINTS: &[&str] = &["udp", "dns", "radius", "sip-udp", "snmp", "wmi"];
    let lower = monitor_type.to_lowercase();
    HINTS.iter().any(|hint| contains_word(&lower, hint))
}

/// True if `needle` appears in `haystack` delimited by `\b` word boundaries
/// (word chars are `[A-Za-z0-9_]`). The hints are all word-char-bounded, so a
/// match needs a non-word char (or string edge) on each side.
fn contains_word(haystack: &str, needle: &str) -> bool {
    let hb = haystack.as_bytes();
    let nb = needle.as_bytes();
    if nb.is_empty() {
        return false;
    }
    let is_word = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    let mut i = 0;
    while i + nb.len() <= hb.len() {
        if &hb[i..i + nb.len()] == nb {
            let after = i + nb.len();
            let start_ok = !is_word(nb[0]) || i == 0 || !is_word(hb[i - 1]);
            let end_ok = !is_word(nb[nb.len() - 1]) || after == hb.len() || !is_word(hb[after]);
            if start_ok && end_ok {
                return true;
            }
        }
        i += 1;
    }
    false
}

fn services_from_monitors(source: &str) -> Vec<(String, i64, String)> {
    let mut out = Vec::new();
    for block in extract_blocks(source) {
        let parts: Vec<&str> = block.header.split_whitespace().collect();
        if parts.len() < 4 || !matches!(parts[0], "ltm" | "gtm") || parts[1] != "monitor" {
            continue;
        }
        let monitor_type = parts[2..parts.len() - 1].join(" ");
        let full_path = parts[parts.len() - 1].to_owned();
        // `^\s*destination\s+(\S+)\s*$` (MULTILINE).
        let Some(dest) = find_destination_line(&block.body) else {
            continue;
        };
        let Some((port, _)) = vs_port_proto(&dest) else {
            continue;
        };
        let proto = if monitor_is_udp(&monitor_type) {
            "udp"
        } else {
            "tcp"
        };
        out.push((label("monitor", &[&full_path]), port, proto.to_owned()));
    }
    out
}

/// Find a line `^\s*destination\s+(\S+)\s*$` and return the single token.
fn find_destination_line(body: &str) -> Option<String> {
    for line in body.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("destination") {
            let rest = rest.trim_start();
            // The original line must be `destination<ws><token>` with only that
            // token (the regex requires a single \S+ and trailing whitespace).
            if rest.len() < trimmed.len() && !rest.is_empty() {
                let mut it = rest.split_whitespace();
                if let Some(tok) = it.next()
                    && it.next().is_none()
                {
                    return Some(tok.to_owned());
                }
            }
        }
    }
    None
}

fn services_from_self_allow(source: &str) -> Vec<(String, i64, String)> {
    let mut out = Vec::new();
    for block in extract_blocks(source) {
        let parts: Vec<&str> = block.header.split_whitespace().collect();
        if parts.len() < 3 || parts[0] != "net" || parts[1] != "self" {
            continue;
        }
        let full_path = parts[2].to_owned();
        let Some(inner) = extract_braced(&block.body, "allow-service") else {
            continue;
        };
        for (proto, port) in allow_service_tokens(&inner) {
            if 0 < port && port < 65536 {
                out.push((label("self", &[&full_path]), port, proto));
            }
        }
    }
    out
}

/// Find `allow-service { ... }` and return the inner text (DOTALL, non-greedy
/// to the first `}`).
fn extract_braced(body: &str, keyword: &str) -> Option<String> {
    let idx = body.find(keyword)?;
    let after = &body[idx + keyword.len()..];
    let after = after.trim_start_matches([' ', '\t', '\n', '\r']);
    let after = after.strip_prefix('{')?;
    let end = after.find('}')?;
    Some(after[..end].to_owned())
}

/// `\b(tcp|udp|sctp):(\d+)\b` over `text`, in left-to-right match order.
/// Returns `(proto_lower, port)` — matching `re.findall`'s document order with
/// non-overlapping, leftmost matches.
fn allow_service_tokens(text: &str) -> Vec<(String, i64)> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let is_word = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    let mut i = 0;
    while i < bytes.len() {
        let mut matched_len = 0usize;
        for proto in ["tcp", "udp", "sctp"] {
            let pb = proto.as_bytes();
            if i + pb.len() < bytes.len()
                && &bytes[i..i + pb.len()] == pb
                && bytes[i + pb.len()] == b':'
            {
                let start_ok = i == 0 || !is_word(bytes[i - 1]);
                if !start_ok {
                    continue;
                }
                let dstart = i + pb.len() + 1;
                let mut j = dstart;
                while j < bytes.len() && bytes[j].is_ascii_digit() {
                    j += 1;
                }
                if j > dstart
                    && let Ok(port) = text[dstart..j].parse::<i64>()
                {
                    out.push((proto.to_owned(), port));
                    matched_len = j - i;
                    break;
                }
            }
        }
        i += if matched_len > 0 { matched_len } else { 1 };
    }
    out
}

fn services_from_gtm_servers(source: &str) -> Vec<(String, i64, String)> {
    let mut out = Vec::new();
    for block in extract_blocks(source) {
        let parts: Vec<&str> = block.header.split_whitespace().collect();
        if parts.len() < 3 || parts[0] != "gtm" || parts[1] != "server" {
            continue;
        }
        let server_path = parts[2].to_owned();
        // `virtual-servers\s*\{(.*)\}\s*$` (DOTALL, greedy).
        let Some(vs_inner) = extract_braced_greedy(&block.body, "virtual-servers") else {
            continue;
        };
        for (vs_name, body) in parse_named_subblocks(&format!("{{{vs_inner}}}")) {
            let inner = parse_properties(&body);
            let dest = property(&inner, "destination").unwrap_or_default();
            let Some((port, _)) = vs_port_proto(&dest) else {
                continue;
            };
            out.push((
                label("gtmvs", &[&server_path, &vs_name]),
                port,
                "tcp".to_owned(),
            ));
        }
    }
    out
}

/// Find `keyword\s*\{(.*)\}\s*$` with a greedy inner capture (DOTALL): the
/// inner text spans from the first `{` after `keyword` to the LAST `}` in the
/// body.
fn extract_braced_greedy(body: &str, keyword: &str) -> Option<String> {
    let idx = body.find(keyword)?;
    let after = &body[idx + keyword.len()..];
    let trimmed = after.trim_start_matches([' ', '\t', '\n', '\r']);
    let open_off = after.len() - trimmed.len();
    let rest = trimmed.strip_prefix('{')?;
    // Greedy: capture to the last `}` in `rest`, with only whitespace after.
    let last_close = rest.rfind('}')?;
    let trailing = rest[last_close + 1..].trim();
    if !trailing.is_empty() {
        // `\s*$` requires only whitespace after the last `}` in the line/string;
        // since `.` matches newlines in DOTALL, this is the end of the body.
        return None;
    }
    let _ = open_off;
    Some(rest[..last_close].to_owned())
}

fn services_from_pool_members(config: &BigipConfig) -> Vec<(String, i64, String)> {
    let mut out = Vec::new();
    for (_path, obj) in typed_objects(config, "pools") {
        let ModelObject::Pool(pool) = obj else {
            continue;
        };
        for item in &pool.members.items {
            let crate::value::ListItemValue::PoolMember(member) = &item.value else {
                continue;
            };
            let Some((port, _)) = vs_port_proto(&member.name) else {
                continue;
            };
            out.push((label("pool", &[&member.name]), port, "tcp".to_owned()));
        }
    }
    out
}

fn services_from_irules(config: &BigipConfig) -> Vec<(String, i64, String)> {
    let mut out = Vec::new();
    for (full_path, obj) in typed_objects(config, "rules") {
        let ModelObject::Rule(rule) = obj else {
            continue;
        };
        let body = &rule.source;
        let mut ports: Vec<i64> = Vec::new();
        ports.extend(irule_host_port(body));
        ports.extend(irule_url_port(body));
        ports.extend(irule_named_port(body));
        let mut valid: Vec<i64> = ports.into_iter().filter(|p| 0 < *p && *p < 65536).collect();
        valid.sort_unstable();
        valid.dedup();
        for port in valid {
            out.push((label("irule", &[full_path]), port, "tcp".to_owned()));
        }
    }
    out
}

fn services_from_ltm_policies(source: &str) -> Vec<(String, i64, String)> {
    let mut out = Vec::new();
    for block in extract_blocks(source) {
        let parts: Vec<&str> = block.header.split_whitespace().collect();
        if parts.len() < 3 || parts[0] != "ltm" || parts[1] != "policy" {
            continue;
        }
        let full_path = parts[2].to_owned();
        let mut valid: Vec<i64> = policy_port_equals(&block.body)
            .into_iter()
            .filter(|p| 0 < *p && *p < 65536)
            .collect();
        valid.sort_unstable();
        valid.dedup();
        for port in valid {
            out.push((label("policy", &[&full_path]), port, "tcp".to_owned()));
        }
    }
    out
}

fn ethers_from_arp(source: &str) -> Vec<(String, String)> {
    extract_arp_entries(source)
        .into_iter()
        .map(|(mac, _ip, full_path)| (mac, label("arp", &[&full_path])))
        .collect()
}

// ── iRule / policy port regex ports ─────────────────────────────────

/// `\b(?:node|connect)\s+\S+\s+(\d{1,5})\b | \bpool\s+\S+\s+member\s+\S+\s+(\d{1,5})\b`.
fn irule_host_port(body: &str) -> Vec<i64> {
    let mut out = Vec::new();
    let toks: Vec<&str> = body.split_whitespace().collect();
    let mut i = 0;
    while i < toks.len() {
        if matches!(toks[i], "node" | "connect")
            && i + 2 < toks.len()
            && let Some(p) = parse_trailing_digits(toks[i + 2])
        {
            out.push(p);
        }
        if toks[i] == "pool"
            && i + 4 < toks.len()
            && toks[i + 2] == "member"
            && let Some(p) = parse_trailing_digits(toks[i + 4])
        {
            out.push(p);
        }
        i += 1;
    }
    out
}

/// `://[^/"\s]+:(\d{1,5})\b`.
fn irule_url_port(body: &str) -> Vec<i64> {
    let mut out = Vec::new();
    let bytes = body.as_bytes();
    let mut i = 0;
    while i + 3 <= bytes.len() {
        if &bytes[i..i + 3] == b"://" {
            // Scan host chars [^/"\s], then need `:` then digits.
            let mut j = i + 3;
            let mut last_colon: Option<usize> = None;
            while j < bytes.len() {
                let b = bytes[j];
                if b == b'/' || b == b'"' || b.is_ascii_whitespace() {
                    break;
                }
                if b == b':' {
                    last_colon = Some(j);
                }
                j += 1;
            }
            if let Some(c) = last_colon {
                let dstart = c + 1;
                let mut k = dstart;
                while k < bytes.len() && k < dstart + 5 && bytes[k].is_ascii_digit() {
                    k += 1;
                }
                if k > dstart
                    && let Ok(p) = body[dstart..k].parse::<i64>()
                {
                    out.push(p);
                }
            }
        }
        i += 1;
    }
    out
}

/// `\bport\s+(\d{1,5})\b`.
fn irule_named_port(body: &str) -> Vec<i64> {
    let mut out = Vec::new();
    let toks: Vec<&str> = body.split_whitespace().collect();
    for w in toks.windows(2) {
        if w[0] == "port"
            && let Some(p) = parse_leading_digits(w[1])
        {
            out.push(p);
        }
    }
    out
}

/// `port[-\s]equals\s+(\d{1,5})\b`.
fn policy_port_equals(body: &str) -> Vec<i64> {
    let mut out = Vec::new();
    let toks: Vec<&str> = body.split_whitespace().collect();
    // Handle the `port equals N` (space) and `port-equals N` token forms.
    let mut i = 0;
    while i < toks.len() {
        if toks[i] == "port-equals" && i + 1 < toks.len() {
            if let Some(p) = parse_leading_digits(toks[i + 1]) {
                out.push(p);
            }
        } else if toks[i] == "port"
            && i + 2 < toks.len()
            && toks[i + 1] == "equals"
            && let Some(p) = parse_leading_digits(toks[i + 2])
        {
            out.push(p);
        }
        i += 1;
    }
    out
}

/// Parse 1–5 leading digits of `tok` as a port (regex `(\d{1,5})\b` anchored at
/// the start of the token after the preceding `\s`).
fn parse_leading_digits(tok: &str) -> Option<i64> {
    let digits: String = tok
        .chars()
        .take_while(char::is_ascii_digit)
        .take(5)
        .collect();
    if digits.is_empty() {
        return None;
    }
    // `\b` after the digit run: the next char (if any) must be a non-word char.
    digits.parse::<i64>().ok()
}

/// Parse 1–5 trailing digits of `tok` matched by `(\d{1,5})\b` at token end.
fn parse_trailing_digits(tok: &str) -> Option<i64> {
    let chars: Vec<char> = tok.chars().collect();
    let mut end = chars.len();
    let mut start = end;
    while start > 0 && chars[start - 1].is_ascii_digit() {
        start -= 1;
    }
    if start == end {
        return None;
    }
    // Keep at most 5 digits from the right (the regex matches up to 5 then \b).
    if end - start > 5 {
        start = end - 5;
    }
    let _ = &mut end;
    let s: String = chars[start..].iter().collect();
    s.parse::<i64>().ok()
}

// ── Color rules + proxy-side filters ────────────────────────────────

fn addresses_for_label_prefix(index: &NameIndex, prefix: &str) -> (Vec<String>, Vec<String>) {
    let needle = format!("{prefix}-");
    let mut v4 = Vec::new();
    let mut v6 = Vec::new();
    for (addr, names) in &index.v4 {
        if names.iter().any(|n| n.starts_with(&needle)) {
            v4.push(addr.clone());
        }
    }
    for (addr, names) in &index.v6 {
        if names.iter().any(|n| n.starts_with(&needle)) {
            v6.push(addr.clone());
        }
    }
    (v4, v6)
}

/// Build a Wireshark `ip.addr in {…}` filter over sorted addresses. Sorting is
/// lexicographic on the string form, matching Python's `sorted(set(...))`.
fn ip_set_filter(addrs_v4: &[String], addrs_v6: &[String]) -> String {
    let mut parts = Vec::new();
    if !addrs_v4.is_empty() {
        let mut v: Vec<String> = dedup_sorted(addrs_v4);
        v.sort();
        parts.push(format!("ip.addr in {{{}}}", v.join(", ")));
    }
    if !addrs_v6.is_empty() {
        let mut v: Vec<String> = dedup_sorted(addrs_v6);
        v.sort();
        parts.push(format!("ipv6.addr in {{{}}}", v.join(", ")));
    }
    parts.join(" or ")
}

fn dedup_sorted(items: &[String]) -> Vec<String> {
    let mut v: Vec<String> = items.to_vec();
    v.sort();
    v.dedup();
    v
}

fn union(a: &[String], b: &[String]) -> Vec<String> {
    let mut v = a.to_vec();
    for x in b {
        if !v.contains(x) {
            v.push(x.clone());
        }
    }
    v
}

const COLOR_CLIENT_SIDE: (u32, u32, u32) = (49152, 57344, 65535);
const COLOR_SERVER_SIDE: (u32, u32, u32) = (65535, 57344, 49152);
const COLOR_SELF: (u32, u32, u32) = (49152, 65535, 49152);
const COLOR_SNAT: (u32, u32, u32) = (65535, 52428, 58982);
const COLOR_WIDEIP: (u32, u32, u32) = (65535, 65535, 49152);

fn build_proxy_side_color_rules(index: &NameIndex) -> Vec<ColorRule> {
    let mut rules = Vec::new();

    let (pool_v4, pool_v6) = addresses_for_label_prefix(index, "pool");
    let (node_v4, node_v6) = addresses_for_label_prefix(index, "node");
    let server_filter = ip_set_filter(&union(&pool_v4, &node_v4), &union(&pool_v6, &node_v6));
    if !server_filter.is_empty() {
        rules.push(ColorRule::new(
            "BIG-IP server side",
            &server_filter,
            COLOR_SERVER_SIDE,
        ));
    }

    let (vs_v4, vs_v6) = addresses_for_label_prefix(index, "vs");
    let client_filter = ip_set_filter(&vs_v4, &vs_v6);
    if !client_filter.is_empty() {
        rules.push(ColorRule::new(
            "BIG-IP client side",
            &client_filter,
            COLOR_CLIENT_SIDE,
        ));
    }

    for (prefix, lbl, bg) in [
        ("snat", "BIG-IP SNAT", COLOR_SNAT),
        ("wideip", "BIG-IP wide-IP", COLOR_WIDEIP),
        ("self", "BIG-IP self IP", COLOR_SELF),
    ] {
        let (v4, v6) = addresses_for_label_prefix(index, prefix);
        let flt = ip_set_filter(&v4, &v6);
        if !flt.is_empty() {
            rules.push(ColorRule::new(lbl, &flt, bg));
        }
    }
    rules
}

fn build_proxy_side_dfilters(index: &NameIndex) -> Vec<(String, String)> {
    let mut buttons = Vec::new();
    let (pool_v4, pool_v6) = addresses_for_label_prefix(index, "pool");
    let (node_v4, node_v6) = addresses_for_label_prefix(index, "node");
    let server_filter = ip_set_filter(&union(&pool_v4, &node_v4), &union(&pool_v6, &node_v6));
    if !server_filter.is_empty() {
        buttons.push(("BIG-IP server side".to_owned(), server_filter));
    }
    let (vs_v4, vs_v6) = addresses_for_label_prefix(index, "vs");
    let client_filter = ip_set_filter(&vs_v4, &vs_v6);
    if !client_filter.is_empty() {
        buttons.push(("BIG-IP client side".to_owned(), client_filter));
    }
    buttons
}

fn build_preferences() -> Vec<String> {
    let column_format = concat!(
        "\"No.\",\"%m\",",
        "\"Time\",\"%t\",",
        "\"Source\",\"%s\",",
        "\"Destination\",\"%d\",",
        "\"Proxy Side\",\"%Cus:frame.coloring_rule.name:0:R\",",
        "\"Protocol\",\"%p\",",
        "\"Length\",\"%L\",",
        "\"Info\",\"%i\""
    );
    vec![format!("gui.column.format: {column_format}")]
}

// ── Builder ─────────────────────────────────────────────────────────

/// Assemble a [`WiresharkProfile`] from one or more `(config, source)` pairs.
#[must_use]
pub fn build_wireshark_profile(configs_with_sources: &[(BigipConfig, String)]) -> WiresharkProfile {
    if configs_with_sources.is_empty() {
        return WiresharkProfile::default();
    }

    let combined_source = configs_with_sources
        .iter()
        .map(|(_, s)| s.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let index = build_merged_name_index(configs_with_sources);

    let mut profile = WiresharkProfile {
        hosts: hosts_from_index(&index),
        subnets: subnets_from_index(&index),
        vlans: extract_vlans(&combined_source)
            .into_iter()
            .map(|(full_path, tag)| (tag, label("vlan", &[&full_path])))
            .collect(),
        dfilters: {
            let mut d = build_proxy_side_dfilters(&index);
            d.extend(dfilters_for_self_ips(&combined_source));
            d
        },
        services: BIGIP_CONTROL_SERVICES
            .iter()
            .map(|(n, p, pr)| ((*n).to_owned(), *p, (*pr).to_owned()))
            .collect(),
        ethers: ethers_from_arp(&combined_source),
        colorfilters: build_proxy_side_color_rules(&index),
        preferences: build_preferences(),
    };

    for (config, _src) in configs_with_sources {
        profile
            .dfilters
            .extend(dfilters_for_virtual_servers(config));
        profile.services.extend(services_from_virtuals(config));
        profile.services.extend(services_from_pool_members(config));
        profile.services.extend(services_from_irules(config));
    }

    profile
        .services
        .extend(services_from_monitors(&combined_source));
    profile
        .services
        .extend(services_from_self_allow(&combined_source));
    profile
        .services
        .extend(services_from_gtm_servers(&combined_source));
    profile
        .services
        .extend(services_from_ltm_policies(&combined_source));

    profile.deduplicate();
    profile
}
