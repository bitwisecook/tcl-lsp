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

//! Field projection of LTM objects: virtuals, pools, nodes, rules, profiles, monitors, persistence, policies and data groups.

use super::{
    Fields, data_group_kind_str, list_str_values, monitor_value, path_ref, path_ref_list,
    path_ref_list_strs, profile_type_str, typed_str,
};
use crate::eval::Root;
use crate::value::{FieldSlot, ObjectRef, Value};
use indexmap::IndexMap;
use std::rc::Rc;
use tcl_bigip::model::{
    BigipDataGroup, BigipLtmSnatTranslation, BigipMonitor, BigipNode, BigipPersistence,
    BigipPolicy, BigipPolicyAction, BigipPolicyCondition, BigipPolicyRule, BigipPool,
    BigipPoolMember, BigipProfile, BigipRule, BigipSnatPool, BigipVirtualAddress,
    BigipVirtualServer,
};
use tcl_bigip::value::{BigipList, ListItemValue};

pub(super) fn project_virtual(o: &BigipVirtualServer, _root: &Rc<Root>) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .v("destination", typed_str(o.destination.as_ref()))
        .v("pool", path_ref(&o.pool, "ltm pool"))
        // `rules` / `policies` / `vlans` name other objects, so they
        // project as PathRefs and deref on field access.
        .v("rules", path_ref_list(&o.rules, "ltm rule"))
        // `profiles` / `persist` carry per-item context (a context tag, a
        // sub-block), so they project as their rendered spelling instead.
        .v("profiles", list_str_values(&o.profiles))
        .v("persist", list_str_values(&o.persist))
        .v("policies", path_ref_list(&o.policies, "ltm policy"))
        .v("snatpool", path_ref(&o.snatpool, "ltm snatpool"))
        .s("source-address-translation", &o.source_address_translation)
        .s("description", &o.description)
        .s("mask", &o.mask)
        .s("source", &o.source)
        .s("ip-protocol", &o.ip_protocol)
        .s("connection-limit", &o.connection_limit)
        .s("rate-limit", &o.rate_limit)
        .s("rate-limit-mode", &o.rate_limit_mode)
        .s("rate-limit-dst-mask", &o.rate_limit_dst_mask)
        .s("rate-limit-src-mask", &o.rate_limit_src_mask)
        .s("auto-lasthop", &o.auto_lasthop)
        .s("translate-address", &o.translate_address)
        .s("translate-port", &o.translate_port)
        .s("state", &o.state)
        .s("address-status", &o.address_status)
        .s("auto-discovery", &o.auto_discovery)
        .s("cmp-enabled", &o.cmp_enabled)
        .s("eviction-protected", &o.eviction_protected)
        .b("dhcp-relay", o.dhcp_relay)
        .b("internal", o.internal)
        .b("ip-forward", o.ip_forward)
        .b("l2-forward", o.l2_forward)
        .b("reject", o.reject)
        .s("nat64", &o.nat64)
        .s("gtm-score", &o.gtm_score)
        .s("mirror", &o.mirror)
        .s(
            "service-down-immediate-action",
            &o.service_down_immediate_action,
        )
        .s("source-port", &o.source_port)
        .s("serverssl-use-sni", &o.serverssl_use_sni)
        .v("rate-class", path_ref(&o.rate_class, "ltm rate-class"))
        .v(
            "per-flow-request-access-policy",
            path_ref(
                &o.per_flow_request_access_policy,
                "apm policy access-policy",
            ),
        )
        .v(
            "transparent-nexthop",
            path_ref(&o.transparent_nexthop, "net vlan"),
        )
        .v("vlans", path_ref_list(&o.vlans, "net vlan"))
        .b("vlans-disabled", o.vlans_disabled)
        .b("vlans-enabled", o.vlans_enabled)
        .v(
            "fallback-persistence",
            path_ref(&o.fallback_persistence, "ltm persistence"),
        )
        .v("last-hop-pool", path_ref(&o.last_hop_pool, "ltm pool"))
        .s("fw-enforced-policy", &o.fw_enforced_policy)
        .s("fw-staged-policy", &o.fw_staged_policy)
        .s("flow-eviction-policy", &o.flow_eviction_policy)
        .s("service-policy", &o.service_policy)
        .v("auth", path_ref_list_strs(&o.auth_profiles, "ltm profile"))
        .v("traffic-classes", str_list(&o.traffic_classes))
        .v(
            "clone-pools",
            path_ref_list_strs(&o.clone_pools, "ltm pool"),
        )
        .done()
}

