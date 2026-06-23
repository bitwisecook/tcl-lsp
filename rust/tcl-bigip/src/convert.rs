//! SCF → AS3 declaration emitter (best-effort, partial coverage).
//!
//! AS3 (F5 Application Services 3)
//! is a declarative API for BIG-IP configuration. This converts the most common
//! LTM objects from a parsed [`BigipConfig`] into an AS3 `Application`; anything
//! without a clear AS3 mapping is recorded in [`AS3ConversionReport::unmapped`]
//! so nothing silently disappears.
//!
//! The declaration is built as an insertion-ordered [`Json`] tree and rendered
//! with [`crate::jsonfmt::json_string`] for `json.dumps(indent=2)` byte-parity.

use std::net::IpAddr;
use std::str::FromStr;

use crate::jsonfmt::json_string;
use crate::model::{ModelObject, ProfileType};
use crate::parser::BigipConfig;
use crate::value::bigip_list::ListItemValue;

/// A minimal insertion-ordered JSON value. Object keys preserve insertion
/// order.
#[derive(Debug, Clone)]
pub enum Json {
    /// A JSON string.
    Str(String),
    /// A JSON integer.
    Int(i64),
    /// A JSON array.
    Array(Vec<Json>),
    /// A JSON object (insertion-ordered key/value pairs).
    Object(Vec<(String, Json)>),
}

impl Json {
    fn write(&self, out: &mut String, level: usize) {
        match self {
            Json::Str(s) => out.push_str(&json_string(s)),
            Json::Int(i) => out.push_str(&i.to_string()),
            Json::Array(items) => {
                if items.is_empty() {
                    out.push_str("[]");
                    return;
                }
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    indent(out, level + 1);
                    item.write(out, level + 1);
                }
                indent(out, level);
                out.push(']');
            }
            Json::Object(entries) => {
                if entries.is_empty() {
                    out.push_str("{}");
                    return;
                }
                out.push('{');
                for (i, (key, val)) in entries.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    indent(out, level + 1);
                    out.push_str(&json_string(key));
                    out.push_str(": ");
                    val.write(out, level + 1);
                }
                indent(out, level);
                out.push('}');
            }
        }
    }

    /// Serialise this value as `json.dumps(value, indent=2)` would (no trailing
    /// newline). `ensure_ascii=True` escaping is handled by `json_string`.
    #[must_use]
    pub fn dumps_indent2(&self) -> String {
        let mut out = String::new();
        self.write(&mut out, 0);
        out
    }
}

fn indent(out: &mut String, level: usize) {
    out.push('\n');
    for _ in 0..level * 2 {
        out.push(' ');
    }
}

/// Result of [`scf_to_as3`]: the AS3 declaration plus a coverage report of how
/// many objects mapped and which were left unmapped.
#[derive(Debug, Clone)]
pub struct AS3ConversionReport {
    /// The AS3 declaration tree (root `{"class": "AS3", ...}`).
    pub declaration: Json,
    /// Count of objects that produced an AS3 class.
    pub mapped: usize,
    /// `kind:full-path` tags for every object with no AS3 mapping.
    pub unmapped: Vec<String>,
}

fn short_name(full_path: &str) -> &str {
    if full_path.is_empty() {
        ""
    } else {
        full_path.rsplit_once('/').map_or(full_path, |(_, n)| n)
    }
}

/// Parse `/Common/ADDR[:.]PORT` -> `(addr, port)`.
///
/// BIG-IP separates address and port with `:` for IPv4 and `.` for IPv6. The
/// port half may be a decimal port or a service name. The IPv6 form is checked
/// first and only accepted when the prefix parses as an IPv6 address.
fn split_destination(dest: &str) -> (String, Option<i64>) {
    if dest.is_empty() {
        return (String::new(), None);
    }
    let base = dest.rsplit_once('/').map_or(dest, |(_, b)| b);
    if base.contains('.') {
        let (host, port) = base.rsplit_once('.').unwrap_or((base, ""));
        if let Some(resolved) = resolve_port(port)
            && !host.is_empty()
            && matches!(IpAddr::from_str(host), Ok(IpAddr::V6(_)))
        {
            return (host.to_owned(), Some(resolved));
        }
    }
    if base.contains(':') {
        let (host, port) = base.rsplit_once(':').unwrap_or((base, ""));
        if let Some(resolved) = resolve_port(port)
            && !host.is_empty()
        {
            return (host.to_owned(), Some(resolved));
        }
    }
    (base.to_owned(), None)
}

