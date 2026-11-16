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

//! BIG-IP object specs for `apm resource remote-desktop`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_resource_remote_desktop_citrix",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["resource remote-desktop citrix"],
        },
        header_types: &[("apm", "resource remote-desktop citrix")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auto-logon",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "customization-group",
                value_type: ValueKind::List,
                default: Some("none"),
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "caption",
                        value_type: ValueKind::String,
                        in_sections: &["customization-group"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "detailed-description",
                        value_type: ValueKind::String,
                        in_sections: &["customization-group"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "caption",
                value_type: ValueKind::String,
                in_sections: &["customization-group"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "detailed-description",
                value_type: ValueKind::String,
                in_sections: &["customization-group"],
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
                name: "domain-source",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none", "session.logon.last.domain"],
                default: Some("session"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "enable-serverside-ssl",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "host",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
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
                name: "password-source",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none", "session.logon.last.password"],
                default: Some("session"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pool",
                value_type: ValueKind::Reference,
                references: &[
                    "analytics_lsn_pool_report",
                    "analytics_lsn_pool_scheduled_report",
                    "analytics_pool_traffic_report",
                    "analytics_pool_traffic_scheduled_report",
                    "gtm_pool_a",
                    "gtm_pool_aaaa",
                    "gtm_pool_cname",
                    "gtm_pool_https",
                    "gtm_pool_mx",
                    "gtm_pool_naptr",
                    "gtm_pool_srv",
                    "gtm_pool_svcb",
                    "gtm_prober_pool",
                    "ltm_lsn_pool",
                    "ltm_pool",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("80"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "username-source",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none"],
                default: Some("session"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_resource_remote_desktop_citrix_client_bundle",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["resource remote-desktop citrix-client-bundle"],
        },
        header_types: &[("apm", "resource remote-desktop citrix-client-bundle")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "download-url",
                value_type: ValueKind::Unknown,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "packages",
                value_type: ValueKind::String,
                allow_none: true,
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sb-windows-package",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "windows-download-url",
                value_type: ValueKind::Unknown,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "windows-min-version",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "windows-package",
                value_type: ValueKind::String,
                allow_none: true,
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_resource_remote_desktop_citrix_client_package_file",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["resource remote-desktop citrix-client-package-file"],
        },
        header_types: &[("apm", "resource remote-desktop citrix-client-package-file")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
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
                name: "original-file-name",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "source-path",
                value_type: ValueKind::String,
                required: true,
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_resource_remote_desktop_quest",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["resource remote-desktop quest"],
        },
        header_types: &[("apm", "resource remote-desktop quest")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auto-logon",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "customization-group",
                value_type: ValueKind::List,
                default: Some("none"),
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "caption",
                        value_type: ValueKind::String,
                        in_sections: &["customization-group"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "detailed-description",
                        value_type: ValueKind::String,
                        in_sections: &["customization-group"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "caption",
                value_type: ValueKind::String,
                in_sections: &["customization-group"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "detailed-description",
                value_type: ValueKind::String,
                in_sections: &["customization-group"],
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
                name: "domain-source",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none", "session.logon.last.domain"],
                default: Some("session"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "enable-serverside-ssl",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "host",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
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
                name: "password-source",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none", "session.logon.last.password"],
                default: Some("session"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pool",
                value_type: ValueKind::Reference,
                references: &[
                    "analytics_lsn_pool_report",
                    "analytics_lsn_pool_scheduled_report",
                    "analytics_pool_traffic_report",
                    "analytics_pool_traffic_scheduled_report",
                    "gtm_pool_a",
                    "gtm_pool_aaaa",
                    "gtm_pool_cname",
                    "gtm_pool_https",
                    "gtm_pool_mx",
                    "gtm_pool_naptr",
                    "gtm_pool_srv",
                    "gtm_pool_svcb",
                    "gtm_prober_pool",
                    "ltm_lsn_pool",
                    "ltm_pool",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("8080"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "username-source",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none"],
                default: Some("session"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_resource_remote_desktop_rdp",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["resource remote-desktop rdp"],
        },
        header_types: &[("apm", "resource remote-desktop rdp")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auto-logon",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "customization-group",
                value_type: ValueKind::List,
                default: Some("none"),
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "app-service",
                        value_type: ValueKind::String,
                        in_sections: &["customization-group"],
                        allow_none: true,
                        default: Some("none"),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "caption",
                        value_type: ValueKind::String,
                        in_sections: &["customization-group"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "detailed-description",
                        value_type: ValueKind::String,
                        in_sections: &["customization-group"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                in_sections: &["customization-group"],
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "caption",
                value_type: ValueKind::String,
                in_sections: &["customization-group"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "detailed-description",
                value_type: ValueKind::String,
                in_sections: &["customization-group"],
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
                name: "domain-source",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none", "session.logon.last.domain"],
                default: Some("session"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "host",
                value_type: ValueKind::Unknown,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip",
                value_type: ValueKind::String,
                required: true,
                shape_kind: Some(ValueKind::IpAddress),
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
                name: "password-source",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none", "session.logon.last.password"],
                default: Some("session"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port",
                value_type: ValueKind::Integer,
                allow_none: true,
                default: Some("0 (zero)"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "username-source",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none"],
                default: Some("session"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "apm_resource_remote_desktop_vmware_view",
            table_name: None,
            resolver_name: None,
            module: Some("apm"),
            object_types: &["resource remote-desktop vmware-view"],
        },
        header_types: &[("apm", "resource remote-desktop vmware-view")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auto-logon",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "customization-group",
                value_type: ValueKind::List,
                default: Some("none"),
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "caption",
                        value_type: ValueKind::String,
                        in_sections: &["customization-group"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "detailed-description",
                        value_type: ValueKind::String,
                        in_sections: &["customization-group"],
                        allow_none: true,
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "caption",
                value_type: ValueKind::String,
                in_sections: &["customization-group"],
                allow_none: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "detailed-description",
                value_type: ValueKind::String,
                in_sections: &["customization-group"],
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
                name: "domain-source",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none", "session.logon.last.domain"],
                default: Some("session"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "enable-serverside-ssl",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                default: Some("disabled"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "host",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
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
                name: "password-source",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none", "session.logon.last.password"],
                default: Some("session"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pool",
                value_type: ValueKind::Reference,
                references: &[
                    "analytics_lsn_pool_report",
                    "analytics_lsn_pool_scheduled_report",
                    "analytics_pool_traffic_report",
                    "analytics_pool_traffic_scheduled_report",
                    "gtm_pool_a",
                    "gtm_pool_aaaa",
                    "gtm_pool_cname",
                    "gtm_pool_https",
                    "gtm_pool_mx",
                    "gtm_pool_naptr",
                    "gtm_pool_srv",
                    "gtm_pool_svcb",
                    "gtm_prober_pool",
                    "ltm_lsn_pool",
                    "ltm_pool",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("80"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "username-source",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none"],
                default: Some("session"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
];
