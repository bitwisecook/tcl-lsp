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

//! BIG-IP object specs for `sys log-config`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_log_config_destination_alertd",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["log-config destination alertd"],
        },
        header_types: &[("sys", "log-config destination alertd")],
        properties: &[BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_log_config_destination_arcsight",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["log-config destination arcsight"],
        },
        header_types: &[("sys", "log-config destination arcsight")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "forward-to",
                value_type: ValueKind::String,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_log_config_destination_ipfix",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["log-config destination ipfix"],
        },
        header_types: &[("sys", "log-config destination ipfix")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pool-name",
                value_type: ValueKind::String,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "protocol-version",
                value_type: ValueKind::Enum,
                enum_values: &["ipfix", "netflow-9"],
                default: Some("ipfix"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "serverssl-profile",
                value_type: ValueKind::Reference,
                default: Some("not to use a server-side SSL profile"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "template-delete-delay",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "template-retransmit-interval",
                value_type: ValueKind::Integer,
                default: Some("30 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "transport-profile",
                value_type: ValueKind::Reference,
                default: Some("the default udp profile"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_log_config_destination_local_database",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["log-config destination local-database"],
        },
        header_types: &[("sys", "log-config destination local-database")],
        properties: &[BigipPropertySpec {
            name: "description",
            value_type: ValueKind::String,
            ..BigipPropertySpec::DEFAULT
        }],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_log_config_destination_local_syslog",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["log-config destination local-syslog"],
        },
        header_types: &[("sys", "log-config destination local-syslog")],
        properties: &[
            BigipPropertySpec {
                name: "default-facility",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "local0", "local1", "local2", "local3", "local4", "local5", "local6", "local7",
                ],
                default: Some("local0"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "default-severity",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "alert", "crit", "debug", "emerg", "err", "info", "notice", "warn",
                ],
                default: Some("info"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_log_config_destination_management_port",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["log-config destination management-port"],
        },
        header_types: &[("sys", "log-config destination management-port")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip-address",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "port",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "protocol",
                value_type: ValueKind::Enum,
                enum_values: &["tcp", "udp"],
                default: Some("tcp"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_log_config_destination_remote_high_speed_log",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["log-config destination remote-high-speed-log"],
        },
        header_types: &[("sys", "log-config destination remote-high-speed-log")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "distribution",
                value_type: ValueKind::Enum,
                required: true,
                enum_values: &["adaptive", "balanced", "replicated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "pool-name",
                value_type: ValueKind::Unknown,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "protocol",
                value_type: ValueKind::Enum,
                enum_values: &["tcp", "udp"],
                default: Some("tcp"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_log_config_destination_remote_syslog",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["log-config destination remote-syslog"],
        },
        header_types: &[("sys", "log-config destination remote-syslog")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "default-facility",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "local0", "local1", "local2", "local3", "local4", "local5", "local6", "local7",
                ],
                default: Some("local0"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "default-severity",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "alert", "crit", "debug", "emerg", "err", "info", "notice", "warn",
                ],
                default: Some("info"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "format",
                value_type: ValueKind::Enum,
                enum_values: &["legacy-bigip", "rfc3164", "rfc5424"],
                default: Some("rfc3164"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "remote-high-speed-log",
                value_type: ValueKind::String,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_log_config_destination_splunk",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["log-config destination splunk"],
        },
        header_types: &[("sys", "log-config destination splunk")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "forward-to",
                value_type: ValueKind::String,
                required: true,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_log_config_filter",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["log-config filter"],
        },
        header_types: &[("sys", "log-config filter")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "level",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "alert", "crit", "debug", "emerg", "err", "info", "notice", "warn",
                ],
                default: Some("debug"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "message-id",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["none"],
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "publisher",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "source",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "accesscontrol",
                    "accessperrequest",
                    "adapt",
                    "adfs-proxy",
                    "alertd",
                    "all",
                    "api-protection",
                    "apmacl",
                    "arp",
                    "authz",
                    "autodiscd",
                    "autodosd",
                    "avr",
                    "based",
                    "bcm56xxd",
                    "bdosd",
                    "big3d",
                    "big3dshim",
                    "bigd",
                    "bigdb",
                    "bigdbd",
                    "bigpipe",
                    "bigstart",
                    "bp",
                    "checkcert",
                    "chmand",
                    "cifs",
                    "clusterd",
                    "coapi",
                    "common",
                    "common-f5logging",
                    "common-fpdd",
                    "config-db",
                    "connapi",
                    "cryptod",
                    "cs",
                    "cssd",
                    "csyncd",
                    "daemon",
                    "debugd",
                    "deflate",
                    "devmgmtd",
                    "diameter",
                    "dmon",
                    "dosprotect",
                    "dpi",
                    "dummy",
                    "dwbld",
                    "dynad",
                    "eca",
                    "em-admin",
                    "em-alert",
                    "em-clientlib",
                    "em-common",
                    "em-device",
                    "em-discovery",
                    "em-file",
                    "em-lib",
                    "em-stats",
                    "em-swim",
                    "errdefsd",
                    "eventd",
                    "evrouted",
                    "fflag",
                    "fips",
                    "firewall-FQDN",
                    "firewall-nat",
                    "fix",
                    "ftp",
                    "get-dossier",
                    "gpa",
                    "gtmd",
                    "gtp",
                    "guestagentd",
                    "ha",
                    "ha-table",
                    "halmsg",
                    "hclientd",
                    "hornet-lib",
                    "hornet-nest",
                    "hornet-nest-flow-manager",
                    "hornet-nest-updater",
                    "hornet-neuron-updater",
                    "hornet-server",
                    "hornet-text-client",
                    "hostagentd",
                    "htconnector",
                    "http",
                    "hwctl",
                    "hwpd",
                    "icr-eventd",
                    "icrd",
                    "imap",
                    "ip",
                    "ipfix",
                    "ipfix-proxy",
                    "ipfixirules",
                    "iprepd",
                    "ips",
                    "ipsec",
                    "isession",
                    "istatsd",
                    "ivs",
                    "keymgmtd",
                    "lacpd",
                    "layer4",
                    "libhal",
                    "lind",
                    "lldpd",
                    "localdb",
                    "lopd",
                    "lsn",
                    "lsnapi",
                    "mamidbridged",
                    "map",
                    "mapi",
                    "mcp",
                    "mcpd",
                    "mcpd-apm",
                    "mcpd-asm",
                    "mcpd-centmgmt",
                    "mcpd-clustering",
                    "mcpd-dev",
                    "mcpd-dpi",
                    "mcpd-firewall",
                    "mcpd-framework",
                    "mcpd-gtm",
                    "mcpd-ips",
                    "mcpd-ltm",
                    "mcpd-net",
                    "mcpd-pem",
                    "mcpd-sys",
                    "mcpd-wam",
                    "mcpd-woc",
                    "mdm",
                    "mgmt-acld",
                    "mr",
                    "mrsip",
                    "msgbusd",
                    "mysqlhad",
                    "natstatsd",
                    "net",
                    "network",
                    "no-source",
                    "packet-filter",
                    "pccd",
                    "pcp",
                    "pem",
                    "pfmand",
                    "pgadmind",
                    "pkcs11d",
                    "pktclass",
                    "plugin",
                    "policy",
                    "pop3",
                    "pptp",
                    "probe-plusplus",
                    "promptstatusd",
                    "provisioning",
                    "pva",
                    "pvad",
                    "qkcloud",
                    "radius",
                    "ramcache",
                    "rba",
                    "rewrite",
                    "rtsp",
                    "rules",
                    "saas",
                    "saspd",
                    "scim",
                    "scriptd",
                    "sctp",
                    "sdmd",
                    "sflow",
                    "shell",
                    "shmmapd",
                    "smtps",
                    "snmp",
                    "sod",
                    "spolicy",
                    "ssl",
                    "ssl-c3d",
                    "ssl-certificate",
                    "ssl-client-auth",
                    "ssl-forward-proxy",
                    "ssl-handshake",
                    "ssl-orchestrator",
                    "sso",
                    "stated",
                    "statsd",
                    "statusd",
                    "stmm",
                    "stpd",
                    "subagents",
                    "swg",
                    "syscall",
                    "system-check",
                    "tamd",
                    "tcl-checker",
                    "tcpdump",
                    "tftp",
                    "tmm",
                    "tmm-tcp",
                    "tmrouted",
                    "tmsh",
                    "ts",
                    "tunnel",
                    "urlc",
                    "urldb",
                    "urldbmgrd",
                    "vcmpd",
                    "vdi",
                    "vxland",
                    "webssh",
                    "websso",
                    "woc-plugin",
                    "wr-urldbd",
                    "xconfig",
                    "xdb",
                    "zfd",
                    "zxfrd",
                ],
                default: Some("all"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "sys_log_config_publisher",
            table_name: None,
            resolver_name: None,
            module: Some("sys"),
            object_types: &["log-config publisher"],
        },
        header_types: &[("sys", "log-config publisher")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "destinations",
                value_type: ValueKind::List,
                allow_none: true,
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
];
