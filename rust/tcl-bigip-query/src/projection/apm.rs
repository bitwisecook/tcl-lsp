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

//! Field projection of APM objects: access policies, policy items and agents.

use super::ltm::str_list;
use super::{Fields, path_ref, path_ref_list_strs};
use crate::value::Value;
use indexmap::IndexMap;
use tcl_bigip::model::{
    BigipApmEphemeralAuthSshSecurityConfig, BigipApmOauthDbInstance, BigipApmPolicyAccessPolicy,
    BigipApmPolicyAgent, BigipApmPolicyCustomizationSource, BigipApmPolicyItem,
    BigipApmReportDefaultReport,
};

// APM projections

/// `apm policy access-policy` — the VPN / webtop policy graph's root.
///
/// `start-item` / `items[]` / `default-ending` are [`PathRef`]s into
/// `apm policy policy-item`, so `.apm["access-policy"][].start-item.caption`
/// walks the policy in one chain.
pub(super) fn project_apm_access_policy(o: &BigipApmPolicyAccessPolicy) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .v(
            "start-item",
            path_ref(&o.start_item, "apm policy policy-item"),
        )
        .v(
            "default-ending",
            path_ref(&o.default_ending, "apm policy policy-item"),
        )
        .v(
            "items",
            path_ref_list_strs(&o.items, "apm policy policy-item"),
        )
        .s("description", &o.description)
        .done()
}

pub(super) fn project_apm_policy_item(o: &BigipApmPolicyItem) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("caption", &o.caption)
        .s("color", &o.color)
        .s("type", &o.item_type)
        .v("agents", path_ref_list_strs(&o.agents, "apm policy agent"))
        .s("description", &o.description)
        .done()
}

pub(super) fn project_apm_policy_agent(o: &BigipApmPolicyAgent) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("type", &o.agent_type)
        // `customization-group` names an `apm policy customization-group`,
        // which has no typed model to navigate into — keep the path as text.
        .s("customization-group", &o.customization_group)
        .s("auth", &o.auth)
        .s("server", &o.server)
        .s("max-logon-attempt", &o.max_logon_attempt)
        .s("auth-max-logon-attempt", &o.auth_max_logon_attempt)
        .s("fetch-nested-groups", &o.fetch_nested_groups)
        .s("fetch-primary-groups", &o.fetch_primary_groups)
        .s("password-source", &o.password_source)
        .s("query", &o.query)
        .s("query-attrname", &o.query_attrname)
        .s("query-filter", &o.query_filter)
        .s("show-extended-error", &o.show_extended_error)
        .s("upn", &o.upn)
        .s("username-source", &o.username_source)
        .s(
            "attribute-consuming-service",
            &o.attribute_consuming_service,
        )
        .s(
            "attr-consuming-service-session-var",
            &o.attr_consuming_service_session_var,
        )
        .s("hints", &o.hints)
        .done()
}

pub(super) fn project_apm_customization_source(
    o: &BigipApmPolicyCustomizationSource,
) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("description", &o.description)
        .done()
}

pub(super) fn project_apm_oauth_db_instance(
    o: &BigipApmOauthDbInstance,
) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("db-name", &o.db_name)
        .s("purge-frequency", &o.purge_frequency)
        .s("purge-time", &o.purge_time)
        .s("description", &o.description)
        .done()
}

pub(super) fn project_apm_ssh_security_config(
    o: &BigipApmEphemeralAuthSshSecurityConfig,
) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .v("ciphers", str_list(&o.ciphers))
        .v("hmacs", str_list(&o.hmacs))
        .v("kex-methods", str_list(&o.kex_methods))
        .v("compressions", str_list(&o.compressions))
        .s("description", &o.description)
        .done()
}

pub(super) fn project_apm_default_report(
    o: &BigipApmReportDefaultReport,
) -> IndexMap<String, Value> {
    Fields::new()
        .s("name", &o.name)
        .s("full-path", &o.full_path)
        .s("report-name", &o.report_name)
        .s("user", &o.user)
        .done()
}
