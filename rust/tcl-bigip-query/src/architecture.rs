// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Multi-device *architecture*: how the loaded configs relate as tiers.
//!
//! A report can be built from several configs at once — a GTM in front of a
//! tier-1 LTM in front of a tier-2 LTM, for example. Individually each config
//! is just a flat estate; the value of loading them together is knowing **what
//! points to what**. This module derives that relationship two ways, and merges
//! them:
//!
//! * **auto-detection** — the default. A device *fronts* another when one of its
//!   outbound targets (an LTM pool member address, or a GTM pool member / server
//!   address) is served by a virtual server (or listener) on the other device.
//!   That IP overlap is the physical evidence of a tier hop, so it needs no
//!   configuration to find.
//! * **a manifest** — an optional user-supplied *Tcl script* (parsed with the
//!   project's own Tcl tokeniser) that names each device's role and tier and can
//!   declare links explicitly. It overrides and augments auto-detection (a
//!   manifest link auto-detection missed is still added; a device's manifest
//!   tier/role wins over the inferred one).
//!
//! The result is an `architecture` object embedded in the report model: the
//! ordered devices with their resolved role/tier, the inter-device links with
//! the object pairs that evidence them, and a ready-to-render Mermaid diagram.

use std::collections::BTreeMap;
use std::net::IpAddr;
use std::str::FromStr;

use serde_json::{Map, Value as J};

/// Borrow a string field from a JSON object (empty when absent / non-string).
fn bstr<'a>(map: &'a Map<String, J>, key: &str) -> &'a str {
    map.get(key).and_then(J::as_str).unwrap_or("")
}

/// Borrow an array field from a JSON object (empty when absent / non-array).
fn barr<'a>(map: &'a Map<String, J>, key: &str) -> &'a [J] {
    match map.get(key) {
        Some(J::Array(a)) => a.as_slice(),
        _ => &[],
    }
}

/// One device's declared role/tier/label from a manifest entry.
struct DeviceSpec {
    matcher: String,
    role: Option<String>,
    tier: Option<i64>,
    label: Option<String>,
}

/// An explicit `from -> to` link declared in a manifest.
struct LinkSpec {
    from: String,
    to: String,
    label: Option<String>,
}

/// A named network zone (`zone dmz -cidr 192.0.2.0/24 …`) — a set of IP ranges.
struct ZoneSpec {
    name: String,
    cidrs: Vec<String>,
}

/// A device interface attached to a zone (`interface edge.ucs ext0 -zone external
/// -address 203.0.113.10`).
struct InterfaceSpec {
    device: String,
    name: String,
    zone: Option<String>,
    address: Option<String>,
}

/// A DNS zone declaration (`dns-zone example.com -file z.zone -zone dmz`). The
/// `file` is supplied separately (uploaded / side-input); the DSL just wires it.
struct DnsZoneSpec {
    name: String,
    file: Option<String>,
    zone: Option<String>,
}

/// A `cidr-name <cidr> <label>` enrichment entry.
struct CidrName {
    cidr: String,
    label: String,
}

/// A parsed manifest: the estate topology + enrichment declarations.
struct Manifest {
    devices: Vec<DeviceSpec>,
    links: Vec<LinkSpec>,
    zones: Vec<ZoneSpec>,
    interfaces: Vec<InterfaceSpec>,
    dns_zones: Vec<DnsZoneSpec>,
    cidr_names: Vec<CidrName>,
    /// Optional CSV of `port,name` service overrides (default: the built-in F5 table).
    service_map_file: Option<String>,
    /// Optional CSV NAT map file (`source,dest[,source_cidr,dest_cidr]`).
    nat_map_file: Option<String>,
}

/// An auto-detected or declared link between two device indices.
struct Link {
    from: usize,
    to: usize,
    /// `"auto"`, `"manifest"`, or `"manifest+auto"` when both agree.
    source: String,
    label: Option<String>,
    /// Object pairs that evidence the hop (empty for a manifest-only link).
    vias: Vec<Via>,
}

/// One piece of evidence for a link: an outbound target on the upstream device
/// that resolves to a served address on the downstream device.
struct Via {
    address: String,
    from_obj: String,
    to_obj: String,
    port: String,
}

