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

//! Cross-reference validator for BIG-IP configurations.
//!
//! Where the sibling [`crate::lint`] engine emits coarse [`crate::lint::Finding`]s
//! keyed only by object path, this validator produces ranged
//! [`ConfigDiagnostic`]s for the LSP diagnostics pipeline: it checks that
//! iRules reference objects (data-groups, pools, profiles, …) that exist
//! in the parsed configuration and that virtual servers reference valid
//! iRules / pools / profiles.
//!
//! Diagnostic codes (all internal — controlled by the BIG-IP dialect
//! toggle):
//!
//! - **BIGIP6001** (WARNING): iRule references data-group not found in config
//! - **BIGIP6002** (WARNING): iRule references pool not found in config
//! - **BIGIP6003** (WARNING): virtual server references iRule not found
//! - **BIGIP6004** (HINT): iRule uses command requiring a profile not on the virtual
//! - **BIGIP6005** (WARNING): virtual server references pool not found
//! - **BIGIP6006** (HINT): data-group defined but never referenced
//! - **BIGIP6007** (WARNING): iRule references SNAT pool not found
//! - **BIGIP6008** (HINT): pool has no members
//! - **BIGIP6009** (WARNING): virtual server has a duplicate iRule attachment
//! - **BIGIP6010** (HINT): iRule `persist` references a profile not on the virtual
//! - **BIGIP6011** (WARNING): invalid IP address in an IP-type data-group record
//! - **BIGIP6012** (WARNING): attached iRules handle an event at the same priority

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::sync::LazyLock;

use regex::Regex;
use tcl_core_types::DiagCode;
use tcl_irules::extract_irules_event_handlers;
use tcl_lexer::LineIndex;
use tcl_registry::base_objects::base_object_identities;
use tcl_registry::profile_defaults::{BigipVersion, profile_field_defaults};

use crate::canonical::Canon;
use crate::lint::{ModelView, resolve_name};
use crate::model::ProfileType;
use crate::parser::driver::BigipConfig;
use crate::parser::helpers::{
    parse_keyed_block_entries, parse_list_block, parse_properties_with_spans,
};
use crate::range::Range;
use crate::value::MonitorExpression;

static FACTORY_OBJECT_IDENTITIES: LazyLock<HashSet<String>> = LazyLock::new(|| {
    base_object_identities()
        .map(|(module, object_type, name)| factory_object_identity(module, object_type, name))
        .collect()
});

/// BIG-IP model diagnostics emitted by this validator.
pub const BIGIP_DIAGNOSTIC_CODES: &[DiagCode] = &[
    DiagCode::Bigip6001,
    DiagCode::Bigip6002,
    DiagCode::Bigip6003,
    DiagCode::Bigip6004,
    DiagCode::Bigip6005,
    DiagCode::Bigip6006,
    DiagCode::Bigip6007,
    DiagCode::Bigip6008,
    DiagCode::Bigip6009,
    DiagCode::Bigip6010,
    DiagCode::Bigip6011,
    DiagCode::Bigip6012,
    DiagCode::Bigip6013,
    DiagCode::Bigip6014,
    DiagCode::Bigip6038,
    DiagCode::Bigip6039,
];

/// Severity of a config diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagSeverity {
    /// A likely-incorrect reference or value.
    Warning,
    /// An advisory observation.
    Hint,
}

/// Configuration object that owns a diagnostic in object-oriented consumers
/// such as the BIG-IP report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigDiagnosticSubject {
    /// An iRule.
    IRule,
    /// A virtual server.
    VirtualServer,
    /// A pool.
    Pool,
    /// A data group.
    DataGroup,
    /// An iApp presentation or implementation.
    IApp,
    /// A registry-declared object whose dedicated report tab is the object
    /// index (rather than an LTM-specific view).
    Object,
}

impl ConfigDiagnosticSubject {
    /// Every subject that can own an F5 configuration diagnostic. Report
    /// builders iterate this catalogue when preparing per-tab finding lists,
    /// so adding a subject cannot silently drop its diagnostics.
    pub const ALL: &'static [Self] = &[
        Self::IRule,
        Self::VirtualServer,
        Self::Pool,
        Self::DataGroup,
        Self::IApp,
        Self::Object,
    ];

    /// The report tab where findings for this subject are actionable.
    #[must_use]
    pub const fn report_tab(self) -> &'static str {
        match self {
            Self::IRule => "rules",
            Self::VirtualServer => "virtuals",
            Self::Pool => "pools",
            Self::DataGroup => "dataGroups",
            Self::IApp => "apps",
            Self::Object => "objectIndex",
        }
    }
}

/// One ranged BIG-IP config diagnostic — the validator analogue of an LSP
/// `Diagnostic`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigDiagnostic {
    /// Diagnostic code (e.g. `"BIGIP6001"`).
    pub code: String,
    /// Human-readable message.
    pub message: String,
    /// Severity.
    pub severity: DiagSeverity,
    /// Object surface where this finding is actionable.
    pub subject: ConfigDiagnosticSubject,
    /// Source range. For per-iRule checks this is relative to the iRule
    /// body (each rule gets a fresh `DocumentBuffer`); for object-level checks
    /// it is the object's own range, or the zero range when the model carries
    /// none.
    pub range: Range,
}

/// How a profile field relates to the versioned BIG-IP default evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileDefaultState {
    /// The field is explicitly set to a value different from the known default.
    Explicit,
    /// The field is omitted and inherits through `defaults-from`.
    Inherited,
    /// The field is omitted without a parent, or explicitly repeats the factory default.
    Factory,
    /// The TMOS version is absent, malformed, or has no matching embedded default.
    UnknownVersion,
}

/// One version-aware profile/default comparison. Consumers can render this as
/// evidence even when there is no defect: customisation is not a vulnerability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileDefaultEvidence {
    /// Full path of the profile or persistence object.
    pub owner: String,
    /// Registry profile family (`HTTP`, `CLIENTSSL`, `TCP`, …).
    pub profile_type: String,
    /// tmsh field name.
    pub field: String,
    /// Explicit value present in the configuration, when any.
    pub configured: Option<String>,
    /// Resolved factory default, only when a TMOS version is known.
    pub default_value: Option<String>,
    /// Whether the value is explicit, inherited, factory-equivalent, or unknown.
    pub state: ProfileDefaultState,
    /// Whether this field can change transport, TLS, HTTP, or persistence security posture.
    pub security_relevant: bool,
}

// iRule source-scanning regexes
//
// The `regex` crate has no look-around, so the two negative look-ahead
// guards (`pool (?!member)`, `persist (?!none)`) are handled by
// filtering the captured name in code instead.

/// The `class match|search` word-operator alternation, derived from the
/// shared `BinOp` source of truth (issue #983/#986's unification) rather
/// than a hand-maintained string — the same canonical names `tcl-irules`'s
/// `is_class_operator` matches against, so a rename of one of these variants
/// is a compile error here too, not silent drift. See the sync test
/// `class_operators_regex_matches_irules_word_operator_set`.
static CLASS_OPERATORS: LazyLock<String> = LazyLock::new(|| {
    use tcl_syntax::expr::ast::BinOp;
    [
        BinOp::StrEquals,
        BinOp::StartsWith,
        BinOp::EndsWith,
        BinOp::Contains,
        BinOp::MatchesGlob,
        BinOp::MatchesRegex,
    ]
    .iter()
    .map(|op| op.as_str())
    .collect::<Vec<_>>()
    .join("|")
});

