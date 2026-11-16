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

//! BIG-IP object specs for `apm aaa endpoint-management-system`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "apm_aaa_endpoint_management_system",
        table_name: None,
        resolver_name: None,
        module: Some("apm"),
        object_types: &["aaa endpoint-management-system"],
    },
    header_types: &[("apm", "aaa endpoint-management-system")],
    properties: &[
        BigipPropertySpec {
            name: "access-key",
            value_type: ValueKind::String,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "app-version",
            value_type: ValueKind::String,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "application-id",
            value_type: ValueKind::String,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "billing-id",
            value_type: ValueKind::String,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "client-id",
            value_type: ValueKind::String,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "client-secret",
            value_type: ValueKind::String,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "dns-resolver",
            value_type: ValueKind::Reference,
            allow_none: true,
            references: &["net_dns_resolver"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "fqdn",
            value_type: ValueKind::String,
            required: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "location-specific",
            value_type: ValueKind::Enum,
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "mdm-token",
            value_type: ValueKind::String,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "password",
            value_type: ValueKind::String,
            required: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "platform",
            value_type: ValueKind::String,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "port",
            value_type: ValueKind::Unknown,
            default: Some("443"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "serverssl-profile",
            value_type: ValueKind::Reference,
            required: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "sync-interval",
            value_type: ValueKind::Integer,
            allow_none: true,
            default: Some("240 minutes"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "tenant-id",
            value_type: ValueKind::String,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "type",
            value_type: ValueKind::Enum,
            required: true,
            enum_values: &["airwatch", "maas360", "ms-intune"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "username",
            value_type: ValueKind::String,
            required: true,
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
