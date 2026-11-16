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

//! BIG-IP object specs for `ltm monitor smb`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "ltm_monitor_smb",
        table_name: None,
        resolver_name: None,
        module: Some("ltm"),
        object_types: &["monitor smb"],
    },
    header_types: &[("ltm", "monitor smb")],
    properties: &[
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "debug",
            value_type: ValueKind::Enum,
            enum_values: &["no", "yes"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("no"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "defaults-from",
            value_type: ValueKind::Reference,
            references: &["ltm_monitor_smb"],
            default: Some("smb"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "destination",
            value_type: ValueKind::String,
            shape_kind: Some(ValueKind::Endpoint),
            default: Some("*:*"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "get",
            value_type: ValueKind::Unknown,
            required: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "interval",
            value_type: ValueKind::Integer,
            default: Some("10 seconds"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "manual-resume",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "password",
            value_type: ValueKind::Unknown,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "server",
            value_type: ValueKind::Reference,
            allow_none: true,
            references: &[
                "api_protection_server",
                "apm_aaa_oauth_server",
                "apm_oauth_oauth_resource_server",
                "apm_policy_agent_api_server_selection",
                "apm_policy_agent_server_cert_response_control",
                "apm_policy_agent_server_cert_status",
                "auth_radius_server",
                "gtm_listener_doh_server",
                "gtm_monitor_real_server",
                "gtm_server",
                "ltm_auth_crldp_server",
                "ltm_auth_radius_server",
                "ltm_monitor_real_server",
                "ltm_profile_doh_server",
                "ltm_profile_server_ldap",
                "ltm_profile_server_ssl",
                "sys_crypto_server",
                "sys_smtp_server",
                "wom_server_discovery",
            ],
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "service",
            value_type: ValueKind::Reference,
            allow_none: true,
            references: &[
                "analytics_ssl_orchestrator_service_virtual_report",
                "analytics_ssl_orchestrator_service_virtual_scheduled_report",
                "apm_aaa_f5_service_connector",
                "apm_saml_artifact_resolution_service",
                "apm_saml_attribute_consuming_service",
                "net_service_policy",
                "pem_service_chain_endpoint",
                "security_bot_defense_micro_service",
                "security_protocol_inspection_service",
                "sys_application_service",
                "sys_service",
            ],
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "time-until-up",
            value_type: ValueKind::Integer,
            default: Some("0 (zero)"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "timeout",
            value_type: ValueKind::Integer,
            default: Some("31 seconds"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "up-interval",
            value_type: ValueKind::Integer,
            default: Some(
                "0 (zero), which specifies that the system uses the value of the interval option whether the resource is up or down",
            ),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "username",
            value_type: ValueKind::Reference,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
