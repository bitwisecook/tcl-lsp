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

//! BIG-IP reference-graph search engine (powers `f5 grep` / `related`).
//!
//! Seeds from every [`ObjectNode`] whose identifier (or, for `--cidr`, whose
//! path / header / body) matches the pattern, then BFS-walks the reference graph
//! produced by [`crate::graph::build_bigip_object_graph`] forward / reverse /
//! both to `max_depth` (bounded by `max_nodes`), and renders a text or JSON
//! report.

use std::collections::{HashMap, HashSet, VecDeque};
use std::net::IpAddr;

use ipnet::{IpNet, Ipv4Net, Ipv6Net};
use regex::Regex;

use crate::error::BigipError;
use crate::graph::{ObjectEdge, ObjectGraph, ObjectNode};
use crate::range::Range;

/// The accepted traversal directions.
pub const DIRECTIONS: [&str; 3] = ["both", "forward", "reverse"];

/// Closed vocabulary backing [`DIRECTIONS`]. `GrepArgs::direction` (CLI input)
/// and `GrepReport::direction` (JSON/text output) stay `&str`/`String` — those
/// are wire spellings a user types and a report emits, so the string is kept
/// there per the #1405 boundary discipline — but the two BFS direction checks
/// in [`expand_bfs`] dispatch on this enum via [`GrepDirection::from_str`]
/// instead of re-matching the raw string at each site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GrepDirection {
    Both,
    Forward,
    Reverse,
}

impl GrepDirection {
    /// Whether this direction walks outgoing edges.
    fn walks_forward(self) -> bool {
        matches!(self, GrepDirection::Forward | GrepDirection::Both)
    }

    /// Whether this direction walks incoming edges.
    fn walks_reverse(self) -> bool {
        matches!(self, GrepDirection::Reverse | GrepDirection::Both)
    }
}

impl std::str::FromStr for GrepDirection {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "both" => Ok(GrepDirection::Both),
            "forward" => Ok(GrepDirection::Forward),
            "reverse" => Ok(GrepDirection::Reverse),
            _ => Err(()),
        }
    }
}

/// One object in the grep result — a seed or a related object.
#[derive(Debug, Clone)]
pub struct GrepObject {
    /// Stable graph node id.
    pub node_id: String,
    /// Source URI this node came from.
    pub uri: String,
    /// tmsh module word.
    pub module: String,
    /// tmsh object-type word(s).
    pub object_type: String,
    /// Object identifier / full-path.
    pub full_path: String,
    /// Resolved registry kind, or `None`.
    pub kind: Option<String>,
    /// Stanza header text.
    pub header: String,
    /// Stanza body.
    pub body: String,
    /// BFS depth (0 for seeds).
    pub depth: usize,
    /// Whether this object directly matched the pattern.
    pub is_seed: bool,
    /// Source span.
    pub range: Range,
}

/// One reference edge between two objects in the grep result.
#[derive(Debug, Clone)]
pub struct GrepEdge {
    /// Referencing node id.
    pub source_id: String,
    /// Referenced node id.
    pub target_id: String,
    /// The property the reference came through.
    pub via_property: String,
    /// The registry kind the reference resolved through.
    pub via_kind: String,
}

/// Result of [`compute_grep`] — seeds, related objects, edges, and the rendered
/// text report.
pub struct GrepReport {
    /// The search pattern.
    pub pattern: String,
    /// Whether the pattern was treated as a regex.
    pub use_regex: bool,
    /// Whether the pattern was treated as a CIDR / IP match.
    pub use_cidr: bool,
    /// The traversal direction.
    pub direction: String,
    /// Whether the related-object BFS ran.
    pub recurse: bool,
    /// Objects that directly matched the pattern, in `_sort_key` order.
    pub seeds: Vec<GrepObject>,
    /// Objects reachable from the seeds, in `(depth, _sort_key)` order.
    pub related: Vec<GrepObject>,
    /// Reference edges among the visited objects.
    pub edges: Vec<GrepEdge>,
    /// Per-kind object counts, in first-seen order (seeds then related).
    pub summary: Vec<(String, usize)>,
    /// The rendered text report.
    pub text_report: String,
}