/// Normalise an address to a canonical IP string, or `None` when it is not a
/// concrete IP (a hostname, `any`, a route-domain-only token, empty).
///
/// Strips a `%route-domain` suffix and a `/prefix` (or `:port` on IPv4) so a
/// pool member `10.0.0.5%2:443` and a virtual-address `10.0.0.5` compare equal.
fn bare_ip(addr: &str) -> Option<String> {
    let addr = addr.trim();
    if addr.is_empty() {
        return None;
    }
    // Drop a route-domain qualifier first (`10.0.0.5%2` -> `10.0.0.5`).
    let addr = addr.split('%').next().unwrap_or(addr);
    // Drop a CIDR prefix (`10.0.0.0/24` -> `10.0.0.0`).
    let addr = addr.split('/').next().unwrap_or(addr);
    // A direct parse handles bare IPv4 and full IPv6.
    if let Ok(ip) = IpAddr::from_str(addr) {
        return Some(ip.to_string());
    }
    // IPv4 with a trailing `:port` (`10.0.0.5:443`). IPv6 with a port is
    // bracketed (`[::1]:443`) and is handled below.
    if let Some((host, _port)) = addr.rsplit_once(':')
        && !host.contains(':')
        && let Ok(ip) = IpAddr::from_str(host)
    {
        return Some(ip.to_string());
    }
    // Bracketed IPv6 literal, optionally with a port.
    if let Some(rest) = addr.strip_prefix('[') {
        let host = rest.split(']').next().unwrap_or(rest);
        if let Ok(ip) = IpAddr::from_str(host) {
            return Some(ip.to_string());
        }
    }
    // BIG-IP spells an IPv6 destination's port with a `.` (the `:` is taken by
    // the address), e.g. `2001:db8::10.443`. Strip a trailing `.<digits>` from a
    // colon-bearing address and re-parse the host.
    if addr.contains(':')
        && let Some((host, port)) = addr.rsplit_once('.')
        && !port.is_empty()
        && port.bytes().all(|b| b.is_ascii_digit())
        && let Ok(ip) = IpAddr::from_str(host)
    {
        return Some(ip.to_string());
    }
    None
}

/// The set of IP addresses a device *serves* — the concrete listener addresses
/// its virtual servers (and, when present, GTM listeners) answer on. A device
/// serving one of another device's outbound targets is the downstream tier.
fn served_addresses(device: &Map<String, J>) -> BTreeMap<String, String> {
    // address -> the serving object's full path (first wins, for a stable label)
    let mut out: BTreeMap<String, String> = BTreeMap::new();
    for v in barr(device, "virtuals") {
        let Some(vm) = v.as_object() else { continue };
        if let Some(ip) = bare_ip(bstr(vm, "destAddr")) {
            out.entry(ip)
                .or_insert_with(|| bstr(vm, "fullPath").to_string());
        }
    }
    // Virtual-address objects carry the raw served IP as their leaf name.
    for va in barr(device, "virtualAddresses") {
        let Some(am) = va.as_object() else { continue };
        let fp = bstr(am, "fullPath");
        if let Some(ip) = bare_ip(bstr(am, "name")).or_else(|| bare_ip(fp)) {
            out.entry(ip).or_insert_with(|| fp.to_string());
        }
    }
    // GTM listeners answer DNS on their own address; a GTM fronts by resolving a
    // name to a downstream address, but its listener address is itself a served
    // address (an upstream resolver could point at it).
    for l in barr(device, "gtmListeners") {
        let Some(lm) = l.as_object() else { continue };
        if let Some(ip) = bare_ip(bstr(lm, "address")) {
            out.entry(ip)
                .or_insert_with(|| bstr(lm, "fullPath").to_string());
        }
    }
    out
}

/// One outbound target a device points at: `(address, from_obj, port)`.
struct Target {
    address: String,
    from_obj: String,
    port: String,
}

