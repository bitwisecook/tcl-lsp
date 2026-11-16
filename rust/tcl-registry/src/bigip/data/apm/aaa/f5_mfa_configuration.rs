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

//! BIG-IP object specs for `apm aaa f5-mfa-configuration`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "apm_aaa_f5_mfa_configuration",
        table_name: None,
        resolver_name: None,
        module: Some("apm"),
        object_types: &["aaa f5-mfa-configuration"],
    },
    header_types: &[("apm", "aaa f5-mfa-configuration")],
    properties: &[
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "f5-service-connector",
            value_type: ValueKind::Reference,
            required: true,
            references: &["apm_aaa_f5_service_connector"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "max-mobile-devices-per-user",
            value_type: ValueKind::Integer,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "permitted-devices-types",
            value_type: ValueKind::List,
            required: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "registration-sms-template",
            value_type: ValueKind::String,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "require-biometric",
            value_type: ValueKind::Enum,
            allow_none: true,
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
