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

//! SCF → AS3 declaration emitter (best-effort, partial coverage).
//!
//! AS3 (F5 Application Services 3)
//! is a declarative API for BIG-IP configuration. This converts the most common
//! LTM objects from a parsed [`BigipConfig`] into an AS3 `Application`; anything
//! without a clear AS3 mapping is recorded in [`AS3ConversionReport::unmapped`]
//! so nothing silently disappears.
//!
//! The declaration is built as an insertion-ordered [`Json`] tree and rendered
//! with [`crate::jsonfmt::json_string`] for byte-exact 2-space-indented JSON.

use std::collections::HashSet;

use crate::jsonfmt::json_string;
use crate::model::{ModelObject, ProfileType};
use crate::parser::BigipConfig;
use crate::value::bigip_list::ListItemValue;
use crate::value::{MonitorExpression, MonitorMode};

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

    /// Serialise this value as 2-space-indented JSON (no trailing
    /// newline). ASCII-escaping is handled by `json_string`.
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
    /// Pool or member settings omitted because their semantics could not be preserved.
    pub warnings: Vec<String>,
}

fn short_name(full_path: &str) -> &str {
    if full_path.is_empty() {
        ""
    } else {
        full_path.rsplit_once('/').map_or(full_path, |(_, n)| n)
    }
}

/// Look up a typed object by table and full-path.
fn find<'a>(cfg: &'a BigipConfig, table: &str, full_path: &str) -> Option<&'a ModelObject> {
    crate::object_names::object_name_candidates(
        full_path,
        cfg.objects
            .iter()
            .filter(|placed| placed.table_name == table)
            .map(|placed| (placed.full_path.as_str(), &placed.object)),
        crate::object_names::ObjectNamePolicy::Exact,
    )
    .first_in_order()
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
    crate::object_names::object_name_candidates(
        name,
        table_paths(cfg, table).map(|path| (path, path)),
        crate::object_names::ObjectNamePolicy::Configuration {
            default_partition: &cfg.default_partition,
        },
    )
    .first_in_order()
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

