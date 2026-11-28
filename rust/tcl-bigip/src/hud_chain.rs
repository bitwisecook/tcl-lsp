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

//! The HUD chain of a virtual server: its HUD filters (profiles) and the
//! `when` handlers of its attached iRules, merged into one table per event.
//!
//! Filter facts (layer, side, the events a filter stack can raise) come from
//! the registry's profile and event tables; handler priorities come from the
//! same `tcl_irules` projection the validator's priority checks use. Handlers
//! are ordered by priority, then rule-list position, then source position:
//! the order F5 documents, unmeasured on a device (plan Q16, Q31), so
//! equal-priority handlers also carry `tied_with`.
//!
//! Profiles resolve to a config object, else a base-configuration profile,
//! then to a registry type through `classify_profile`. Two of the stack
//! assumptions are open lab questions: http2, http-compression and
//! web-acceleration type as HTTP (Q18), and a non-UDP virtual without a
//! transport profile gets TCP (Q19). Plan item 444 replaces the typing with
//! `schema-kind` and projects the chain into the positioned context's
//! `HostBinding`.

use std::collections::HashMap;

use serde::Serialize;
use tcl_bigip_schema::base_objects::base_object_identities;
use tcl_irules::{OriginalIrulesSourceContext, extract_original_irules_event_handlers};
use tcl_registry::events::EventRegistry;
use tcl_registry::profiles::ProfileRegistry;

use crate::lint::{ModelView, resolve_name};
use crate::model::{BigipVirtualServer, ProfileType};
use crate::parser::driver::BigipConfig;
use crate::value::bigip_list::ListItemValue;

/// Where a filter's profile type was read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FilterTypeSource {
    /// A profile object in the supplied configuration.
    Config,
    /// A profile the BIG-IP base configuration ships (`/Common/http`, …).
    BaseObject,
    /// Neither: the type is unknown.
    Unresolved,
}

/// One HUD filter: a profile attached to the virtual server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HudFilter {
    /// The reference as written on the virtual server.
    pub reference: String,
    /// The resolved full path, when the configuration defines the profile.
    pub full_path: Option<String>,
    /// The registry's profile-stack name (`HTTP`, `CLIENTSSL`, …).
    pub profile_type: Option<&'static str>,
    /// Where the type came from.
    pub type_source: FilterTypeSource,
    /// The registry's protocol-stack layer (`transport`, `tls`, …).
    pub layer: Option<&'static str>,
    /// The registry's side for the profile type (`client`, `server`, `both`).
    pub side: Option<&'static str>,
    /// The attachment's `context` (`clientside`, `serverside`, `all`), if written.
    pub context: Option<String>,
}

/// One iRule attached to the virtual server, in rule-list order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HudRule {
    /// The reference as written on the virtual server.
    pub reference: String,
    /// The resolved full path, when the configuration defines the rule.
    pub full_path: Option<String>,
    /// One-based position in the virtual server's rule list.
    pub position: usize,
    /// Whether the rule's handlers could be read from its source.
    pub handlers_known: bool,
}

/// One `when` handler in an event's merged table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HudHandler {
    /// Full path of the rule that declares it.
    pub rule: String,
    /// The rule's one-based rule-list position.
    pub rule_position: usize,
    /// One-based position among this rule's handlers for the event.
    pub ordinal: usize,
    /// Effective priority (explicit, inherited from `priority N`, or 500).
    pub priority: u16,
    /// Other rules with a handler for this event at the same priority; their
    /// relative order is F5-documented, not measured.
    pub tied_with: Vec<String>,
}

/// Every handler for one event, in execution order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HudEvent {
    /// The event name.
    pub event: String,
    /// The registry's multiplicity (`init`, `once_per_connection`, `per_request`, …).
    pub multiplicity: &'static str,
    /// Profile types the registry says the event needs (any one of them).
    pub requires: Vec<&'static str>,
    /// Whether the filter stack raises the event: `None` when the event is
    /// unknown to the registry or an unresolved filter leaves it undecided.
    pub raised: Option<bool>,
    /// Handlers in execution order.
    pub handlers: Vec<HudHandler>,
}