/// `class match|search ?opts? <item> <operator> <dg>` — capture the dg name.
static CLASS_MATCH_RE: LazyLock<Regex> = LazyLock::new(|| {
    let ops = &*CLASS_OPERATORS;
    Regex::new(&format!(
        r"\bclass\s+(?:match|search)\s+(?:(?:-\w+\s+)*)(?:--\s+)?\S+\s+(?:{ops})\s+(\S+)"
    ))
    .expect("static class-match regex")
});

/// `class lookup ?--? <key> <dg>` — capture the dg name (last argument).
static CLASS_LOOKUP_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\bclass\s+lookup\s+(?:--\s+)?\S+\s+(\S+)").expect("static regex")
});

/// `class exists|size|type|get|startsearch <dg>` — capture the dg name.
static CLASS_SINGLE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\bclass\s+(?:exists|size|type|get|startsearch)\s+(\S+)").expect("static regex")
});

/// `pool <name>` (the `pool member` subcommand is filtered out in code).
static POOL_CMD_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bpool\s+(/[\w/.-]+|\w[\w.-]*)").expect("static regex"));

/// `snatpool <name>`.
static SNATPOOL_CMD_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bsnatpool\s+(/[\w/.-]+|\w[\w.-]*)").expect("static regex"));

/// `persist <name>` (the `persist none` form is filtered out in code).
static PERSIST_CMD_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bpersist\s+(/[\w/.-]+|\w[\w.-]*)").expect("static regex"));

/// Any `HTTP::<cmd>` command (requires an HTTP profile).
static HTTP_COMMANDS_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bHTTP::\w+").expect("static regex"));

/// Any `SSL::<cmd>` / `ssl::<cmd>` command (requires an SSL profile).
static SSL_COMMANDS_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b(?:SSL|ssl)::\w+").expect("static regex"));

/// Strip braces, quotes, and brackets from a captured name.
fn clean_name(name: &str) -> &str {
    name.trim_matches(|c| matches!(c, '{' | '}' | '"' | '\'' | '[' | ']'))
}

/// Build a [`Range`] from a capture group's byte offsets (inclusive end =
/// `max(start, end - 1)`).
fn range_from_capture(source: &str, line_index: &LineIndex, start: usize, end: usize) -> Range {
    let end_inclusive = if end > start { end - 1 } else { start };
    Range::from_offsets(source, line_index, start, end_inclusive)
}

/// Collect `(capture_start, capture_end, data_group_name)` for every
/// `class` data-group reference in `source`. Dynamic names (`$var`, `[cmd]`)
/// are skipped.
fn iter_class_dg_references(source: &str) -> Vec<(usize, usize, String)> {
    let mut out: Vec<(usize, usize, String)> = Vec::new();
    for re in [&*CLASS_MATCH_RE, &*CLASS_LOOKUP_RE, &*CLASS_SINGLE_RE] {
        for caps in re.captures_iter(source) {
            let Some(m) = caps.get(1) else { continue };
            let name = clean_name(m.as_str());
            if name.starts_with('$') || name.starts_with('[') {
                continue;
            }
            out.push((m.start(), m.end(), name.to_owned()));
        }
    }
    out
}

/// Profile types attached to a virtual server.
fn profile_types_for_virtual(
    view: &ModelView<'_>,
    vs: &crate::model::BigipVirtualServer,
) -> Vec<ProfileType> {
    let mut types: Vec<ProfileType> = Vec::new();
    for pref in vs.profiles.paths() {
        if let Some(resolved) = resolve_name(&pref, &view.profiles, view.default_partition)
            && let Some(p) = view.profiles.get(&resolved)
            && !types.contains(&p.profile_type)
        {
            types.push(p.profile_type);
        }
    }
    types
}

/// The object range, or the zero range when the model carries none (port
/// of `... or _null_range()`).
fn object_range(range: Option<Range>) -> Range {
    range.unwrap_or_else(Range::zero)
}

// Per-iRule checks

/// BIGIP6001: iRule references a data-group not found in config.
fn check_irule_data_groups(
    rule: &crate::model::BigipRule,
    view: &ModelView<'_>,
    out: &mut Vec<ConfigDiagnostic>,
) {
    if rule.source.is_empty() {
        return;
    }
    let line_index = LineIndex::new(&rule.source);
    for (start, end, dg_name) in iter_class_dg_references(&rule.source) {
        if resolve_name(&dg_name, &view.data_groups, view.default_partition).is_none() {
            out.push(ConfigDiagnostic {
                code: DiagCode::Bigip6001.as_str().to_owned(),
                message: format!("Data-group '{dg_name}' not found in BIG-IP configuration."),
                severity: DiagSeverity::Warning,
                subject: ConfigDiagnosticSubject::IRule,
                range: range_from_capture(&rule.source, &line_index, start, end),
            });
        }
    }
}

/// BIGIP6002: iRule references a pool not found in config.
fn check_irule_pools(
    rule: &crate::model::BigipRule,
    view: &ModelView<'_>,
    out: &mut Vec<ConfigDiagnostic>,
) {
    if rule.source.is_empty() {
        return;
    }
    let line_index = LineIndex::new(&rule.source);
    for caps in POOL_CMD_RE.captures_iter(&rule.source) {
        let Some(m) = caps.get(1) else { continue };
        let pool_name = clean_name(m.as_str());
        // The `pool member` subcommand is not a pool reference.
        if pool_name == "member" || pool_name.starts_with('$') || pool_name.starts_with('[') {
            continue;
        }
        if resolve_name(pool_name, &view.pools, view.default_partition).is_none() {
            out.push(ConfigDiagnostic {
                code: DiagCode::Bigip6002.as_str().to_owned(),
                message: format!("Pool '{pool_name}' not found in BIG-IP configuration."),
                severity: DiagSeverity::Warning,
                subject: ConfigDiagnosticSubject::IRule,
                range: range_from_capture(&rule.source, &line_index, m.start(), m.end()),
            });
        }
    }
}

/// BIGIP6007: iRule references a SNAT pool not found in config.
fn check_irule_snatpools(
    rule: &crate::model::BigipRule,
    view: &ModelView<'_>,
    out: &mut Vec<ConfigDiagnostic>,
) {
    if rule.source.is_empty() {
        return;
    }
    let line_index = LineIndex::new(&rule.source);
    for caps in SNATPOOL_CMD_RE.captures_iter(&rule.source) {
        let Some(m) = caps.get(1) else { continue };
        let sp_name = clean_name(m.as_str());
        if sp_name.starts_with('$') || sp_name.starts_with('[') {
            continue;
        }
        if resolve_name(sp_name, &view.snat_pools, view.default_partition).is_none() {
            out.push(ConfigDiagnostic {
                code: DiagCode::Bigip6007.as_str().to_owned(),
                message: format!("SNAT pool '{sp_name}' not found in BIG-IP configuration."),
                severity: DiagSeverity::Warning,
                subject: ConfigDiagnosticSubject::IRule,
                range: range_from_capture(&rule.source, &line_index, m.start(), m.end()),
            });
        }
    }
}

/// All data-group names referenced in an iRule body.
fn collect_referenced_data_groups(rule: &crate::model::BigipRule) -> HashSet<String> {
    if rule.source.is_empty() {
        return HashSet::new();
    }
    iter_class_dg_references(&rule.source)
        .into_iter()
        .map(|(_, _, name)| name)
        .collect()
}

// Virtual-server-level checks