/// A parsed IP network (host bits cleared / non-strict).
#[derive(Clone, Copy)]
enum Net {
    V4(Ipv4Net),
    V6(Ipv6Net),
}

impl Net {
    fn version(self) -> u8 {
        match self {
            Net::V4(_) => 4,
            Net::V6(_) => 6,
        }
    }

    /// `self.overlaps(other)` — network overlap (same version only; the caller
    /// guards the version match).
    fn overlaps(self, other: Net) -> bool {
        match (self, other) {
            (Net::V4(a), Net::V4(b)) => a.contains(&b) || b.contains(&a),
            (Net::V6(a), Net::V6(b)) => a.contains(&b) || b.contains(&a),
            _ => false,
        }
    }
}

/// Parse one token as `ipaddress.ip_network(token, strict=False)` — a bare
/// address becomes a host network (`/32` or `/128`); a `addr/prefix` form is
/// masked to its network. Returns `None` on anything the stdlib would reject.
fn parse_ip_network(token: &str) -> Option<Net> {
    match token.parse::<IpNet>() {
        Ok(IpNet::V4(n)) => Some(Net::V4(n.trunc())),
        Ok(IpNet::V6(n)) => Some(Net::V6(n.trunc())),
        Err(_) => match token.parse::<IpAddr>() {
            Ok(IpAddr::V4(a)) => Ipv4Net::new(a, 32).ok().map(Net::V4),
            Ok(IpAddr::V6(a)) => Ipv6Net::new(a, 128).ok().map(Net::V6),
            Err(_) => None,
        },
    }
}

/// Parse one or more whitespace / comma-separated IPs or CIDRs. Returns `Err`
/// when the pattern is empty or any token is not a valid IP / network.
fn parse_cidr_pattern(pattern: &str) -> Result<Vec<Net>, BigipError> {
    let tokens: Vec<&str> = pattern
        .trim()
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|t| !t.is_empty())
        .collect();
    if tokens.is_empty() {
        return Err(BigipError::grep(
            "CIDR pattern must contain at least one IP address or network",
        ));
    }
    let mut networks = Vec::with_capacity(tokens.len());
    for token in tokens {
        match parse_ip_network(token) {
            Some(net) => networks.push(net),
            None => return Err(BigipError::grep(format!("invalid CIDR pattern {token:?}"))),
        }
    }
    Ok(networks)
}

/// The shared IPv4 / IPv6 scan regexes (ports of `shared.ip_utils.IPV4_RE` /
/// `IPV6_RE`). The IPv6 lookbehind / lookahead are emulated by [`extract_ip_networks`]
/// since the `regex` crate has no lookaround.
fn ip_regexes() -> &'static (Regex, Regex) {
    use std::sync::OnceLock;
    static RE: OnceLock<(Regex, Regex)> = OnceLock::new();
    RE.get_or_init(|| {
        let ipv4 = Regex::new(r"\b(\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3})(?:/(\d{1,2}))?\b")
            .expect("valid IPv4 regex");
        // The IPv6 core alternation, minus the `(?<![:\w])` /
        // `(?![:\w])` boundary lookarounds (handled manually below).
        let ipv6 = Regex::new(concat!(
            r"(",
            r"(?:[0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}",
            r"|(?:[0-9a-fA-F]{1,4}:){1,7}:",
            r"|:(?::[0-9a-fA-F]{1,4}){1,7}",
            r"|(?:[0-9a-fA-F]{1,4}:){1,6}:[0-9a-fA-F]{1,4}",
            r"|(?:[0-9a-fA-F]{1,4}:){1,5}(?::[0-9a-fA-F]{1,4}){1,2}",
            r"|(?:[0-9a-fA-F]{1,4}:){1,4}(?::[0-9a-fA-F]{1,4}){1,3}",
            r"|(?:[0-9a-fA-F]{1,4}:){1,3}(?::[0-9a-fA-F]{1,4}){1,4}",
            r"|(?:[0-9a-fA-F]{1,4}:){1,2}(?::[0-9a-fA-F]{1,4}){1,5}",
            r"|[0-9a-fA-F]{1,4}:(?::[0-9a-fA-F]{1,4}){1,6}",
            r"|::(?:ffff(?::0{1,4})?:)?(?:\d{1,3}\.){3}\d{1,3}",
            r"|(?:[0-9a-fA-F]{1,4}:){1,4}:(?:\d{1,3}\.){3}\d{1,3}",
            r"|::",
            r")",
            r"(?:/(\d{1,3}))?",
        ))
        .expect("valid IPv6 regex");
        (ipv4, ipv6)
    })
}

