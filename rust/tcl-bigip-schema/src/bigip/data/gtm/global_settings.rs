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

//! BIG-IP object specs for `gtm global-settings`. Hand-maintained: edit them directly.

// Large tmsh bounds appear as bare f64 literals.
#![allow(clippy::unreadable_literal)]
use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_global_settings_general",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["global-settings general"],
        },
        header_types: &[("gtm", "global-settings general")],
        properties: &[
            BigipPropertySpec {
                name: "allow-nxdomain-override",
                value_type: ValueKind::Enum,
                enum_values: &["disable", "enable"],
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auto-discovery",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auto-discovery-interval",
                value_type: ValueKind::Integer,
                default: Some("30"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "automatic-configuration-save-timeout",
                value_type: ValueKind::Integer,
                default: Some("15 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cache-ldns-servers",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("yes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "domain-name-check",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["allow-underscore", "none"],
                default: Some("allow-underscore"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "drain-persistent-requests",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("yes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "forward-status",
                value_type: ValueKind::Enum,
                enum_values: &["disable", "enable"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "gtm-sets-recursion",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "heartbeat-interval",
                value_type: ValueKind::Integer,
                default: Some("10"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ignore-ltm-rate-limit-modes",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "iquery-cipher-list",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "iquery-crl-validation-depth",
                value_type: ValueKind::Enum,
                required: true,
                enum_values: &["device", "full"],
                default: Some("full"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "iquery-minimum-tls-version",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "iquery-reverify-on-crl-becoming-active",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("yes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "iquery-reverify-on-crl-expiring",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("yes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "iquery-reverify-on-crl-file-update",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("yes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "iquery-use-expired-crls",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("yes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "iquery-use-not-yet-active-crls",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("yes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "iquery-use-revoked-certs",
                value_type: ValueKind::Enum,
                enum_values: &["always", "existing", "never"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "monitor-disabled-objects",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "nethsm-timeout",
                value_type: ValueKind::Integer,
                default: Some("20 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "nsec3-types-bitmap-strict",
                value_type: ValueKind::Enum,
                enum_values: &["disable", "enable"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "peer-leader",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "send-wildcard-rrs",
                value_type: ValueKind::Enum,
                enum_values: &["disable", "enable"],
                default: Some("disable"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "static-persist-cidr-ipv4",
                value_type: ValueKind::Integer,
                default: Some("32"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "static-persist-cidr-ipv6",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "synchronization",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "synchronization-group-name",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "synchronization-time-tolerance",
                value_type: ValueKind::Integer,
                default: Some("10 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "synchronization-timeout",
                value_type: ValueKind::Integer,
                default: Some("180"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "synchronize-zone-files",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "synchronize-zone-files-timeout",
                value_type: ValueKind::Integer,
                default: Some("300"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "topology-allow-zero-scores",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("yes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "virtuals-depend-on-server-state",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("yes"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "wideip-zone-nameserver",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_global_settings_load_balancing",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["global-settings load-balancing"],
        },
        header_types: &[("gtm", "global-settings load-balancing")],
        properties: &[
            BigipPropertySpec {
                name: "failure-rcode",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "formerr", "noerror", "notimpl", "nxdomain", "refused", "servfail",
                ],
                default: Some("noerror"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure-rcode-response",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "failure-rcode-ttl",
                value_type: ValueKind::Integer,
                default: Some("0, meaning no SOA is included (i"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ignore-path-ttl",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "respect-fallback-dependency",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "topology-longest-match",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "topology-prefer-edns0-client-subnet",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "verify-vs-availability",
                value_type: ValueKind::Enum,
                enum_values: &["no", "yes"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("no"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_global_settings_metrics",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["global-settings metrics"],
        },
        header_types: &[("gtm", "global-settings metrics")],
        properties: &[
            BigipPropertySpec {
                name: "default-probe-limit",
                value_type: ValueKind::Integer,
                default: Some("12"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hops-packet-length",
                value_type: ValueKind::Integer,
                default: Some("64"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hops-sample-count",
                value_type: ValueKind::Integer,
                default: Some("3"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hops-timeout",
                value_type: ValueKind::Integer,
                default: Some("3"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hops-ttl",
                value_type: ValueKind::Integer,
                default: Some("604800"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "inactive-ldns-ttl",
                value_type: ValueKind::Integer,
                default: Some("2419200 (28 days)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "inactive-paths-ttl",
                value_type: ValueKind::Integer,
                default: Some("604800 (7 days)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ldns-update-interval",
                value_type: ValueKind::Integer,
                default: Some("20 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "max-synchronous-monitor-requests",
                value_type: ValueKind::Integer,
                default: Some("20"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "metrics-caching",
                value_type: ValueKind::Integer,
                min_value: Some(0f64),
                max_value: Some(604800f64),
                default: Some("3600"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "metrics-collection-protocols",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "path-ttl",
                value_type: ValueKind::Integer,
                default: Some("2400"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "paths-retry",
                value_type: ValueKind::Integer,
                default: Some("120"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "gtm_global_settings_metrics_exclusions",
            table_name: None,
            resolver_name: None,
            module: Some("gtm"),
            object_types: &["global-settings metrics-exclusions"],
        },
        header_types: &[("gtm", "global-settings metrics-exclusions")],
        properties: &[BigipPropertySpec {
            name: "addresses",
            value_type: ValueKind::List,
            allow_none: true,
            list_operators: &["add", "delete", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        }],
    },
];
