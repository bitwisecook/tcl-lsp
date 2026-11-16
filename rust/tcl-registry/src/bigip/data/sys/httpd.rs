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

//! BIG-IP object specs for `sys httpd`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "sys_httpd",
        table_name: None,
        resolver_name: None,
        module: Some("sys"),
        object_types: &["httpd"],
    },
    header_types: &[("sys", "httpd")],
    properties: &[
        BigipPropertySpec {
            name: "allow",
            value_type: ValueKind::List,
            allow_none: true,
            default: Some("All"),
            list_operators: &["add", "delete", "replace-all-with"],
            block: &[BigipPropertySpec {
                name: "hostname",
                value_type: ValueKind::Unknown,
                in_sections: &["allow"],
                repeated: true,
                ..BigipPropertySpec::DEFAULT
            }],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "hostname",
            value_type: ValueKind::Unknown,
            in_sections: &["allow"],
            repeated: true,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "auth-name",
            value_type: ValueKind::String,
            default: Some("BIG-IP"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "auth-pam-dashboard-timeout",
            value_type: ValueKind::Enum,
            enum_values: &["off", "on"],
            default: Some("off"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "auth-pam-idle-timeout",
            value_type: ValueKind::Integer,
            default: Some("1200 seconds"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "auth-pam-validate-ip",
            value_type: ValueKind::Enum,
            enum_values: &["off", "on"],
            default: Some("on"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "fastcgi-timeout",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "hostname-lookup",
            value_type: ValueKind::Enum,
            enum_values: &["double", "off", "on"],
            default: Some("off"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "include",
            value_type: ValueKind::String,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "log-level",
            value_type: ValueKind::Enum,
            enum_values: &[
                "alert", "crit", "debug", "emerg", "error", "info", "notice", "warn",
            ],
            default: Some("warn"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "redirect-http-to-https",
            value_type: ValueKind::Enum,
            enum_values: &["disabled", "enabled"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("disabled"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "request-body-max-timeout",
            value_type: ValueKind::Integer,
            default: Some("0"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "request-body-min-rate",
            value_type: ValueKind::Integer,
            default: Some("500"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "request-body-timeout",
            value_type: ValueKind::Integer,
            default: Some("60"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "request-header-max-timeout",
            value_type: ValueKind::Integer,
            default: Some("40"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "request-header-min-rate",
            value_type: ValueKind::Integer,
            default: Some("500"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "request-header-timeout",
            value_type: ValueKind::Integer,
            default: Some("20"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ssl-ca-cert-file",
            value_type: ValueKind::String,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ssl-certchainfile",
            value_type: ValueKind::String,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ssl-certfile",
            value_type: ValueKind::String,
            default: Some("/etc/httpd/conf/ssl"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ssl-certkeyfile",
            value_type: ValueKind::String,
            default: Some("/etc/httpd/conf/ssl"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ssl-ciphersuite",
            value_type: ValueKind::String,
            default: Some(
                "\"ECDHE-RSA-AES128-GCM-SHA256:ECDHE-RSA-AES256-GCM-SHA384:ECDHE-RSA-AES128-SHA:ECDHE-RSA-AES256-SHA:ECDHE-RSA-AES128-SHA256:ECDHE-RSA-AES256-SHA384:ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-ECDSA-AES256-GCM-SHA384:ECDHE-ECDSA-AES128-SHA:ECDHE-ECDSA-AES256-SHA:ECDHE-ECDSA-AES128-SHA256:ECDHE-ECDSA-AES256-SHA384:AES128-GCM-SHA256:AES256-GCM-SHA384:AES128-SHA:AES256-SHA:AES128-SHA256:AES256-SHA256\"",
            ),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ssl-include",
            value_type: ValueKind::String,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ssl-ocsp-default-responder",
            value_type: ValueKind::String,
            default: Some("http://localhost"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ssl-ocsp-enable",
            value_type: ValueKind::Enum,
            enum_values: &["off", "on"],
            default: Some("off"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ssl-ocsp-override-responder",
            value_type: ValueKind::Enum,
            enum_values: &["off", "on"],
            default: Some("off"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ssl-ocsp-responder-timeout",
            value_type: ValueKind::Integer,
            default: Some("300 seconds"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ssl-ocsp-response-max-age",
            value_type: ValueKind::Integer,
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ssl-ocsp-response-time-skew",
            value_type: ValueKind::Integer,
            default: Some("300 seconds"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ssl-port",
            value_type: ValueKind::Integer,
            default: Some("443"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ssl-protocol",
            value_type: ValueKind::String,
            default: Some("all -SSLv2 -SSLv3 -TLSv1"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ssl-verify-client",
            value_type: ValueKind::Enum,
            enum_values: &["no", "optional", "optional-no-ca", "require"],
            default: Some("no"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "ssl-verify-depth",
            value_type: ValueKind::Integer,
            default: Some("10"),
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