/// `c` is in the `[:\w]` boundary class the IPv6 lookarounds exclude.
fn is_ipv6_boundary_char(c: char) -> bool {
    c == ':' || c == '_' || c.is_alphanumeric()
}

/// Yield every IP / CIDR token in `text` as a network: scan IPv4 then IPv6
/// literals and validate each via the stdlib parser, silently skipping
/// anything it rejects.
fn extract_ip_networks(text: &str) -> Vec<Net> {
    let (ipv4_re, ipv6_re) = ip_regexes();
    let mut out = Vec::new();
    for m in ipv4_re.find_iter(text) {
        if let Some(net) = parse_ip_network(m.as_str()) {
            out.push(net);
        }
    }
    // Emulate the `(?<![:\w])` / `(?![:\w])` boundaries: reject a match whose
    // immediately-preceding or -following char is in the boundary class.
    let bytes = text.as_bytes();
    for m in ipv6_re.find_iter(text) {
        let before_ok = m.start() == 0
            || text[..m.start()]
                .chars()
                .next_back()
                .is_none_or(|c| !is_ipv6_boundary_char(c));
        let after_ok = m.end() == bytes.len()
            || text[m.end()..]
                .chars()
                .next()
                .is_none_or(|c| !is_ipv6_boundary_char(c));
        if before_ok
            && after_ok
            && let Some(net) = parse_ip_network(m.as_str())
        {
            out.push(net);
        }
    }
    out
}

/// Whether any IP / CIDR mentioned by `node` overlaps a seed network:
/// identifier + header + body are searched together.
fn node_matches_cidrs(node: &ObjectNode, seed_networks: &[Net]) -> bool {
    let haystack = format!("{}\n{}\n{}", node.identifier, node.header, node.body);
    for found in extract_ip_networks(&haystack) {
        for seed in seed_networks {
            if found.version() != seed.version() {
                continue;
            }
            if seed.overlaps(found) {
                return true;
            }
        }
    }
    false
}

/// The compiled seed matcher.
enum Matcher {
    Substring(String),
    Regex(Regex),
    Cidr(Vec<Net>),
    Exact(String),
}

impl Matcher {
    fn matches(&self, node: &ObjectNode) -> bool {
        match self {
            Matcher::Substring(p) => node.identifier.contains(p.as_str()),
            Matcher::Regex(re) => re.is_match(&node.identifier),
            Matcher::Cidr(nets) => node_matches_cidrs(node, nets),
            Matcher::Exact(p) => node.identifier == *p,
        }
    }
}

