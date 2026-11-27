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

//! BIG-IP object specs for `sys file`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_file_apache_ssl_cert",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["file apache-ssl-cert"],
        },
        header_types: &[("sys", "file apache-ssl-cert")],
        properties: &[BigipPropertySpec {
            name: "source-path",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_file_browser_capabilities_db",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["file browser-capabilities-db"],
        },
        header_types: &[("sys", "file browser-capabilities-db")],
        properties: &[BigipPropertySpec {
            name: "source-path",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_file_data_group",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["file data-group"],
        },
        header_types: &[("sys", "file data-group")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "data-group-description",
                value_type: ValueKind::String,
                usage_flags: &["optional"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "data-group-name",
                value_type: ValueKind::Reference,
                usage_flags: &["optional"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "separator",
                value_type: ValueKind::String,
                default: Some(":="),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "source-path",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "type",
                value_type: ValueKind::Integer,
                required: true,
                enum_values: &["ip"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_file_device_capabilities_db",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["file device-capabilities-db"],
        },
        header_types: &[("sys", "file device-capabilities-db")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "source-path",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_file_external_monitor",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["file external-monitor"],
        },
        header_types: &[("sys", "file external-monitor")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "source-path",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_file_ifile",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["file ifile"],
        },
        header_types: &[("sys", "file ifile")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "source-path",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_file_lwtunneltbl",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["file lwtunneltbl"],
        },
        header_types: &[("sys", "file lwtunneltbl")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "source-path",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_file_rewrite_rule",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["file rewrite-rule"],
        },
        header_types: &[("sys", "file rewrite-rule")],
        properties: &[BigipPropertySpec {
            name: "local-path",
            value_type: ValueKind::Unknown,
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_file_ssl_cert",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["file ssl-cert"],
        },
        header_types: &[("sys", "file ssl-cert")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cert-validation-options",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none", "ocsp"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "cert-validators",
                value_type: ValueKind::Reference,
                references: &[
                    "sys_crypto_cert_validator_crl",
                    "sys_crypto_cert_validator_ocsp",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "issuer-cert",
                value_type: ValueKind::Reference,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "source-path",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_file_ssl_crl",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["file ssl-crl"],
        },
        header_types: &[("sys", "file ssl-crl")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "source-path",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_file_ssl_key",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["file ssl-key"],
        },
        header_types: &[("sys", "file ssl-key")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "passphrase",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "source-path",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
];
