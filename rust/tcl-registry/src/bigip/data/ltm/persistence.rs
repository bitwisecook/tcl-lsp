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

//! BIG-IP object specs for `ltm persistence`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_persistence_cookie",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["persistence cookie"],
        },
        header_types: &[("ltm", "persistence cookie")],
        properties: &[
            BigipPropertySpec {
                name: "always-send",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cookie-encryption",
                value_type: ValueKind::Enum,
                required: true,
                enum_values: &["disabled", "preferred", "required"],
                default: Some("required"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cookie-encryption-passphrase",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cookie-name",
                value_type: ValueKind::Reference,
                required: true,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                references: &["ltm_persistence_cookie"],
                default: Some("cookie, the system default cookie persistence profile"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "encrypt-cookie-poolname",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "expiration",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hash-length",
                value_type: ValueKind::Integer,
                default: Some("0 (zero) bytes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hash-offset",
                value_type: ValueKind::Integer,
                default: Some("0(zero) bytes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "httponly",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "match-across-pools",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "match-across-services",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "match-across-virtuals",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "method",
                value_type: ValueKind::Enum,
                enum_values: &["hash", "insert", "passive", "rewrite"],
                default: Some("insert"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mirror",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "override-connection-limit",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "secure",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("enabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "timeout",
                value_type: ValueKind::Integer,
                default: Some("180 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_persistence_dest_addr",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["persistence dest-addr"],
        },
        header_types: &[("ltm", "persistence dest-addr")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                references: &["ltm_persistence_dest_addr"],
                default: Some(
                    "dest_addr, the system default destination address affinity persistence profile",
                ),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hash-algorithm",
                value_type: ValueKind::Enum,
                enum_values: &["carp", "default"],
                default: Some("default (no hash persistence)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mask",
                value_type: ValueKind::String,
                allow_none: true,
                shape_kind: Some(ValueKind::IpAddress),
                default: Some("::"),
                address_grammar: Some(crate::bigip::AddressGrammar::Mask),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "match-across-pools",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "match-across-services",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "match-across-virtuals",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mirror",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "override-connection-limit",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "timeout",
                value_type: ValueKind::Integer,
                default: Some("180 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_persistence_global_settings",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["persistence global-settings"],
        },
        header_types: &[("ltm", "persistence global-settings")],
        properties: &[
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dest-addr-limit-mode",
                value_type: ValueKind::Enum,
                enum_values: &["maxcount", "timeout"],
                default: Some("timeout"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dest-addr-max",
                value_type: ValueKind::Integer,
                default: Some("2048 entries"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "proxy-group",
                value_type: ValueKind::String,
                default: Some("aol"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_persistence_hash",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["persistence hash"],
        },
        header_types: &[("ltm", "persistence hash")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                references: &["ltm_persistence_hash"],
                default: Some("hash, the system default cookie persistence profile"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hash-algorithm",
                value_type: ValueKind::Enum,
                enum_values: &["carp", "default"],
                default: Some("default"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hash-buffer-limit",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hash-end-pattern",
                value_type: ValueKind::String,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hash-length",
                value_type: ValueKind::Integer,
                default: Some("0 (zero) bytes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hash-offset",
                value_type: ValueKind::Integer,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hash-start-pattern",
                value_type: ValueKind::String,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "match-across-pools",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "match-across-services",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "match-across-virtuals",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mirror",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "override-connection-limit",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rule",
                value_type: ValueKind::Reference,
                references: &[
                    "gtm_rule",
                    "ltm_cipher_rule",
                    "ltm_global_settings_rule",
                    "ltm_rule",
                    "ltm_rule_profiler",
                    "security_firewall_matching_rule",
                    "security_firewall_on_demand_rule_deploy",
                    "security_firewall_rule_list",
                    "security_firewall_rule_stat",
                    "security_packet_filter_rule_stat",
                    "sys_file_rewrite_rule",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "timeout",
                value_type: ValueKind::Integer,
                default: Some("180 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_persistence_host",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["persistence host"],
        },
        header_types: &[("ltm", "persistence host")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                references: &["ltm_persistence_host"],
                default: Some("host, the system default host persistence profile"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "match-across-pools",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "match-across-services",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "match-across-virtuals",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mirror",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "override-connection-limit",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "timeout",
                value_type: ValueKind::Integer,
                default: Some("180 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_persistence_msrdp",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["persistence msrdp"],
        },
        header_types: &[("ltm", "persistence msrdp")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                references: &["ltm_persistence_msrdp"],
                default: Some("msrdp, the system default cookie persistence profile"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "has-session-dir",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("yes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "match-across-pools",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "match-across-services",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "match-across-virtuals",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mirror",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "override-connection-limit",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "timeout",
                value_type: ValueKind::Integer,
                default: Some("300 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_persistence_persist_records",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["persistence persist-records"],
        },
        header_types: &[("ltm", "persistence persist-records")],
        properties: &[],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_persistence_sip",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["persistence sip"],
        },
        header_types: &[("ltm", "persistence sip")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                references: &["ltm_persistence_sip"],
                default: Some("sip_info, the system default cookie persistence profile"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "match-across-pools",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "match-across-services",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "match-across-virtuals",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mirror",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "override-connection-limit",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sip-info",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["Call-ID", "From", "SIP-ETag", "Subject", "To", "none"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "timeout",
                value_type: ValueKind::Integer,
                default: Some("180 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_persistence_source_addr",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["persistence source-addr"],
        },
        header_types: &[("ltm", "persistence source-addr")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                references: &["ltm_persistence_source_addr"],
                default: Some("source_addr, the system default cookie persistence profile"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hash-algorithm",
                value_type: ValueKind::Enum,
                enum_values: &["carp", "default"],
                default: Some("default (no hash persistence)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "map-proxies",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "map-proxy-address",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                address_grammar: Some(crate::bigip::AddressGrammar::Address),
                default: Some(
                    "any which results in the default behavior of using one of the IP addresses from the proxy data-group/class",
                ),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "map-proxy-class",
                value_type: ValueKind::Reference,
                default: Some(
                    "none which will result in map_proxies using the class defined by the DB variable Persist",
                ),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mask",
                value_type: ValueKind::String,
                allow_none: true,
                shape_kind: Some(ValueKind::IpAddress),
                default: Some("::"),
                address_grammar: Some(crate::bigip::AddressGrammar::Mask),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "match-across-pools",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "match-across-services",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "match-across-virtuals",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mirror",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "override-connection-limit",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "timeout",
                value_type: ValueKind::Integer,
                default: Some("180 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_persistence_ssl",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["persistence ssl"],
        },
        header_types: &[("ltm", "persistence ssl")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                references: &["ltm_persistence_ssl"],
                default: Some("ssl, the system default cookie persistence profile"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "match-across-pools",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "match-across-services",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "match-across-virtuals",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mirror",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "override-connection-limit",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "timeout",
                value_type: ValueKind::Integer,
                default: Some("300 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "ltm_persistence_universal",
            table_name: None,
            resolver_name: None,
            module: Some("ltm"),
            object_types: &["persistence universal"],
        },
        header_types: &[("ltm", "persistence universal")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "defaults-from",
                value_type: ValueKind::Reference,
                references: &["ltm_persistence_universal"],
                default: Some("universal, the system default cookie persistence profile"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "match-across-pools",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "match-across-services",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "match-across-virtuals",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "method",
                value_type: ValueKind::Enum,
                enum_values: &["hash", "insert", "passive", "rewrite"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mirror",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "override-connection-limit",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rule",
                value_type: ValueKind::Reference,
                allow_none: true,
                references: &[
                    "gtm_rule",
                    "ltm_cipher_rule",
                    "ltm_global_settings_rule",
                    "ltm_rule",
                    "ltm_rule_profiler",
                    "security_firewall_matching_rule",
                    "security_firewall_on_demand_rule_deploy",
                    "security_firewall_rule_list",
                    "security_firewall_rule_stat",
                    "security_packet_filter_rule_stat",
                    "sys_file_rewrite_rule",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "timeout",
                value_type: ValueKind::Integer,
                default: Some("180 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
];
