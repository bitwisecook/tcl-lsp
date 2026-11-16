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

//! BIG-IP object specs for `pem quota-mgmt`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "pem_quota_mgmt_rating_group",
        table_name: None,
        resolver_name: None,
        module: Some("pem"),
        object_types: &["quota-mgmt rating-group"],
    },
    header_types: &[("pem", "quota-mgmt rating-group")],
    properties: &[
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "default-breach-action",
            value_type: ValueKind::Enum,
            enum_values: &["allow", "redirect", "terminate"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "default-forwarding-endpoint",
            value_type: ValueKind::Reference,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "default-quota",
            value_type: ValueKind::Unknown,
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "interval",
                    value_type: ValueKind::Integer,
                    in_sections: &["default-quota"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "time",
                    value_type: ValueKind::Unknown,
                    in_sections: &["default-quota"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "volume",
                    value_type: ValueKind::Unknown,
                    in_sections: &["default-quota"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "default-quota-holding-time",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "interval",
            value_type: ValueKind::Integer,
            in_sections: &["default-quota"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "time",
            value_type: ValueKind::Unknown,
            in_sections: &["default-quota"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "consumption-time",
                    value_type: ValueKind::Unknown,
                    in_sections: &["default-quota", "time"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "usage-time",
                    value_type: ValueKind::Unknown,
                    in_sections: &["default-quota", "time"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "consumption-time",
            value_type: ValueKind::Unknown,
            in_sections: &["default-quota", "time"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "usage-time",
            value_type: ValueKind::Unknown,
            in_sections: &["default-quota", "time"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "volume",
            value_type: ValueKind::Unknown,
            in_sections: &["default-quota"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "input-octets",
                    value_type: ValueKind::Unknown,
                    in_sections: &["default-quota", "volume"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "output-octets",
                    value_type: ValueKind::Unknown,
                    in_sections: &["default-quota", "volume"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "total-octets",
                    value_type: ValueKind::Unknown,
                    in_sections: &["default-quota", "volume"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "input-octets",
            value_type: ValueKind::Unknown,
            in_sections: &["default-quota", "volume"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "output-octets",
            value_type: ValueKind::Unknown,
            in_sections: &["default-quota", "volume"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "total-octets",
            value_type: ValueKind::Unknown,
            in_sections: &["default-quota", "volume"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "default-threshold",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "default-validity-time",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "initial-quota-request",
            value_type: ValueKind::Unknown,
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "interval",
                    value_type: ValueKind::Integer,
                    in_sections: &["initial-quota-request"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "volume",
                    value_type: ValueKind::Unknown,
                    in_sections: &["initial-quota-request"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "interval",
            value_type: ValueKind::Integer,
            in_sections: &["initial-quota-request"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "volume",
            value_type: ValueKind::Unknown,
            in_sections: &["initial-quota-request"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "input-octets",
                    value_type: ValueKind::Unknown,
                    in_sections: &["initial-quota-request", "volume"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "output-octets",
                    value_type: ValueKind::Unknown,
                    in_sections: &["initial-quota-request", "volume"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "total-octets",
                    value_type: ValueKind::Unknown,
                    in_sections: &["initial-quota-request", "volume"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "input-octets",
            value_type: ValueKind::Unknown,
            in_sections: &["initial-quota-request", "volume"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "output-octets",
            value_type: ValueKind::Unknown,
            in_sections: &["initial-quota-request", "volume"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "total-octets",
            value_type: ValueKind::Unknown,
            in_sections: &["initial-quota-request", "volume"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "rating-group-id",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "request-on-install",
            value_type: ValueKind::Enum,
            enum_values: &["no", "yes"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