fn monitor_fields(
    monitor_paths: &HashSet<&str>,
    default_partition: &str,
    owner: &str,
    raw: &str,
    member: bool,
) -> Result<Vec<(String, Json)>, &'static str> {
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }
    let expression = MonitorExpression::try_parse(raw).ok_or("invalid monitor expression")?;
    let threshold = match expression.mode {
        MonitorMode::Default => return Ok(Vec::new()),
        MonitorMode::None if member => {
            return Err("member `monitor none` cannot be preserved");
        }
        MonitorMode::None | MonitorMode::Single => None,
        MonitorMode::All if !member => Some(Json::Str("all".to_owned())),
        MonitorMode::All => Some(Json::Int(
            i64::try_from(expression.len()).unwrap_or(i64::MAX),
        )),
        MonitorMode::MinOf => expression.minimum.map(Json::Int),
    };
    if member && matches!(threshold, Some(Json::Int(n)) if n > 63) {
        return Err("AS3 member minimumMonitors is limited to 63");
    }

    let monitors = expression
        .references()
        .iter()
        .map(|name| {
            let candidates = crate::object_names::owner_partition_reference_paths(
                name,
                owner,
                default_partition,
            );
            let (key, reference) = if let Some(path) = candidates
                .iter()
                .find(|path| monitor_paths.contains(path.as_str()))
            {
                ("use", short_name(path))
            } else if name.starts_with('/') {
                ("bigip", name.as_str())
            } else {
                return Err("monitor path could not be resolved");
            };
            Ok(Json::Object(vec![(
                key.to_owned(),
                Json::Str(reference.to_owned()),
            )]))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut fields = vec![("monitors".to_owned(), Json::Array(monitors))];
    if let Some(threshold) = threshold {
        fields.push(("minimumMonitors".to_owned(), threshold));
    }
    Ok(fields)
}

fn convert_pool(
    monitor_paths: &HashSet<&str>,
    default_partition: &str,
    pool: &crate::model::BigipPool,
    warnings: &mut Vec<String>,
) -> Json {
    let mut members = Vec::new();
    for item in &pool.members.items {
        let ListItemValue::PoolMember(member) = &item.value else {
            continue;
        };
        let addr_text = member.address.as_ref().map_or_default(ToString::to_string);
        let addr = if addr_text.is_empty() {
            let node = crate::value::MemberKey::try_parse(&member.name)
                .map_or(member.name.as_str(), |key| key.node);
            short_name(node).to_owned()
        } else {
            addr_text
        };
        let server_addresses = if addr.is_empty() {
            Json::Array(vec![])
        } else {
            Json::Array(vec![Json::Str(addr)])
        };
        let mut fields = vec![
            ("servicePort".to_owned(), Json::Int(member.port)),
            ("serverAddresses".to_owned(), server_addresses),
        ];
        match monitor_fields(
            monitor_paths,
            default_partition,
            &pool.full_path,
            &member.monitor,
            true,
        ) {
            Ok(monitor) => fields.extend(monitor),
            Err(reason) => warnings.push(format!(
                "pool {} member {} monitor {:?}: {reason}",
                pool.full_path, member.name, member.monitor
            )),
        }
        members.push(Json::Object(fields));
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
    match monitor_fields(
        monitor_paths,
        default_partition,
        &pool.full_path,
        &pool.monitor,
        false,
    ) {
        Ok(monitor) => out.extend(monitor),
        Err(reason) => warnings.push(format!(
            "pool {} monitor {:?}: {reason}",
            pool.full_path, pool.monitor
        )),
    }
    Json::Object(out)
}

fn convert_virtual(cfg: &BigipConfig, vs: &crate::model::BigipVirtualServer) -> Json {
    let dest = vs.destination.as_ref();
    let virtual_addresses = cfg
        .virtual_destination_address(vs)
        .map_or_else(Vec::new, |address| vec![Json::Str(address.to_string())]);
    let mut out = vec![
        (
            "class".to_owned(),
            Json::Str(classify_virtual_class(cfg, vs).to_owned()),
        ),
        (
            "virtualAddresses".to_owned(),
            Json::Array(virtual_addresses),
        ),
    ];
    if let Some(destination) = dest.filter(|destination| !destination.port.spelling.is_empty()) {
        out.push((
            "virtualPort".to_owned(),
            Json::Int(i64::from(destination.port.port)),
        ));
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
    let mut warnings = Vec::new();
    let monitor_paths = table_paths(cfg, "monitors").collect();

    for placed in &cfg.objects {
        if placed.table_name != "pools" {
            continue;
        }
        if let ModelObject::Pool(p) = &placed.object {
            app.push((
                short_name(&placed.full_path).to_owned(),
                convert_pool(&monitor_paths, &cfg.default_partition, p, &mut warnings),
            ));
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
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as3_preserves_compound_monitors_and_reports_loss() {
        let config = crate::parser::parse_bigip_conf(
            r#"
ltm monitor https /Tenant/custom { }
ltm pool /Tenant/web {
    monitor "/Common/http" and custom
    members {
        /Tenant/192.0.2.1:80 {
            monitor min 1 of {
                /Common/http
                custom
            }
        }
        /Tenant/192.0.2.2:80 { monitor none }
    }
}
ltm pool /Tenant/quorum { monitor "min 1 of { /Common/http /Tenant/custom }" }
ltm pool /Tenant/invalid { monitor /Common/http and }
"#,
            "Common",
        );
        let report = scf_to_as3(&config, "tenant", "app");
        let declaration: serde_json::Value =
            serde_json::from_str(&report.declaration.dumps_indent2()).unwrap();
        let app = &declaration["declaration"]["tenant"]["app"];
        let monitors = serde_json::json!([{"bigip": "/Common/http"}, {"use": "custom"}]);
        assert_eq!(app["web"]["monitors"], monitors);
        assert_eq!(app["web"]["minimumMonitors"], "all");
        assert_eq!(app["web"]["members"][0]["monitors"], monitors);
        assert_eq!(app["web"]["members"][0]["minimumMonitors"], 1);
        assert_eq!(app["quorum"]["monitors"], monitors);
        assert_eq!(app["quorum"]["minimumMonitors"], 1);
        assert!(app["invalid"]["monitors"].is_null());
        assert!(
            report
                .warnings
                .iter()
                .any(|warning| warning.contains("/Tenant/invalid"))
        );
        assert!(
            report
                .warnings
                .iter()
                .any(|warning| warning.contains("/Tenant/192.0.2.2:80"))
        );
    }

    #[test]
    fn as3_does_not_weaken_a_member_threshold_above_its_limit() {
        let monitors = ["/Common/http"; 64].join(" ");
        let source = format!(
            "ltm pool /Common/web {{ members {{ /Common/192.0.2.1:80 {{ monitor min 64 of {{ {monitors} }} }} }} }}"
        );
        let config = crate::parser::parse_bigip_conf(&source, "Common");
        let report = scf_to_as3(&config, "tenant", "app");
        let declaration: serde_json::Value =
            serde_json::from_str(&report.declaration.dumps_indent2()).unwrap();
        let member = &declaration["declaration"]["tenant"]["app"]["web"]["members"][0];
        assert_eq!(member["servicePort"], 80);
        assert!(member["monitors"].is_null());
        assert!(member["minimumMonitors"].is_null());
        assert_eq!(report.warnings.len(), 1);
        assert!(report.warnings[0].contains("minimumMonitors is limited to 63"));
    }

    #[test]
    fn pool_member_ports_survive_scf_parsing_and_as3_conversion() {
        let probe = include_str!(
            "../../../docs/work/name-handling/handover/oo-dispatch-probe/bigip-addresses/addresses.conf"
        );
        let device = include_str!("../../bigip-report-gen/python/tests/data/device-01.bigip.conf");
        let probe_ports: &[(&str, &[i64])] = &[
            ("p_v4", &[80, 80]),
            ("p_v6", &[80, 443]),
            ("p_fqdn", &[443]),
            ("p_rel", &[80]),
            ("p_http", &[80]),
            ("app_pool", &[8080]),
        ];
        let device_ports: &[(&str, &[i64])] = &[
            ("app1_t80_pool", &[80, 80]),
            ("app2_t80_pool", &[80, 80]),
            ("app3_t8443_pool", &[8443, 8443]),
            ("manyOptions_pool", &[80, 443, 8443, 8443, 443]),
            ("app4_pool", &[443]),
            ("css_pool", &[]),
            ("jpg.pool", &[]),
            ("js.io_t80_pool", &[]),
            ("bigiq.benlab.io_t443_pool", &[443]),
        ];
        for (source, expected) in [(probe, probe_ports), (device, device_ports)] {
            let config = crate::parser::parse_bigip_conf(source, "Common");
            let report = scf_to_as3(&config, "tenant", "app");
            let declaration: serde_json::Value =
                serde_json::from_str(&report.declaration.dumps_indent2()).unwrap();
            let app = &declaration["declaration"]["tenant"]["app"];
            let pools = config.objects.iter().filter_map(|placed| {
                if let ModelObject::Pool(pool) = &placed.object {
                    Some((short_name(&placed.full_path), pool))
                } else {
                    None
                }
            });
            assert_eq!(pools.clone().count(), expected.len());
            for (name, pool) in pools {
                let (_, expected_ports) = expected.iter().find(|entry| entry.0 == name).unwrap();
                let parsed: Vec<i64> = pool
                    .members
                    .items
                    .iter()
                    .map(|item| match &item.value {
                        ListItemValue::PoolMember(member) => member.port,
                        other => panic!("unexpected member value: {other:?}"),
                    })
                    .collect();
                assert_eq!(&parsed, expected_ports, "{name}");
                let converted: Vec<i64> = app[name]["members"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|member| member["servicePort"].as_i64().unwrap())
                    .collect();
                assert_eq!(&converted, expected_ports, "{name}");
            }
        }
    }

    #[test]
    fn as3_virtual_uses_the_configured_ipv6_address_and_named_service() {
        let config = crate::parser::parse_bigip_conf(
            "ltm virtual /Common/v6 { destination /Common/2001:db8::53%2.http }",
            "Common",
        );
        let report = scf_to_as3(&config, "tenant", "app");
        let value: serde_json::Value =
            serde_json::from_str(&report.declaration.dumps_indent2()).unwrap();
        let virtual_server = &value["declaration"]["tenant"]["app"]["v6"];
        assert_eq!(
            virtual_server["virtualAddresses"],
            serde_json::json!(["2001:db8::53%2"])
        );
        assert_eq!(virtual_server["virtualPort"], 80);
    }

    #[test]
    fn json_indent2_shapes() {
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
}