/// BIGIP6003 + BIGIP6009: virtual references an undefined iRule / has a
/// duplicate iRule attachment.
fn check_virtual_rules(view: &ModelView<'_>, out: &mut Vec<ConfigDiagnostic>) {
    for (_path, vs) in view.virtual_servers.iter() {
        let mut seen: HashSet<String> = HashSet::new();
        for rule_ref in vs.rules.paths() {
            if seen.contains(&rule_ref) {
                out.push(ConfigDiagnostic {
                    code: DiagCode::Bigip6009.as_str().to_owned(),
                    message: format!(
                        "Virtual server '{}' has duplicate iRule attachment '{rule_ref}'.",
                        vs.name
                    ),
                    severity: DiagSeverity::Warning,
                    subject: ConfigDiagnosticSubject::VirtualServer,
                    range: object_range(vs.range),
                });
            }
            seen.insert(rule_ref.clone());

            if resolve_name(&rule_ref, &view.rules, view.default_partition).is_none() {
                out.push(ConfigDiagnostic {
                    code: DiagCode::Bigip6003.as_str().to_owned(),
                    message: format!(
                        "Virtual server '{}' references iRule '{rule_ref}' which is not defined.",
                        vs.name
                    ),
                    severity: DiagSeverity::Warning,
                    subject: ConfigDiagnosticSubject::VirtualServer,
                    range: object_range(vs.range),
                });
            }
        }
    }
}

/// BIGIP6012: two or more iRules attached to a virtual server handle the same
/// event at the same effective priority.
fn check_virtual_rule_priority_conflicts(view: &ModelView<'_>, out: &mut Vec<ConfigDiagnostic>) {
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    for (_path, vs) in view.virtual_servers.iter() {
        let mut handlers: BTreeMap<(String, u16), BTreeSet<String>> = BTreeMap::new();
        for rule_ref in vs.rules.paths() {
            let Some(resolved) = resolve_name(&rule_ref, &view.rules, view.default_partition)
            else {
                continue;
            };
            let Some(rule) = view.rules.get(&resolved) else {
                continue;
            };
            for handler in extract_irules_event_handlers(&rule.source, registry) {
                handlers
                    .entry((handler.event, handler.priority))
                    .or_default()
                    .insert(rule.full_path.clone());
            }
        }

        for ((event, priority), rules) in handlers {
            if rules.len() < 2 {
                continue;
            }
            let names = rules
                .iter()
                .map(|name| format!("'{name}'"))
                .collect::<Vec<_>>()
                .join(", ");
            out.push(ConfigDiagnostic {
                code: DiagCode::Bigip6012.as_str().to_owned(),
                message: format!(
                    "Virtual server '{}' attaches iRules {names} which all handle event \
                     '{event}' at priority {priority}; execution order between them is ambiguous.",
                    vs.name
                ),
                severity: DiagSeverity::Warning,
                subject: ConfigDiagnosticSubject::VirtualServer,
                range: object_range(vs.range),
            });
        }
    }
}

/// BIGIP6005: virtual references a pool not found in config.
fn check_virtual_pools(view: &ModelView<'_>, out: &mut Vec<ConfigDiagnostic>) {
    for (_path, vs) in view.virtual_servers.iter() {
        if !vs.pool.is_empty()
            && resolve_name(&vs.pool, &view.pools, view.default_partition).is_none()
        {
            out.push(ConfigDiagnostic {
                code: DiagCode::Bigip6005.as_str().to_owned(),
                message: format!(
                    "Virtual server '{}' references pool '{}' which is not defined.",
                    vs.name, vs.pool
                ),
                severity: DiagSeverity::Warning,
                subject: ConfigDiagnosticSubject::VirtualServer,
                range: object_range(vs.range),
            });
        }
    }
}

/// BIGIP6004: an iRule on a virtual uses `HTTP::` / `SSL::` commands but the
/// virtual has no matching profile.
fn check_virtual_profile_requirements(view: &ModelView<'_>, out: &mut Vec<ConfigDiagnostic>) {
    for (_path, vs) in view.virtual_servers.iter() {
        let ptypes = profile_types_for_virtual(view, vs);
        let has_http = ptypes.contains(&ProfileType::Http);
        let has_ssl =
            ptypes.contains(&ProfileType::ClientSsl) || ptypes.contains(&ProfileType::ServerSsl);

        for rule_ref in vs.rules.paths() {
            let Some(resolved) = resolve_name(&rule_ref, &view.rules, view.default_partition)
            else {
                continue;
            };
            let Some(rule) = view.rules.get(&resolved) else {
                continue;
            };
            if rule.source.is_empty() {
                continue;
            }
            if !has_http && HTTP_COMMANDS_RE.is_match(&rule.source) {
                out.push(ConfigDiagnostic {
                    code: DiagCode::Bigip6004.as_str().to_owned(),
                    message: format!(
                        "iRule '{}' on virtual '{}' uses HTTP:: commands but no HTTP profile is attached.",
                        rule.name, vs.name
                    ),
                    severity: DiagSeverity::Hint,
                    subject: ConfigDiagnosticSubject::VirtualServer,
                    range: object_range(vs.range),
                });
            }
            if !has_ssl && SSL_COMMANDS_RE.is_match(&rule.source) {
                out.push(ConfigDiagnostic {
                    code: DiagCode::Bigip6004.as_str().to_owned(),
                    message: format!(
                        "iRule '{}' on virtual '{}' uses SSL:: commands but no SSL profile is attached.",
                        rule.name, vs.name
                    ),
                    severity: DiagSeverity::Hint,
                    subject: ConfigDiagnosticSubject::VirtualServer,
                    range: object_range(vs.range),
                });
            }
        }
    }
}

/// Convert the typed BIG-IP model spelling to the profile registry's canonical
/// stack vocabulary. The model deliberately preserves the tmsh-ish
/// `CLIENT_SSL` / `SERVER_SSL` names; the event/profile graph owns the compact
/// `CLIENTSSL` / `SERVERSSL` names.
fn registry_profile_name(profile: ProfileType) -> &'static str {
    match profile {
        ProfileType::ClientSsl => "CLIENTSSL",
        ProfileType::ServerSsl => "SERVERSSL",
        ProfileType::OneConnect => "ONECONNECT",
        ProfileType::Websocket => "WS",
        ProfileType::Persistence => "PERSIST",
        ProfileType::Other => "OTHER",
        _ => profile.py_name(),
    }
}

