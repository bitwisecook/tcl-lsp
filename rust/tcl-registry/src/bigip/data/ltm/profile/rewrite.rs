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

//! BIG-IP object specs for `ltm profile rewrite`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[BigipObjectSpec {
    kind_spec: BigipObjectKindSpec {
        kind: "ltm_profile_rewrite",
        table_name: None,
        resolver_name: None,
        module: Some("ltm"),
        object_types: &["profile rewrite"],
    },
    header_types: &[("ltm", "profile rewrite")],
    properties: &[
        BigipPropertySpec {
            name: "app-service",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "bypass-list",
            value_type: ValueKind::List,
            allow_none: true,
            default: Some("none"),
            list_operators: &["add", "delete", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "client-caching-type",
            value_type: ValueKind::Enum,
            enum_values: &["cache-all", "cache-css-js", "cache-img-css-js", "no-cache"],
            default: Some("cache-css-js"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "defaults-from",
            value_type: ValueKind::Reference,
            allow_none: true,
            references: &["ltm_profile_rewrite"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "java-ca-file",
            value_type: ValueKind::Unknown,
            allow_none: true,
            default: Some("ca-bundle"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "java-crl",
            value_type: ValueKind::Unknown,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "java-sign-key",
            value_type: ValueKind::Unknown,
            allow_none: true,
            default: Some("default"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "java-sign-key-passphrase",
            value_type: ValueKind::String,
            allow_none: true,
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "java-signer",
            value_type: ValueKind::Unknown,
            allow_none: true,
            default: Some("default"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "location-specific",
            value_type: ValueKind::Enum,
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("none"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "rewrite-list",
            value_type: ValueKind::List,
            allow_none: true,
            default: Some("none"),
            list_operators: &["add", "delete", "replace-all-with"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "rewrite-mode",
            value_type: ValueKind::Enum,
            enum_values: &["uri-translation"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "set-cookie-rules",
            value_type: ValueKind::List,
            allow_none: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "client",
                    value_type: ValueKind::Unknown,
                    in_sections: &["set-cookie-rules"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "server",
                    value_type: ValueKind::Unknown,
                    in_sections: &["set-cookie-rules"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "client",
            value_type: ValueKind::Unknown,
            in_sections: &["set-cookie-rules"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "domain",
                    value_type: ValueKind::String,
                    in_sections: &["set-cookie-rules", "client"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "path",
                    value_type: ValueKind::String,
                    in_sections: &["set-cookie-rules", "client"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "domain",
            value_type: ValueKind::String,
            in_sections: &["set-cookie-rules", "client"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "path",
            value_type: ValueKind::String,
            in_sections: &["set-cookie-rules", "client"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "server",
            value_type: ValueKind::Unknown,
            in_sections: &["set-cookie-rules"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "domain",
                    value_type: ValueKind::String,
                    in_sections: &["set-cookie-rules", "server"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "path",
                    value_type: ValueKind::String,
                    in_sections: &["set-cookie-rules", "server"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "domain",
            value_type: ValueKind::String,
            in_sections: &["set-cookie-rules", "server"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "path",
            value_type: ValueKind::String,
            in_sections: &["set-cookie-rules", "server"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "split-tunneling",
            value_type: ValueKind::Enum,
            enum_values: &["false", "true"],
            shape_kind: Some(ValueKind::Boolean),
            default: Some("false"),
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "uri-rules",
            value_type: ValueKind::List,
            allow_none: true,
            list_operators: &["add", "delete", "modify", "replace-all-with"],
            block: &[
                BigipPropertySpec {
                    name: "client",
                    value_type: ValueKind::Unknown,
                    in_sections: &["uri-rules"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "server",
                    value_type: ValueKind::Unknown,
                    in_sections: &["uri-rules"],
                    shape_kind: Some(ValueKind::Object),
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "client",
            value_type: ValueKind::Unknown,
            in_sections: &["uri-rules"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "host",
                    value_type: ValueKind::String,
                    in_sections: &["uri-rules", "client"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "path",
                    value_type: ValueKind::String,
                    in_sections: &["uri-rules", "client"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "port",
                    value_type: ValueKind::String,
                    in_sections: &["uri-rules", "client"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "scheme",
                    value_type: ValueKind::String,
                    in_sections: &["uri-rules", "client"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "host",
            value_type: ValueKind::String,
            in_sections: &["uri-rules", "client"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "path",
            value_type: ValueKind::String,
            in_sections: &["uri-rules", "client"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "port",
            value_type: ValueKind::String,
            in_sections: &["uri-rules", "client"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "scheme",
            value_type: ValueKind::String,
            in_sections: &["uri-rules", "client"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "server",
            value_type: ValueKind::Unknown,
            in_sections: &["uri-rules"],
            shape_kind: Some(ValueKind::Object),
            block: &[
                BigipPropertySpec {
                    name: "host",
                    value_type: ValueKind::String,
                    in_sections: &["uri-rules", "server"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "path",
                    value_type: ValueKind::String,
                    in_sections: &["uri-rules", "server"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "port",
                    value_type: ValueKind::String,
                    in_sections: &["uri-rules", "server"],
                    ..BigipPropertySpec::DEFAULT
                },
                BigipPropertySpec {
                    name: "scheme",
                    value_type: ValueKind::String,
                    in_sections: &["uri-rules", "server"],
                    ..BigipPropertySpec::DEFAULT
                },
            ],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "host",
            value_type: ValueKind::String,
            in_sections: &["uri-rules", "server"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "path",
            value_type: ValueKind::String,
            in_sections: &["uri-rules", "server"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "port",
            value_type: ValueKind::String,
            in_sections: &["uri-rules", "server"],
            ..BigipPropertySpec::DEFAULT
        },
        BigipPropertySpec {
            name: "scheme",
            value_type: ValueKind::String,
            in_sections: &["uri-rules", "server"],
            ..BigipPropertySpec::DEFAULT
        },
    ],
}];