/// Resolve a port token to an integer (decimal string or BIG-IP service name).
fn resolve_port(value: &str) -> Option<i64> {
    if value.is_empty() {
        return None;
    }
    if value.bytes().all(|b| b.is_ascii_digit()) {
        let num: i64 = value.parse().ok()?;
        return (0..=65535).contains(&num).then_some(num);
    }
    crate::model::port_names::name_to_port(&value.to_ascii_lowercase())
}

/// Look up a typed object by table and full-path.
fn find<'a>(cfg: &'a BigipConfig, table: &str, full_path: &str) -> Option<&'a ModelObject> {
    cfg.objects
        .iter()
        .find(|p| p.table_name == table && p.full_path == full_path)
        .map(|p| &p.object)
}

/// Iterate the full-paths of a typed table in source order.
fn table_paths<'a>(cfg: &'a BigipConfig, table: &str) -> impl Iterator<Item = &'a str> {
    cfg.objects
        .iter()
        .filter(move |p| p.table_name == table)
        .map(|p| p.full_path.as_str())
}

/// `resolve_name`: resolve a possibly-short name to a full path within `table`.
fn resolve_name(cfg: &BigipConfig, name: &str, table: &str) -> Option<String> {
    if find(cfg, table, name).is_some() {
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
            if find(cfg, table, &candidate).is_some() {
                return Some(candidate);
            }
        }
        if partition != "Common" {
            let candidate = format!("/Common/{name}");
            if find(cfg, table, &candidate).is_some() {
                return Some(candidate);
            }
        }
    }
    let suffix = format!("/{name}");
    table_paths(cfg, table)
        .find(|key| key.ends_with(&suffix))
        .map(ToOwned::to_owned)
}

/// `profile_types_for_virtual`: the set of profile types attached to `vs`.
fn profile_types_for_virtual(
    cfg: &BigipConfig,
    vs: &crate::model::BigipVirtualServer,
) -> Vec<ProfileType> {
    let mut out = Vec::new();
    for pref in vs.profiles.paths() {
        if let Some(resolved) = resolve_name(cfg, &pref, "profiles")
            && let Some(ModelObject::Profile(p)) = find(cfg, "profiles", &resolved)
            && !out.contains(&p.profile_type)
        {
            out.push(p.profile_type);
        }
    }
    out
}

fn classify_virtual_class(
    cfg: &BigipConfig,
    vs: &crate::model::BigipVirtualServer,
) -> &'static str {
    let types = profile_types_for_virtual(cfg, vs);
    if types.contains(&ProfileType::Http) {
        return "Service_HTTP";
    }
    if types.contains(&ProfileType::ClientSsl) {
        return "Service_HTTPS";
    }
    if types.contains(&ProfileType::Udp) {
        return "Service_UDP";
    }
    if types.contains(&ProfileType::Tcp) {
        return "Service_TCP";
    }
    "Service_L4"
}

fn convert_pool(pool: &crate::model::BigipPool) -> Json {
    let mut members = Vec::new();
    for item in &pool.members.items {
        let ListItemValue::PoolMember(member) = &item.value else {
            continue;
        };
        let addr_text = member
            .address
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_default();
        let addr = if addr_text.is_empty() {
            split_destination(&member.name).0
        } else {
            addr_text
        };
        let port = if member.port != 0 {
            member.port
        } else {
            split_destination(&member.name).1.unwrap_or(0)
        };
        let server_addresses = if addr.is_empty() {
            Json::Array(vec![])
        } else {
            Json::Array(vec![Json::Str(addr)])
        };
        members.push(Json::Object(vec![
            ("servicePort".to_owned(), Json::Int(port)),
            ("serverAddresses".to_owned(), server_addresses),
        ]));
    }
    let mut out = vec![
        ("class".to_owned(), Json::Str("Pool".to_owned())),
        ("members".to_owned(), Json::Array(members)),
    ];
    if !pool.load_balancing_mode.is_empty() {
        out.push((
            "loadBalancingMode".to_owned(),
            Json::Str(pool.load_balancing_mode.clone()),
        ));
    }
    if !pool.monitor.is_empty() {
        let first = pool.monitor.split(' ').next().unwrap_or("");
        let monitor_name = short_name(first);
        if !monitor_name.is_empty() {
            out.push((
                "monitors".to_owned(),
                Json::Array(vec![Json::Object(vec![(
                    "use".to_owned(),
                    Json::Str(monitor_name.to_owned()),
                )])]),
            ));
        }
    }
    Json::Object(out)
}