/// The HUD chain of one virtual server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HudChain {
    /// The virtual server's full path.
    pub virtual_server: String,
    /// The destination as written, if any.
    pub destination: Option<String>,
    /// Filters ordered client to server by registry layer, then as written.
    pub filters: Vec<HudFilter>,
    /// Filters assumed but not listed: TCP on a non-UDP virtual with no
    /// transport profile (unmeasured, Q19).
    pub implied_filters: Vec<&'static str>,
    /// Attached iRules in rule-list order.
    pub rules: Vec<HudRule>,
    /// Events with at least one handler, in canonical firing order.
    pub events: Vec<HudEvent>,
}

/// The HUD chain of `virtual_name` (full path or short name), or `None` when
/// the configuration has no such virtual server.
#[must_use]
pub fn hud_chain(config: &BigipConfig, virtual_name: &str) -> Option<HudChain> {
    let view = ModelView::build(config);
    let path = resolve_name(virtual_name, &view.virtual_servers, view.default_partition)?;
    let vs = view.virtual_servers.get(&path)?;
    Some(build(&view, &path, vs))
}

/// The HUD chain of every virtual server, in configuration order.
#[must_use]
pub fn hud_chains(config: &BigipConfig) -> Vec<HudChain> {
    let view = ModelView::build(config);
    view.virtual_servers
        .iter()
        .map(|(path, vs)| build(&view, path, vs))
        .collect()
}

/// The registry profile-stack name for a typed model profile, or `None` for
/// a profile the model does not classify.
#[must_use]
pub(crate) fn registry_profile_name(profile: ProfileType) -> Option<&'static str> {
    Some(match profile {
        ProfileType::ClientSsl => "CLIENTSSL",
        ProfileType::ServerSsl => "SERVERSSL",
        ProfileType::OneConnect => "ONECONNECT",
        ProfileType::Websocket => "WS",
        ProfileType::Persistence => "PERSIST",
        ProfileType::Other => return None,
        _ => profile.py_name(),
    })
}

/// The typed profile of each attachment on `vs`: a configuration object
/// first, then a profile of the shipped base configuration.
pub(crate) fn attached_profile_types(
    view: &ModelView<'_>,
    vs: &BigipVirtualServer,
) -> Vec<(
    String,
    Option<String>,
    Option<ProfileType>,
    FilterTypeSource,
)> {
    vs.profiles
        .paths()
        .into_iter()
        .map(|reference| {
            let resolved = resolve_name(&reference, &view.profiles, view.default_partition);
            if let Some(profile) = resolved.as_deref().and_then(|p| view.profiles.get(p)) {
                let full_path = resolved.clone();
                return (
                    reference,
                    full_path,
                    Some(profile.profile_type),
                    FilterTypeSource::Config,
                );
            }
            match base_profile_type(&reference, view.default_partition) {
                Some(kind) => (reference, None, Some(kind), FilterTypeSource::BaseObject),
                None => (reference, None, None, FilterTypeSource::Unresolved),
            }
        })
        .collect()
}

/// The type of a base-configuration profile named by `reference`. Base
/// profiles live in `/Common`.
fn base_profile_type(reference: &str, default_partition: &str) -> Option<ProfileType> {
    let name = match reference.strip_prefix("/Common/") {
        Some(name) => name,
        None if !reference.starts_with('/') && default_partition == "Common" => reference,
        None => return None,
    };
    base_object_identities()
        .find(|(module, object_type, base)| {
            *module == "ltm" && object_type.starts_with("profile ") && *base == name
        })
        .map(|(_, object_type, _)| {
            crate::parser::bespoke::classify_profile(&object_type["profile ".len()..])
        })
}

fn attachment_context(vs: &BigipVirtualServer, reference: &str) -> Option<String> {
    vs.profiles.items.iter().find_map(|item| match &item.value {
        ListItemValue::Profile(attachment)
            if attachment.path == reference && !attachment.context.is_empty() =>
        {
            Some(attachment.context.clone())
        }
        _ => None,
    })
}

