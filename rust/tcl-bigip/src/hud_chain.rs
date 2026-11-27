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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::driver::parse_bigip_conf;

    const CONF: &str = "\
ltm rule /Common/r1 {
when HTTP_REQUEST {
    set u [HTTP::uri]
}
when CLIENT_ACCEPTED { set c 1 }
}
ltm rule /Common/r2 {
priority 100
when HTTP_REQUEST { set u x }
when HTTP_REQUEST priority 500 { log local0. late }
}
ltm rule /Common/r3 {
when HTTP_REQUEST { log local0. r3 }
when CLIENTSSL_HANDSHAKE { log local0. tls }
}
ltm profile client-ssl /Common/my_clientssl { }
ltm virtual /Common/vs1 {
    destination /Common/10.0.0.1:443
    ip-protocol tcp
    profiles {
        /Common/http { }
        /Common/my_clientssl { context clientside }
        /Common/f5-tcp-progressive { }
    }
    rules {
        /Common/r1
        /Common/r2
        /Common/r3
    }
}
ltm virtual /Common/vs_tcp {
    destination /Common/10.0.0.2:80
    profiles {
        /Common/custom_thing { }
    }
    rules {
        /Common/r1
    }
}
";

    fn chain(name: &str) -> HudChain {
        hud_chain(&parse_bigip_conf(CONF, "Common"), name).expect("virtual server")
    }

    #[test]
    fn filters_resolve_config_and_base_profiles_in_layer_order() {
        let c = chain("vs1");
        let types: Vec<_> = c.filters.iter().map(|f| f.profile_type).collect();
        assert_eq!(types, [Some("TCP"), Some("CLIENTSSL"), Some("HTTP")]);
        let sources: Vec<_> = c.filters.iter().map(|f| f.type_source).collect();
        assert_eq!(
            sources,
            [
                FilterTypeSource::BaseObject,
                FilterTypeSource::Config,
                FilterTypeSource::BaseObject
            ]
        );
        assert_eq!(c.filters[1].context.as_deref(), Some("clientside"));
        assert_eq!(c.implied_filters, Vec::<&str>::new());
        assert_eq!(c.destination.as_deref(), Some("/Common/10.0.0.1:443"));
    }

    #[test]
    fn handlers_merge_by_priority_then_rule_list_position() {
        let c = chain("/Common/vs1");
        let req = c
            .events
            .iter()
            .find(|e| e.event == "HTTP_REQUEST")
            .expect("HTTP_REQUEST");
        let order: Vec<_> = req
            .handlers
            .iter()
            .map(|h| (h.rule.as_str(), h.priority, h.ordinal))
            .collect();
        assert_eq!(
            order,
            [
                ("/Common/r2", 100, 1),
                ("/Common/r1", 500, 1),
                ("/Common/r2", 500, 2),
                ("/Common/r3", 500, 1),
            ]
        );
        assert_eq!(req.handlers[0].tied_with, Vec::<String>::new());
        assert_eq!(req.handlers[1].tied_with, ["/Common/r2", "/Common/r3"]);
        assert_eq!(req.raised, Some(true));
        assert_eq!(req.multiplicity, "per_request");
    }

    #[test]
    fn events_follow_canonical_order() {
        let c = chain("vs1");
        let names: Vec<_> = c.events.iter().map(|e| e.event.as_str()).collect();
        assert_eq!(
            names,
            ["CLIENT_ACCEPTED", "CLIENTSSL_HANDSHAKE", "HTTP_REQUEST"]
        );
    }

    #[test]
    fn unresolved_filters_leave_unraised_events_undecided() {
        let c = chain("vs_tcp");
        assert_eq!(c.filters[0].type_source, FilterTypeSource::Unresolved);
        assert_eq!(c.implied_filters, ["TCP"]);
        let req = c
            .events
            .iter()
            .find(|e| e.event == "HTTP_REQUEST")
            .expect("HTTP_REQUEST");
        assert_eq!(req.raised, None);
        let accepted = c
            .events
            .iter()
            .find(|e| e.event == "CLIENT_ACCEPTED")
            .expect("CLIENT_ACCEPTED");
        assert_eq!(accepted.raised, Some(true));
    }

    #[test]
    fn a_complete_stack_without_the_filter_reports_the_event_unraised() {
        let conf = CONF.replace("/Common/custom_thing { }", "/Common/tcp { }");
        let c = hud_chain(&parse_bigip_conf(&conf, "Common"), "vs_tcp").expect("vs");
        let req = c
            .events
            .iter()
            .find(|e| e.event == "HTTP_REQUEST")
            .expect("HTTP_REQUEST");
        assert_eq!(req.raised, Some(false));
        assert_eq!(req.requires, ["FASTHTTP", "HTTP"]);
    }

    #[test]
    fn unknown_rules_are_listed_without_handlers() {
        let conf = CONF.replace("/Common/r3\n", "/Common/missing\n");
        let c = hud_chain(&parse_bigip_conf(&conf, "Common"), "vs1").expect("vs");
        let missing = &c.rules[2];
        assert_eq!(missing.reference, "/Common/missing");
        assert_eq!(missing.full_path, None);
        assert!(!missing.handlers_known);
        assert_eq!(missing.position, 3);
    }

    fn bigip6038(source: &str) -> bool {
        crate::validator::validate_bigip_source(source, "Common")
            .iter()
            .any(|d| d.code == "BIGIP6038")
    }

    #[test]
    fn bigip6038_types_base_configuration_profiles_and_abstains_on_unknown_ones() {
        let rule = "ltm rule /Common/http_rule {\nwhen HTTP_REQUEST { return }\n}\n";
        let vs = |profiles: &str| {
            format!(
                "{rule}ltm virtual /Common/vs {{\n  rules {{ /Common/http_rule }}\n  \
                 profiles {{ {profiles} }}\n}}\n"
            )
        };
        assert!(!bigip6038(&vs(
            "/Common/f5-tcp-progressive { } /Common/http { }"
        )));
        assert!(!bigip6038(&vs("/Common/defined_elsewhere { }")));
        assert!(bigip6038(&vs("/Common/tcp { }")));
    }

    #[test]
    fn every_virtual_server_gets_a_chain() {
        let chains = hud_chains(&parse_bigip_conf(CONF, "Common"));
        let names: Vec<_> = chains.iter().map(|c| c.virtual_server.as_str()).collect();
        assert_eq!(names, ["/Common/vs1", "/Common/vs_tcp"]);
        assert!(hud_chain(&parse_bigip_conf(CONF, "Common"), "nope").is_none());
    }
}