pub(super) fn project_virtual_address(o: &BigipVirtualAddress) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .v("address", typed_str(o.address.as_ref()))
        .s("mask", &o.mask)
        .s("arp", &o.arp)
        .s("icmp-echo", &o.icmp_echo)
        .s("auto-delete", &o.auto_delete)
        .s("connection-limit", &o.connection_limit)
        .v(
            "traffic-group",
            path_ref(&o.traffic_group, "cm traffic-group"),
        )
        .s("inherited-traffic-group", &o.inherited_traffic_group)
        .s("route-advertisement", &o.route_advertisement)
        .s("server-scope", &o.server_scope)
        .s("spanning", &o.spanning)
        .s("unit", &o.unit)
        .s("description", &o.description)
        .s("state", &o.state)
        .s("floating", &o.floating)
        .s("traffic-group-restored", &o.traffic_group_restored)
        .done()
}

pub(super) fn project_pool(o: &BigipPool, root: &Rc<Root>) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("module", &o.module)
        .v("monitor", monitor_value(&o.monitor))
        .s("load-balancing-mode", &o.load_balancing_mode)
        .v("members", project_pool_members(&o.members, root))
        .s("description", &o.description)
        .s("min-active-members", &o.min_active_members)
        .s("min-up-members", &o.min_up_members)
        .s("service-down-action", &o.service_down_action)
        .s("slow-ramp-time", &o.slow_ramp_time)
        .s("allow-snat", &o.allow_snat)
        .s("allow-nat", &o.allow_nat)
        .s("reselect-tries", &o.reselect_tries)
        .s("queue-depth-limit", &o.queue_depth_limit)
        .s("queue-time-limit", &o.queue_time_limit)
        .s("connection-limit", &o.connection_limit)
        .s("rate-limit", &o.rate_limit)
        .s("ratio", &o.ratio)
        .s("down-interval", &o.down_interval)
        .s("interval", &o.interval)
        .s("min-up-members-action", &o.min_up_members_action)
        .s("min-up-members-checking", &o.min_up_members_checking)
        .s("ip-tos-to-client", &o.ip_tos_to_client)
        .s("ip-tos-to-server", &o.ip_tos_to_server)
        .s("link-qos-to-client", &o.link_qos_to_client)
        .s("link-qos-to-server", &o.link_qos_to_server)
        .s("gateway-failsafe-device", &o.gateway_failsafe_device)
        .s("ignore-persisted-weight", &o.ignore_persisted_weight)
        .s("inherit-profile", &o.inherit_profile)
        .s("queue-on-connection-limit", &o.queue_on_connection_limit)
        .s("address-family", &o.address_family)
        .s("autopopulate", &o.autopopulate)
        .v("profiles", path_ref_list_strs(&o.profiles, "ltm profile"))
        .done()
}

fn project_pool_members(members: &BigipList, root: &Rc<Root>) -> Value {
    let mut out = Vec::new();
    for item in &members.items {
        if let ListItemValue::PoolMember(m) = &item.value {
            out.push(member_object_ref(m, root));
        }
    }
    Value::List(out)
}

fn member_object_ref(member: &BigipPoolMember, root: &Rc<Root>) -> Value {
    let fields = Fields::new()
        .s("name", &member.name)
        .v("address", typed_str(member.address.as_ref()))
        .i("port", member.port)
        .v(
            "monitor",
            if member.monitor.is_empty() {
                Value::Str(String::new())
            } else {
                path_ref(&member.monitor, "ltm monitor")
            },
        )
        .s("description", &member.description)
        .s("state", &member.state)
        .s("ratio", &member.ratio)
        .s("priority-group", &member.priority_group)
        .s("connection-limit", &member.connection_limit)
        .s("rate-limit", &member.rate_limit)
        .done();
    // Each captured field offset becomes a `FieldSlot` over the value span,
    // so member properties (`address`, `description`, …) are individually
    // editable even though the member is not a stanza of its own.
    let mut field_slots: IndexMap<String, FieldSlot> = IndexMap::new();
    for (key, (start, end)) in &member.field_offsets {
        if let Some(raw_text) = root.source.get(*start..*end) {
            field_slots.insert(
                key.clone(),
                FieldSlot {
                    source_uri: root.uri.clone(),
                    start: *start,
                    end: *end,
                    raw_text: raw_text.to_owned(),
                },
            );
        }
    }
    Value::ObjectRef(Rc::new(ObjectRef {
        kind: "ltm pool-member".to_owned(),
        full_path: member.name.clone(),
        fields,
        field_slots,
        stanza_slot: None,
        config_uri: root.uri.clone(),
    }))
}

