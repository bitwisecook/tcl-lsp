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

//! APM access-profile walk — the "walk the whole profile, link to everything"
//! view.
//!
//! For every `apm profile access` stanza this follows each reference out to the
//! objects it depends on — the virtual servers that attach it, its connectivity
//! profile, the access policy, every policy item (with the `next-item` flow
//! between them), the agents on each item, the AAA servers those agents
//! authenticate against, and the resources a resource-assign agent hands out
//! (network-access → lease pool / client DNS, webtops, remote-desktop
//! resources) — and emits a per-profile `{nodes, edges}` dependency-graph
//! model. The report's elkjs renderer (`elk-graph.js`) lays it out left-to-right
//! with rectangular nodes and true orthogonal edge routing — right-angle
//! connectors, separated channels, arrowheads seated on the node border — the
//! way the F5 Visual Policy Editor draws it.
//!
//! Why a bespoke parse rather than the `f5-query` engine: the query projection
//! only covers the `ltm` module, and even the parsed `tcl-bigip` model keeps
//! APM objects as *minimal* records that drop exactly the fields the walk needs
//! — the profile's `access-policy` pointer, a policy item's `next-item` edges,
//! and a resource-assign agent's assigned resources. So this module reads the
//! `apm …` stanzas straight out of the config text (the same file-first
//! approach `certs` / `secrets` take for their non-LTM data).

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value as J};

use crate::jutil::bstr;

// --- a tiny TMSH reader ------------------------------------------------------

/// One parsed TMSH statement: the space-separated words on the line plus, when
/// the statement opened a `{ … }`, its parsed child statements.
#[derive(Debug, Default)]
struct Node {
    words: Vec<String>,
    children: Vec<Node>,
    has_block: bool,
}

impl Node {
    /// The scalar value of the `key` field: `key v1 v2 …` → `"v1 v2 …"`.
    fn scalar(&self, key: &str) -> Option<String> {
        self.child(key)
            .filter(|n| n.words.len() > 1)
            .map(|n| n.words[1..].join(" "))
    }

    /// The first child statement whose leading word is `key`.
    fn child(&self, key: &str) -> Option<&Node> {
        self.children
            .iter()
            .find(|n| n.words.first().map(String::as_str) == Some(key))
    }