/// BIGIP6038: every attached iRule handler must be reachable through the
/// registry's full event-to-profile graph. This subsumes the old HTTP/SSL-only
/// namespace heuristic without hardcoding an event or command family.
fn check_virtual_event_profile_graph(view: &ModelView<'_>, out: &mut Vec<ConfigDiagnostic>) {
    let event_registry = tcl_registry::events::EventRegistry::build();
    let profiles = tcl_registry::profiles::ProfileRegistry::build();
    let command_registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    for (_path, vs) in view.virtual_servers.iter() {
        let profile_types = profile_types_for_virtual(view, vs);
        let mut active: Vec<&str> = profile_types
            .iter()
            .map(|profile| registry_profile_name(*profile))
            .filter(|name| *name != "OTHER")
            .collect();
        // TCP is the transport default on a standard LTM virtual. It is
        // inherited rather than explicitly listed in many SCFs and must not
        // make every TCP-gated event look unreachable.
        if !active.contains(&"TCP") && !vs.ip_protocol.eq_ignore_ascii_case("udp") {
            active.push("TCP");
        }

        // Profile conflicts are graph data on ProfileSpec, not a handwritten
        // pair table. Report each unordered pair once.
        let mut conflict_pairs = BTreeSet::new();
        for profile in &active {
            let Some(spec) = profiles.get_profile(profile) else {
                continue;
            };
            for conflict in spec.conflicts {
                if active.iter().any(|present| present == conflict) {
                    let pair = if *profile < *conflict {
                        (*profile, *conflict)
                    } else {
                        (*conflict, *profile)
                    };
                    conflict_pairs.insert(pair);
                }
            }
        }
        for (left, right) in conflict_pairs {
            out.push(ConfigDiagnostic {
                code: DiagCode::Bigip6039.as_str().to_owned(),
                message: format!(
                    "Virtual server '{}' attaches incompatible profile types {left} and {right}.",
                    vs.name
                ),
                severity: DiagSeverity::Warning,
                subject: ConfigDiagnosticSubject::VirtualServer,
                range: object_range(vs.range),
            });
        }

        for rule_ref in vs.rules.paths() {
            let Some(resolved) = resolve_name(&rule_ref, &view.rules, view.default_partition)
            else {
                continue;
            };
            let Some(rule) = view.rules.get(&resolved) else {
                continue;
            };
            for handler in extract_irules_event_handlers(&rule.source, command_registry) {
                let Some(props) = event_registry.get_props(&handler.event) else {
                    // Unknown/dynamic event names are evidence limitations,
                    // not a profile false positive.
                    continue;
                };
                if !profiles.stack_satisfies(props.implied_profiles, &active) {
                    let requirements = props.implied_profiles.join(" or ");
                    out.push(ConfigDiagnostic {
                        code: DiagCode::Bigip6038.as_str().to_owned(),
                        message: format!(
                            "Virtual server '{}' cannot reach iRule '{}' handler '{}' at effective priority {}: the event requires profile {requirements}, but active profiles are {}.",
                            vs.name,
                            rule.name,
                            handler.event,
                            handler.priority,
                            active.join(", ")
                        ),
                        severity: DiagSeverity::Warning,
                        subject: ConfigDiagnosticSubject::VirtualServer,
                        range: object_range(vs.range),
                    });
                }
            }
        }
    }
}

/// BIGIP6010: an iRule `persist` references a persistence profile that is
/// not attached to the virtual server.
fn check_virtual_persistence(view: &ModelView<'_>, out: &mut Vec<ConfigDiagnostic>) {
    for (_path, vs) in view.virtual_servers.iter() {
        let vs_persist: Vec<String> = vs.persist.paths();
        for rule_ref in vs.rules.paths() {
            let Some(resolved) = resolve_name(&rule_ref, &view.rules, view.default_partition)
            else {
                continue;
            };
            let Some(rule) = view.rules.get(&resolved) else {
                continue;
            };
            if rule.source.is_empty() {
                continue;
            }
            for caps in PERSIST_CMD_RE.captures_iter(&rule.source) {
                let Some(m) = caps.get(1) else { continue };
                let persist_name = clean_name(m.as_str());
                if persist_name == "none"
                    || persist_name.starts_with('$')
                    || persist_name.starts_with('[')
                {
                    continue;
                }
                let Some(resolved_persist) =
                    resolve_name(persist_name, &view.persistence, view.default_partition)
                else {
                    // Unknown profile — BIGIP6002 covers missing objects.
                    continue;
                };
                let in_vs = vs_persist.iter().any(|vp| {
                    resolve_name(vp, &view.persistence, view.default_partition).as_deref()
                        == Some(resolved_persist.as_str())
                });
                if !in_vs {
                    out.push(ConfigDiagnostic {
                        code: DiagCode::Bigip6010.as_str().to_owned(),
                        message: format!(
                            "iRule '{}' on virtual '{}' uses persistence profile '{persist_name}' \
                             which is not attached to the virtual server.",
                            rule.name, vs.name
                        ),
                        severity: DiagSeverity::Hint,
                        subject: ConfigDiagnosticSubject::VirtualServer,
                        range: object_range(vs.range),
                    });
                }
            }
        }
    }
}

// Config-wide checks

/// BIGIP6006: data-group defined but never referenced by any iRule.
fn check_unused_data_groups(view: &ModelView<'_>, out: &mut Vec<ConfigDiagnostic>) {
    let mut referenced: HashSet<String> = HashSet::new();
    for (_path, rule) in view.rules.iter() {
        for raw in collect_referenced_data_groups(rule) {
            if let Some(resolved) = resolve_name(&raw, &view.data_groups, view.default_partition) {
                referenced.insert(resolved);
            }
        }
    }
    for (dg_path, dg) in view.data_groups.iter() {
        if !referenced.contains(dg_path) {
            out.push(ConfigDiagnostic {
                code: DiagCode::Bigip6006.as_str().to_owned(),
                message: format!(
                    "Data-group '{}' is defined but not referenced by any iRule in this configuration.",
                    dg.name
                ),
                severity: DiagSeverity::Hint,
                subject: ConfigDiagnosticSubject::DataGroup,
                range: object_range(dg.range),
            });
        }
    }
}

/// BIGIP6008: pool has no members.
fn check_empty_pools(view: &ModelView<'_>, out: &mut Vec<ConfigDiagnostic>) {
    for (_path, pool) in view.pools.iter() {
        if pool.members.is_empty() {
            out.push(ConfigDiagnostic {
                code: DiagCode::Bigip6008.as_str().to_owned(),
                message: format!("Pool '{}' has no members defined.", pool.name),
                severity: DiagSeverity::Hint,
                subject: ConfigDiagnosticSubject::Pool,
                range: object_range(pool.range),
            });
        }
    }
}

/// BIGIP6011: validate records in IP-type data-groups are valid IPv4/IPv6
/// addresses or networks.
fn check_ip_data_group_records(view: &ModelView<'_>, out: &mut Vec<ConfigDiagnostic>) {
    for (_path, dg) in view.data_groups.iter() {
        if dg.value_type != "ip" {
            continue;
        }
        for record in &dg.records {
            let addr_text = if record.contains('/') {
                record.split('/').next().unwrap_or("").trim()
            } else {
                record.trim()
            };
            if addr_text.is_empty() {
                continue;
            }
            let addr_ok = addr_text.parse::<std::net::IpAddr>().is_ok();
            // Fall back to a network form ("10.0.0.0/8"), parsed non-strictly
            // (host bits need not be zero).
            let net_ok = record.trim().parse::<ipnet::IpNet>().is_ok();
            if !addr_ok && !net_ok {
                out.push(ConfigDiagnostic {
                    code: DiagCode::Bigip6011.as_str().to_owned(),
                    message: format!(
                        "Invalid IP address '{record}' in IP-type data-group '{}'",
                        dg.name
                    ),
                    severity: DiagSeverity::Warning,
                    subject: ConfigDiagnosticSubject::DataGroup,
                    range: object_range(dg.range),
                });
            }
        }
    }
}

// Registry-object checks

/// True when an object-reference spelling is static enough to validate. Tcl
/// substitutions and glob expressions deliberately remain unknown: reporting
/// them as dangling would turn a runtime choice into a false positive.
fn is_static_reference(value: &str) -> bool {
    let value = clean_name(value).trim();
    !value.is_empty()
        && !value.starts_with('$')
        && !value.starts_with('[')
        && !value.contains('$')
        && !value.contains('[')
        && !value.contains('*')
        && !matches!(
            value.to_ascii_lowercase().as_str(),
            "none" | "default" | "enabled" | "disabled"
        )
}