/// The IP addresses a device *reaches out to* — LTM pool member addresses and
/// GTM pool member / server addresses. Each downstream match evidences a link.
fn outbound_targets(device: &Map<String, J>) -> Vec<Target> {
    let mut out: Vec<Target> = Vec::new();
    for p in barr(device, "pools") {
        let Some(pm) = p.as_object() else { continue };
        let pool_fp = bstr(pm, "fullPath").to_string();
        for m in barr(pm, "members") {
            let Some(mm) = m.as_object() else { continue };
            // Prefer the explicit `address`; fall back to the member name, which
            // carries the IP for members declared only by keyed name
            // (`/Common/10.2.0.20:443 { ... }` with no `address` property).
            let ip = bare_ip(bstr(mm, "address")).or_else(|| {
                let name = bstr(mm, "name");
                bare_ip(name.rsplit('/').next().unwrap_or(name))
            });
            if let Some(ip) = ip {
                out.push(Target {
                    address: ip,
                    from_obj: pool_fp.clone(),
                    port: bstr(mm, "port").to_string(),
                });
            }
        }
    }
    // GTM servers list their virtual-server destinations (e.g. `10.2.0.20:443`)
    // — the downstream LTM virtual addresses this GTM balances across. That
    // overlap is the GTM -> LTM tier hop.
    for s in barr(device, "gtmServers") {
        let Some(sm) = s.as_object() else { continue };
        let server_fp = bstr(sm, "fullPath").to_string();
        for a in barr(sm, "virtualServers") {
            if let Some(ip) = a.as_str().and_then(bare_ip) {
                out.push(Target {
                    address: ip,
                    from_obj: server_fp.clone(),
                    port: String::new(),
                });
            }
        }
    }
    // Static GTM pool members carry an explicit target IP (`static-target`).
    for p in barr(device, "gtmPools") {
        let Some(pm) = p.as_object() else { continue };
        let pool_fp = bstr(pm, "fullPath").to_string();
        for m in barr(pm, "members") {
            let Some(mm) = m.as_object() else { continue };
            if let Some(ip) = bare_ip(bstr(mm, "staticTarget")) {
                out.push(Target {
                    address: ip,
                    from_obj: pool_fp.clone(),
                    port: bstr(mm, "servicePort").to_string(),
                });
            }
        }
    }
    out
}

/// Whether a device looks like a GTM/DNS node (has any GTM objects).
fn looks_like_gtm(device: &Map<String, J>) -> bool {
    ["gtmWideips", "gtmPools", "gtmServers", "gtmListeners"]
        .iter()
        .any(|k| !barr(device, k).is_empty())
}

/// Resolve a manifest matcher string to a device index.
///
/// Tries, in order: exact URI, exact name, URI substring, name substring, and
/// finally the URI's file-name stem. Returns `None` when nothing matches.
fn resolve_device(matcher: &str, devices: &[J]) -> Option<usize> {
    let m = matcher.trim();
    if m.is_empty() {
        return None;
    }
    let field = |d: &J, k: &str| d.get(k).and_then(J::as_str).unwrap_or("").to_string();
    // exact uri / name
    if let Some(i) = devices
        .iter()
        .position(|d| field(d, "uri") == m || field(d, "name") == m)
    {
        return Some(i);
    }
    // substring on uri / name
    if let Some(i) = devices
        .iter()
        .position(|d| field(d, "uri").contains(m) || field(d, "name").contains(m))
    {
        return Some(i);
    }
    // file-name stem of the uri (`/path/lab-device-01.ucs` -> `lab-device-01`)
    devices.iter().position(|d| {
        let uri = field(d, "uri");
        let stem = uri
            .rsplit('/')
            .next()
            .and_then(|f| f.rsplit_once('.').map_or(Some(f), |(s, _)| Some(s)))
            .unwrap_or("");
        !stem.is_empty() && stem == m
    })
}

/// Parse the architecture manifest — a small Tcl script — with the project's
/// own Tcl tokeniser. Recognised commands are `device <match> [-role R] [-tier
/// N] [-label L]` and `link <from> <to> [-label L]`; unknown commands and flags
/// are skipped leniently. Unbalanced braces / quotes make the tokeniser fail,
/// surfaced as `Err(message)` so the report can show it without aborting.
///
/// ```tcl
/// # one line per device; options are Tcl-style flags
/// device gtm.ucs  -role gtm -tier 0 -label "DNS Edge"
/// device edge.ucs -role ltm -tier 1
/// device core.ucs -role ltm -tier 2
///
/// # explicit links (auto-detection still runs and is merged in)
/// link gtm.ucs  edge.ucs
/// link edge.ucs core.ucs -label internal
/// ```
/// Collect every value for a repeated `-flag` (e.g. `zone dmz -cidr a -cidr b`).
fn collect_flag(words: &[String], flag: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0;
    while i + 1 < words.len() {
        if words[i] == flag {
            out.push(words[i + 1].clone());
        }
        i += 2;
    }
    out
}

/// First value for a `-flag`, if present.
fn first_flag(words: &[String], flag: &str) -> Option<String> {
    let mut i = 0;
    while i + 1 < words.len() {
        if words[i] == flag {
            return Some(words[i + 1].clone());
        }
        i += 2;
    }
    None
}

