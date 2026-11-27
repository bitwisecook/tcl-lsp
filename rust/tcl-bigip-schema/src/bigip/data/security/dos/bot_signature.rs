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

//! BIG-IP object specs for `security dos bot-signature`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "security_dos_bot_signature",
        table_name: None,
        resolver_name: None,
        module: Some("security"),
        object_types: &["dos bot-signature"],
    },
    header_types: &[("security", "dos bot-signature")],
    properties: &[
        BigipPropertySpec {
            name: "category",
            value_type: ValueKind::Reference,
            references: &[
                "ltm_classification_category",
                "ltm_classification_stats_url_category",
                "ltm_classification_url_category",
                "security_blacklist_publisher_by_category",
                "security_blacklist_publisher_category",
                "security_bot_defense_anomaly_category",
                "security_bot_defense_signature_category",
                "security_dos_bot_signature_category",
                "security_firewall_ipi_category_info",
                "security_ip_intelligence_blacklist_category",
                "security_scrubber_dwbl_scrubber_category_stats",
                "sys_url_db_url_category",
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "domains",
            value_type: ValueKind::List,
            repeated: true,
            allow_none: true,
            list_operators: &["add", "delete", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "risk",
            value_type: ValueKind::Enum,
            enum_values: &["high", "low", "medium"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "rule",
            value_type: ValueKind::String,
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
            name: "signature-references",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "url",
            value_type: ValueKind::Unknown,
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "match-type",
                    value_type: ValueKind::Enum,
                    in_sections: &["url"],
                    enum_values: &["contains"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "search-string",
                    value_type: ValueKind::String,
                    in_sections: &["url"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "match-type",
            value_type: ValueKind::Enum,
            in_sections: &["url"],
            enum_values: &["contains"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "search-string",
            value_type: ValueKind::String,
            in_sections: &["url"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "user-agent",
            value_type: ValueKind::Unknown,
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "match-type",
                    value_type: ValueKind::Enum,
                    in_sections: &["user-agent"],
                    enum_values: &["contains"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "search-string",
                    value_type: ValueKind::String,
                    in_sections: &["user-agent"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "match-type",
            value_type: ValueKind::Enum,
            in_sections: &["user-agent"],
            enum_values: &["contains"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "search-string",
            value_type: ValueKind::String,
            in_sections: &["user-agent"],
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
