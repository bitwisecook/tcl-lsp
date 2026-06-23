//! Emit a sequence of `tmsh create / modify` commands from a parsed config.
//!
//! [`emit_tmsh`]
//! renders each parsed object back to a `tmsh create|modify` stanza in
//! dependency order; [`emit_tmsh_delta`] renders only the changes between
//! two configs (`create` for added, `modify` for changed, `delete` for
//! removed). The tmsh syntax (indentation, brace layout, field order,
//! list / sub-block formatting, quoting) is rendered byte-for-byte.
//!
//! Properties the typed model doesn't capture are recovered from the
//! original SCF stanza text (via the same block extractor the SCF emitter
//! uses), so iRule bodies / monitor send-recv strings / profile bodies
//! survive the round-trip.

use std::collections::HashMap;

use crate::model::{
    BigipDataGroup, BigipMonitor, BigipNode, BigipPersistence, BigipPool, BigipPoolMember,
    BigipProfile, BigipRule, BigipSnatPool, BigipVirtualServer, DataGroupType, ModelObject,
};
use crate::parser::BigipConfig;
use crate::parser::header::{ObjectTypeIndex, parse_generic_header};
use crate::parser::helpers::extract_blocks;
use crate::value::ListItemValue;
use tcl_registry::bigip::BigipRegistry;

/// The order in which kinds get emitted. Generic foundation objects come
/// first so any LTM object that references them resolves cleanly.
pub const DEFAULT_ORDER: [&str; 10] = [
    "generic-foundation",
    "node",
    "monitor",
    "data-group",
    "profile",
    "snatpool",
    "persistence",
    "pool",
    "rule",
    "virtual",
];

/// `(module, object_type)` pairs that must be created before any LTM object.
const FOUNDATION_TYPES: [(&str, &str); 6] = [
    ("auth", "partition"),
    ("net", "route-domain"),
    ("net", "vlan"),
    ("net", "self"),
    ("net", "trunk"),
    ("net", "interface"),
];

fn is_foundation(module: &str, object_type: &str) -> bool {
    FOUNDATION_TYPES.contains(&(module, object_type))
}

/// The rendered tmsh script plus a command count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TmshScript {
    /// The full tmsh script text (trailing newline when non-empty).
    pub text: String,
    /// Number of emitted commands.
    pub command_count: usize,
}

// ── Block index ─────────────────────────────────────────────────────────

/// `(module, object_type, identifier)` → full stanza text.
fn index_blocks(source: &str) -> HashMap<(String, String, String), String> {
    let mut out = HashMap::new();
    if source.is_empty() {
        return out;
    }
    let registry = BigipRegistry::build();
    let index = ObjectTypeIndex::build(&registry);
    let bytes = source.as_bytes();
    for block in extract_blocks(source) {
        let Some((module, object_type, identifier)) = parse_generic_header(&block.header, &index)
        else {
            continue;
        };
        // Full stanza text including its header line.
        let mut header_start = block.start_offset;
        while header_start > 0 && bytes[header_start - 1] != b'\n' {
            header_start -= 1;
        }
        let text = source[header_start..block.end_offset].to_owned();
        out.insert((module, object_type, identifier), text);
    }
    out
}

// ── Per-kind container accessors ─────────────────────────────────────────

macro_rules! kind_iter {
    ($cfg:expr, $table:expr, $variant:path) => {{
        $cfg.objects.iter().filter_map(|p| {
            if p.table_name == $table {
                if let $variant(obj) = &p.object {
                    return Some(obj);
                }
            }
            None
        })
    }};
}