/// Build a seed matcher for the given mode. The three flag modes are mutually
/// exclusive.
///
/// # Errors
/// Returns `Err` on conflicting modes, a bad regex, or an unparseable CIDR.
fn build_matcher(
    pattern: &str,
    use_regex: bool,
    use_cidr: bool,
    use_exact: bool,
) -> Result<Matcher, BigipError> {
    let selected: Vec<&str> = [
        ("regex", use_regex),
        ("cidr", use_cidr),
        ("exact", use_exact),
    ]
    .into_iter()
    .filter_map(|(name, flag)| flag.then_some(name))
    .collect();
    if selected.len() > 1 {
        return Err(BigipError::grep(format!(
            "match modes are mutually exclusive; got {selected:?}"
        )));
    }
    if use_cidr {
        return Ok(Matcher::Cidr(parse_cidr_pattern(pattern)?));
    }
    if use_regex {
        return Regex::new(pattern)
            .map(Matcher::Regex)
            .map_err(|exc| BigipError::grep(format!("invalid regex pattern {pattern:?}: {exc}")));
    }
    if use_exact {
        return Ok(Matcher::Exact(pattern.to_owned()));
    }
    Ok(Matcher::Substring(pattern.to_owned()))
}

fn to_grep_object(node: &ObjectNode, depth: usize, is_seed: bool) -> GrepObject {
    GrepObject {
        node_id: node.node_id.clone(),
        uri: node.uri.clone(),
        module: node.module.clone(),
        object_type: node.object_type.clone(),
        full_path: node.identifier.clone(),
        kind: node.kind.map(str::to_owned),
        header: node.header.clone(),
        body: node.body.clone(),
        depth,
        is_seed,
        range: node.range,
    }
}

/// Split `text` into lines: split on every
/// Unicode line boundary and never emit a trailing empty string for a final
/// terminator.
fn splitlines(text: &str) -> Vec<&str> {
    let is_boundary = |c: char| {
        matches!(
            c,
            '\n' | '\r'
                | '\u{0b}'
                | '\u{0c}'
                | '\u{1c}'
                | '\u{1d}'
                | '\u{1e}'
                | '\u{85}'
                | '\u{2028}'
                | '\u{2029}'
        )
    };
    let mut out = Vec::new();
    let mut start = 0;
    let mut chars = text.char_indices().peekable();
    while let Some((idx, c)) = chars.next() {
        if is_boundary(c) {
            out.push(&text[start..idx]);
            // Treat `\r\n` as a single terminator.
            if c == '\r'
                && let Some(&(_, '\n')) = chars.peek()
            {
                chars.next();
            }
            start = chars.peek().map_or(text.len(), |&(i, _)| i);
        }
    }
    if start < text.len() {
        out.push(&text[start..]);
    }
    out
}

/// Metadata threaded into [`format_text_report`].
struct ReportMeta<'a> {
    pattern: &'a str,
    use_regex: bool,
    use_cidr: bool,
    direction: &'a str,
    recurse: bool,
    source_uris: Vec<String>,
}

/// Render the text report.
fn format_text_report(
    meta: &ReportMeta,
    seeds: &[GrepObject],
    related: &[GrepObject],
    summary: &[(String, usize)],
    include_body: bool,
) -> String {
    let mode_suffix = if meta.use_cidr {
        " (cidr)"
    } else if meta.use_regex {
        " (regex)"
    } else {
        ""
    };
    let related_line = if meta.recurse {
        format!("# Related: {} object(s)", related.len())
    } else {
        "# Related: 0 object(s) (skipped — --no-recurse)".to_owned()
    };
    let sources = if meta.source_uris.is_empty() {
        "(none)".to_owned()
    } else {
        meta.source_uris.join(", ")
    };
    let mut lines: Vec<String> = vec![
        "# tcl-lsp BIG-IP grep".to_owned(),
        format!("# Pattern: {}{mode_suffix}", meta.pattern),
        format!("# Direction: {}", meta.direction),
        format!("# Sources: {sources}"),
        format!("# Searches: {} matched object(s)", seeds.len()),
        related_line,
    ];
    if !summary.is_empty() {
        let mut kinds: Vec<&(String, usize)> = summary.iter().collect();
        kinds.sort_by(|a, b| a.0.cmp(&b.0));
        for (kind, count) in kinds {
            lines.push(format!("#   {kind}: {count}"));
        }
    }
    lines.push(String::new());

    if seeds.is_empty() {
        lines.push(format!("# No objects match pattern: {}", meta.pattern));
        lines.push(String::new());
        return lines.join("\n");
    }

    let emit = |obj: &GrepObject, lines: &mut Vec<String>| {
        let marker = if obj.is_seed { "*" } else { " " };
        let kind = obj.kind.as_deref().unwrap_or("?");
        lines.push(format!(
            "{marker} [{kind}] {}  (depth {})",
            obj.full_path, obj.depth
        ));
        if include_body {
            lines.push(format!("    {} {{", obj.header.trim_end()));
            for body_line in splitlines(&obj.body) {
                lines.push(format!("    {body_line}").trim_end().to_owned());
            }
            lines.push("    }".to_owned());
            lines.push(String::new());
        }
    };

    lines.push("# Searches (matched by pattern):".to_owned());
    for obj in seeds {
        emit(obj, &mut lines);
    }
    lines.push(String::new());

    if !related.is_empty() {
        lines.push("# Related objects (reachable through reference edges):".to_owned());
        for obj in related {
            emit(obj, &mut lines);
        }
        lines.push(String::new());
    }

    lines.join("\n")
}