/// Read `-flag value` pairs from a word slice into (role, tier, label).
fn read_opts(words: &[String]) -> (Option<String>, Option<i64>, Option<String>) {
    let (mut role, mut tier, mut label) = (None, None, None);
    let mut i = 0;
    while i + 1 < words.len() {
        let (flag, val) = (words[i].as_str(), words[i + 1].clone());
        match flag {
            "-role" => role = Some(val),
            "-tier" => tier = val.parse::<i64>().ok(),
            "-label" | "-name" => label = Some(val),
            _ => {}
        }
        i += 2;
    }
    (role, tier, label)
}

fn parse_manifest(text: &str) -> Result<Manifest, String> {
    let commands = tcl_commands(text)?;
    let mut devices = Vec::new();
    let mut links = Vec::new();
    let mut zones = Vec::new();
    let mut interfaces = Vec::new();
    let mut dns_zones = Vec::new();
    let mut cidr_names = Vec::new();
    let mut service_map_file = None;
    let mut nat_map_file = None;

    for cmd in commands {
        match cmd.first().map(String::as_str) {
            Some("device") if cmd.len() >= 2 => {
                let (role, tier, label) = read_opts(&cmd[2..]);
                devices.push(DeviceSpec {
                    matcher: cmd[1].clone(),
                    role,
                    tier,
                    label,
                });
            }
            Some("link") if cmd.len() >= 3 => {
                let (_r, _t, label) = read_opts(&cmd[3..]);
                links.push(LinkSpec {
                    from: cmd[1].clone(),
                    to: cmd[2].clone(),
                    label,
                });
            }
            // zone <name> -cidr A -cidr B …
            Some("zone") if cmd.len() >= 2 => {
                zones.push(ZoneSpec {
                    name: cmd[1].clone(),
                    cidrs: collect_flag(&cmd[2..], "-cidr"),
                });
            }
            // interface <device> <ifname> [-zone Z] [-address A]
            Some("interface") if cmd.len() >= 3 => {
                interfaces.push(InterfaceSpec {
                    device: cmd[1].clone(),
                    name: cmd[2].clone(),
                    zone: first_flag(&cmd[3..], "-zone"),
                    address: first_flag(&cmd[3..], "-address"),
                });
            }
            // dns-zone <name> [-file F] [-zone Z]
            Some("dns-zone") if cmd.len() >= 2 => {
                dns_zones.push(DnsZoneSpec {
                    name: cmd[1].clone(),
                    file: first_flag(&cmd[2..], "-file"),
                    zone: first_flag(&cmd[2..], "-zone"),
                });
            }
            // cidr-name <cidr> <label>
            Some("cidr-name") if cmd.len() >= 3 => {
                cidr_names.push(CidrName {
                    cidr: cmd[1].clone(),
                    label: cmd[2].clone(),
                });
            }
            // service-map [-file F]
            Some("service-map") => {
                service_map_file = first_flag(&cmd[1..], "-file");
            }
            // nat-map [-file F]
            Some("nat-map") => {
                nat_map_file = first_flag(&cmd[1..], "-file");
            }
            _ => {}
        }
    }
    Ok(Manifest {
        devices,
        links,
        zones,
        interfaces,
        dns_zones,
        cidr_names,
        service_map_file,
        nat_map_file,
    })
}

