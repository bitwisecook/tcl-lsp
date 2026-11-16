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

//! Field projection of AFM firewall and NAT objects.

use super::ltm::str_list;
use super::{Fields, path_ref_list_strs};
use crate::value::Value;
use indexmap::IndexMap;
use tcl_bigip::model::{
    BigipSecurityFirewallAddressList, BigipSecurityFirewallPolicy, BigipSecurityFirewallPortList,
    BigipSecurityFirewallRuleList, BigipSecurityNatDestinationTranslation, BigipSecurityNatPolicy,
    BigipSecurityNatSourceTranslation,
};
use tcl_bigip::value::{FirewallEndpoint, FirewallRule};

// Security (AFM firewall + NAT) projections

/// Project a firewall endpoint (source / destination 5-tuple side).
fn fw_endpoint(e: &FirewallEndpoint) -> Value {
    Value::Object(
        Fields::new()
            .v("addresses", str_list(&e.addresses))
            .v(
                "address-lists",
                path_ref_list_strs(&e.address_lists, "security firewall address-list"),
            )
            .v("ports", str_list(&e.ports))
            .v(
                "port-lists",
                path_ref_list_strs(&e.port_lists, "security firewall port-list"),
            )
            .done(),
    )
}

/// Project one firewall / NAT rule to a structured object.
fn fw_rule(r: &FirewallRule) -> Value {
    Value::Object(
        Fields::new()
            .s("name", &r.name)
            .s("action", &r.action)
            .s("ip-protocol", &r.ip_protocol)
            .b("log", r.log)
            .v("source", fw_endpoint(&r.source))
            .v("destination", fw_endpoint(&r.destination))
            .s("rule-list", &r.rule_list)
            .done(),
    )
}

fn fw_rule_list_value(rules: &[FirewallRule]) -> Value {
    Value::List(rules.iter().map(fw_rule).collect())
}

pub(super) fn project_fw_policy(o: &BigipSecurityFirewallPolicy) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("description", &o.description)
        .v("rules", str_list(&o.rules))
        .v(
            "rule-lists",
            path_ref_list_strs(&o.rule_lists, "security firewall rule-list"),
        )
        .done()
}

pub(super) fn project_fw_rule_list(o: &BigipSecurityFirewallRuleList) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("description", &o.description)
        .v("rules", fw_rule_list_value(&o.rule_objects))
        .done()
}

pub(super) fn project_fw_address_list(
    o: &BigipSecurityFirewallAddressList,
) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("description", &o.description)
        .v("addresses", str_list(&o.addresses))
        .v(
            "address-lists",
            path_ref_list_strs(&o.address_lists, "security firewall address-list"),
        )
        .v("fqdns", str_list(&o.fqdns))
        .done()
}

pub(super) fn project_fw_port_list(o: &BigipSecurityFirewallPortList) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("description", &o.description)
        .v("ports", str_list(&o.ports))
        .done()
}

pub(super) fn project_nat_policy(o: &BigipSecurityNatPolicy) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("description", &o.description)
        .v("rules", str_list(&o.rules))
        .v(
            "rule-lists",
            path_ref_list_strs(&o.rule_lists, "security nat rule-list"),
        )
        .done()
}

pub(super) fn project_nat_source_translation(
    o: &BigipSecurityNatSourceTranslation,
) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("description", &o.description)
        .s("type", &o.type_)
        .v("addresses", str_list(&o.addresses))
        .v("ports", str_list(&o.ports))
        .done()
}

pub(super) fn project_nat_destination_translation(
    o: &BigipSecurityNatDestinationTranslation,
) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("description", &o.description)
        .s("type", &o.type_)
        .v("addresses", str_list(&o.addresses))
        .v("ports", str_list(&o.ports))
        .done()
}