/// Arguments for [`compute_grep`].
// One field per CLI flag; collapsing the mutually-related booleans into enums
// would obscure that structure.
#[allow(clippy::struct_excessive_bools)]
pub struct GrepArgs<'a> {
    /// The search pattern.
    pub pattern: &'a str,
    /// Treat the pattern as a regex.
    pub use_regex: bool,
    /// Treat the pattern as one or more IP / CIDR values.
    pub use_cidr: bool,
    /// Require an exact identifier match (used by rename / references-to).
    pub use_exact: bool,
    /// Traversal direction (`forward` / `reverse` / `both`).
    pub direction: &'a str,
    /// Stop the BFS after this many hops (unbounded when `None`).
    pub max_depth: Option<usize>,
    /// Cap the result at this many objects.
    pub max_nodes: usize,
    /// Embed each object's body in the text report.
    pub include_body: bool,
    /// Walk the related-object BFS (skip it when `false`).
    pub recurse: bool,
}

impl Default for GrepArgs<'_> {
    fn default() -> Self {
        Self {
            pattern: "",
            use_regex: false,
            use_cidr: false,
            use_exact: false,
            direction: "both",
            max_depth: None,
            max_nodes: 1000,
            include_body: false,
            recurse: true,
        }
    }
}

/// Validate the direction / `max_nodes` invariants `compute_grep` relies on.
fn validate_grep_args(args: &GrepArgs) -> Result<(), BigipError> {
    if args.direction.parse::<GrepDirection>().is_err() {
        let mut sorted = DIRECTIONS;
        sorted.sort_unstable();
        return Err(BigipError::grep(format!(
            "direction must be one of {sorted:?}, got {:?}",
            args.direction
        )));
    }
    if args.max_nodes < 1 {
        return Err(BigipError::grep(format!(
            "max_nodes must be >= 1, got {}",
            args.max_nodes
        )));
    }
    Ok(())
}

/// Per-node edge adjacency: node id → the edges touching it.
type Adjacency<'a> = HashMap<&'a str, Vec<&'a ObjectEdge>>;

/// Build the `(outgoing, incoming)` per-node edge adjacency for the graph.
fn build_adjacency(graph: &ObjectGraph) -> (Adjacency<'_>, Adjacency<'_>) {
    let mut outgoing: HashMap<&str, Vec<&ObjectEdge>> = HashMap::new();
    let mut incoming: HashMap<&str, Vec<&ObjectEdge>> = HashMap::new();
    for edge in &graph.edges {
        outgoing
            .entry(edge.source_id.as_str())
            .or_default()
            .push(edge);
        incoming
            .entry(edge.target_id.as_str())
            .or_default()
            .push(edge);
    }
    (outgoing, incoming)
}