fn nodes(cfg: &BigipConfig) -> impl Iterator<Item = &BigipNode> {
    kind_iter!(cfg, "nodes", ModelObject::Node)
}
fn monitors(cfg: &BigipConfig) -> impl Iterator<Item = &BigipMonitor> {
    kind_iter!(cfg, "monitors", ModelObject::Monitor)
}
fn data_groups(cfg: &BigipConfig) -> impl Iterator<Item = &BigipDataGroup> {
    kind_iter!(cfg, "data_groups", ModelObject::DataGroup)
}
fn profiles(cfg: &BigipConfig) -> impl Iterator<Item = &BigipProfile> {
    kind_iter!(cfg, "profiles", ModelObject::Profile)
}
fn snat_pools(cfg: &BigipConfig) -> impl Iterator<Item = &BigipSnatPool> {
    kind_iter!(cfg, "snat_pools", ModelObject::SnatPool)
}
fn persistences(cfg: &BigipConfig) -> impl Iterator<Item = &BigipPersistence> {
    kind_iter!(cfg, "persistence", ModelObject::Persistence)
}
fn pools(cfg: &BigipConfig) -> impl Iterator<Item = &BigipPool> {
    kind_iter!(cfg, "pools", ModelObject::Pool)
}
fn rules(cfg: &BigipConfig) -> impl Iterator<Item = &BigipRule> {
    kind_iter!(cfg, "rules", ModelObject::Rule)
}
fn virtuals(cfg: &BigipConfig) -> impl Iterator<Item = &BigipVirtualServer> {
    kind_iter!(cfg, "virtual_servers", ModelObject::VirtualServer)
}

/// The typed pool members carried by a pool's `members` `BigipList`.
fn pool_members(pool: &BigipPool) -> Vec<&BigipPoolMember> {
    pool.members
        .items
        .iter()
        .filter_map(|item| match &item.value {
            ListItemValue::PoolMember(m) => Some(m),
            _ => None,
        })
        .collect()
}

/// The string members carried by a snatpool's `members` `BigipList`.
fn snatpool_members(sp: &BigipSnatPool) -> Vec<String> {
    sp.members.paths()
}

// ── emit_tmsh ───────────────────────────────────────────────────────────

/// Render `cfg` as a tmsh script.
///
/// `source` is the original SCF text, used to recover stanza bodies for
/// objects the model doesn't fully capture. `include` narrows the kinds
/// emitted. `use_modify_for_existing` swaps `create` for `modify`.
#[must_use]
pub fn emit_tmsh(
    cfg: &BigipConfig,
    source: &str,
    include: &[&str],
    use_modify_for_existing: bool,
) -> TmshScript {
    let mut lines: Vec<String> = Vec::new();
    let verb = if use_modify_for_existing {
        "modify"
    } else {
        "create"
    };
    let blocks = index_blocks(source);
    let has = |k: &str| include.contains(&k);

    if has("generic-foundation") {
        for (_key, obj) in &cfg.generic_objects {
            if !is_foundation(&obj.module, &obj.object_type) {
                continue;
            }
            lines.push(render_generic(
                verb,
                &obj.module,
                &obj.object_type,
                &obj.identifier,
                &blocks,
            ));
        }
    }
    if has("node") {
        for node in nodes(cfg) {
            lines.push(render_node(verb, node));
        }
    }
    if has("monitor") {
        for monitor in monitors(cfg) {
            lines.push(render_monitor(verb, monitor, &blocks));
        }
    }
    if has("data-group") {
        for dg in data_groups(cfg) {
            lines.push(render_data_group(verb, dg));
        }
    }
    if has("profile") {
        for profile in profiles(cfg) {
            lines.push(render_profile(verb, profile, &blocks));
        }
    }
    if has("snatpool") {
        for sp in snat_pools(cfg) {
            lines.push(render_snatpool(verb, sp));
        }
    }
    if has("persistence") {
        for pers in persistences(cfg) {
            lines.push(render_persistence(verb, pers, &blocks));
        }
    }
    if has("pool") {
        for pool in pools(cfg) {
            lines.push(render_pool(verb, pool));
        }
    }
    if has("rule") {
        for rule in rules(cfg) {
            lines.push(render_rule(verb, rule));
        }
    }
    if has("virtual") {
        for vs in virtuals(cfg) {
            lines.push(render_virtual(verb, vs));
        }
    }

    finish(&lines)
}

fn finish(lines: &[String]) -> TmshScript {
    let count = lines.len();
    let text = if lines.is_empty() {
        String::new()
    } else {
        let mut t = lines.join("\n");
        t.push('\n');
        t
    };
    TmshScript {
        text,
        command_count: count,
    }
}

// ── emit_tmsh_delta ─────────────────────────────────────────────────────

