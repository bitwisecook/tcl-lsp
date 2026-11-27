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

//! BIG-IP object specs for `security dos dos-signature`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "security_dos_dos_signature",
        table_name: None,
        resolver_name: None,
        module: Some("security"),
        object_types: &["dos dos-signature"],
    },
    header_types: &[("security", "dos dos-signature")],
    properties: &[
        BigipPropertySpec {
            name: "alias",
            value_type: ValueKind::String,
            default: Some("empty string"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::Enum,
            allow_none: true,
            enum_values: &["none"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "approval-state",
            value_type: ValueKind::Enum,
            enum_values: &["manually-approved", "unapproved"],
            default: Some("manually-approved"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "family",
            value_type: ValueKind::Enum,
            enum_values: &["dns", "http", "network", "tls"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "hardware-offload",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("enabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "manual-detection-threshold",
            value_type: ValueKind::Integer,
            default: Some("infinite(4294967295)"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "manual-mitigation-threshold",
            value_type: ValueKind::Integer,
            default: Some("infinite(4294967295)"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "multiplier-mitigation-percentage",
            value_type: ValueKind::Integer,
            default: Some(
                "inherited from the corresponding device level/profile mitigation multiplier value of the same dos family",
            ),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "origin",
            value_type: ValueKind::Enum,
            enum_values: &["dynamic-bdos", "user-defined"],
            default: Some("user-defined"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "parent-context",
            value_type: ValueKind::String,
            default: Some("empty string"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "parent-context-type",
            value_type: ValueKind::Enum,
            enum_values: &["device", "device-netflow", "virtual-server"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "parent-profile",
            value_type: ValueKind::String,
            required: true,
            default: Some("empty string"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "predicates",
            value_type: ValueKind::Unknown,
            required: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "shareability-state",
            value_type: ValueKind::Enum,
            enum_values: &["fully-shareable", "not-shareable"],
            default: Some("fully-shareable"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "state",
            value_type: ValueKind::Enum,
            enum_values: &["detect-only", "disabled", "learn-only", "mitigate"],
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "tags",
            value_type: ValueKind::Unknown,
            default: Some("empty"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "threshold-mode",
            value_type: ValueKind::Enum,
            enum_values: &[
                "fully-automatic",
                "manual",
                "manual-multiplier-mitigation",
                "stress-based-mitigation",
            ],
            default: Some("manual"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "type",
            value_type: ValueKind::Enum,
            enum_values: &["dynamic", "persistent"],
            default: Some("persistent"),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