fn build(view: &ModelView<'_>, path: &str, vs: &BigipVirtualServer) -> HudChain {
    let profiles = ProfileRegistry::build();
    let filters = build_filters(view, vs, &profiles);
    let mut active: Vec<&str> = filters.iter().filter_map(|f| f.profile_type).collect();
    let mut implied_filters = Vec::new();
    if !active.contains(&"TCP") && !vs.ip_protocol.eq_ignore_ascii_case("udp") {
        active.push("TCP");
        implied_filters.push("TCP");
    }
    let stack_complete = filters.iter().all(|f| f.profile_type.is_some());
    let (rules, handlers) = collect_handlers(view, vs);
    let events = event_tables(&handlers, &profiles, &active, stack_complete);
    HudChain {
        virtual_server: path.to_owned(),
        destination: vs.destination.as_ref().map(ToString::to_string),
        filters,
        implied_filters,
        rules,
        events,
    }
}

fn build_filters(
    view: &ModelView<'_>,
    vs: &BigipVirtualServer,
    profiles: &ProfileRegistry,
) -> Vec<HudFilter> {
    let mut filters: Vec<HudFilter> = attached_profile_types(view, vs)
        .into_iter()
        .map(|(reference, full_path, kind, type_source)| {
            let profile_type = kind.and_then(registry_profile_name);
            let spec = profile_type.and_then(|name| profiles.get_profile(name));
            HudFilter {
                context: attachment_context(vs, &reference),
                reference,
                full_path,
                profile_type,
                type_source,
                layer: spec.map(|s| s.layer),
                side: spec.map(|s| s.side),
            }
        })
        .collect();
    filters.sort_by_key(|f| f.profile_type.map_or(u8::MAX, |t| profiles.layer_rank(t)));
    filters
}

/// The attached rules in rule-list order and every handler they declare,
/// keyed by event.
fn collect_handlers(
    view: &ModelView<'_>,
    vs: &BigipVirtualServer,
) -> (Vec<HudRule>, Vec<(String, HudHandler)>) {
    let commands = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let mut rules = Vec::new();
    let mut handlers = Vec::new();
    for (index, reference) in vs.rules.paths().into_iter().enumerate() {
        let position = index + 1;
        let full_path = resolve_name(&reference, &view.rules, view.default_partition);
        let rule = full_path.as_deref().and_then(|p| view.rules.get(p));
        let context = rule.and_then(|r| OriginalIrulesSourceContext::capture(&r.source, commands));
        if let (Some(rule), Some(context)) = (rule, context.as_ref()) {
            let mut ordinals: HashMap<String, usize> = HashMap::new();
            for handler in extract_original_irules_event_handlers(&rule.source, context) {
                let ordinal = ordinals.entry(handler.event.clone()).or_default();
                *ordinal += 1;
                handlers.push((
                    handler.event,
                    HudHandler {
                        rule: rule.full_path.clone(),
                        rule_position: position,
                        ordinal: *ordinal,
                        priority: handler.priority,
                        tied_with: Vec::new(),
                    },
                ));
            }
        }
        rules.push(HudRule {
            handlers_known: context.is_some(),
            reference,
            full_path,
            position,
        });
    }
    (rules, handlers)
}

fn event_tables(
    handlers: &[(String, HudHandler)],
    profiles: &ProfileRegistry,
    active: &[&str],
    stack_complete: bool,
) -> Vec<HudEvent> {
    let events = EventRegistry::build();
    let mut names: Vec<String> = handlers.iter().map(|(e, _)| e.clone()).collect();
    names.sort();
    names.dedup();
    events
        .order_events(&names)
        .into_iter()
        .map(|event| {
            let mut table: Vec<HudHandler> = handlers
                .iter()
                .filter(|(e, _)| *e == event)
                .map(|(_, h)| h.clone())
                .collect();
            table.sort_by_key(|h| (h.priority, h.rule_position, h.ordinal));
            let snapshot = table.clone();
            for handler in &mut table {
                handler.tied_with = snapshot
                    .iter()
                    .filter(|o| o.priority == handler.priority && o.rule != handler.rule)
                    .map(|o| o.rule.clone())
                    .collect();
                handler.tied_with.dedup();
            }
            let props = events.get_props(&event);
            let raised = props.and_then(|p| {
                if profiles.stack_satisfies(p.implied_profiles, active) {
                    Some(true)
                } else if stack_complete {
                    Some(false)
                } else {
                    None
                }
            });
            HudEvent {
                multiplicity: events.multiplicity(&event),
                requires: props.map_or_else(Vec::new, |p| p.implied_profiles.to_vec()),
                raised,
                handlers: table,
                event,
            }
        })
        .collect()
}