/// BFS from the (already capped) seeds, returning depth-per-node (0 for seeds).
/// Honours `direction`, `max_depth`, and the strict `max_nodes` cap.
fn expand_bfs(
    seed_ids: &[String],
    args: &GrepArgs,
    outgoing: &HashMap<&str, Vec<&ObjectEdge>>,
    incoming: &HashMap<&str, Vec<&ObjectEdge>>,
) -> HashMap<String, usize> {
    let mut depths: HashMap<String, usize> = seed_ids.iter().map(|s| (s.clone(), 0)).collect();
    if !args.recurse {
        return depths;
    }
    // `compute_grep` always runs `validate_grep_args` (which rejects any
    // direction not in `DIRECTIONS`) before calling this, so the parse cannot
    // fail in practice; `Both` is a harmless fallback rather than a silent
    // behaviour change if that invariant is ever loosened.
    let direction: GrepDirection = args.direction.parse().unwrap_or(GrepDirection::Both);
    let mut queue: VecDeque<String> = seed_ids.iter().cloned().collect();
    while let Some(nid) = queue.pop_front() {
        if depths.len() >= args.max_nodes {
            break;
        }
        let depth = depths[&nid];
        if args.max_depth.is_some_and(|m| depth >= m) {
            continue;
        }
        let mut neighbours: Vec<String> = Vec::new();
        if direction.walks_forward()
            && let Some(edges) = outgoing.get(nid.as_str())
        {
            for edge in edges {
                neighbours.push(edge.target_id.clone());
            }
        }
        if direction.walks_reverse()
            && let Some(edges) = incoming.get(nid.as_str())
        {
            for edge in edges {
                neighbours.push(edge.source_id.clone());
            }
        }
        for neighbour in neighbours {
            if depths.contains_key(&neighbour) {
                continue;
            }
            depths.insert(neighbour.clone(), depth + 1);
            queue.push_back(neighbour);
            if depths.len() >= args.max_nodes {
                break;
            }
        }
    }
    depths
}

/// Per-kind count summary in first-seen order (seeds first, then related).
fn build_summary(seeds: &[GrepObject], related: &[GrepObject]) -> Vec<(String, usize)> {
    let mut summary_order: Vec<String> = Vec::new();
    let mut summary_counts: HashMap<String, usize> = HashMap::new();
    for obj in seeds.iter().chain(related.iter()) {
        let key = obj.kind.clone().unwrap_or_else(|| "<unknown>".to_owned());
        if let Some(c) = summary_counts.get_mut(&key) {
            *c += 1;
        } else {
            summary_order.push(key.clone());
            summary_counts.insert(key, 1);
        }
    }
    summary_order
        .iter()
        .map(|k| (k.clone(), summary_counts[k]))
        .collect()
}

/// Collect the deduped edges whose endpoints are both in `visited`.
fn collect_visible_edges(graph: &ObjectGraph, visited: &HashSet<&str>) -> Vec<GrepEdge> {
    let mut edge_items: Vec<GrepEdge> = Vec::new();
    let mut seen_edges: HashSet<(&str, &str, &str, &str)> = HashSet::new();
    for edge in &graph.edges {
        if !visited.contains(edge.source_id.as_str()) || !visited.contains(edge.target_id.as_str())
        {
            continue;
        }
        let key = (
            edge.source_id.as_str(),
            edge.target_id.as_str(),
            edge.via_property.as_str(),
            edge.via_kind.as_str(),
        );
        if !seen_edges.insert(key) {
            continue;
        }
        edge_items.push(GrepEdge {
            source_id: edge.source_id.clone(),
            target_id: edge.target_id.clone(),
            via_property: edge.via_property.clone(),
            via_kind: edge.via_kind.clone(),
        });
    }
    edge_items
}