/// Emit only the *changed* objects between `old_cfg` and `new_cfg`.
#[must_use]
pub fn emit_tmsh_delta(
    old_cfg: &BigipConfig,
    new_cfg: &BigipConfig,
    old_source: &str,
    new_source: &str,
    include: &[&str],
) -> TmshScript {
    let new_blocks = index_blocks(new_source);
    let old_blocks = index_blocks(old_source);

    let changed = |key: &(String, String, String)| -> bool {
        new_blocks.get(key).map_or("", String::as_str)
            != old_blocks.get(key).map_or("", String::as_str)
    };

    let mut lines: Vec<String> = Vec::new();
    let has = |k: &str| include.contains(&k);

    // ── 1. Modifies (foundation first, then dependency order) ──────────
    if has("generic-foundation") {
        for (gkey, obj) in &new_cfg.generic_objects {
            if !is_foundation(&obj.module, &obj.object_type) {
                continue;
            }
            let key = (
                obj.module.clone(),
                obj.object_type.clone(),
                obj.identifier.clone(),
            );
            // Match by the same generic_objects key (module::type::id).
            let old_present = old_cfg.generic_objects.iter().any(|(k, _)| k == gkey);
            if old_present && changed(&key) {
                lines.push(render_generic(
                    "modify",
                    &obj.module,
                    &obj.object_type,
                    &obj.identifier,
                    &new_blocks,
                ));
            }
        }
    }

    for kind_name in KIND_TABLE {
        if !has(kind_name) {
            continue;
        }
        delta_modifies(
            kind_name,
            old_cfg,
            new_cfg,
            new_source,
            &new_blocks,
            &changed,
            &mut lines,
        );
    }

    // ── 2. Deletes (reverse dependency order) ──────────────────────────
    for kind_name in include.iter().rev() {
        if *kind_name == "generic-foundation" {
            continue;
        }
        delta_deletes(kind_name, old_cfg, new_cfg, &mut lines);
    }
    // Generic foundation deletes, emitted after the per-kind deletes.
    if has("generic-foundation") {
        for (key, obj) in &old_cfg.generic_objects {
            if !is_foundation(&obj.module, &obj.object_type) {
                continue;
            }
            if new_cfg.generic_objects.iter().any(|(k, _)| k == key) {
                continue;
            }
            lines.push(format!(
                "tmsh delete {} {} {}",
                obj.module, obj.object_type, obj.identifier
            ));
        }
    }

    // ── 3. Creates (foundation first, then dependency order) ───────────
    if has("generic-foundation") {
        for (key, obj) in &new_cfg.generic_objects {
            if !is_foundation(&obj.module, &obj.object_type) {
                continue;
            }
            if old_cfg.generic_objects.iter().any(|(k, _)| k == key) {
                continue;
            }
            lines.push(render_generic(
                "create",
                &obj.module,
                &obj.object_type,
                &obj.identifier,
                &new_blocks,
            ));
        }
    }
    for kind_name in KIND_TABLE {
        if !has(kind_name) {
            continue;
        }
        delta_creates(
            kind_name,
            old_cfg,
            new_cfg,
            new_source,
            &new_blocks,
            &mut lines,
        );
    }

    finish(&lines)
}

/// Kinds walked in dependency order by the delta emitter.
const KIND_TABLE: [&str; 9] = [
    "node",
    "monitor",
    "data-group",
    "profile",
    "snatpool",
    "persistence",
    "pool",
    "rule",
    "virtual",
];

/// `(module, object_type)` block-index key for a kind.
fn block_kind(kind_name: &str) -> (&'static str, &'static str) {
    match kind_name {
        "node" => ("ltm", "node"),
        "monitor" => ("ltm", "monitor"),
        "data-group" => ("ltm", "data-group"),
        "profile" => ("ltm", "profile"),
        "snatpool" => ("ltm", "snatpool"),
        "persistence" => ("ltm", "persistence"),
        "pool" => ("ltm", "pool"),
        "rule" => ("ltm", "rule"),
        "virtual" => ("ltm", "virtual"),
        _ => ("ltm", ""),
    }
}

