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

//! BIG-IP object specs for `apm profile exchange`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "apm_profile_exchange",
        table_name: None,
        resolver_name: None,
        module: Some("apm"),
        object_types: &["profile exchange"],
    },
    header_types: &[("apm", "profile exchange")],
    properties: &[
        BigipPropertySpec {
            name: "active-sync-auth-type",
            value_type: ValueKind::Enum,
            enum_values: &["basic", "basic-ntlm", "ntlm"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "active-sync-sso-config",
            value_type: ValueKind::String,
            allow_none: true,
            usage_flags: &["optional"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "active-sync-url",
            value_type: ValueKind::String,
            required: true,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "auto-discover-auth-type",
            value_type: ValueKind::Enum,
            enum_values: &["basic", "basic-ntlm", "ntlm"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "auto-discover-sso-config",
            value_type: ValueKind::String,
            allow_none: true,
            usage_flags: &["optional"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "auto-discover-url",
            value_type: ValueKind::String,
            required: true,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ntlm-auth-name",
            value_type: ValueKind::String,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "offline-address-book-auth-type",
            value_type: ValueKind::Enum,
            enum_values: &["basic", "basic-ntlm", "ntlm"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "offline-address-book-sso-config",
            value_type: ValueKind::String,
            allow_none: true,
            usage_flags: &["optional"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "offline-address-book-url",
            value_type: ValueKind::String,
            required: true,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "rpc-over-http-auth-type",
            value_type: ValueKind::Enum,
            enum_values: &["basic", "basic-ntlm", "ntlm"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "rpc-over-http-sso-config",
            value_type: ValueKind::String,
            allow_none: true,
            usage_flags: &["optional"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "rpc-over-http-url",
            value_type: ValueKind::String,
            required: true,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "user-agent-pattern-for-utf8",
            value_type: ValueKind::String,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "web-service-auth-type",
            value_type: ValueKind::Enum,
            enum_values: &["basic", "basic-ntlm", "ntlm"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "web-service-sso-config",
            value_type: ValueKind::String,
            allow_none: true,
            usage_flags: &["optional"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "web-service-url",
            value_type: ValueKind::String,
            allow_none: true,
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