    /// The names (leading words) declared inside the `key { … }` block. Handles
    /// both one-per-line named sub-objects (`items { /a { } /b { } }`) and an
    /// inline word list (`… { de en es }`).
    fn names_in(&self, key: &str) -> Vec<String> {
        self.child(key)
            .map(|n| {
                // Each child may be a named sub-object (`/a { … }` → its name is
                // the first word) or, for an inline list (`{ /a /b }`), several
                // bare words in one statement — take every word either way.
                n.children
                    .iter()
                    .flat_map(|c| c.words.iter().cloned())
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Split the config into top-level stanzas, returning `(header words, body)`
/// for each. Quote-aware brace tracking so a value like
/// `expression "expr {[mcget {x}] == 1}"` never unbalances the scan.
fn top_level_objects(src: &str) -> Vec<(Vec<String>, String)> {
    let mut out = Vec::new();
    let mut header = String::new();
    let mut body = String::new();
    let mut depth = 0usize;
    let mut in_quote = false;
    let mut escaped = false;

    for ch in src.chars() {
        if depth == 0 {
            // Accumulating a stanza header up to its opening brace.
            match ch {
                '{' => {
                    depth = 1;
                    body.clear();
                }
                _ => header.push(ch),
            }
            continue;
        }

        // Inside a stanza body: copy verbatim, tracking depth and quotes.
        if in_quote {
            body.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_quote = false;
            }
            continue;
        }
        match ch {
            '"' => {
                in_quote = true;
                body.push(ch);
            }
            '{' => {
                depth += 1;
                body.push(ch);
            }
            '}' => {
                depth -= 1;
                if depth == 0 {
                    let words: Vec<String> = header.split_whitespace().map(str::to_owned).collect();
                    if !words.is_empty() {
                        out.push((words, std::mem::take(&mut body)));
                    }
                    header.clear();
                } else {
                    body.push(ch);
                }
            }
            _ => body.push(ch),
        }
    }
    out
}

/// Tokens of a TMSH body: words, block delimiters, and statement boundaries.
enum Tok {
    Word(String),
    Open,
    Close,
    Break,
}

fn tokenise(body: &str) -> Vec<Tok> {
    let mut toks = Vec::new();
    let mut word = String::new();
    let mut in_quote = false;
    let mut escaped = false;
    let flush = |word: &mut String, toks: &mut Vec<Tok>| {
        if !word.is_empty() {
            toks.push(Tok::Word(std::mem::take(word)));
        }
    };
    for ch in body.chars() {
        if in_quote {
            if escaped {
                word.push(ch);
                escaped = false;
            } else if ch == '\\' {
                word.push(ch);
                escaped = true;
            } else if ch == '"' {
                in_quote = false;
            } else {
                word.push(ch);
            }
            continue;
        }
        match ch {
            '"' => in_quote = true,
            '{' => {
                flush(&mut word, &mut toks);
                toks.push(Tok::Open);
            }
            '}' => {
                flush(&mut word, &mut toks);
                toks.push(Tok::Close);
            }
            '\n' => {
                flush(&mut word, &mut toks);
                toks.push(Tok::Break);
            }
            c if c.is_whitespace() => flush(&mut word, &mut toks),
            c => word.push(c),
        }
    }
    flush(&mut word, &mut toks);
    toks
}

/// Parse a TMSH body into its child statements.
fn parse_body(body: &str) -> Vec<Node> {
    let toks = tokenise(body);
    let mut pos = 0;
    parse_nodes(&toks, &mut pos)
}

fn parse_nodes(toks: &[Tok], pos: &mut usize) -> Vec<Node> {
    let mut nodes = Vec::new();
    let mut cur = Node::default();
    let flush = |cur: &mut Node, nodes: &mut Vec<Node>| {
        if !cur.words.is_empty() || cur.has_block {
            nodes.push(std::mem::take(cur));
        }
    };
    while *pos < toks.len() {
        match &toks[*pos] {
            Tok::Word(w) => cur.words.push(w.clone()),
            Tok::Break => flush(&mut cur, &mut nodes),
            Tok::Open => {
                *pos += 1;
                cur.children = parse_nodes(toks, pos);
                cur.has_block = true;
                flush(&mut cur, &mut nodes);
            }
            Tok::Close => {
                flush(&mut cur, &mut nodes);
                *pos += 1;
                return nodes;
            }
        }
        *pos += 1;
    }
    flush(&mut cur, &mut nodes);
    nodes
}

// --- the parsed APM object index --------------------------------------------

/// Every APM object we care about, keyed by full path, plus the parsed body.
#[derive(Default)]
struct Apm {
    access_profiles: BTreeMap<String, Vec<Node>>,
    connectivity_profiles: BTreeSet<String>,
    access_policies: BTreeMap<String, Vec<Node>>,
    policy_items: BTreeMap<String, Vec<Node>>,
    policy_agents: BTreeMap<String, Vec<Node>>,
    aaa_servers: BTreeMap<String, Vec<Node>>,
    resources: BTreeMap<String, (String, Vec<Node>)>,
}

/// `/Common/x` → `x`.
fn leaf(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

fn partition_of(path: &str) -> String {
    if path.starts_with('/') {
        path.split('/').nth(1).unwrap_or("").to_string()
    } else {
        String::new()
    }
}

/// The full paths a virtual's profile reference could resolve to. A reference is
/// either an absolute `/Partition/name` or a partition-relative `name` — in the
/// common same-partition config a virtual attaches its access / connectivity
/// profiles by bare name, so resolve those against the virtual's own partition
/// and the shared `/Common` fallback.
fn profile_candidates(profile_ref: &str, virtual_full_path: &str) -> Vec<String> {
    if profile_ref.starts_with('/') {
        return vec![profile_ref.to_string()];
    }
    let mut out = vec![format!("/Common/{profile_ref}")];
    let part = partition_of(virtual_full_path);
    if !part.is_empty() && part != "Common" {
        out.push(format!("/{part}/{profile_ref}"));
    }
    out
}

/// Whether `virtual`'s profile list attaches the object at `target_full_path`.
fn attaches(profiles: &[String], virtual_full_path: &str, target_full_path: &str) -> bool {
    profiles.iter().any(|p| {
        profile_candidates(p, virtual_full_path)
            .iter()
            .any(|c| c == target_full_path)
    })
}

fn index_apm(source: &str) -> Apm {
    let mut apm = Apm::default();
    for (words, body) in top_level_objects(source) {
        if words.first().map(String::as_str) != Some("apm") {
            continue;
        }
        // The last word is the object path; the words before it are the kind.
        let Some(path) = words.last().filter(|p| p.starts_with('/')).cloned() else {
            continue;
        };
        let kind = words[..words.len() - 1].join(" ");
        match kind.as_str() {
            "apm profile access" => {
                apm.access_profiles.insert(path, parse_body(&body));
            }
            "apm profile connectivity" => {
                apm.connectivity_profiles.insert(path);
            }
            "apm policy access-policy" => {
                apm.access_policies.insert(path, parse_body(&body));
            }
            "apm policy policy-item" => {
                apm.policy_items.insert(path, parse_body(&body));
            }
            "apm aaa active-directory"
            | "apm aaa ldap"
            | "apm aaa radius"
            | "apm aaa localdb"
            | "apm aaa kerberos"
            | "apm aaa tacacsplus"
            | "apm aaa securid"
            | "apm aaa http"
            | "apm aaa oauth-server"
            | "apm aaa saml" => {
                apm.aaa_servers.insert(path, parse_body(&body));
            }
            _ if kind.starts_with("apm policy agent ") => {
                apm.policy_agents.insert(path, parse_body(&body));
            }
            _ if kind.starts_with("apm resource ") => {
                let res_kind = kind.trim_start_matches("apm resource ").to_string();
                apm.resources.insert(path, (res_kind, parse_body(&body)));
            }
            _ => {}
        }
    }
    apm
}

// --- graph builder -----------------------------------------------------------

/// Accumulates the nodes and edges for one profile's walk, de-duping so a
/// shared object (an agent, a lease pool) is drawn once. Serialised to a
/// `{nodes, edges}` model that the client-side elkjs renderer lays out with
/// orthogonal edge routing (`elk-graph.js`).
#[derive(Default)]
struct Graph {
    ids: BTreeMap<String, String>,
    nodes: Vec<J>,
    edges: Vec<J>,
    seen_edges: BTreeSet<String>,
    objects: Vec<J>,
}

impl Graph {
    /// Register a node keyed by `id_key` (usually its full path), returning its
    /// stable short id. `class` drives the box styling via CSS. A `\n` in the
    /// label is a line break the renderer honours.
    fn node(&mut self, id_key: &str, label: &str, class: &str) -> String {
        if let Some(id) = self.ids.get(id_key) {
            return id.clone();
        }
        let id = format!("N{}", self.ids.len());
        self.ids.insert(id_key.to_string(), id.clone());
        let mut o = Map::new();
        o.insert("id".into(), J::String(id.clone()));
        o.insert("label".into(), J::String(label.into()));
        o.insert("cls".into(), J::String(class.into()));
        self.nodes.push(J::Object(o));
        id
    }

    fn edge(&mut self, from: &str, to: &str, label: &str) {
        let key = format!("{from}->{to}:{label}");
        if !self.seen_edges.insert(key) {
            return;
        }
        let mut o = Map::new();
        o.insert("from".into(), J::String(from.into()));
        o.insert("to".into(), J::String(to.into()));
        if !label.is_empty() {
            o.insert("label".into(), J::String(label.into()));
        }
        self.edges.push(J::Object(o));
    }

    /// Record a linked object for the panel's object list / search index.
    fn object(&mut self, ty: &str, full_path: &str) {
        let mut o = Map::new();
        o.insert("type".into(), J::String(ty.into()));
        o.insert("name".into(), J::String(leaf(full_path).into()));
        o.insert("fullPath".into(), J::String(full_path.into()));
        self.objects.push(J::Object(o));
    }
}

/// The style class for a policy item, from its `item-type` / caption: the
/// green start / green allow / red deny / plain action fills.
fn item_class(item_type: &str, caption: &str) -> &'static str {
    let cap = caption.to_ascii_lowercase();
    match item_type {
        "ending" if cap.contains("den") => "deny",
        "ending" => "allow",
        _ if cap == "start" => "start",
        _ => "action",
    }
}

/// Build the walk graph for one access profile. Returns `(graph, objects)`.
fn walk_profile(
    apm: &Apm,
    full_path: &str,
    profile_body: &[Node],
    virtuals: &[(String, Vec<String>)],
) -> (J, Vec<J>) {
    let mut g = Graph::default();

    let ap = g.node(
        &format!("ap:{full_path}"),
        &format!("Access Profile\n{}", leaf(full_path)),
        "ap",
    );
    g.object("apm profile access", full_path);

    // Virtual servers that attach this access profile, and their connectivity
    // profile (a sibling profile on the same virtual). Profile references may be
    // partition-relative, so resolve each against the virtual's partition.
    for (vfp, profiles) in virtuals {
        if !attaches(profiles, vfp, full_path) {
            continue;
        }
        let vs = g.node(
            &format!("vs:{vfp}"),
            &format!("Virtual Server\n{}", leaf(vfp)),
            "vs",
        );
        g.object("ltm virtual", vfp);
        g.edge(&vs, &ap, "");
        for cp in profiles {
            for cand in profile_candidates(cp, vfp) {
                if apm.connectivity_profiles.contains(&cand) {
                    let cpn = g.node(
                        &format!("cp:{cand}"),
                        &format!("Connectivity\n{}", leaf(&cand)),
                        "cp",
                    );
                    g.object("apm profile connectivity", &cand);
                    g.edge(&vs, &cpn, "");
                }
            }
        }
    }

    // The access policy and its item graph.
    if let Some(pol_path) = scalar(profile_body, "access-policy")
        && let Some(pol_body) = apm.access_policies.get(&pol_path)
    {
        let pol = g.node(
            &format!("pol:{pol_path}"),
            &format!("Access Policy\n{}", leaf(&pol_path)),
            "pol",
        );
        g.object("apm policy access-policy", &pol_path);
        g.edge(&ap, &pol, "access-policy");
        walk_policy(&mut g, apm, &pol, pol_body);
    }

    (graph_model(&g), g.objects)
}

/// Walk an access policy: its items, the `next-item` flow between them, and the
/// agents / resources each item pulls in.
fn walk_policy(g: &mut Graph, apm: &Apm, pol_id: &str, pol_body: &[Node]) {
    let start = pol_root(pol_body).scalar("start-item");
    let items = pol_root(pol_body).names_in("items");

    // Draw every item first, so `next-item` edges always find their target.
    for item_path in &items {
        item_node(g, apm, item_path);
    }
    if let Some(start_path) = &start {
        let start_id = item_node(g, apm, start_path);
        g.edge(pol_id, &start_id, "start");
    }

    for item_path in &items {
        let item_id = item_node(g, apm, item_path);
        let Some(item_body) = apm.policy_items.get(item_path) else {
            continue;
        };
        // `next-item` flow edges, labelled with the branch caption.
        if let Some(rules) = child(item_body, "rules") {
            for rule in &rules.children {
                if let Some(next) = rule.scalar("next-item") {
                    let next_id = item_node(g, apm, &next);
                    let cap = rule.scalar("caption").unwrap_or_default();
                    g.edge(&item_id, &next_id, &cap);
                }
            }
        }
        // Agents on this item: AAA servers and assigned resources.
        if let Some(agents) = child(item_body, "agents") {
            for agent in &agents.children {
                let Some(agent_path) = agent.words.first() else {
                    continue;
                };
                walk_agent(g, apm, &item_id, agent_path);
            }
        }
    }
}

/// Link an agent's downstream objects onto its policy item.
fn walk_agent(g: &mut Graph, apm: &Apm, item_id: &str, agent_path: &str) {
    let Some(agent_body) = apm.policy_agents.get(agent_path) else {
        return;
    };
    // An auth agent points at an AAA server.
    if let Some(server) = scalar(agent_body, "server") {
        let sid = g.node(
            &format!("aaa:{server}"),
            &format!("AAA Server\n{}", leaf(&server)),
            "aaa",
        );
        g.object("apm aaa", &server);
        g.edge(item_id, &sid, "auth");
        if let Some(sbody) = apm.aaa_servers.get(&server)
            && let Some(domain) = scalar(sbody, "domain")
        {
            let did = g.node(
                &format!("{server}#domain"),
                &format!("Domain\n{domain}"),
                "res",
            );
            g.edge(&sid, &did, "");
        }
    }
    // A resource-assign agent hands out network-access / webtop / remote-desktop
    // resources through its `rules { { … } }` block.
    if let Some(rules) = child(agent_body, "rules") {
        for rule in &rules.children {
            for na in rule.names_in("network-access-resources") {
                walk_network_access(g, apm, item_id, &na);
            }
            for rd in rule.names_in("remote-desktop-resources") {
                let rid = g.node(
                    &format!("rd:{rd}"),
                    &format!("Remote Desktop\n{}", leaf(&rd)),
                    "res",
                );
                g.object("apm resource remote-desktop", &rd);
                g.edge(item_id, &rid, "assign");
            }
            for pa in rule.names_in("portal-access-resources") {
                let pid = g.node(
                    &format!("pa:{pa}"),
                    &format!("Portal Access\n{}", leaf(&pa)),
                    "res",
                );
                g.object("apm resource portal-access", &pa);
                g.edge(item_id, &pid, "assign");
            }
            if let Some(webtop) = rule.scalar("webtop") {
                let wid = g.node(
                    &format!("wt:{webtop}"),
                    &format!("Webtop\n{}", leaf(&webtop)),
                    "res",
                );
                g.object("apm resource webtop", &webtop);
                g.edge(item_id, &wid, "webtop");
            }
        }
    }
}

/// A network-access resource, plus its lease pool and client DNS.
fn walk_network_access(g: &mut Graph, apm: &Apm, item_id: &str, na_path: &str) {
    let nid = g.node(
        &format!("na:{na_path}"),
        &format!("Network Access\n{}", leaf(na_path)),
        "res",
    );
    g.object("apm resource network-access", na_path);
    g.edge(item_id, &nid, "network access");
    let Some((_, body)) = apm.resources.get(na_path) else {
        return;
    };
    if let Some(lp) = scalar(body, "leasepool-name") {
        let lid = g.node(
            &format!("lp:{lp}"),
            &format!("Lease Pool\n{}", leaf(&lp)),
            "res",
        );
        g.object("apm resource leasepool", &lp);
        g.edge(&nid, &lid, "");
    }
    let dns: Vec<String> = ["dns-primary", "dns-secondary"]
        .iter()
        .filter_map(|k| scalar(body, k))
        .collect();
    if !dns.is_empty() {
        let did = g.node(
            &format!("{na_path}#dns"),
            &format!("Client DNS\n{}", dns.join("\n")),
            "res",
        );
        g.edge(&nid, &did, "");
    }
}

/// Draw a policy-item node (idempotent), styled by its type/caption.
fn item_node(g: &mut Graph, apm: &Apm, item_path: &str) -> String {
    let key = format!("item:{item_path}");
    if let Some(id) = g.ids.get(&key) {
        return id.clone();
    }
    let (caption, item_type) = apm.policy_items.get(item_path).map_or_else(
        || (leaf(item_path).to_string(), String::new()),
        |body| {
            (
                scalar(body, "caption").unwrap_or_else(|| leaf(item_path).to_string()),
                scalar(body, "item-type").unwrap_or_default(),
            )
        },
    );
    let class = item_class(&item_type, &caption);
    g.object("apm policy policy-item", item_path);
    g.node(&key, &caption, class)
}

// --- top-level `Node` helpers (a body is a `&[Node]`) ------------------------

fn pol_root(nodes: &[Node]) -> Node {
    // Convenience: wrap a body slice so `.scalar` / `.names_in` are reachable.
    Node {
        words: Vec::new(),
        children: nodes.iter().map(clone_node).collect(),
        has_block: true,
    }
}

fn clone_node(n: &Node) -> Node {
    Node {
        words: n.words.clone(),
        children: n.children.iter().map(clone_node).collect(),
        has_block: n.has_block,
    }
}

fn child<'a>(nodes: &'a [Node], key: &str) -> Option<&'a Node> {
    nodes
        .iter()
        .find(|n| n.words.first().map(String::as_str) == Some(key))
}

fn scalar(nodes: &[Node], key: &str) -> Option<String> {
    child(nodes, key)
        .filter(|n| n.words.len() > 1)
        .map(|n| n.words[1..].join(" "))
}

// --- graph model assembly ----------------------------------------------------

/// The `{nodes, edges}` model the client-side elkjs renderer lays out. Node
/// `cls` values map to CSS fills; the item flow is drawn left-to-right like the
/// F5 Visual Policy Editor.
fn graph_model(g: &Graph) -> J {
    let mut o = Map::new();
    o.insert("nodes".into(), J::Array(g.nodes.clone()));
    o.insert("edges".into(), J::Array(g.edges.clone()));
    J::Object(o)
}

// --- public entry ------------------------------------------------------------

/// Build the APM access-profile walk for a device: one entry per
/// `apm profile access`, each carrying its dependency-graph model and the flat
/// list of objects it links to.
pub(crate) fn collect_apm(source: &str, device: &Map<String, J>) -> J {
    let apm = index_apm(source);
    if apm.access_profiles.is_empty() {
        return J::Array(Vec::new());
    }

    // (virtual full-path, its profile full-paths) for the vs → profile links.
    let virtuals: Vec<(String, Vec<String>)> = device
        .get("virtuals")
        .and_then(J::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(J::as_object)
                .map(|vm| {
                    let fp = bstr(vm, "fullPath").to_string();
                    let profs = vm
                        .get("profiles")
                        .and_then(J::as_array)
                        .map(|p| p.iter().filter_map(J::as_str).map(str::to_owned).collect())
                        .unwrap_or_default();
                    (fp, profs)
                })
                .collect()
        })
        .unwrap_or_default();