/// Values which can occur in a registry-declared reference property. A keyed
/// block's keys are references (profile attachments, pool members, and the
/// like); a list contributes each literal item; the scalar fallback keeps
/// ordinary `pool /Common/p` properties covered.
fn reference_values(raw: &str) -> Vec<String> {
    let mut values = Vec::new();
    values.extend(
        parse_keyed_block_entries(raw)
            .into_iter()
            .map(|(key, _)| key),
    );
    if values.is_empty() {
        values.extend(parse_list_block(raw));
    }
    if values.is_empty() {
        values.push(raw.trim().to_owned());
    }
    values
        .into_iter()
        .filter(|value| is_static_reference(value))
        .collect()
}

fn property_reference_values(property: &str, raw: &str) -> Vec<String> {
    if property == "monitor"
        && let Some(expression) = MonitorExpression::try_parse(raw)
    {
        return expression
            .monitors
            .into_iter()
            .filter(|value| is_static_reference(value))
            .collect();
    }
    reference_values(raw)
}

fn reference_candidates(reference: &str, owner: &str, default_partition: &str) -> Vec<String> {
    let mut reference = clean_name(reference).trim().to_owned();
    // Node and virtual-address references often include `:port`, while their
    // declaration keys do not. Keeping this here makes the generic registry
    // path agree with the graph resolver.
    let path_length = if reference.matches(':').count() == 1 {
        reference
            .rsplit_once(':')
            .filter(|(_, port)| port.bytes().all(|byte| byte.is_ascii_digit()))
            .map(|(path, _)| path.len())
    } else {
        None
    };
    if let Some(path_length) = path_length {
        reference.truncate(path_length);
    }
    if reference.starts_with('/') {
        return vec![reference];
    }
    let partition = owner
        .strip_prefix('/')
        .and_then(|path| path.split('/').next())
        .filter(|partition| !partition.is_empty())
        .unwrap_or(default_partition.trim_matches('/'));
    let mut candidates = vec![format!("/{partition}/{reference}")];
    if partition != "Common" {
        candidates.push(format!("/Common/{reference}"));
    }
    candidates
}