/// Ordered `(full_path, rendered)` for a kind, indexed by `full_path`.
fn collect_kind<'a>(cfg: &'a BigipConfig, kind_name: &str) -> Vec<(String, ObjRef<'a>)> {
    match kind_name {
        "node" => nodes(cfg)
            .map(|o| (o.full_path.clone(), ObjRef::Node(o)))
            .collect(),
        "monitor" => monitors(cfg)
            .map(|o| (o.full_path.clone(), ObjRef::Monitor(o)))
            .collect(),
        "data-group" => data_groups(cfg)
            .map(|o| (o.full_path.clone(), ObjRef::DataGroup(o)))
            .collect(),
        "profile" => profiles(cfg)
            .map(|o| (o.full_path.clone(), ObjRef::Profile(o)))
            .collect(),
        "snatpool" => snat_pools(cfg)
            .map(|o| (o.full_path.clone(), ObjRef::SnatPool(o)))
            .collect(),
        "persistence" => persistences(cfg)
            .map(|o| (o.full_path.clone(), ObjRef::Persistence(o)))
            .collect(),
        "pool" => pools(cfg)
            .map(|o| (o.full_path.clone(), ObjRef::Pool(o)))
            .collect(),
        "rule" => rules(cfg)
            .map(|o| (o.full_path.clone(), ObjRef::Rule(o)))
            .collect(),
        "virtual" => virtuals(cfg)
            .map(|o| (o.full_path.clone(), ObjRef::Virtual(o)))
            .collect(),
        _ => Vec::new(),
    }
}