fn convert_virtual(cfg: &BigipConfig, vs: &crate::model::BigipVirtualServer) -> Json {
    let dest_text = vs
        .destination
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or_default();
    let (addr, port) = split_destination(&dest_text);
    let virtual_addresses = if addr.is_empty() {
        Json::Array(vec![])
    } else {
        Json::Array(vec![Json::Str(addr)])
    };
    let mut out = vec![
        (
            "class".to_owned(),
            Json::Str(classify_virtual_class(cfg, vs).to_owned()),
        ),
        ("virtualAddresses".to_owned(), virtual_addresses),
    ];
    if let Some(port) = port {
        out.push(("virtualPort".to_owned(), Json::Int(port)));
    }
    if !vs.pool.is_empty() {
        let resolved = resolve_name(cfg, &vs.pool, "pools").unwrap_or_else(|| vs.pool.clone());
        out.push((
            "pool".to_owned(),
            Json::Str(short_name(&resolved).to_owned()),
        ));
    }
    if !vs.rules.is_empty() {
        let irules = vs
            .rules
            .iter()
            .map(|r| {
                let path = match r {
                    ListItemValue::Str(s) => s.clone(),
                    _ => String::new(),
                };
                let resolved = resolve_name(cfg, &path, "rules").unwrap_or(path);
                Json::Str(short_name(&resolved).to_owned())
            })
            .collect();
        out.push(("iRules".to_owned(), Json::Array(irules)));
    }
    if !vs.persist.is_empty() {
        let methods = vs
            .persist
            .paths()
            .iter()
            .map(|p| {
                let resolved = resolve_name(cfg, p, "persistence").unwrap_or_else(|| p.clone());
                Json::Str(short_name(&resolved).to_owned())
            })
            .collect();
        out.push(("persistenceMethods".to_owned(), Json::Array(methods)));
    }
    Json::Object(out)
}

fn convert_irule(rule: &crate::model::BigipRule) -> Json {
    Json::Object(vec![
        ("class".to_owned(), Json::Str("iRule".to_owned())),
        ("iRule".to_owned(), Json::Str(rule.source.clone())),
    ])
}

fn convert_monitor(monitor: &crate::model::BigipMonitor) -> Json {
    let monitor_type = if monitor.monitor_type.is_empty() {
        "tcp".to_owned()
    } else {
        monitor.monitor_type.clone()
    };
    Json::Object(vec![
        ("class".to_owned(), Json::Str("Monitor".to_owned())),
        ("monitorType".to_owned(), Json::Str(monitor_type)),
    ])
}

/// The set of `(module, object_type)` generic kinds that have a typed AS3
/// mapping and so are *not* re-reported as unmapped generics.
const MAPPED_GENERIC_KINDS: &[(&str, &str)] = &[
    ("ltm", "pool"),
    ("ltm", "virtual"),
    ("ltm", "rule"),
    ("ltm", "monitor"),
    ("ltm", "node"),
    ("ltm", "profile"),
    ("ltm", "data-group"),
    ("ltm", "persistence"),
    ("ltm", "snatpool"),
];