/// Split a Tcl script into commands, each a list of literal words, using the
/// real Tcl tokeniser (so braces, quotes, comments and `;` separators are
/// handled exactly as Tcl would). Substitutions (`$var`, `[cmd]`) are taken
/// literally — the manifest DSL never needs them. A tokeniser failure (an
/// unbalanced brace / quote) is returned as `Err(message)`.
fn tcl_commands(src: &str) -> Result<Vec<Vec<String>>, String> {
    use tcl_lexer::{Lexer, LexerConfig, SourceMap, TokenType};

    // Strict quoting so an unbalanced brace / quote is a hard error we can
    // report, rather than a silently truncated word.
    let config = LexerConfig {
        strict_quoting: true,
        // dialect-drift-ok: an architecture-manifest DSL, not a Tcl document —
        // a fixed declarative vocabulary with no dialect axis.
        ..LexerConfig::default()
    };
    let tokens = Lexer::with_source_map(SourceMap::new(src), config)
        .tokenise_all()
        .map_err(|e| format!("invalid Tcl manifest: {e}"))?;
    let sm = SourceMap::new(src);
    let mut commands: Vec<Vec<String>> = Vec::new();
    let mut cmd: Vec<String> = Vec::new();
    let mut word = String::new();
    let mut in_word = false;
    for tok in tokens {
        match tok.kind {
            TokenType::Sep => {
                if in_word {
                    cmd.push(std::mem::take(&mut word));
                    in_word = false;
                }
            }
            TokenType::Eol => {
                if in_word {
                    cmd.push(std::mem::take(&mut word));
                    in_word = false;
                }
                if !cmd.is_empty() {
                    commands.push(std::mem::take(&mut cmd));
                }
            }
            TokenType::Comment | TokenType::Eof | TokenType::Expand => {}
            // Esc / Str / Cmd / Var — a (fragment of a) word.
            _ => {
                word.push_str(sm.token_text(tok));
                in_word = true;
            }
        }
    }
    if in_word {
        cmd.push(word);
    }
    if !cmd.is_empty() {
        commands.push(cmd);
    }
    Ok(commands)
}

/// Auto-detect links by IP overlap: an upstream device targets an address that a
/// downstream device serves.
fn detect_links(devices: &[J]) -> Vec<Link> {
    // Pre-compute each device's served-address index.
    let served: Vec<BTreeMap<String, String>> = devices
        .iter()
        .map(|d| d.as_object().map(served_addresses).unwrap_or_default())
        .collect();

    let mut links: Vec<Link> = Vec::new();
    for (i, dev) in devices.iter().enumerate() {
        let Some(dm) = dev.as_object() else { continue };
        // Accumulate vias per downstream device before emitting one link each.
        let mut per_target: BTreeMap<usize, Vec<Via>> = BTreeMap::new();
        for t in outbound_targets(dm) {
            for (j, srv) in served.iter().enumerate() {
                if i == j {
                    continue; // a device serving its own member is not a tier hop
                }
                if let Some(to_obj) = srv.get(&t.address) {
                    per_target.entry(j).or_default().push(Via {
                        address: t.address.clone(),
                        from_obj: t.from_obj.clone(),
                        to_obj: to_obj.clone(),
                        port: t.port.clone(),
                    });
                }
            }
        }
        for (j, vias) in per_target {
            links.push(Link {
                from: i,
                to: j,
                source: "auto".to_string(),
                label: None,
                vias,
            });
        }
    }
    links
}

/// Merge manifest-declared links into the auto-detected set, resolving matchers
/// to indices. A manifest link that coincides with an auto link promotes its
/// source to `manifest+auto`; a new one is appended as `manifest`.
fn merge_manifest_links(auto: &mut Vec<Link>, manifest: &Manifest, devices: &[J]) {
    for spec in &manifest.links {
        let (Some(from), Some(to)) = (
            resolve_device(&spec.from, devices),
            resolve_device(&spec.to, devices),
        ) else {
            continue;
        };
        if from == to {
            continue;
        }
        if let Some(existing) = auto.iter_mut().find(|l| l.from == from && l.to == to) {
            existing.source = "manifest+auto".to_string();
            if existing.label.is_none() {
                existing.label.clone_from(&spec.label);
            }
        } else {
            auto.push(Link {
                from,
                to,
                source: "manifest".to_string(),
                label: spec.label.clone(),
                vias: Vec::new(),
            });
        }
    }
}