/// Find every BIG-IP object related to seeds whose identifiers match the
/// pattern. `graph` must come from
/// [`crate::graph::build_bigip_object_graph`] (built with the iRules dialect
/// active, which the caller arranges). `source_uris` are the input URIs used
/// only for the report header.
///
/// # Errors
/// Returns `Err` on an invalid direction, a non-positive `max_nodes`, a negative
/// `max_depth`, or a bad regex / CIDR pattern.
pub fn compute_grep(
    graph: &ObjectGraph,
    source_uris: &[String],
    args: &GrepArgs,
) -> Result<GrepReport, BigipError> {
    validate_grep_args(args)?;

    let matcher = build_matcher(args.pattern, args.use_regex, args.use_cidr, args.use_exact)?;

    // Flatten nodes by id, in source order.
    let mut all_nodes: HashMap<&str, &ObjectNode> = HashMap::new();
    for (_uri, nodes) in &graph.nodes_by_uri {
        for node in nodes {
            all_nodes.insert(node.node_id.as_str(), node);
        }
    }

    let (outgoing, incoming) = build_adjacency(graph);

    let sort_key = |nid: &str| -> (String, String) {
        let node = all_nodes[nid];
        (node.kind.unwrap_or("").to_owned(), node.identifier.clone())
    };

    let mut seed_ids: Vec<String> = all_nodes
        .iter()
        .filter(|(_, node)| matcher.matches(node))
        .map(|(id, _)| (*id).to_owned())
        .collect();
    seed_ids.sort_by_key(|nid| sort_key(nid));

    // Strict cap on total collected objects: truncate seeds before any BFS.
    if seed_ids.len() > args.max_nodes {
        seed_ids.truncate(args.max_nodes);
    }

    let depths = expand_bfs(&seed_ids, args, &outgoing, &incoming);

    let seed_set: HashSet<&str> = seed_ids.iter().map(String::as_str).collect();

    let mut sorted_seed_ids = seed_ids.clone();
    sorted_seed_ids.sort_by_key(|nid| sort_key(nid));
    let seeds_list: Vec<GrepObject> = sorted_seed_ids
        .iter()
        .map(|nid| to_grep_object(all_nodes[nid.as_str()], 0, true))
        .collect();

    let mut related_ids: Vec<String> = depths
        .keys()
        .filter(|nid| !seed_set.contains(nid.as_str()))
        .cloned()
        .collect();
    related_ids.sort_by(|a, b| (depths[a], sort_key(a)).cmp(&(depths[b], sort_key(b))));
    let related_list: Vec<GrepObject> = related_ids
        .iter()
        .map(|nid| to_grep_object(all_nodes[nid.as_str()], depths[nid], false))
        .collect();

    let summary = build_summary(&seeds_list, &related_list);

    let visited: HashSet<&str> = depths.keys().map(String::as_str).collect();
    let edge_items = collect_visible_edges(graph, &visited);

    let mut uris = source_uris.to_vec();
    uris.sort();
    let meta = ReportMeta {
        pattern: args.pattern,
        use_regex: args.use_regex,
        use_cidr: args.use_cidr,
        direction: args.direction,
        recurse: args.recurse,
        source_uris: uris,
    };
    let text_report = format_text_report(
        &meta,
        &seeds_list,
        &related_list,
        &summary,
        args.include_body,
    );

    Ok(GrepReport {
        pattern: args.pattern.to_owned(),
        use_regex: args.use_regex,
        use_cidr: args.use_cidr,
        direction: args.direction.to_owned(),
        recurse: args.recurse,
        seeds: seeds_list,
        related: related_list,
        edges: edge_items,
        summary,
        text_report,
    })
}