/// BIGIP6013: validate every static registry-declared outbound edge that is
/// not already owned by a specialised BIGIP6001–6012 check. This intentionally
/// consumes `REFERENCE_EDGES` rather than a hand-maintained object-name table.
fn check_registry_references(config: &BigipConfig, out: &mut Vec<ConfigDiagnostic>) {
    let registry = tcl_registry::bigip::BigipRegistry::build();
    let objects: Vec<(&crate::model::BigipGenericObject, &'static str)> = config
        .generic_objects
        .iter()
        .filter_map(|(_, object)| {
            registry
                .kind_for_header(&object.module, &object.object_type)
                .map(|kind| (object, kind))
        })
        .collect();

    for (object, from_kind) in &objects {
        for (property, parsed) in parse_properties_with_spans(&object.body) {
            // BIGIP6003 and BIGIP6005 already own these two virtual-server
            // links. All other registry edges, including monitors, members,
            // profile attachments, policies and tenant-local objects, converge
            // on this generic validator.
            if *from_kind == "ltm_virtual" && matches!(property.as_str(), "rules" | "pool") {
                continue;
            }
            for edge in tcl_registry::bigip::reference_edges_from(from_kind)
                .filter(|edge| edge.section.is_none() && edge.property == property)
            {
                for value in property_reference_values(&property, &parsed.value) {
                    let candidates =
                        reference_candidates(&value, &object.identifier, &config.default_partition);
                    let resolved =
                        objects.iter().any(|(target, target_kind)| {
                            edge.to_kinds.contains(target_kind)
                                && candidates
                                    .iter()
                                    .any(|candidate| candidate == &target.identifier)
                        }) || resolves_factory_reference(&registry, edge.to_kinds, &candidates);
                    if !resolved {
                        out.push(ConfigDiagnostic {
                            code: DiagCode::Bigip6013.as_str().to_owned(),
                            message: format!(
                                "{} '{}' has registry-declared reference '{}' in property '{}' which could not be resolved; define a compatible target or use a static full path.",
                                object.object_type, object.identifier, value, property
                            ),
                            severity: DiagSeverity::Warning,
                            subject: ConfigDiagnosticSubject::Object,
                            range: object_range(object.range),
                        });
                    }
                }
            }
        }
    }
}

fn resolves_factory_reference(
    registry: &tcl_registry::bigip::BigipRegistry,
    target_kinds: &[&str],
    candidates: &[String],
) -> bool {
    target_kinds
        .iter()
        .filter_map(|kind| registry.get(kind))
        .flat_map(|spec| spec.header_types)
        .any(|&(module, object_type)| {
            candidates.iter().any(|candidate| {
                factory_object_exists(module, object_type, candidate)
                    || candidate
                        .strip_prefix("/Common/")
                        .is_some_and(|name| factory_object_exists(module, object_type, name))
            })
        })
}

fn factory_object_exists(module: &str, object_type: &str, name: &str) -> bool {
    FACTORY_OBJECT_IDENTITIES.contains(&factory_object_identity(module, object_type, name))
}

fn factory_object_identity(module: &str, object_type: &str, name: &str) -> String {
    format!("{module}\0{object_type}\0{name}")
}

/// BIGIP6014: duplicate declarations are ambiguous even when the source
/// parser's last-write-wins model can continue. The registry kind prevents two
/// unrelated object families that happen to share a path from being conflated.
fn check_duplicate_registry_objects(config: &BigipConfig, out: &mut Vec<ConfigDiagnostic>) {
    let registry = tcl_registry::bigip::BigipRegistry::build();
    let mut seen: HashSet<(&str, &str)> = HashSet::new();
    for (_, object) in &config.generic_objects {
        let Some(kind) = registry.kind_for_header(&object.module, &object.object_type) else {
            continue;
        };
        if !object.identifier.is_empty() && !seen.insert((kind, &object.identifier)) {
            out.push(ConfigDiagnostic {
                code: DiagCode::Bigip6014.as_str().to_owned(),
                message: format!(
                    "Duplicate {} declaration for '{}'; retain one authoritative object definition.",
                    object.object_type, object.identifier
                ),
                severity: DiagSeverity::Warning,
                subject: ConfigDiagnosticSubject::Object,
                range: object_range(object.range),
            });
        }
    }
}

fn profile_default_security_field(profile: &str, field: &str) -> bool {
    matches!(
        (profile, field),
        (
            "HTTP",
            "insert-xforwarded-for" | "request-chunking" | "response-chunking",
        ) | ("TCP", "nagle" | "reset-on-timeout")
            | (
                "CLIENTSSL" | "SERVERSSL",
                "ciphers"
                    | "cipher-group"
                    | "secure-renegotiation"
                    | "renegotiation"
                    | "peer-cert-mode"
                    | "sni-require"
                    | "ca-file"
                    | "crl-file"
            )
            | (
                "PERSIST",
                "cookie-encryption" | "httponly" | "secure" | "timeout"
            )
    )
}

/// Compare each explicitly-modelled HTTP, TCP and SSL profile field with the
/// embedded default table for `version`. With no version this deliberately
/// emits `UnknownVersion` evidence instead of applying the newest snapshot.
///
/// The report and LSP can use this structured evidence to distinguish an
/// inherited value from an explicit security-relevant deviation without ever
/// describing normal customisation as a vulnerability. Persistence is
/// represented separately by its own tmsh family and currently has no
/// versioned default snapshot, so its evidence remains explicitly unknown.
#[must_use]
pub fn collect_profile_default_evidence(
    config: &BigipConfig,
    version: Option<BigipVersion>,
) -> Vec<ProfileDefaultEvidence> {
    let view = ModelView::build(config);
    let mut evidence = Vec::new();
    for (_path, profile) in view.profiles.iter() {
        let profile_type = registry_profile_name(profile.profile_type);
        if profile_type == "OTHER" {
            continue;
        }
        let fields = profile.canon_fields();
        let Some(fields) = fields.as_object() else {
            continue;
        };
        let defaults = profile_field_defaults(profile_type, version);
        let default_fields: Vec<(String, Option<String>)> = match defaults.as_slice() {
            [_first, ..] => defaults
                .into_iter()
                .map(|(field, value)| (field.to_owned(), version.map(|_| value.to_owned())))
                .collect(),
            _ => match profile_type {
                // Requested first-class families with no known versioned row
                // still produce an explicit evidence limitation.
                "HTTP" | "TCP" | "CLIENTSSL" | "SERVERSSL" => {
                    vec![("defaults-from".to_owned(), None)]
                }
                _ => Vec::new(),
            },
        };
        for (field, default_value) in default_fields {
            let configured = fields
                .get(&field.replace('-', "_"))
                .and_then(serde_json::Value::as_str)
                .filter(|value| !value.is_empty())
                .map(str::to_owned);
            let state = match (&configured, &default_value, version) {
                (_, None, _) | (_, _, None) => ProfileDefaultState::UnknownVersion,
                (Some(value), Some(default), _) if value == default => ProfileDefaultState::Factory,
                (Some(_), Some(_), _) => ProfileDefaultState::Explicit,
                (None, Some(_), _) if !profile.defaults_from.is_empty() => {
                    ProfileDefaultState::Inherited
                }
                (None, Some(_), _) => ProfileDefaultState::Factory,
            };
            evidence.push(ProfileDefaultEvidence {
                owner: profile.full_path.clone(),
                profile_type: profile_type.to_owned(),
                security_relevant: profile_default_security_field(profile_type, &field),
                field,
                configured,
                default_value,
                state,
            });
        }
    }
    for (_path, persistence) in view.persistence.iter() {
        for (field, value) in [
            ("cookie-encryption", &persistence.cookie_encryption),
            ("httponly", &persistence.httponly),
            ("secure", &persistence.secure),
            ("timeout", &persistence.timeout),
        ] {
            evidence.push(ProfileDefaultEvidence {
                owner: persistence.full_path.clone(),
                profile_type: "PERSIST".to_owned(),
                field: field.to_owned(),
                configured: (!value.is_empty()).then(|| value.clone()),
                default_value: None,
                state: ProfileDefaultState::UnknownVersion,
                security_relevant: profile_default_security_field("PERSIST", field),
            });
        }
    }
    evidence
}

// Public API

/// Run all BIG-IP cross-reference validations over a parsed config,
/// returning ranged diagnostics in a stable, deterministic order (per-iRule
/// checks, then virtual-server checks, then config-wide checks).
#[must_use]
pub fn validate_bigip_config(config: &BigipConfig) -> Vec<ConfigDiagnostic> {
    let view = ModelView::build(config);
    let mut out: Vec<ConfigDiagnostic> = Vec::new();

    // Per-iRule checks.
    for (_path, rule) in view.rules.iter() {
        check_irule_data_groups(rule, &view, &mut out);
        check_irule_pools(rule, &view, &mut out);
        check_irule_snatpools(rule, &view, &mut out);
    }

    // Virtual-server reference checks.
    check_virtual_rules(&view, &mut out);
    check_virtual_rule_priority_conflicts(&view, &mut out);
    check_virtual_pools(&view, &mut out);
    check_virtual_profile_requirements(&view, &mut out);
    check_virtual_event_profile_graph(&view, &mut out);
    check_virtual_persistence(&view, &mut out);

    // Config-wide checks.
    check_unused_data_groups(&view, &mut out);
    check_empty_pools(&view, &mut out);
    check_ip_data_group_records(&view, &mut out);
    check_registry_references(config, &mut out);
    check_duplicate_registry_objects(config, &mut out);

    out
}

/// Convenience: parse `source` as a BIG-IP config and validate it.
#[must_use]
pub fn validate_bigip_source(source: &str, default_partition: &str) -> Vec<ConfigDiagnostic> {
    let config = crate::parser::driver::parse_bigip_conf(source, default_partition);
    validate_bigip_config(&config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn published_metadata_describes_each_bigip_validator_rule() {
        let expected = [
            (DiagCode::Bigip6001, "data group not found"),
            (DiagCode::Bigip6002, "pool not found"),
            (DiagCode::Bigip6003, "iRule that is not defined"),
            (DiagCode::Bigip6004, "HTTP:: or SSL:: commands"),
            (DiagCode::Bigip6005, "pool that is not defined"),
            (DiagCode::Bigip6006, "not referenced by any iRule"),
            (DiagCode::Bigip6007, "SNAT pool not found"),
            (DiagCode::Bigip6008, "no members defined"),
            (DiagCode::Bigip6009, "duplicate iRule attachment"),
            (DiagCode::Bigip6010, "persistence profile"),
            (DiagCode::Bigip6011, "invalid IP address or network record"),
            (
                DiagCode::Bigip6012,
                "same event at the same effective priority",
            ),
            (DiagCode::Bigip6013, "reference could not be resolved"),
            (DiagCode::Bigip6014, "duplicates another object"),
            (DiagCode::Bigip6038, "event requires a profile"),
            (DiagCode::Bigip6039, "incompatible profile types"),
        ];
        assert_eq!(BIGIP_DIAGNOSTIC_CODES, expected.map(|(code, _)| code));
        for (code, expected_phrase) in expected {
            assert!(
                code.description().contains(expected_phrase),
                "{} metadata no longer describes its producer: {}",
                code,
                code.description()
            );
        }
    }

    #[test]
    fn every_diagnostic_subject_has_one_report_tab() {
        let tabs: std::collections::BTreeSet<_> = ConfigDiagnosticSubject::ALL
            .iter()
            .map(|subject| subject.report_tab())
            .collect();
        assert_eq!(
            tabs,
            [
                "apps",
                "dataGroups",
                "objectIndex",
                "pools",
                "rules",
                "virtuals"
            ]
            .into()
        );
    }

    fn codes(source: &str) -> Vec<String> {
        validate_bigip_source(source, "Common")
            .into_iter()
            .map(|d| d.code)
            .collect()
    }

    fn has(source: &str, code: &str) -> bool {
        codes(source).iter().any(|c| c == code)
    }

    /// Sync test (#983 residual): the `class match`/`search` operator
    /// alternation derived here from the shared `BinOp` source of truth must
    /// contain exactly the same word set as `tcl-irules`'s own
    /// `is_class_operator`, so the two independent consumers of the iRules
    /// word-operator family can never drift apart again.
    #[test]
    fn class_operators_regex_matches_irules_word_operator_set() {
        use tcl_syntax::expr::ast::BinOp;
        let expected: HashSet<&str> = [
            BinOp::StrEquals,
            BinOp::StartsWith,
            BinOp::EndsWith,
            BinOp::Contains,
            BinOp::MatchesGlob,
            BinOp::MatchesRegex,
        ]
        .iter()
        .map(|op| op.as_str())
        .collect();
        let derived: HashSet<&str> = CLASS_OPERATORS.split('|').collect();
        assert_eq!(
            derived, expected,
            "CLASS_OPERATORS must contain exactly the iRules word-operator set"
        );
        // And each word is actually recognised by the built regex.
        for word in &expected {
            let src = format!("class match [HTTP::host] {word} /Common/dg\n");
            assert!(
                CLASS_MATCH_RE.is_match(&src),
                "expected '{word}' to be matched by CLASS_MATCH_RE"
            );
        }
    }

    #[test]
    fn bigip6002_fires_for_missing_pool() {
        let src =
            "ltm rule /Common/r {\n  when HTTP_REQUEST {\n    pool /Common/no_such_pool\n  }\n}\n";
        assert!(has(src, "BIGIP6002"));
    }

    #[test]
    fn bigip6002_quiet_when_pool_exists() {
        let src = "ltm pool /Common/web {\n  members {\n    /Common/n:80 { address 10.0.0.1 }\n  }\n}\nltm rule /Common/r {\n  when HTTP_REQUEST {\n    pool /Common/web\n  }\n}\n";
        assert!(!has(src, "BIGIP6002"));
    }

    #[test]
    fn bigip6008_fires_for_empty_pool() {
        let src = "ltm pool /Common/empty {\n}\n";
        assert!(has(src, "BIGIP6008"));
    }

    #[test]
    fn bigip6001_fires_for_missing_data_group() {
        let src = "ltm rule /Common/r {\n  when HTTP_REQUEST {\n    if { [class match [HTTP::host] equals /Common/no_dg] } { }\n  }\n}\n";
        assert!(has(src, "BIGIP6001"));
    }

    #[test]
    fn bigip6011_fires_for_invalid_ip_record() {
        let src = "ltm data-group internal /Common/ips {\n  type ip\n  records {\n    not-an-ip { }\n  }\n}\n";
        assert!(has(src, "BIGIP6011"));
    }

    #[test]
    fn bigip6011_quiet_for_valid_ip_records() {
        let src = "ltm data-group internal /Common/ips {\n  type ip\n  records {\n    10.0.0.0/8 { }\n    192.168.1.1 { }\n  }\n}\n";
        assert!(!has(src, "BIGIP6011"));
    }

    #[test]
    fn bigip6003_fires_for_virtual_referencing_undefined_irule() {
        let src = "ltm virtual /Common/vs {\n  rules {\n    /Common/no_such_rule\n  }\n}\n";
        assert!(has(src, "BIGIP6003"));
    }

    #[test]
    fn bigip6003_quiet_when_irule_is_defined() {
        let src = "ltm rule /Common/r {\n  when HTTP_REQUEST { }\n}\nltm virtual /Common/vs {\n  rules {\n    /Common/r\n  }\n}\n";
        assert!(!has(src, "BIGIP6003"));
    }

    #[test]
    fn bigip6013_uses_registry_edges_and_keeps_dynamic_references_unknown() {
        let missing = "ltm virtual /Tenant/vs {\n  fallback-persistence /Tenant/missing\n}\n";
        let finding = validate_bigip_source(missing, "Tenant")
            .into_iter()
            .find(|diagnostic| diagnostic.code == "BIGIP6013")
            .expect("registry-declared fallback-persistence edge is validated");
        assert_eq!(finding.subject, ConfigDiagnosticSubject::Object);
        assert!(finding.message.contains("fallback-persistence"));

        let dynamic = "ltm virtual /Tenant/vs {\n  fallback-persistence $profile_name\n}\n";
        assert!(!has(dynamic, "BIGIP6013"));
    }

    #[test]
    fn bigip6013_resolves_tenant_and_common_targets_without_a_handwritten_table() {
        let source = "ltm persistence cookie /Common/common_persist { }\n\
                      ltm persistence cookie /Tenant/local_persist { }\n\
                      ltm virtual /Tenant/vs {\n\
                        fallback-persistence local_persist\n\
                      }\n";
        assert!(!has(source, "BIGIP6013"));
        let common = source.replace("local_persist", "common_persist");
        assert!(!has(&common, "BIGIP6013"));
    }

    #[test]
    fn bigip6013_recognises_registry_owned_factory_profiles_only() {
        let factory = "ltm virtual /Common/vs {\n  profiles {\n    /Common/http { }\n    /Common/tcp { }\n    /Common/clientssl { context clientside }\n  }\n}\n";
        assert!(!has(factory, "BIGIP6013"));

        let mutated = factory.replace("/Common/clientssl", "/Common/not-a-factory-profile");
        assert!(has(&mutated, "BIGIP6013"));
    }

    #[test]
    fn bigip6014_reports_only_same_kind_and_path_duplicates() {
        let source = "ltm pool /Tenant/p { }\nltm pool /Tenant/p { }\n\
                      ltm rule /Tenant/p { when RULE_INIT { } }\n";
        let diagnostics: Vec<_> = validate_bigip_source(source, "Tenant")
            .into_iter()
            .filter(|diagnostic| diagnostic.code == "BIGIP6014")
            .collect();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].subject, ConfigDiagnosticSubject::Object);
    }

    #[test]
    fn bigip6038_uses_the_event_profile_graph_for_all_registered_events() {
        let source = "ltm rule /Common/http_rule {\n\
                        when HTTP_REQUEST { return }\n\
                      }\n\
                      ltm virtual /Common/vs {\n\
                        rules { /Common/http_rule }\n\
                        profiles { /Common/tcp { } }\n\
                      }\n";
        let finding = validate_bigip_source(source, "Common")
            .into_iter()
            .find(|diagnostic| diagnostic.code == "BIGIP6038")
            .expect("HTTP_REQUEST is unreachable without HTTP");
        assert!(finding.message.contains("HTTP_REQUEST"));
        assert!(finding.message.contains("priority 500"));

        let with_http = format!(
            "ltm profile http /Common/http_profile {{ }}\n{}",
            source.replace(
                "profiles { /Common/tcp { } }",
                "profiles { /Common/tcp { } /Common/http_profile { } }",
            )
        );
        assert!(!has(&with_http, "BIGIP6038"));
    }

    #[test]
    fn versioned_profile_default_evidence_never_guesses_an_unknown_tmos_release() {
        let source = "ltm profile client-ssl /Common/client {\n  ciphers WEAK\n}\n\
                      ltm persistence cookie /Common/persist {\n  secure disabled\n}\n";
        let config = crate::parser::driver::parse_bigip_conf(source, "Common");
        let unknown = collect_profile_default_evidence(&config, None);
        assert!(
            unknown.iter().any(|evidence| {
                evidence.owner == "/Common/client"
                    && evidence.field == "ciphers"
                    && evidence.state == ProfileDefaultState::UnknownVersion
            }),
            "{unknown:#?}"
        );
        assert!(unknown.iter().any(|evidence| {
            evidence.owner == "/Common/persist"
                && evidence.field == "secure"
                && evidence.state == ProfileDefaultState::UnknownVersion
                && evidence.security_relevant
        }));

        let known = collect_profile_default_evidence(&config, Some(BigipVersion::new(17, 1, 0, 0)));
        assert!(known.iter().any(|evidence| {
            evidence.owner == "/Common/client"
                && evidence.field == "ciphers"
                && evidence.state == ProfileDefaultState::Explicit
                && evidence.default_value.is_some()
                && evidence.security_relevant
        }));
    }

    #[test]
    fn bigip6009_fires_for_duplicate_irule_attachment() {
        let src = "ltm rule /Common/r {\n  when HTTP_REQUEST { }\n}\nltm virtual /Common/vs {\n  rules {\n    /Common/r\n    /Common/r\n  }\n}\n";
        assert!(has(src, "BIGIP6009"));
    }

    #[test]
    fn bigip6012_fires_for_implicit_priority_conflict() {
        let src = "ltm rule /Common/first {\n  when HTTP_REQUEST { one }\n}\n\
                   ltm rule /Common/second {\n  when HTTP_REQUEST { two }\n}\n\
                   ltm virtual /Common/vs {\n  rules { /Common/first /Common/second }\n}\n";
        let diagnostics = validate_bigip_source(src, "Common");
        let diagnostic = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "BIGIP6012")
            .expect("same implicit priority conflicts");
        assert_eq!(diagnostic.severity, DiagSeverity::Warning);
        assert!(diagnostic.message.contains("HTTP_REQUEST"));
        assert!(diagnostic.message.contains("priority 500"));
        assert!(diagnostic.message.contains("first"));
        assert!(diagnostic.message.contains("second"));
    }

    #[test]
    fn bigip6012_honours_top_level_priority_and_inline_override() {
        let src = "ltm rule /Common/first {\n  priority 200\n  when HTTP_REQUEST { one }\n}\n\
                   ltm rule /Common/second {\n  priority 100\n  when CLIENT_ACCEPTED { ignored }\n  priority 200\n  when HTTP_REQUEST { two }\n  when HTTP_RESPONSE priority 300 { three }\n}\n\
                   ltm rule /Common/third {\n  when HTTP_RESPONSE priority 300 { four }\n}\n\
                   ltm virtual /Common/vs {\n  rules { /Common/first /Common/second /Common/third }\n}\n";
        let diagnostics: Vec<_> = validate_bigip_source(src, "Common")
            .into_iter()
            .filter(|diagnostic| diagnostic.code == "BIGIP6012")
            .collect();
        assert_eq!(diagnostics.len(), 2);
        assert!(diagnostics[0].message.contains("HTTP_REQUEST"));
        assert!(diagnostics[0].message.contains("priority 200"));
        assert!(diagnostics[1].message.contains("HTTP_RESPONSE"));
        assert!(diagnostics[1].message.contains("priority 300"));
    }

    #[test]
    fn bigip6012_quiet_for_different_priorities_or_events() {
        let src = "ltm rule /Common/first {\n  when HTTP_REQUEST priority 100 { one }\n}\n\
                   ltm rule /Common/second {\n  when HTTP_REQUEST priority 200 { two }\n  when HTTP_RESPONSE priority 100 { three }\n}\n\
                   ltm virtual /Common/vs {\n  rules { /Common/first /Common/second }\n}\n";
        assert!(!has(src, "BIGIP6012"));
    }

    #[test]
    fn bigip6012_distinguishes_same_named_rules_in_different_partitions() {
        let src = "ltm rule /TenantA/shared {\n  when HTTP_REQUEST { one }\n}\n\
                   ltm rule /TenantB/shared {\n  when HTTP_REQUEST { two }\n}\n\
                   ltm virtual /Common/vs {\n  rules { /TenantA/shared /TenantB/shared }\n}\n";
        let diagnostics = validate_bigip_source(src, "Common");
        let diagnostic = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "BIGIP6012")
            .expect("same-named cross-partition rules remain distinct");
        assert!(diagnostic.message.contains("/TenantA/shared"));
        assert!(diagnostic.message.contains("/TenantB/shared"));
    }

    #[test]
    fn bigip6005_fires_for_virtual_referencing_undefined_pool() {
        let src = "ltm virtual /Common/vs {\n  destination /Common/1.2.3.4:80\n  pool /Common/no_such_pool\n}\n";
        assert!(has(src, "BIGIP6005"));
    }

    #[test]
    fn bigip6004_fires_for_http_command_without_http_profile() {
        let src = "ltm rule /Common/r {\n  when HTTP_REQUEST {\n    HTTP::respond 200\n  }\n}\nltm virtual /Common/vs {\n  rules {\n    /Common/r\n  }\n}\n";
        assert!(has(src, "BIGIP6004"));
    }

    #[test]
    fn bigip6007_fires_for_missing_snatpool() {
        let src = "ltm rule /Common/r {\n  when CLIENT_ACCEPTED {\n    snatpool /Common/no_such_snat\n  }\n}\n";
        assert!(has(src, "BIGIP6007"));
    }

    #[test]
    fn bigip6007_skips_variable_snatpool_reference() {
        // A `$var` / `[cmd]` operand is dynamic — the checker must not flag it.
        let src =
            "ltm rule /Common/r {\n  when CLIENT_ACCEPTED {\n    snatpool $dynamic_sp\n  }\n}\n";
        assert!(!has(src, "BIGIP6007"));
    }

    #[test]
    fn bigip6006_fires_for_unused_data_group() {
        let src = "ltm data-group internal /Common/unused {\n  type string\n  records {\n    foo { }\n  }\n}\n";
        assert!(has(src, "BIGIP6006"));
    }

    #[test]
    fn bigip6006_quiet_when_data_group_is_referenced() {
        let src = "ltm data-group internal /Common/used {\n  type string\n  records {\n    foo { }\n  }\n}\nltm rule /Common/r {\n  when HTTP_REQUEST {\n    if { [class match [HTTP::host] equals /Common/used] } { }\n  }\n}\n";
        assert!(!has(src, "BIGIP6006"));
    }

    #[test]
    fn monitor_reference_extraction_ignores_expression_grammar() {
        assert_eq!(
            property_reference_values("monitor", "/Common/http and /Common/tcp"),
            ["/Common/http", "/Common/tcp"]
        );
        assert_eq!(
            property_reference_values("monitor", "min 1 of { /Common/http /Common/tcp }"),
            ["/Common/http", "/Common/tcp"]
        );
    }

    #[test]
    fn bigip6013_does_not_treat_monitor_operators_as_references() {
        let src = r"
ltm monitor http /Common/http { }
ltm monitor tcp /Common/tcp { }
ltm pool /Common/all {
    monitor /Common/http and /Common/tcp
}
ltm pool /Common/quorum {
    monitor min 1 of { /Common/http /Common/tcp }
}
";
        assert!(!has(src, "BIGIP6013"));
    }

    #[test]
    fn bigip_diagnostic_catalogue_is_complete_and_user_configurable() {
        let expected = [
            "BIGIP6001",
            "BIGIP6002",
            "BIGIP6003",
            "BIGIP6004",
            "BIGIP6005",
            "BIGIP6006",
            "BIGIP6007",
            "BIGIP6008",
            "BIGIP6009",
            "BIGIP6010",
            "BIGIP6011",
            "BIGIP6012",
            "BIGIP6013",
            "BIGIP6014",
            "BIGIP6038",
            "BIGIP6039",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
        let actual: Vec<_> = BIGIP_DIAGNOSTIC_CODES
            .iter()
            .map(|code| code.as_str().to_owned())
            .collect();
        assert_eq!(actual, expected);
        for code in BIGIP_DIAGNOSTIC_CODES {
            assert_eq!(
                code.diag_section(),
                Some(tcl_core_types::DiagSection::Bigip)
            );
            assert!(!code.is_internal());
            assert!(!code.is_reserved());
        }
    }
}