pub(super) fn project_node(o: &BigipNode, _root: &Rc<Root>) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .v("address", typed_str(o.address.as_ref()))
        .s("description", &o.description)
        .v("monitor", monitor_value(&o.monitor))
        .s("state", &o.state)
        .s("connection-limit", &o.connection_limit)
        .s("rate-limit", &o.rate_limit)
        .s("ratio", &o.ratio)
        .v("fqdn", typed_str(o.fqdn.as_ref()))
        .done()
}

pub(super) fn project_rule(o: &BigipRule, root: &Rc<Root>) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("body", &o.source)
        .s("description", &o.description)
        // `.refs` is the synthesised reference sub-object — the same
        // forward-walk (`max_depth=1`) `f5 grep` uses, classified into
        // pool / persistence / data-group buckets.
        .v("refs", rule_refs_value(o, root))
        .done()
}

/// The synthesised `ltm rule .refs` sub-object.
///
/// Each ref slot is a list of [`PathRef`]s drawn from the same reference
/// graph `f5 grep` walks (forward, `max_depth=1`), so the query DSL and the
/// grep verb always agree on what an iRule "uses". The graph is built once
/// per [`Root`] and memoised (see [`Root::graph`]).
pub(super) fn rule_refs_value(o: &BigipRule, root: &Rc<Root>) -> Value {
    let (pools, persists, data_groups) = crate::builtins::extract_rule_refs(&o.full_path, root);
    let fields = Fields::new()
        .v("pools", path_ref_list_strs(&pools, "ltm pool"))
        .v("persists", path_ref_list_strs(&persists, "ltm persistence"))
        .v(
            "data-groups",
            path_ref_list_strs(&data_groups, "ltm data-group"),
        )
        .done();
    Value::ObjectRef(Rc::new(ObjectRef {
        kind: "ltm rule-refs".to_owned(),
        full_path: o.full_path.clone(),
        fields,
        field_slots: IndexMap::new(),
        stanza_slot: None,
        config_uri: root.uri.clone(),
    }))
}

pub(super) fn project_profile(o: &BigipProfile, _root: &Rc<Root>) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .v("type", profile_type_str(o.profile_type))
        .v("defaults-from", path_ref(&o.defaults_from, "ltm profile"))
        .s("description", &o.description)
        .s("idle-timeout", &o.idle_timeout)
        .s("insert-xforwarded-for", &o.insert_xforwarded_for)
        .s("request-chunking", &o.request_chunking)
        .s("response-chunking", &o.response_chunking)
        .s("lws-max-columns", &o.lws_max_columns)
        .s("lws-separator", &o.lws_separator)
        .s("server-agent-name", &o.server_agent_name)
        .s("via-request", &o.via_request)
        .s("via-response", &o.via_response)
        .s("ciphers", &o.ciphers)
        .v(
            "cipher-group",
            path_ref(&o.cipher_group, "ltm cipher group"),
        )
        .v("cert", path_ref(&o.cert, "sys file ssl-cert"))
        .v("key", path_ref(&o.key, "sys file ssl-key"))
        .v("chain", path_ref(&o.chain, "sys file ssl-cert"))
        .v("ca-file", path_ref(&o.ca_file, "sys file ssl-cert"))
        .s("crl-file", &o.crl_file)
        .s("cert-extension-includes", &o.cert_extension_includes)
        .s("options", &o.options)
        .s("peer-cert-mode", &o.peer_cert_mode)
        .s("sni-default", &o.sni_default)
        .s("sni-require", &o.sni_require)
        .s("server-name", &o.server_name)
        .s("renegotiation", &o.renegotiation)
        .s("secure-renegotiation", &o.secure_renegotiation)
        .v(
            "proxy-ca-cert",
            path_ref(&o.proxy_ca_cert, "sys file ssl-cert"),
        )
        .v(
            "proxy-ca-key",
            path_ref(&o.proxy_ca_key, "sys file ssl-key"),
        )
        .s("keep-alive-interval", &o.keep_alive_interval)
        .s("ip-tos-to-client", &o.ip_tos_to_client)
        .s("ip-tos-to-server", &o.ip_tos_to_server)
        .s("link-qos-to-client", &o.link_qos_to_client)
        .s("link-qos-to-server", &o.link_qos_to_server)
        .s("nagle", &o.nagle)
        .s("reset-on-timeout", &o.reset_on_timeout)
        .s("send-buffer-size", &o.send_buffer_size)
        .s("receive-window-size", &o.receive_window_size)
        .s("proxy-buffer-low", &o.proxy_buffer_low)
        .s("proxy-buffer-high", &o.proxy_buffer_high)
        .s("pva-acceleration", &o.pva_acceleration)
        .s("pva-dynamic-client-packets", &o.pva_dynamic_client_packets)
        .s("pva-dynamic-server-packets", &o.pva_dynamic_server_packets)
        .s("loose-close", &o.loose_close)
        .s("loose-initialization", &o.loose_initialization)
        .s("datagram-load-balancing", &o.datagram_load_balancing)
        .s("allow-no-payload", &o.allow_no_payload)
        .s("source-mask", &o.source_mask)
        .s("idle-timeout-override", &o.idle_timeout_override)
        .s("max-age", &o.max_age)
        .s("max-reuse", &o.max_reuse)
        .s("max-size", &o.max_size)
        .s("source", &o.source)
        .s("target", &o.target)
        .v("pool", path_ref(&o.pool, "ltm pool"))
        .s(
            "collected-stats-internal-logging",
            &o.collected_stats_internal_logging,
        )
        .s(
            "collected-stats-external-logging",
            &o.collected_stats_external_logging,
        )
        .v(
            "publisher",
            path_ref(&o.publisher, "sys log-config publisher"),
        )
        .s("maximum-bytes", &o.maximum_bytes)
        .s("maximum-entries", &o.maximum_entries)
        .s("maximum-non-json-bytes", &o.maximum_non_json_bytes)
        .s("max-buffered-msg-bytes", &o.max_buffered_msg_bytes)
        .s("max-field-name-size", &o.max_field_name_size)
        .done()
}

