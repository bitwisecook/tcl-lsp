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

//! BIG-IP object specs for `net ipsec`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_ipsec_ike_daemon",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["ipsec ike-daemon"],
        },
        header_types: &[("net", "ipsec ike-daemon")],
        properties: &[
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "isakmp-natt-port",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "isakmp-port",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-level",
                value_type: ValueKind::Enum,
                enum_values: &["debug", "debug2", "error", "info", "notify", "warning"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "log-publisher",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "natt-keep-alive",
                value_type: ValueKind::Unknown,
                default: Some("20 seconds"),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_ipsec_ike_peer",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["ipsec ike-peer"],
        },
        header_types: &[("net", "ipsec ike-peer")],
        properties: &[
            BigipPropertySpec {
                name: "address-list",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ca-cert-file",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "crl-file",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "debug-payloads",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "dpd-delay",
                value_type: ValueKind::Integer,
                default: Some("30"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "generate-policy",
                value_type: ValueKind::Enum,
                enum_values: &["off", "on", "unique"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip-macro",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip4-dhcp",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                address_grammar: Some(crate::bigip::AddressGrammar::Address),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip4-dns",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                address_grammar: Some(crate::bigip::AddressGrammar::Address),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip6-dhcp",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                address_grammar: Some(crate::bigip::AddressGrammar::Address),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip6-dns",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                address_grammar: Some(crate::bigip::AddressGrammar::Address),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "lifetime",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "local-port",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mode",
                value_type: ValueKind::Enum,
                enum_values: &["aggressive", "main"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "my-cert-file",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "my-cert-key-file",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "my-cert-key-passphrase",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "my-id-type",
                value_type: ValueKind::Enum,
                enum_values: &["asn1dn", "fqdn", "keyid-tag", "user-fqdn"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "my-id-value",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "nat-traversal",
                value_type: ValueKind::Enum,
                enum_values: &["force", "off", "on"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ocsp-cert-validator",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ocsp-ha-reauth",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ocsp-jitter-percent",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ocsp-lifetime",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ocsp-reauth-fail-open",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "passive",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "peer-dynamic-ip",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                address_grammar: Some(crate::bigip::AddressGrammar::Address),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "peers-cert-file",
                value_type: ValueKind::Unknown,
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "peers-cert-type",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["certfile", "none"],
                usage_flags: &["deprecated"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "peers-id-type",
                value_type: ValueKind::Enum,
                enum_values: &["asn1dn", "fqdn", "keyid-tag", "user-fqdn"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "peers-id-value",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "phase1-auth-method",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "dss",
                    "ecdsa-256",
                    "ecdsa-384",
                    "ecdsa-521",
                    "pre-shared-key",
                    "rsa-signature",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "phase1-encrypt-algorithm",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "3des",
                    "aes",
                    "aes-gcm128",
                    "aes-gcm192",
                    "aes-gcm256",
                    "blowfish",
                    "camellia",
                    "cast128",
                    "des",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "phase1-hash-algorithm",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "aes-gcm128",
                    "aes-gcm192",
                    "aes-gcm256",
                    "md5",
                    "sha1",
                    "sha256",
                    "sha384",
                    "sha512",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "phase1-perfect-forward-secrecy",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "ecp256", "ecp384", "ecp521", "modp1024", "modp1536", "modp2048", "modp3072",
                    "modp4096", "modp6144", "modp768", "modp8192",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "preshared-key",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "preshared-key-encrypted",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "prf",
                value_type: ValueKind::Enum,
                enum_values: &["sha1", "sha256", "sha384", "sha512"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "proxy-support",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "remote-address",
                value_type: ValueKind::String,
                required: true,
                shape_kind: Some(ValueKind::IpAddress),
                address_grammar: Some(crate::bigip::AddressGrammar::Address),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "remote-port",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "replay-window-size",
                value_type: ValueKind::Integer,
                default: Some("64"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "state",
                value_type: ValueKind::Enum,
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "traffic-selector",
                value_type: ValueKind::Reference,
                references: &["net_ipsec_traffic_selector"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "verify-cert",
                value_type: ValueKind::Enum,
                enum_values: &["false", "true"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "version",
                value_type: ValueKind::List,
                allow_none: true,
                default: Some("v1"),
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_ipsec_ipsec_policy",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["ipsec ipsec-policy"],
        },
        header_types: &[("net", "ipsec ipsec-policy")],
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
                name: "ike-phase2-auth-algorithm",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "aes-gcm128",
                    "aes-gcm192",
                    "aes-gcm256",
                    "aes-gmac128",
                    "aes-gmac192",
                    "aes-gmac256",
                    "sha1",
                    "sha256",
                    "sha384",
                    "sha512",
                ],
                default: Some("aes-gcm128"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ike-phase2-encrypt-algorithm",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "3des",
                    "aes-gcm128",
                    "aes-gcm192",
                    "aes-gcm256",
                    "aes-gmac128",
                    "aes-gmac192",
                    "aes-gmac256",
                    "aes128",
                    "aes192",
                    "aes256",
                    "null",
                ],
                default: Some("aes-gcm128"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ike-phase2-lifetime",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ike-phase2-lifetime-kilobytes",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ike-phase2-perfect-forward-secrecy",
                value_type: ValueKind::Enum,
                enum_values: &[
                    "modp1024", "modp1536", "modp2048", "modp3072", "modp4096", "modp6144",
                    "modp768", "modp8192",
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ipcomp",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["deflate", "none", "null"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "mode",
                value_type: ValueKind::Enum,
                enum_values: &["interface", "tunnel"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "protocol",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "tunnel-local-address",
                value_type: ValueKind::String,
                required: true,
                shape_kind: Some(ValueKind::IpAddress),
                address_grammar: Some(crate::bigip::AddressGrammar::Address),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "tunnel-remote-address",
                value_type: ValueKind::String,
                required: true,
                shape_kind: Some(ValueKind::IpAddress),
                address_grammar: Some(crate::bigip::AddressGrammar::Address),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_ipsec_manual_security_association",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["ipsec manual-security-association"],
        },
        header_types: &[("net", "ipsec manual-security-association")],
        properties: &[
            BigipPropertySpec {
                name: "app-service",
                value_type: ValueKind::String,
                allow_none: true,
                default: Some("none"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auth-algorithm",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auth-key",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "destination-address",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                address_grammar: Some(crate::bigip::AddressGrammar::Network),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "encrypt-algorithm",
                value_type: ValueKind::Enum,
                enum_values: &["3des", "aes128", "aes192", "aes256", "null"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "encrypt-key",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ipsec-policy",
                value_type: ValueKind::Reference,
                references: &["net_ipsec_ipsec_policy"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "protocol",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "source-address",
                value_type: ValueKind::String,
                shape_kind: Some(ValueKind::IpAddress),
                address_grammar: Some(crate::bigip::AddressGrammar::Network),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "spi",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "net_ipsec_traffic_selector",
            table_name: None,
            resolver_name: None,
            module: Some("net"),
            object_types: &["ipsec traffic-selector"],
        },
        header_types: &[("net", "ipsec traffic-selector")],
        properties: &[
            BigipPropertySpec {
                name: "action",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
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
                name: "destination-address",
                value_type: ValueKind::String,
                required: true,
                shape_kind: Some(ValueKind::IpAddress),
                address_grammar: Some(crate::bigip::AddressGrammar::Network),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "destination-port",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "direction",
                value_type: ValueKind::Enum,
                enum_values: &["both", "in", "out"],
                default: Some("both"),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ip-protocol",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ipsec-policy",
                value_type: ValueKind::Reference,
                references: &["net_ipsec_ipsec_policy"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "order",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "source-address",
                value_type: ValueKind::String,
                required: true,
                shape_kind: Some(ValueKind::IpAddress),
                address_grammar: Some(crate::bigip::AddressGrammar::Network),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "source-port",
                value_type: ValueKind::Unknown,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
];