/// Convert `cfg` to a minimal AS3 declaration.
#[must_use]
pub fn scf_to_as3(cfg: &BigipConfig, tenant: &str, application: &str) -> AS3ConversionReport {
    let mut app: Vec<(String, Json)> = vec![
        ("class".to_owned(), Json::Str("Application".to_owned())),
        ("template".to_owned(), Json::Str("generic".to_owned())),
    ];
    let mut mapped = 0usize;
    let mut unmapped: Vec<String> = Vec::new();

    for placed in &cfg.objects {
        if placed.table_name != "pools" {
            continue;
        }
        if let ModelObject::Pool(p) = &placed.object {
            app.push((short_name(&placed.full_path).to_owned(), convert_pool(p)));
            mapped += 1;
        }
    }
    for placed in &cfg.objects {
        if placed.table_name != "virtual_servers" {
            continue;
        }
        if let ModelObject::VirtualServer(v) = &placed.object {
            app.push((
                short_name(&placed.full_path).to_owned(),
                convert_virtual(cfg, v),
            ));
            mapped += 1;
        }
    }
    for placed in &cfg.objects {
        if placed.table_name != "rules" {
            continue;
        }
        if let ModelObject::Rule(r) = &placed.object {
            app.push((short_name(&placed.full_path).to_owned(), convert_irule(r)));
            mapped += 1;
        }
    }
    for placed in &cfg.objects {
        if placed.table_name != "monitors" {
            continue;
        }
        if let ModelObject::Monitor(m) = &placed.object {
            app.push((short_name(&placed.full_path).to_owned(), convert_monitor(m)));
            mapped += 1;
        }
    }

    // Profiles, data-groups, persistence, snat-pools and standalone nodes are
    // listed as unmapped — supporting them needs deeper schema work.
    for name in table_paths(cfg, "profiles") {
        unmapped.push(format!("profile:{name}"));
    }
    for name in table_paths(cfg, "data_groups") {
        unmapped.push(format!("data-group:{name}"));
    }
    for name in table_paths(cfg, "persistence") {
        unmapped.push(format!("persistence:{name}"));
    }
    for name in table_paths(cfg, "snat_pools") {
        unmapped.push(format!("snatpool:{name}"));
    }
    for name in table_paths(cfg, "nodes") {
        unmapped.push(format!("node:{name}"));
    }
    for (name, obj) in &cfg.generic_objects {
        if MAPPED_GENERIC_KINDS.contains(&(obj.module.as_str(), obj.object_type.as_str())) {
            continue;
        }
        unmapped.push(format!("generic:{name}"));
    }

    let declaration = Json::Object(vec![
        ("class".to_owned(), Json::Str("AS3".to_owned())),
        (
            "declaration".to_owned(),
            Json::Object(vec![
                ("class".to_owned(), Json::Str("ADC".to_owned())),
                ("schemaVersion".to_owned(), Json::Str("3.45.0".to_owned())),
                (
                    tenant.to_owned(),
                    Json::Object(vec![
                        ("class".to_owned(), Json::Str("Tenant".to_owned())),
                        (application.to_owned(), Json::Object(app)),
                    ]),
                ),
            ]),
        ),
    ]);

    AS3ConversionReport {
        declaration,
        mapped,
        unmapped,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_destination_ipv4_named_and_numeric() {
        assert_eq!(
            split_destination("/Common/10.5.5.5:443"),
            ("10.5.5.5".to_owned(), Some(443))
        );
        // BIG-IP service-name ports resolve via the mcpd name table.
        assert_eq!(
            split_destination("/Common/10.5.5.5:https"),
            ("10.5.5.5".to_owned(), Some(443))
        );
        // No port half → address only.
        assert_eq!(
            split_destination("/Common/10.5.5.5"),
            ("10.5.5.5".to_owned(), None)
        );
    }

    #[test]
    fn split_destination_ipv6_dot_port() {
        // IPv6 uses `.` to separate the port; the prefix must parse as IPv6.
        assert_eq!(
            split_destination("/Common/2001:db8::1.443"),
            ("2001:db8::1".to_owned(), Some(443))
        );
    }

    #[test]
    fn json_indent2_matches_python_shapes() {
        let v = Json::Object(vec![
            ("class".to_owned(), Json::Str("Pool".to_owned())),
            ("members".to_owned(), Json::Array(vec![])),
            ("port".to_owned(), Json::Int(80)),
        ]);
        assert_eq!(
            v.dumps_indent2(),
            "{\n  \"class\": \"Pool\",\n  \"members\": [],\n  \"port\": 80\n}"
        );
    }

    #[test]
    fn resolve_port_bounds() {
        assert_eq!(resolve_port("0"), Some(0));
        assert_eq!(resolve_port("65535"), Some(65535));
        assert_eq!(resolve_port("65536"), None);
        assert_eq!(resolve_port(""), None);
        assert_eq!(resolve_port("https"), Some(443));
        assert_eq!(resolve_port("not-a-port"), None);
    }
}