pub(super) fn project_monitor(o: &BigipMonitor, _root: &Rc<Root>) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("type", &o.monitor_type)
        .s("description", &o.description)
        .s("interval", &o.interval)
        .s("timeout", &o.timeout)
        .s("destination", &o.destination)
        .s("send", &o.send)
        .s("recv", &o.recv)
        .s("recv-disable", &o.recv_disable)
        .s("username", &o.username)
        .s("password", &o.password)
        .s("base", &o.base)
        .s("filter", &o.filter)
        .s("count", &o.count)
        .s("database", &o.database)
        .s("args", &o.args)
        .s("run", &o.run)
        .s("adaptive", &o.adaptive)
        .s("adaptive-divergence-type", &o.adaptive_divergence_type)
        .s("adaptive-divergence-value", &o.adaptive_divergence_value)
        .s("adaptive-limit", &o.adaptive_limit)
        .s("adaptive-sampling-timespan", &o.adaptive_sampling_timespan)
        .s("transparent", &o.transparent)
        .s("reverse", &o.reverse)
        .s("manual-resume", &o.manual_resume)
        .s("ignore-down-response", &o.ignore_down_response)
        .s("ip-dscp", &o.ip_dscp)
        .s("up-interval", &o.up_interval)
        .s("time-until-up", &o.time_until_up)
        .s("cipherlist", &o.cipherlist)
        .v("cert", path_ref(&o.cert, "sys file ssl-cert"))
        .v("key", path_ref(&o.key, "sys file ssl-key"))
        .s("compatibility", &o.compatibility)
        .s("community", &o.community)
        .s("version", &o.version)
        .s("agent-type", &o.agent_type)
        .s("cpu-coefficient", &o.cpu_coefficient)
        .s("cpu-threshold", &o.cpu_threshold)
        .s("disk-coefficient", &o.disk_coefficient)
        .s("disk-threshold", &o.disk_threshold)
        .s("memory-coefficient", &o.memory_coefficient)
        .s("memory-threshold", &o.memory_threshold)
        .s("headers", &o.headers)
        .s("request", &o.request)
        .s("response", &o.response)
        .s("mode", &o.mode)
        .s("alias-address", &o.alias_address)
        .s("alias-service-port", &o.alias_service_port)
        .v("defaults-from", path_ref(&o.defaults_from, "ltm monitor"))
        .done()
}

pub(super) fn project_persistence(
    o: &BigipPersistence,
    _root: &Rc<Root>,
) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("type", &o.persistence_type)
        .v(
            "defaults-from",
            path_ref(&o.defaults_from, "ltm persistence"),
        )
        .s("description", &o.description)
        .s("timeout", &o.timeout)
        .s("match-across-pools", &o.match_across_pools)
        .s("match-across-services", &o.match_across_services)
        .s("match-across-virtuals", &o.match_across_virtuals)
        .s("mirror", &o.mirror)
        .s("override-connection-limit", &o.override_connection_limit)
        .s("always-send", &o.always_send)
        .s("cookie-name", &o.cookie_name)
        .s("cookie-encryption", &o.cookie_encryption)
        .s(
            "cookie-encryption-passphrase",
            &o.cookie_encryption_passphrase,
        )
        .s("httponly", &o.httponly)
        .s("secure", &o.secure)
        .s("expiration", &o.expiration)
        .s("method", &o.method)
        .s("hash-length", &o.hash_length)
        .s("hash-offset", &o.hash_offset)
        .s("mcp-encryption-passphrase", &o.mcp_encryption_passphrase)
        .done()
}