enum ObjRef<'a> {
    Node(&'a BigipNode),
    Monitor(&'a BigipMonitor),
    DataGroup(&'a BigipDataGroup),
    Profile(&'a BigipProfile),
    SnatPool(&'a BigipSnatPool),
    Persistence(&'a BigipPersistence),
    Pool(&'a BigipPool),
    Rule(&'a BigipRule),
    Virtual(&'a BigipVirtualServer),
}

impl ObjRef<'_> {
    fn full_path(&self) -> &str {
        match self {
            ObjRef::Node(o) => &o.full_path,
            ObjRef::Monitor(o) => &o.full_path,
            ObjRef::DataGroup(o) => &o.full_path,
            ObjRef::Profile(o) => &o.full_path,
            ObjRef::SnatPool(o) => &o.full_path,
            ObjRef::Persistence(o) => &o.full_path,
            ObjRef::Pool(o) => &o.full_path,
            ObjRef::Rule(o) => &o.full_path,
            ObjRef::Virtual(o) => &o.full_path,
        }
    }
    fn render(&self, verb: &str, blocks: &HashMap<(String, String, String), String>) -> String {
        match self {
            ObjRef::Node(o) => render_node(verb, o),
            ObjRef::Monitor(o) => render_monitor(verb, o, blocks),
            ObjRef::DataGroup(o) => render_data_group(verb, o),
            ObjRef::Profile(o) => render_profile(verb, o, blocks),
            ObjRef::SnatPool(o) => render_snatpool(verb, o),
            ObjRef::Persistence(o) => render_persistence(verb, o, blocks),
            ObjRef::Pool(o) => render_pool(verb, o),
            ObjRef::Rule(o) => render_rule(verb, o),
            ObjRef::Virtual(o) => render_virtual(verb, o),
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn delta_modifies(
    kind_name: &str,
    old_cfg: &BigipConfig,
    new_cfg: &BigipConfig,
    _new_source: &str,
    new_blocks: &HashMap<(String, String, String), String>,
    changed: &dyn Fn(&(String, String, String)) -> bool,
    lines: &mut Vec<String>,
) {
    let (module, object_type) = block_kind(kind_name);
    let new_objs = collect_kind(new_cfg, kind_name);
    let old_paths: std::collections::HashSet<String> = collect_kind(old_cfg, kind_name)
        .into_iter()
        .map(|(p, _)| p)
        .collect();
    let old_objs = collect_kind(old_cfg, kind_name);

    for (full_path, obj) in &new_objs {
        if !old_paths.contains(full_path) {
            continue; // creates handled in the creates pass
        }
        let key = (module.to_owned(), object_type.to_owned(), full_path.clone());
        if !changed(&key) {
            continue;
        }
        // Pools: emit granular member add/delete/modify commands.
        if kind_name == "pool"
            && let (ObjRef::Pool(new_pool), Some(ObjRef::Pool(old_pool))) = (
                obj,
                old_objs
                    .iter()
                    .find(|(p, _)| p == full_path)
                    .map(|(_, o)| o),
            )
            && let Some(member_lines) = emit_pool_member_delta(old_pool, new_pool)
        {
            for l in member_lines {
                lines.push(l);
            }
            if !pool_non_member_changed(old_pool, new_pool) {
                continue;
            }
        }
        lines.push(obj.render("modify", new_blocks));
    }
}

fn delta_deletes(
    kind_name: &str,
    old_cfg: &BigipConfig,
    new_cfg: &BigipConfig,
    lines: &mut Vec<String>,
) {
    let new_paths: std::collections::HashSet<String> = collect_kind(new_cfg, kind_name)
        .into_iter()
        .map(|(p, _)| p)
        .collect();
    let spelt = match kind_name {
        "node" => "ltm node",
        "monitor" => "ltm monitor",
        "data-group" => "ltm data-group",
        "profile" => "ltm profile",
        "snatpool" => "ltm snatpool",
        "persistence" => "ltm persistence",
        "pool" => "ltm pool",
        "rule" => "ltm rule",
        "virtual" => "ltm virtual",
        _ => return,
    };
    for (full_path, _obj) in collect_kind(old_cfg, kind_name) {
        if new_paths.contains(&full_path) {
            continue;
        }
        lines.push(format!("tmsh delete {spelt} {full_path}"));
    }
}

fn delta_creates(
    kind_name: &str,
    old_cfg: &BigipConfig,
    new_cfg: &BigipConfig,
    _new_source: &str,
    new_blocks: &HashMap<(String, String, String), String>,
    lines: &mut Vec<String>,
) {
    let old_paths: std::collections::HashSet<String> = collect_kind(old_cfg, kind_name)
        .into_iter()
        .map(|(p, _)| p)
        .collect();
    for (full_path, obj) in collect_kind(new_cfg, kind_name) {
        if old_paths.contains(&full_path) {
            continue; // modify, already emitted in the modifies pass
        }
        let _ = obj.full_path();
        lines.push(obj.render("create", new_blocks));
    }
}

// ── pool member delta ───────────────────────────────────────────────────

/// Granular `tmsh modify ltm pool X members <op> { ... }` lines, or `None`
/// when the member list is identical.
fn emit_pool_member_delta(old_pool: &BigipPool, new_pool: &BigipPool) -> Option<Vec<String>> {
    let old_members = pool_members(old_pool);
    let new_members = pool_members(new_pool);
    let old_by_name: HashMap<&str, &BigipPoolMember> =
        old_members.iter().map(|m| (m.name.as_str(), *m)).collect();
    let new_by_name: HashMap<&str, &BigipPoolMember> =
        new_members.iter().map(|m| (m.name.as_str(), *m)).collect();

    let added: Vec<&BigipPoolMember> = new_members
        .iter()
        .filter(|m| !old_by_name.contains_key(m.name.as_str()))
        .copied()
        .collect();
    let removed: Vec<&BigipPoolMember> = old_members
        .iter()
        .filter(|m| !new_by_name.contains_key(m.name.as_str()))
        .copied()
        .collect();
    let mut modified: Vec<&BigipPoolMember> = Vec::new();
    for new_m in &new_members {
        if let Some(old_m) = old_by_name.get(new_m.name.as_str())
            && pool_member_body(old_m) != pool_member_body(new_m)
        {
            modified.push(new_m);
        }
    }

    if added.is_empty() && removed.is_empty() && modified.is_empty() {
        return None;
    }

    let head = format!("tmsh modify ltm pool {}", new_pool.full_path);
    let mut lines = Vec::new();
    if !added.is_empty() {
        let body = added
            .iter()
            .map(|m| render_member_with_body(m))
            .collect::<Vec<_>>()
            .join(" ");
        lines.push(format!("{head} members add {{ {body} }}"));
    }
    if !removed.is_empty() {
        let names = removed
            .iter()
            .map(|m| m.name.clone())
            .collect::<Vec<_>>()
            .join(" ");
        lines.push(format!("{head} members delete {{ {names} }}"));
    }
    if !modified.is_empty() {
        let body = modified
            .iter()
            .map(|m| render_member_with_body(m))
            .collect::<Vec<_>>()
            .join(" ");
        lines.push(format!("{head} members modify {{ {body} }}"));
    }
    Some(lines)
}

/// Render one member's name + `{ ... }` body for tmsh list ops.
fn render_member_with_body(member: &BigipPoolMember) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(addr) = &member.address {
        parts.push(format!("address {addr}"));
    }
    if !member.monitor.is_empty() {
        parts.push(format!("monitor {}", quote(&member.monitor)));
    }
    if !member.state.is_empty() && member.state != "enabled" {
        parts.push(format!("state {}", member.state));
    }
    if !member.ratio.is_empty() {
        parts.push(format!("ratio {}", member.ratio));
    }
    if !member.priority_group.is_empty() {
        parts.push(format!("priority-group {}", member.priority_group));
    }
    if !member.connection_limit.is_empty() {
        parts.push(format!("connection-limit {}", member.connection_limit));
    }
    if !member.rate_limit.is_empty() {
        parts.push(format!("rate-limit {}", member.rate_limit));
    }
    let body = parts.join(" ");
    if body.is_empty() {
        format!("{} {{ }}", member.name)
    } else {
        format!("{} {{ {body} }}", member.name)
    }
}

/// Hashable representation of a member's body for change detection.
fn pool_member_body(
    member: &BigipPoolMember,
) -> (String, &str, &str, &str, &str, &str, &str, &str) {
    let addr = member
        .address
        .as_ref()
        .map_or_else(String::new, ToString::to_string);
    (
        addr,
        &member.monitor,
        &member.state,
        &member.ratio,
        &member.priority_group,
        &member.connection_limit,
        &member.rate_limit,
        &member.description,
    )
}

/// True when any non-`members` pool field differs (skips `members` + `range`).
fn pool_non_member_changed(old_pool: &BigipPool, new_pool: &BigipPool) -> bool {
    let mut a = old_pool.clone();
    let mut b = new_pool.clone();
    a.members = crate::value::BigipList::default();
    b.members = crate::value::BigipList::default();
    a.range = None;
    b.range = None;
    a != b
}

// ── Renderers ───────────────────────────────────────────────────────────

/// Quote `value` for tmsh if it contains whitespace or braces.
fn quote(value: &str) -> String {
    if value.is_empty() {
        return "\"\"".to_owned();
    }
    if value
        .chars()
        .any(|ch| matches!(ch, ' ' | '\t' | '{' | '}' | '"'))
    {
        let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
        return format!("\"{escaped}\"");
    }
    value.to_owned()
}

fn render_node(verb: &str, node: &BigipNode) -> String {
    let mut fields = Vec::new();
    if let Some(addr) = &node.address {
        fields.push(format!("address {addr}"));
    }
    let body = fields.join(" ");
    if body.is_empty() {
        format!("tmsh {verb} ltm node {}", node.full_path)
    } else {
        format!("tmsh {verb} ltm node {} {{ {body} }}", node.full_path)
    }
}

fn render_pool(verb: &str, pool: &BigipPool) -> String {
    let mut fields: Vec<String> = Vec::new();
    if !pool.load_balancing_mode.is_empty() {
        fields.push(format!("load-balancing-mode {}", pool.load_balancing_mode));
    }
    if !pool.monitor.is_empty() {
        fields.push(format!("monitor {}", quote(&pool.monitor)));
    }
    let members = pool_members(pool);
    if !members.is_empty() {
        let mut member_lines = Vec::new();
        for m in &members {
            let mut inner = Vec::new();
            if let Some(addr) = &m.address {
                inner.push(format!("address {addr}"));
            }
            if !m.monitor.is_empty() {
                inner.push(format!("monitor {}", quote(&m.monitor)));
            }
            let inner_str = inner.join(" ");
            if inner_str.is_empty() {
                member_lines.push(format!("{} {{ }}", m.name));
            } else {
                member_lines.push(format!("{} {{ {inner_str} }}", m.name));
            }
        }
        fields.push(list_field("members", &member_lines.join(" ")));
    }
    let body = fields.join(" ");
    if body.is_empty() {
        format!("tmsh {verb} ltm pool {}", pool.full_path)
    } else {
        format!("tmsh {verb} ltm pool {} {{ {body} }}", pool.full_path)
    }
}

/// Render a list-valued field's brace body. The registry returns
/// `replace-all-with` for every list property the emitter touches (pool
/// members, data-group records, snatpool members, virtual
/// profiles/rules/persist), so the operator is constant here.
fn list_field(name: &str, items_text: &str) -> String {
    format!("{name} replace-all-with {{ {items_text} }}")
}

fn render_monitor(
    verb: &str,
    monitor: &BigipMonitor,
    blocks: &HashMap<(String, String, String), String>,
) -> String {
    let monitor_type = if monitor.monitor_type.is_empty() {
        "tcp"
    } else {
        &monitor.monitor_type
    };
    let text = blocks
        .get(&(
            "ltm".to_owned(),
            format!("monitor {monitor_type}"),
            monitor.full_path.clone(),
        ))
        .or_else(|| {
            blocks.get(&(
                "ltm".to_owned(),
                "monitor".to_owned(),
                monitor.full_path.clone(),
            ))
        })
        .map_or("", String::as_str);
    let props = extract_block_properties(text);
    let body = props
        .iter()
        .map(|(k, v)| format!("{k} {}", quote(v)))
        .collect::<Vec<_>>()
        .join(" ");
    let head = format!(
        "tmsh {verb} ltm monitor {monitor_type} {}",
        monitor.full_path
    );
    if body.is_empty() {
        head
    } else {
        format!("{head} {{ {body} }}")
    }
}

fn render_data_group(verb: &str, dg: &BigipDataGroup) -> String {
    let kind = if dg.kind == DataGroupType::Internal {
        "internal"
    } else {
        "external"
    };
    let mut parts: Vec<String> = Vec::new();
    if !dg.value_type.is_empty() {
        parts.push(format!("type {}", dg.value_type));
    }
    if !dg.records.is_empty() {
        let record_block = dg
            .records
            .iter()
            .map(|r| format!("{r} {{ }}"))
            .collect::<Vec<_>>()
            .join(" ");
        parts.push(list_field("records", &record_block));
    }
    let body = parts.join(" ");
    let head = format!("tmsh {verb} ltm data-group {kind} {}", dg.full_path);
    if body.is_empty() {
        head
    } else {
        format!("{head} {{ {body} }}")
    }
}

fn render_profile(
    verb: &str,
    profile: &BigipProfile,
    blocks: &HashMap<(String, String, String), String>,
) -> String {
    let profile_type = profile_type_str(profile);
    let body = passthrough_body(
        blocks,
        "ltm",
        &format!("profile {profile_type}"),
        &profile.full_path,
    );
    let head = format!(
        "tmsh {verb} ltm profile {profile_type} {} ",
        profile.full_path
    );
    format!("{head}{body}").trim_end().to_owned()
}

fn render_snatpool(verb: &str, sp: &BigipSnatPool) -> String {
    let members = snatpool_members(sp);
    let body = if members.is_empty() {
        String::new()
    } else {
        list_field("members", &members.join(" "))
    };
    if body.is_empty() {
        format!("tmsh {verb} ltm snatpool {}", sp.full_path)
    } else {
        format!("tmsh {verb} ltm snatpool {} {{ {body} }}", sp.full_path)
    }
}

fn render_persistence(
    verb: &str,
    pers: &BigipPersistence,
    blocks: &HashMap<(String, String, String), String>,
) -> String {
    let persistence_type = if pers.persistence_type.is_empty() {
        "source-addr"
    } else {
        &pers.persistence_type
    };
    let body = passthrough_body(
        blocks,
        "ltm",
        &format!("persistence {persistence_type}"),
        &pers.full_path,
    );
    let head = format!(
        "tmsh {verb} ltm persistence {persistence_type} {} ",
        pers.full_path
    );
    format!("{head}{body}").trim_end().to_owned()
}

fn render_rule(verb: &str, rule: &BigipRule) -> String {
    let body = rule.source.trim();
    format!("tmsh {verb} ltm rule {} {{\n{body}\n}}", rule.full_path)
}

fn render_virtual(verb: &str, vs: &BigipVirtualServer) -> String {
    let mut fields: Vec<String> = Vec::new();
    if let Some(dest) = &vs.destination {
        fields.push(format!("destination {dest}"));
    }
    if !vs.pool.is_empty() {
        fields.push(format!("pool {}", vs.pool));
    }
    if !vs.profiles.is_empty() {
        let items = vs
            .profiles
            .paths()
            .iter()
            .map(|p| format!("{p} {{ }}"))
            .collect::<Vec<_>>()
            .join(" ");
        fields.push(list_field("profiles", &items));
    }
    if !vs.rules.is_empty() {
        fields.push(list_field("rules", &vs.rules.paths().join(" ")));
    }
    if !vs.persist.is_empty() {
        let items = vs
            .persist
            .paths()
            .iter()
            .map(|p| format!("{p} {{ }}"))
            .collect::<Vec<_>>()
            .join(" ");
        fields.push(list_field("persist", &items));
    }
    if !vs.snatpool.is_empty() {
        fields.push(format!("snatpool {}", vs.snatpool));
    }
    if !vs.source_address_translation.is_empty() {
        fields.push(format!(
            "source-address-translation {{ type {} }}",
            vs.source_address_translation
        ));
    }
    let body = fields.join(" ");
    if body.is_empty() {
        format!("tmsh {verb} ltm virtual {}", vs.full_path)
    } else {
        format!("tmsh {verb} ltm virtual {} {{ {body} }}", vs.full_path)
    }
}

fn render_generic(
    verb: &str,
    module: &str,
    object_type: &str,
    identifier: &str,
    blocks: &HashMap<(String, String, String), String>,
) -> String {
    let body_text = passthrough_body(blocks, module, object_type, identifier);
    let head = format!("tmsh {verb} {module} {object_type} {identifier}")
        .trim_end()
        .to_owned();
    if body_text.is_empty() {
        head
    } else {
        format!("{head} {body_text}").trim_end().to_owned()
    }
}

/// Return the original `{ ... }` body of a block, collapsed to one line.
fn passthrough_body(
    blocks: &HashMap<(String, String, String), String>,
    module: &str,
    object_type: &str,
    identifier: &str,
) -> String {
    let Some(text) = blocks.get(&(
        module.to_owned(),
        object_type.to_owned(),
        identifier.to_owned(),
    )) else {
        return String::new();
    };
    let Some(inner) = inner_brace_body(text) else {
        return String::new();
    };
    let inner = inner.trim();
    if inner.is_empty() {
        return String::new();
    }
    let collapsed = collapse_whitespace(inner);
    format!("{{ {collapsed} }}")
}

/// Top-level prop extractor for monitor stanzas (no nesting). Preserves
/// source order.
fn extract_block_properties(text: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    if text.is_empty() {
        return out;
    }
    let Some(inner) = inner_brace_body(text) else {
        return out;
    };
    for raw_line in inner.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.contains('{') || line.contains('}') {
            continue;
        }
        if let Some(idx) = line.find(char::is_whitespace) {
            let key = line[..idx].to_owned();
            let value = line[idx..].trim_start().trim().to_owned();
            upsert(&mut out, key, value);
        } else {
            upsert(&mut out, line.to_owned(), String::new());
        }
    }
    out
}

/// Insert or overwrite `key` (last-wins on value, key keeps first position).
fn upsert(out: &mut Vec<(String, String)>, key: String, value: String) {
    if let Some(slot) = out.iter_mut().find(|(k, _)| *k == key) {
        slot.1 = value;
    } else {
        out.push((key, value));
    }
}

/// The text between the *last* top-level `{` … `}` at the end of `text`.
fn inner_brace_body(text: &str) -> Option<String> {
    let open = text.find('{')?;
    let close = text.rfind('}')?;
    if close < open {
        return None;
    }
    // Require only whitespace after the closing brace (the `\s*\Z` anchor).
    if !text[close + 1..].trim().is_empty() {
        return None;
    }
    Some(text[open + 1..close].to_owned())
}

/// Collapse all runs of whitespace to a single space (`\s+` → ` `).
fn collapse_whitespace(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_ws = false;
    for ch in s.chars() {
        if ch.is_whitespace() {
            if !in_ws {
                out.push(' ');
                in_ws = true;
            }
        } else {
            out.push(ch);
            in_ws = false;
        }
    }
    out
}

/// tmsh profile-type spelling: the `ProfileType` member name lowercased
/// with `_` → `-`.
fn profile_type_str(profile: &BigipProfile) -> String {
    profile
        .profile_type
        .py_name()
        .to_lowercase()
        .replace('_', "-")
}