    let mut out = Vec::new();
    for (full_path, body) in &apm.access_profiles {
        let (graph, objects) = walk_profile(&apm, full_path, body, &virtuals);
        let using: Vec<J> = virtuals
            .iter()
            .filter(|(vfp, profs)| attaches(profs, vfp, full_path))
            .map(|(vfp, _)| J::String(leaf(vfp).to_string()))
            .collect();
        let policy = scalar(body, "access-policy").unwrap_or_default();

        let mut o = Map::new();
        o.insert("name".into(), J::String(leaf(full_path).into()));
        o.insert("fullPath".into(), J::String(full_path.clone()));
        o.insert("partition".into(), J::String(partition_of(full_path)));
        o.insert("policy".into(), J::String(leaf(&policy).to_string()));
        o.insert("policyPath".into(), J::String(policy));
        o.insert("virtuals".into(), J::Array(using));
        o.insert("objectCount".into(), J::from(objects.len()));
        o.insert("objects".into(), J::Array(objects));
        o.insert("graph".into(), graph);
        out.push(J::Object(o));
    }
    J::Array(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCF: &str = include_str!("../tests/data/apm.scf");

    #[test]
    fn parses_nested_blocks() {
        let objs = top_level_objects(SCF);
        let item = objs
            .iter()
            .find(|(w, _)| {
                w.join(" ") == "apm policy policy-item /Common/mycave_act_active_directory"
            })
            .expect("policy item present");
        let body = parse_body(&item.1);
        assert_eq!(scalar(&body, "caption").as_deref(), Some("AD Auth"));
        assert_eq!(scalar(&body, "item-type").as_deref(), Some("action"));
        // The `rules { { … next-item … } }` block: two branches.
        let rules = child(&body, "rules").expect("rules block");
        let nexts: Vec<String> = rules
            .children
            .iter()
            .filter_map(|r| r.scalar("next-item"))
            .collect();
        assert_eq!(nexts.len(), 2);
        assert!(
            nexts
                .iter()
                .any(|n| n.contains("mycave_act_resource_assign"))
        );
    }

    #[test]
    fn quoted_braces_do_not_unbalance() {
        // The AD branch expression carries `{[mcget {…}] == 1}` inside quotes.
        let objs = top_level_objects(SCF);
        // Every top-level object after the rules must still be found — i.e. the
        // quote-aware scan stayed balanced through the expression.
        assert!(
            objs.iter()
                .any(|(w, _)| w.join(" ") == "apm profile access /Common/mycave")
        );
        assert!(
            objs.iter()
                .any(|(w, _)| w.join(" ") == "apm resource network-access /Common/mycave_na_res")
        );
    }

    #[test]
    fn walk_links_everything() {
        // Minimal device with the mycave virtual attaching the access profile.
        let mut vm = Map::new();
        vm.insert("fullPath".into(), J::String("/Common/mycave_vs".into()));
        vm.insert(
            "profiles".into(),
            J::Array(vec![
                J::String("/Common/mycave".into()),
                J::String("/Common/mycave_cp".into()),
            ]),
        );
        let mut device = Map::new();
        device.insert("virtuals".into(), J::Array(vec![J::Object(vm)]));

        let result = collect_apm(SCF, &device);
        let arr = result.as_array().expect("array");
        assert_eq!(arr.len(), 1);
        let p = arr[0].as_object().unwrap();
        assert_eq!(bstr(p, "name"), "mycave");
        assert_eq!(bstr(p, "policy"), "mycave");
        // The graph model carries nodes + edges for the elkjs renderer.
        let graph = &p["graph"];
        let node_labels: Vec<&str> = graph["nodes"]
            .as_array()
            .expect("nodes")
            .iter()
            .filter_map(|n| n["label"].as_str())
            .collect();
        assert!(!graph["edges"].as_array().expect("edges").is_empty());
        // The full walk reached the AAA server, the network-access resource, its
        // lease pool and a webtop — linking to everything.
        for needle in [
            "Access Profile",
            "Access Policy",
            "AAA Server",
            "Network Access",
            "Lease Pool",
            "Webtop",
            "Virtual Server",
            "Connectivity",
        ] {
            assert!(
                node_labels.iter().any(|l| l.contains(needle)),
                "missing {needle} in graph"
            );
        }
        // The connectivity profile edge came from the sibling virtual profile.
        assert!(
            p["objects"]
                .as_array()
                .unwrap()
                .iter()
                .any(|o| o["type"] == "apm resource leasepool"),
            "lease pool object linked"
        );
    }

    #[test]
    fn attaches_by_partition_relative_name() {
        // A same-partition virtual commonly attaches its profiles by bare name
        // (`profiles { mycave { } mycave_cp { } }`); the walk must still link it
        // and draw the connectivity-profile edge.
        let mut vm = Map::new();
        vm.insert("fullPath".into(), J::String("/Common/mycave_vs".into()));
        vm.insert(
            "profiles".into(),
            J::Array(vec![
                J::String("mycave".into()),
                J::String("mycave_cp".into()),
            ]),
        );
        let mut device = Map::new();
        device.insert("virtuals".into(), J::Array(vec![J::Object(vm)]));

        let result = collect_apm(SCF, &device);
        let p = result.as_array().unwrap()[0].as_object().unwrap();
        // The virtual is recognised as attaching the profile.
        assert!(
            p["virtuals"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v == "mycave_vs"),
            "relative profile ref should still link the virtual"
        );
        let types: Vec<&str> = p["objects"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|o| o["type"].as_str())
            .collect();
        assert!(types.contains(&"ltm virtual"), "virtual node present");
        assert!(
            types.contains(&"apm profile connectivity"),
            "connectivity edge resolved from the relative sibling profile"
        );
    }
}