pub(super) fn project_snatpool(o: &BigipSnatPool) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .v("members", list_str_values(&o.members))
        .s("description", &o.description)
        .done()
}

pub(super) fn project_policy(o: &BigipPolicy, root: &Rc<Root>) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("strategy", &o.strategy)
        .v("controls", str_list(&o.controls))
        .v("requires", str_list(&o.requires))
        .v(
            "rules",
            Value::List(o.rules.iter().map(|r| policy_rule_ref(r, root)).collect()),
        )
        .s("description", &o.description)
        .s("status", &o.status)
        .s("last-modified", &o.last_modified)
        .done()
}

fn policy_rule_ref(rule: &BigipPolicyRule, root: &Rc<Root>) -> Value {
    let fields = Fields::new()
        .s("name", &rule.name)
        .i("ordinal", rule.ordinal)
        .v(
            "conditions",
            Value::List(rule.conditions.iter().map(policy_condition_ref).collect()),
        )
        .v(
            "actions",
            Value::List(
                rule.actions
                    .iter()
                    .map(|a| policy_action_ref(a, root))
                    .collect(),
            ),
        )
        .done();
    Value::ObjectRef(Rc::new(ObjectRef {
        kind: "ltm policy-rule".to_owned(),
        full_path: rule.name.clone(),
        fields,
        field_slots: IndexMap::new(),
        stanza_slot: None,
        config_uri: root.uri.clone(),
    }))
}

fn policy_condition_ref(cond: &BigipPolicyCondition) -> Value {
    let fields = Fields::new()
        .i("index", cond.index)
        .s("operand", &cond.operand)
        .s("selector", &cond.selector)
        .s("operator", &cond.operator)
        .v("values", str_list(&cond.values))
        .s("name", &cond.name)
        .b("negate", cond.negate)
        .b("case-insensitive", cond.case_insensitive)
        .s("event", &cond.event)
        .done();
    Value::ObjectRef(Rc::new(ObjectRef {
        kind: "ltm policy-condition".to_owned(),
        full_path: cond.index.to_string(),
        fields,
        field_slots: IndexMap::new(),
        stanza_slot: None,
        config_uri: String::new(),
    }))
}

fn policy_action_ref(action: &BigipPolicyAction, root: &Rc<Root>) -> Value {
    let fields = Fields::new()
        .i("index", action.index)
        .s("target", &action.target)
        .s("verb", &action.verb)
        .v("pool", path_ref(&action.pool, "ltm pool"))
        .s("location", &action.location)
        .s("name", &action.name)
        .s("value", &action.value)
        .s("path", &action.path)
        .s("query", &action.query)
        .s("host", &action.host)
        .s("event", &action.event)
        .done();
    Value::ObjectRef(Rc::new(ObjectRef {
        kind: "ltm policy-action".to_owned(),
        full_path: action.index.to_string(),
        fields,
        field_slots: IndexMap::new(),
        stanza_slot: None,
        config_uri: root.uri.clone(),
    }))
}

pub(super) fn project_data_group(o: &BigipDataGroup) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("type", &o.value_type)
        .v("kind", data_group_kind_str(o.kind))
        .v("records", str_list(&o.records))
        .s("description", &o.description)
        .done()
}

/// A `Vec<String>` projected as a list of strings.
pub(super) fn str_list(values: &[String]) -> Value {
    Value::List(values.iter().map(|s| Value::Str(s.clone())).collect())
}

pub(super) fn project_snat_translation(o: &BigipLtmSnatTranslation) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("address", &o.address)
        .s("description", &o.description)
        .v(
            "traffic-group",
            path_ref(&o.traffic_group, "cm traffic-group"),
        )
        .s("inherited-traffic-group", &o.inherited_traffic_group)
        .s("connection-limit", &o.connection_limit)
        .s("ip-idle-timeout", &o.ip_idle_timeout)
        .s("tcp-idle-timeout", &o.tcp_idle_timeout)
        .s("udp-idle-timeout", &o.udp_idle_timeout)
        .s("state", &o.state)
        .done()
}