/// Serialise one [`GrepObject`].
fn obj_to_json(obj: &GrepObject, include_body: bool) -> String {
    use std::fmt::Write as _;

    use crate::jsonfmt::json_string as q;
    let kind = obj.kind.as_deref().map_or_else(|| "null".to_owned(), q);
    let mut s = format!(
        concat!(
            "    {{\n",
            "      \"nodeId\": {},\n",
            "      \"uri\": {},\n",
            "      \"module\": {},\n",
            "      \"objectType\": {},\n",
            "      \"fullPath\": {},\n",
            "      \"kind\": {},\n",
            "      \"header\": {},\n",
            "      \"depth\": {},\n",
            "      \"isSearch\": {},\n",
            "      \"range\": {{\n",
            "        \"start\": {{\n",
            "          \"line\": {},\n",
            "          \"character\": {}\n",
            "        }},\n",
            "        \"end\": {{\n",
            "          \"line\": {},\n",
            "          \"character\": {}\n",
            "        }}\n",
            "      }}",
        ),
        q(&obj.node_id),
        q(&obj.uri),
        q(&obj.module),
        q(&obj.object_type),
        q(&obj.full_path),
        kind,
        q(&obj.header),
        obj.depth,
        if obj.is_seed { "true" } else { "false" },
        obj.range.start.line,
        obj.range.start.character,
        obj.range.end.line,
        obj.range.end.character,
    );
    if include_body {
        let _ = write!(s, ",\n      \"body\": {}", q(&obj.body));
    }
    s.push_str("\n    }");
    s
}

/// Render `report` as 2-space-indented JSON. Built by
/// hand for a deterministic key order (including the insertion-ordered
/// `summary`), like the graph / stats exports.
#[must_use]
pub fn report_to_json(report: &GrepReport, include_body: bool) -> String {
    use std::fmt::Write as _;

    use crate::jsonfmt::json_string as q;

    let obj_list = |objs: &[GrepObject]| -> String {
        if objs.is_empty() {
            return "[]".to_owned();
        }
        let items: Vec<String> = objs.iter().map(|o| obj_to_json(o, include_body)).collect();
        format!("[\n{}\n  ]", items.join(",\n"))
    };

    let edges = if report.edges.is_empty() {
        "[]".to_owned()
    } else {
        let items: Vec<String> = report
            .edges
            .iter()
            .map(|e| {
                format!(
                    concat!(
                        "    {{\n",
                        "      \"source\": {},\n",
                        "      \"target\": {},\n",
                        "      \"viaProperty\": {},\n",
                        "      \"viaKind\": {}\n",
                        "    }}",
                    ),
                    q(&e.source_id),
                    q(&e.target_id),
                    q(&e.via_property),
                    q(&e.via_kind),
                )
            })
            .collect();
        format!("[\n{}\n  ]", items.join(",\n"))
    };

    let summary = if report.summary.is_empty() {
        "{}".to_owned()
    } else {
        let items: Vec<String> = report
            .summary
            .iter()
            .map(|(k, v)| format!("    {}: {v}", q(k)))
            .collect();
        format!("{{\n{}\n  }}", items.join(",\n"))
    };

    let mut out = String::from("{\n");
    let _ = writeln!(out, "  \"pattern\": {},", q(&report.pattern));
    let _ = writeln!(
        out,
        "  \"useRegex\": {},",
        if report.use_regex { "true" } else { "false" }
    );
    let _ = writeln!(
        out,
        "  \"useCidr\": {},",
        if report.use_cidr { "true" } else { "false" }
    );
    let _ = writeln!(
        out,
        "  \"recurse\": {},",
        if report.recurse { "true" } else { "false" }
    );
    let _ = writeln!(out, "  \"direction\": {},", q(&report.direction));
    let _ = writeln!(out, "  \"searches\": {},", obj_list(&report.seeds));
    let _ = writeln!(out, "  \"related\": {},", obj_list(&report.related));
    let _ = writeln!(out, "  \"edges\": {edges},");
    let _ = writeln!(out, "  \"summary\": {summary},");
    let _ = write!(out, "  \"textReport\": {}", q(&report.text_report));
    out.push_str("\n}\n");
    out
}
