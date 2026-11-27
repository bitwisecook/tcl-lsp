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

//! BIG-IP object specs for `net routing bfd`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "net_routing_bfd",
        table_name: None,
        resolver_name: None,
        module: Some("net"),
        object_types: &["routing bfd"],
    },
    header_types: &[("net", "routing bfd")],
    properties: &[
        BigipPropertySpec {
            name: "gtsm",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "gtsm-ttl",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "multihop-peer",
            value_type: ValueKind::Reference,
            shape_kind: Some(ValueKind::List),
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "interval",
                    value_type: ValueKind::Boolean,
                    in_sections: &["multihop-peer"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "minrx",
                    value_type: ValueKind::Boolean,
                    in_sections: &["multihop-peer"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "multiplier",
                    value_type: ValueKind::Boolean,
                    in_sections: &["multihop-peer"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "interval",
            value_type: ValueKind::Boolean,
            in_sections: &["multihop-peer"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "minrx",
            value_type: ValueKind::Boolean,
            in_sections: &["multihop-peer"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "multiplier",
            value_type: ValueKind::Boolean,
            in_sections: &["multihop-peer"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "notification",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "route-domain",
            value_type: ValueKind::Reference,
            allow_none: true,
            references: &["net_route_domain"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "slow-timer",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "vlan",
            value_type: ValueKind::Reference,
            references: &["net_vlan"],
            shape_kind: Some(ValueKind::List),
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "enabled",
                    value_type: ValueKind::Enum,
                    in_sections: &["vlan"],
                    enum_values: &["false", "true"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "interval",
                    value_type: ValueKind::Boolean,
                    in_sections: &["vlan"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "minrx",
                    value_type: ValueKind::Boolean,
                    in_sections: &["vlan"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "multiplier",
                    value_type: ValueKind::Boolean,
                    in_sections: &["vlan"],
                    allow_none: true,
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "enabled",
            value_type: ValueKind::Enum,
            in_sections: &["vlan"],
            enum_values: &["false", "true"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "interval",
            value_type: ValueKind::Boolean,
            in_sections: &["vlan"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "minrx",
            value_type: ValueKind::Boolean,
            in_sections: &["vlan"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "multiplier",
            value_type: ValueKind::Boolean,
            in_sections: &["vlan"],
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