/// Longest-path tier assignment over the link DAG: a root (no inbound link) is
/// tier 0, and every node sits one tier below its deepest predecessor. Cycles
/// are broken by a fixed iteration cap so the pass always terminates.
fn assign_tiers(n: usize, links: &[Link], overrides: &BTreeMap<usize, i64>) -> Vec<i64> {
    let mut tier = vec![0i64; n];
    // Relax edges up to `n` times (Bellman-Ford-style longest path on a DAG;
    // the cap bounds work even if the manifest introduced a cycle).
    for _ in 0..n.max(1) {
        let mut changed = false;
        for l in links {
            let cand = tier[l.from] + 1;
            if cand > tier[l.to] {
                tier[l.to] = cand;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    // Manifest overrides win outright.
    for (i, t) in overrides {
        if *i < n {
            tier[*i] = *t;
        }
    }
    tier
}

/// Build the `architecture` model object from the shaped device list and an
/// optional manifest. Auto-detection always runs; the manifest (when present and
/// well-formed) overrides roles/tiers and augments links.
#[must_use]
pub fn build_architecture(devices: &[J], manifest_text: Option<&str>) -> J {
    let n = devices.len();

    // Parse the manifest up front; a parse error is reported but non-fatal.
    let (manifest, manifest_error) = match manifest_text {
        Some(t) if !t.trim().is_empty() => match parse_manifest(t) {
            Ok(m) => (Some(m), None),
            Err(e) => (None, Some(e)),
        },
        _ => (None, None),
    };

    // Resolve manifest device specs to per-index role/tier/label overrides.
    let mut role_over: BTreeMap<usize, String> = BTreeMap::new();
    let mut tier_over: BTreeMap<usize, i64> = BTreeMap::new();
    let mut label_over: BTreeMap<usize, String> = BTreeMap::new();
    if let Some(m) = &manifest {
        for spec in &m.devices {
            let Some(i) = resolve_device(&spec.matcher, devices) else {
                continue;
            };
            if let Some(r) = &spec.role {
                role_over.insert(i, r.clone());
            }
            if let Some(t) = spec.tier {
                tier_over.insert(i, t);
            }
            if let Some(l) = &spec.label {
                label_over.insert(i, l.clone());
            }
        }
    }

    // Links: auto-detect, then merge the manifest's declarations.
    let mut links = detect_links(devices);
    if let Some(m) = &manifest {
        merge_manifest_links(&mut links, m, devices);
    }

    let tiers = assign_tiers(n, &links, &tier_over);

    let dev_json = shape_device_json(devices, &role_over, &label_over, &tiers);

    // Shape the links, sorted for a stable, tier-ascending render.
    links.sort_by(|a, b| (tiers[a.from], a.from, a.to).cmp(&(tiers[b.from], b.from, b.to)));
    let link_json = shape_link_json(&links, &dev_json);

    let graph = build_graph(&dev_json, &links, &tiers);

    // Pre-group devices by tier (ascending) so the report can render tier
    // columns without regrouping in the template.
    let mut tier_map: BTreeMap<i64, Vec<J>> = BTreeMap::new();
    for (i, d) in dev_json.iter().enumerate() {
        tier_map.entry(tiers[i]).or_default().push(d.clone());
    }
    let tier_json: Vec<J> = tier_map
        .into_iter()
        .map(|(tier, devs)| {
            let mut o = Map::new();
            o.insert("tier".into(), J::from(tier));
            o.insert("devices".into(), J::Array(devs));
            J::Object(o)
        })
        .collect();

    let mut arch = Map::new();
    arch.insert("defined".into(), J::Bool(manifest.is_some()));
    arch.insert("deviceCount".into(), J::from(n));
    arch.insert("devices".into(), J::Array(dev_json));
    arch.insert("tiers".into(), J::Array(tier_json));
    arch.insert("links".into(), J::Array(link_json));
    arch.insert("graph".into(), J::String(graph));

    // Topology + enrichment declarations from the manifest (empty without one).
    let (zones_json, iface_json, dns_json, maps_json) = manifest.as_ref().map_or_else(
        || (Vec::new(), Vec::new(), Vec::new(), Map::new()),
        manifest_enrichment_json,
    );
    arch.insert("zones".into(), J::Array(zones_json));
    arch.insert("interfaces".into(), J::Array(iface_json));
    arch.insert("dnsZones".into(), J::Array(dns_json));
    arch.insert("maps".into(), J::Object(maps_json));

    if let Some(e) = manifest_error {
        arch.insert("manifestError".into(), J::String(e));
    }
    J::Object(arch)
}

/// Shape the device entries with their resolved role/tier/label.
fn shape_device_json(
    devices: &[J],
    role_over: &BTreeMap<usize, String>,
    label_over: &BTreeMap<usize, String>,
    tiers: &[i64],
) -> Vec<J> {
    let mut dev_json: Vec<J> = Vec::with_capacity(devices.len());
    for (i, d) in devices.iter().enumerate() {
        let dm = d.as_object();
        let uri = dm.map_or("", |m| bstr(m, "uri")).to_string();
        let name = dm.map_or("", |m| bstr(m, "name")).to_string();
        let role = role_over.get(&i).cloned().unwrap_or_else(|| {
            if dm.is_some_and(looks_like_gtm) {
                "gtm".to_string()
            } else {
                "ltm".to_string()
            }
        });
        let label = label_over.get(&i).cloned().unwrap_or_else(|| name.clone());
        let mut o = Map::new();
        o.insert("index".into(), J::from(i));
        o.insert("uri".into(), J::String(uri));
        o.insert("name".into(), J::String(name));
        o.insert("label".into(), J::String(label));
        o.insert("role".into(), J::String(role));
        o.insert("tier".into(), J::from(tiers[i]));
        dev_json.push(J::Object(o));
    }
    dev_json
}

/// Shape the (already-sorted) links into their JSON form, labelling endpoints
/// from the shaped device entries.
fn shape_link_json(links: &[Link], dev_json: &[J]) -> Vec<J> {
    links
        .iter()
        .map(|l| {
            let vias: Vec<J> = l
                .vias
                .iter()
                .map(|v| {
                    let mut vo = Map::new();
                    vo.insert("address".into(), J::String(v.address.clone()));
                    vo.insert("fromObj".into(), J::String(v.from_obj.clone()));
                    vo.insert("toObj".into(), J::String(v.to_obj.clone()));
                    vo.insert("port".into(), J::String(v.port.clone()));
                    J::Object(vo)
                })
                .collect();
            let label_of = |i: usize| -> String {
                dev_json
                    .get(i)
                    .and_then(J::as_object)
                    .map_or(String::new(), |m| bstr(m, "label").to_string())
            };
            let mut o = Map::new();
            o.insert("from".into(), J::from(l.from));
            o.insert("to".into(), J::from(l.to));
            o.insert("fromLabel".into(), J::String(label_of(l.from)));
            o.insert("toLabel".into(), J::String(label_of(l.to)));
            o.insert("source".into(), J::String(l.source.clone()));
            if let Some(lbl) = &l.label {
                o.insert("label".into(), J::String(lbl.clone()));
            }
            o.insert("viaCount".into(), J::from(l.vias.len()));
            o.insert("vias".into(), J::Array(vias));
            J::Object(o)
        })
        .collect()
}

/// Shape the manifest's zone / interface / dns-zone / map declarations into the
/// `(zones, interfaces, dnsZones, maps)` JSON the report + editor consume.
fn manifest_enrichment_json(m: &Manifest) -> (Vec<J>, Vec<J>, Vec<J>, Map<String, J>) {
    let zones = m
        .zones
        .iter()
        .map(|z| {
            let mut o = Map::new();
            o.insert("name".into(), J::String(z.name.clone()));
            o.insert(
                "cidrs".into(),
                J::Array(z.cidrs.iter().map(|c| J::String(c.clone())).collect()),
            );
            J::Object(o)
        })
        .collect();
    let interfaces = m
        .interfaces
        .iter()
        .map(|i| {
            let mut o = Map::new();
            o.insert("device".into(), J::String(i.device.clone()));
            o.insert("name".into(), J::String(i.name.clone()));
            if let Some(z) = &i.zone {
                o.insert("zone".into(), J::String(z.clone()));
            }
            if let Some(a) = &i.address {
                o.insert("address".into(), J::String(a.clone()));
            }
            J::Object(o)
        })
        .collect();
    let dns = m
        .dns_zones
        .iter()
        .map(|d| {
            let mut o = Map::new();
            o.insert("name".into(), J::String(d.name.clone()));
            if let Some(f) = &d.file {
                o.insert("file".into(), J::String(f.clone()));
            }
            if let Some(z) = &d.zone {
                o.insert("zone".into(), J::String(z.clone()));
            }
            J::Object(o)
        })
        .collect();
    let mut maps = Map::new();
    maps.insert(
        "cidrNames".into(),
        J::Array(
            m.cidr_names
                .iter()
                .map(|c| {
                    let mut o = Map::new();
                    o.insert("cidr".into(), J::String(c.cidr.clone()));
                    o.insert("label".into(), J::String(c.label.clone()));
                    J::Object(o)
                })
                .collect(),
        ),
    );
    if let Some(f) = &m.service_map_file {
        maps.insert("serviceMap".into(), J::String(f.clone()));
    }
    if let Some(f) = &m.nat_map_file {
        maps.insert("natMap".into(), J::String(f.clone()));
    }
    (zones, interfaces, dns, maps)
}

/// Render the architecture as a left-to-right Mermaid flowchart, one subgraph
/// per tier, edges labelled with the number of evidencing flows.
/// Build the cross-device architecture graph as JSON —
/// `{"nodes":[{"id","label","cls"}], "edges":[{"from","to","label"}]}` for the
/// report's elkjs renderer. One node per device (tier folded into the label,
/// since the flat renderer has no tier subgraphs), ordered left-to-right by the
/// inter-device links.
fn build_graph(dev_json: &[J], links: &[Link], tiers: &[i64]) -> String {
    if dev_json.is_empty() {
        return String::new();
    }
    let nodes: Vec<J> = dev_json
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let m = d.as_object();
            let label = m.map_or("", |m| bstr(m, "label"));
            let role = m.map_or("", |m| bstr(m, "role"));
            let text = if label.is_empty() {
                format!("device {i}")
            } else {
                label.to_string()
            };
            let sub = if role.is_empty() {
                format!("tier {}", tiers[i])
            } else {
                format!("{role} · tier {}", tiers[i])
            };
            serde_json::json!({ "id": format!("d{i}"), "label": format!("{text}\n{sub}"), "cls": "device" })
        })
        .collect();
    let edges: Vec<J> = links
        .iter()
        .map(|l| {
            let lbl = l.label.clone().unwrap_or_else(|| {
                if l.vias.is_empty() {
                    String::new()
                } else {
                    format!("{} flow(s)", l.vias.len())
                }
            });
            serde_json::json!({ "from": format!("d{}", l.from), "to": format!("d{}", l.to), "label": lbl })
        })
        .collect();
    serde_json::json!({ "nodes": nodes, "edges": edges }).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_ip_normalises() {
        assert_eq!(bare_ip("10.0.0.5%2:443").as_deref(), Some("10.0.0.5"));
        assert_eq!(bare_ip("10.0.0.0/24").as_deref(), Some("10.0.0.0"));
        assert_eq!(bare_ip("10.0.0.5").as_deref(), Some("10.0.0.5"));
        assert_eq!(bare_ip("[::1]:443").as_deref(), Some("::1"));
        assert_eq!(bare_ip("2001:db8::1%3").as_deref(), Some("2001:db8::1"));
        // BIG-IP's unbracketed IPv6 destination-port form (`.port`).
        assert_eq!(bare_ip("2001:db8::10.443").as_deref(), Some("2001:db8::10"));
        assert_eq!(bare_ip("2001:db8::1").as_deref(), Some("2001:db8::1"));
        assert_eq!(bare_ip("api.example.com"), None);
        assert_eq!(bare_ip("any"), None);
        assert_eq!(bare_ip(""), None);
    }

    #[test]
    fn manifest_dsl_topology_and_enrichment() {
        let devices = vec![
            serde_json::json!({"uri": "edge.ucs", "name": "edge"}),
            serde_json::json!({"uri": "core.ucs", "name": "core"}),
        ];
        let manifest = r"
device edge.ucs -role ltm -tier 1
zone external -cidr 0.0.0.0/0
zone dmz -cidr 192.0.2.0/24 -cidr 2001:db8:dmz::/48
interface edge.ucs ext0 -zone external -address 203.0.113.10
dns-zone example.com -file example.com.zone -zone dmz
cidr-name 10.1.0.0/16 {Datacenter A}
service-map -file services.csv
nat-map -file nat.csv
";
        let arch = build_architecture(&devices, Some(manifest));
        let o = arch.as_object().unwrap();

        let zones = o["zones"].as_array().unwrap();
        assert_eq!(zones.len(), 2);
        assert_eq!(zones[1]["name"], "dmz");
        assert_eq!(zones[1]["cidrs"].as_array().unwrap().len(), 2);

        let ifs = o["interfaces"].as_array().unwrap();
        assert_eq!(ifs[0]["device"], "edge.ucs");
        assert_eq!(ifs[0]["zone"], "external");
        assert_eq!(ifs[0]["address"], "203.0.113.10");

        let dns = o["dnsZones"].as_array().unwrap();
        assert_eq!(dns[0]["name"], "example.com");
        assert_eq!(dns[0]["file"], "example.com.zone");
        assert_eq!(dns[0]["zone"], "dmz");

        let maps = o["maps"].as_object().unwrap();
        assert_eq!(
            maps["cidrNames"].as_array().unwrap()[0]["label"],
            "Datacenter A"
        );
        assert_eq!(maps["serviceMap"], "services.csv");
        assert_eq!(maps["natMap"], "nat.csv");
    }
}
