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

//! BIG-IP object specs for `security ip-intelligence`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "security_ip_intelligence_blacklist_category",
            table_name: None,
            resolver_name: None,
            module: Some("security"),
            object_types: &["ip-intelligence blacklist-category"],
        },
        header_types: &[("security", "ip-intelligence blacklist-category")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::Reference,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "bl-match-direction",
                value_type: ValueKind::Enum,
                enum_values: &["destination", "source", "source-and-destination"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "recursive",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "security_ip_intelligence_feed_list",
            table_name: None,
            resolver_name: None,
            module: Some("security"),
            object_types: &["ip-intelligence feed-list"],
        },
        header_types: &[("security", "ip-intelligence feed-list")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::Reference,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "feeds",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "default-blacklist-category",
                        value_type: ValueKind::String,
                        in_sections: &["feeds"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "default-list-type",
                        value_type: ValueKind::Enum,
                        in_sections: &["feeds"],
                        enum_values: &["blacklist", "whitelist"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "poll",
                        value_type: ValueKind::Unknown,
                        in_sections: &["feeds"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "default-blacklist-category",
                value_type: ValueKind::String,
                in_sections: &["feeds"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "default-list-type",
                value_type: ValueKind::Enum,
                in_sections: &["feeds"],
                enum_values: &["blacklist", "whitelist"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "poll",
                value_type: ValueKind::Unknown,
                in_sections: &["feeds"],
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "interval",
                        value_type: ValueKind::Integer,
                        in_sections: &["feeds", "poll"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "password",
                        value_type: ValueKind::String,
                        in_sections: &["feeds", "poll"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "url",
                        value_type: ValueKind::String,
                        in_sections: &["feeds", "poll"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "user",
                        value_type: ValueKind::String,
                        in_sections: &["feeds", "poll"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "interval",
                value_type: ValueKind::Integer,
                in_sections: &["feeds", "poll"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "password",
                value_type: ValueKind::String,
                in_sections: &["feeds", "poll"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "url",
                value_type: ValueKind::String,
                in_sections: &["feeds", "poll"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "user",
                value_type: ValueKind::String,
                in_sections: &["feeds", "poll"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "load",
                value_type: ValueKind::Unknown,
                references: &["gtm_global_settings_load_balancing", "load"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "security_ip_intelligence_global_policy",
            table_name: None,
            resolver_name: None,
            module: Some("security"),
            object_types: &["ip-intelligence global-policy"],
        },
        header_types: &[("security", "ip-intelligence global-policy")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::Reference,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip-intelligence-categories",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip-intelligence-policy",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &[
                    "analytics_asm_policy_changes_report",
                    "apm_policy_access_policy",
                    "apm_policy_agent_aaa_active_directory",
                    "apm_policy_agent_aaa_client_cert",
                    "apm_policy_agent_aaa_crldp",
                    "apm_policy_agent_aaa_http",
                    "apm_policy_agent_aaa_ldap",
                    "apm_policy_agent_aaa_oauth",
                    "apm_policy_agent_aaa_ocsp",
                    "apm_policy_agent_aaa_radius",
                    "apm_policy_agent_aaa_saml",
                    "apm_policy_agent_aaa_securid",
                    "apm_policy_agent_acct_radius",
                    "apm_policy_agent_acct_tacacsplus",
                    "apm_policy_agent_api_authentication",
                    "apm_policy_agent_api_server_selection",
                    "apm_policy_agent_decision_box",
                    "apm_policy_agent_dynamic_acl",
                    "apm_policy_agent_ending_allow",
                    "apm_policy_agent_ending_deny",
                    "apm_policy_agent_ending_redirect",
                    "apm_policy_agent_endpoint_check_machine_cert",
                    "apm_policy_agent_endpoint_check_software",
                    "apm_policy_agent_endpoint_linux_check_file",
                    "apm_policy_agent_endpoint_linux_check_process",
                    "apm_policy_agent_endpoint_mac_check_file",
                    "apm_policy_agent_endpoint_mac_check_process",
                    "apm_policy_agent_endpoint_machine_info",
                    "apm_policy_agent_endpoint_windows_browser_cache_cleaner",
                    "apm_policy_agent_endpoint_windows_check_file",
                    "apm_policy_agent_endpoint_windows_check_process",
                    "apm_policy_agent_endpoint_windows_check_registry",
                    "apm_policy_agent_endpoint_windows_group_policy",
                    "apm_policy_agent_endpoint_windows_info_os",
                    "apm_policy_agent_endpoint_windows_protected_workspace",
                    "apm_policy_agent_external_logon_page",
                    "apm_policy_agent_http_header_modify",
                    "apm_policy_agent_ip_geolocation_lookup",
                    "apm_policy_agent_ip_reputation_lookup",
                    "apm_policy_agent_irule_event",
                    "apm_policy_agent_kerberos",
                    "apm_policy_agent_l7_protocol_lookup",
                    "apm_policy_agent_logging",
                    "apm_policy_agent_logon_page",
                    "apm_policy_agent_message_box",
                    "apm_policy_agent_oam",
                    "apm_policy_agent_oauth_authz",
                    "apm_policy_agent_request_classification",
                    "apm_policy_agent_resource_assign",
                    "apm_policy_agent_response_selection",
                    "apm_policy_agent_route_domain_selection",
                    "apm_policy_agent_server_cert_response_control",
                    "apm_policy_agent_server_cert_status",
                    "apm_policy_agent_session_check",
                    "apm_policy_agent_ssl_check",
                    "apm_policy_agent_tacacsplus",
                    "apm_policy_agent_variable_assign",
                    "apm_policy_customization_group",
                    "apm_policy_customization_languages",
                    "apm_policy_image_file",
                    "apm_policy_policy_item",
                    "apm_policy_windows_group_policy_file",
                    "asm_policy",
                    "asm_predefined_policy",
                    "auth_password_policy",
                    "ltm_classification_url_cat_policy",
                    "ltm_eviction_policy",
                    "ltm_policy",
                    "ltm_policy_strategy",
                    "net_bwc_policy",
                    "net_ipsec_ipsec_policy",
                    "net_rate_shaping_drop_policy",
                    "net_rate_shaping_shaping_policy",
                    "net_service_policy",
                    "net_timer_policy",
                    "pem_global_settings_policy",
                    "pem_policy",
                    "security_firewall_global_fqdn_policy",
                    "security_firewall_policy",
                    "security_firewall_port_misuse_policy",
                    "security_ip_intelligence_global_policy",
                    "security_ip_intelligence_policy",
                    "security_nat_policy",
                    "security_packet_filter_policy",
                    "wam_ad_policy",
                    "wam_policy",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "recursive",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "security_ip_intelligence_policy",
            table_name: None,
            resolver_name: None,
            module: Some("security"),
            object_types: &["ip-intelligence policy"],
        },
        header_types: &[("security", "ip-intelligence policy")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::Reference,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "blacklist-categories",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "action",
                        value_type: ValueKind::Enum,
                        in_sections: &["blacklist-categories"],
                        enum_values: &["accept", "drop", "use-policy-setting"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "app-service",
                        value_type: ValueKind::Unknown,
                        in_sections: &["blacklist-categories"],
                        allow_none: true,
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "description",
                        value_type: ValueKind::Unknown,
                        in_sections: &["blacklist-categories"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "log-blacklist-hit-only",
                        value_type: ValueKind::Enum,
                        in_sections: &["blacklist-categories"],
                        enum_values: &["no", "use-policy-setting", "yes"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "log-blacklist-whitelist-hit",
                        value_type: ValueKind::Enum,
                        in_sections: &["blacklist-categories"],
                        enum_values: &["no", "use-policy-setting", "yes"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "match-direction-override",
                        value_type: ValueKind::Enum,
                        in_sections: &["blacklist-categories"],
                        enum_values: &[
                            "match-destination",
                            "match-source",
                            "match-source-and-destination",
                        ],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "action",
                value_type: ValueKind::Enum,
                in_sections: &["blacklist-categories"],
                enum_values: &["accept", "drop", "use-policy-setting"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::Unknown,
                in_sections: &["blacklist-categories"],
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::Unknown,
                in_sections: &["blacklist-categories"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-blacklist-hit-only",
                value_type: ValueKind::Enum,
                in_sections: &["blacklist-categories"],
                enum_values: &["no", "use-policy-setting", "yes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-blacklist-whitelist-hit",
                value_type: ValueKind::Enum,
                in_sections: &["blacklist-categories"],
                enum_values: &["no", "use-policy-setting", "yes"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "match-direction-override",
                value_type: ValueKind::Enum,
                in_sections: &["blacklist-categories"],
                enum_values: &[
                    "match-destination",
                    "match-source",
                    "match-source-and-destination",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "default-action",
                value_type: ValueKind::Enum,
                enum_values: &["accept", "drop"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "default-log-blacklist-hit-only",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "default-log-blacklist-whitelist-hit",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "feed-lists",
                value_type: ValueKind::List,
                references: &["security_ip_intelligence_feed_list"],
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
];
