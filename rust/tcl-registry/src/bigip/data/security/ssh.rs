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

//! BIG-IP object specs for `security ssh`. Hand-maintained: edit them directly.

use crate::bigip::{BigipObjectKindSpec, BigipObjectSpec, BigipPropertySpec, ValueKind};

pub(super) static SPECS: &[BigipObjectSpec] = &[
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "security_ssh_ciphers",
            table_name: None,
            resolver_name: None,
            module: Some("security"),
            object_types: &["ssh ciphers"],
        },
        header_types: &[("security", "ssh ciphers")],
        properties: &[
            BigipPropertySpec {
                name: "client",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "aes128-cbc",
                        value_type: ValueKind::Enum,
                        in_sections: &["client"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "aes128-ctr",
                        value_type: ValueKind::Enum,
                        in_sections: &["client"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "aes128-gcm-openssh-com",
                        value_type: ValueKind::Enum,
                        in_sections: &["client"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "aes192-cbc",
                        value_type: ValueKind::Enum,
                        in_sections: &["client"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "aes192-ctr",
                        value_type: ValueKind::Enum,
                        in_sections: &["client"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "aes256-cbc",
                        value_type: ValueKind::Enum,
                        in_sections: &["client"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "aes256-ctr",
                        value_type: ValueKind::Enum,
                        in_sections: &["client"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "aes256-gcm-openssh-com",
                        value_type: ValueKind::Enum,
                        in_sections: &["client"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "curve25519-sha256",
                        value_type: ValueKind::Enum,
                        in_sections: &["client"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "curve25519-sha256-libssh-org",
                        value_type: ValueKind::Enum,
                        in_sections: &["client"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "diffie-hellman-group-exchange-sha256",
                        value_type: ValueKind::Enum,
                        in_sections: &["client"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "diffie-hellman-group1-sha1",
                        value_type: ValueKind::Enum,
                        in_sections: &["client"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "diffie-hellman-group14-sha1",
                        value_type: ValueKind::Enum,
                        in_sections: &["client"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "diffie-hellman-group14-sha256",
                        value_type: ValueKind::Enum,
                        in_sections: &["client"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "diffie-hellman-group16-sha512",
                        value_type: ValueKind::Enum,
                        in_sections: &["client"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "diffie-hellman-group18-sha512",
                        value_type: ValueKind::Enum,
                        in_sections: &["client"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "ecdh-sha2-nistp256",
                        value_type: ValueKind::Enum,
                        in_sections: &["client"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "ecdh-sha2-nistp384",
                        value_type: ValueKind::Enum,
                        in_sections: &["client"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "ecdh-sha2-nistp521",
                        value_type: ValueKind::Enum,
                        in_sections: &["client"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "ext-info-c",
                        value_type: ValueKind::Enum,
                        in_sections: &["client"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "hmac-sha1",
                        value_type: ValueKind::Enum,
                        in_sections: &["client"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "hmac-sha1-etm-openssh-com",
                        value_type: ValueKind::Enum,
                        in_sections: &["client"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "hmac-sha2-256",
                        value_type: ValueKind::Enum,
                        in_sections: &["client"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "hmac-sha2-256-etm-openssh-com",
                        value_type: ValueKind::Enum,
                        in_sections: &["client"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "hmac-sha2-512",
                        value_type: ValueKind::Enum,
                        in_sections: &["client"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "hmac-sha2-512-etm-openssh-com",
                        value_type: ValueKind::Enum,
                        in_sections: &["client"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "three-des-cbc",
                        value_type: ValueKind::Enum,
                        in_sections: &["client"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "aes128-cbc",
                value_type: ValueKind::Enum,
                in_sections: &["client"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "aes128-ctr",
                value_type: ValueKind::Enum,
                in_sections: &["client"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "aes128-gcm-openssh-com",
                value_type: ValueKind::Enum,
                in_sections: &["client"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "aes192-cbc",
                value_type: ValueKind::Enum,
                in_sections: &["client"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "aes192-ctr",
                value_type: ValueKind::Enum,
                in_sections: &["client"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "aes256-cbc",
                value_type: ValueKind::Enum,
                in_sections: &["client"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "aes256-ctr",
                value_type: ValueKind::Enum,
                in_sections: &["client"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "aes256-gcm-openssh-com",
                value_type: ValueKind::Enum,
                in_sections: &["client"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "curve25519-sha256",
                value_type: ValueKind::Enum,
                in_sections: &["client"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "curve25519-sha256-libssh-org",
                value_type: ValueKind::Enum,
                in_sections: &["client"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "diffie-hellman-group-exchange-sha256",
                value_type: ValueKind::Enum,
                in_sections: &["client"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "diffie-hellman-group1-sha1",
                value_type: ValueKind::Enum,
                in_sections: &["client"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "diffie-hellman-group14-sha1",
                value_type: ValueKind::Enum,
                in_sections: &["client"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "diffie-hellman-group14-sha256",
                value_type: ValueKind::Enum,
                in_sections: &["client"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "diffie-hellman-group16-sha512",
                value_type: ValueKind::Enum,
                in_sections: &["client"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "diffie-hellman-group18-sha512",
                value_type: ValueKind::Enum,
                in_sections: &["client"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ecdh-sha2-nistp256",
                value_type: ValueKind::Enum,
                in_sections: &["client"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ecdh-sha2-nistp384",
                value_type: ValueKind::Enum,
                in_sections: &["client"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ecdh-sha2-nistp521",
                value_type: ValueKind::Enum,
                in_sections: &["client"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ext-info-c",
                value_type: ValueKind::Enum,
                in_sections: &["client"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hmac-sha1",
                value_type: ValueKind::Enum,
                in_sections: &["client"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hmac-sha1-etm-openssh-com",
                value_type: ValueKind::Enum,
                in_sections: &["client"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hmac-sha2-256",
                value_type: ValueKind::Enum,
                in_sections: &["client"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hmac-sha2-256-etm-openssh-com",
                value_type: ValueKind::Enum,
                in_sections: &["client"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hmac-sha2-512",
                value_type: ValueKind::Enum,
                in_sections: &["client"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hmac-sha2-512-etm-openssh-com",
                value_type: ValueKind::Enum,
                in_sections: &["client"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "three-des-cbc",
                value_type: ValueKind::Enum,
                in_sections: &["client"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "server",
                value_type: ValueKind::Unknown,
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "aes128-cbc",
                        value_type: ValueKind::Enum,
                        in_sections: &["server"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "aes128-ctr",
                        value_type: ValueKind::Enum,
                        in_sections: &["server"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "aes128-gcm-openssh-com",
                        value_type: ValueKind::Enum,
                        in_sections: &["server"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "aes192-cbc",
                        value_type: ValueKind::Enum,
                        in_sections: &["server"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "aes192-ctr",
                        value_type: ValueKind::Enum,
                        in_sections: &["server"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "aes256-cbc",
                        value_type: ValueKind::Enum,
                        in_sections: &["server"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "aes256-ctr",
                        value_type: ValueKind::Enum,
                        in_sections: &["server"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "aes256-gcm-openssh-com",
                        value_type: ValueKind::Enum,
                        in_sections: &["server"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "curve25519-sha256",
                        value_type: ValueKind::Enum,
                        in_sections: &["server"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "curve25519-sha256-libssh-org",
                        value_type: ValueKind::Enum,
                        in_sections: &["server"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "diffie-hellman-group-exchange-sha256",
                        value_type: ValueKind::Enum,
                        in_sections: &["server"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "diffie-hellman-group1-sha1",
                        value_type: ValueKind::Enum,
                        in_sections: &["server"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "diffie-hellman-group14-sha1",
                        value_type: ValueKind::Enum,
                        in_sections: &["server"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "diffie-hellman-group14-sha256",
                        value_type: ValueKind::Enum,
                        in_sections: &["server"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "diffie-hellman-group16-sha512",
                        value_type: ValueKind::Enum,
                        in_sections: &["server"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "diffie-hellman-group18-sha512",
                        value_type: ValueKind::Enum,
                        in_sections: &["server"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "ecdh-sha2-nistp256",
                        value_type: ValueKind::Enum,
                        in_sections: &["server"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "ecdh-sha2-nistp384",
                        value_type: ValueKind::Enum,
                        in_sections: &["server"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "ecdh-sha2-nistp521",
                        value_type: ValueKind::Enum,
                        in_sections: &["server"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "ext-info-c",
                        value_type: ValueKind::Enum,
                        in_sections: &["server"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "hmac-sha1",
                        value_type: ValueKind::Enum,
                        in_sections: &["server"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "hmac-sha1-etm-openssh-com",
                        value_type: ValueKind::Enum,
                        in_sections: &["server"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "hmac-sha2-256",
                        value_type: ValueKind::Enum,
                        in_sections: &["server"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "hmac-sha2-256-etm-openssh-com",
                        value_type: ValueKind::Enum,
                        in_sections: &["server"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "hmac-sha2-512",
                        value_type: ValueKind::Enum,
                        in_sections: &["server"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "hmac-sha2-512-etm-openssh-com",
                        value_type: ValueKind::Enum,
                        in_sections: &["server"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "three-des-cbc",
                        value_type: ValueKind::Enum,
                        in_sections: &["server"],
                        enum_values: &["disabled", "enabled"],
                        shape_kind: Some(ValueKind::Boolean),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "aes128-cbc",
                value_type: ValueKind::Enum,
                in_sections: &["server"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "aes128-ctr",
                value_type: ValueKind::Enum,
                in_sections: &["server"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "aes128-gcm-openssh-com",
                value_type: ValueKind::Enum,
                in_sections: &["server"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "aes192-cbc",
                value_type: ValueKind::Enum,
                in_sections: &["server"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "aes192-ctr",
                value_type: ValueKind::Enum,
                in_sections: &["server"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "aes256-cbc",
                value_type: ValueKind::Enum,
                in_sections: &["server"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "aes256-ctr",
                value_type: ValueKind::Enum,
                in_sections: &["server"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "aes256-gcm-openssh-com",
                value_type: ValueKind::Enum,
                in_sections: &["server"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "curve25519-sha256",
                value_type: ValueKind::Enum,
                in_sections: &["server"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "curve25519-sha256-libssh-org",
                value_type: ValueKind::Enum,
                in_sections: &["server"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "diffie-hellman-group-exchange-sha256",
                value_type: ValueKind::Enum,
                in_sections: &["server"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "diffie-hellman-group1-sha1",
                value_type: ValueKind::Enum,
                in_sections: &["server"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "diffie-hellman-group14-sha1",
                value_type: ValueKind::Enum,
                in_sections: &["server"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "diffie-hellman-group14-sha256",
                value_type: ValueKind::Enum,
                in_sections: &["server"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "diffie-hellman-group16-sha512",
                value_type: ValueKind::Enum,
                in_sections: &["server"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "diffie-hellman-group18-sha512",
                value_type: ValueKind::Enum,
                in_sections: &["server"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ecdh-sha2-nistp256",
                value_type: ValueKind::Enum,
                in_sections: &["server"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ecdh-sha2-nistp384",
                value_type: ValueKind::Enum,
                in_sections: &["server"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ecdh-sha2-nistp521",
                value_type: ValueKind::Enum,
                in_sections: &["server"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "ext-info-c",
                value_type: ValueKind::Enum,
                in_sections: &["server"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hmac-sha1",
                value_type: ValueKind::Enum,
                in_sections: &["server"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hmac-sha1-etm-openssh-com",
                value_type: ValueKind::Enum,
                in_sections: &["server"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hmac-sha2-256",
                value_type: ValueKind::Enum,
                in_sections: &["server"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hmac-sha2-256-etm-openssh-com",
                value_type: ValueKind::Enum,
                in_sections: &["server"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hmac-sha2-512",
                value_type: ValueKind::Enum,
                in_sections: &["server"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "hmac-sha2-512-etm-openssh-com",
                value_type: ValueKind::Enum,
                in_sections: &["server"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "three-des-cbc",
                value_type: ValueKind::Enum,
                in_sections: &["server"],
                enum_values: &["disabled", "enabled"],
                shape_kind: Some(ValueKind::Boolean),
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
    BigipObjectSpec {
        kind_spec: BigipObjectKindSpec {
            kind: "security_ssh_profile",
            table_name: None,
            resolver_name: None,
            module: Some("security"),
            object_types: &["ssh profile"],
        },
        header_types: &[("security", "ssh profile")],
        properties: &[
            BigipPropertySpec {
                name: "actions",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify"],
                block: &[
                    BigipPropertySpec {
                        name: "agent-action",
                        value_type: ValueKind::Unknown,
                        in_sections: &["actions"],
                        enum_values: &["allow", "disallow", "no", "terminate", "yes"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "local-forward-action",
                        value_type: ValueKind::Unknown,
                        in_sections: &["actions"],
                        enum_values: &["allow", "disallow", "no", "terminate", "yes"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "other-action",
                        value_type: ValueKind::Unknown,
                        in_sections: &["actions"],
                        enum_values: &["allow", "disallow", "no", "terminate", "yes"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "remote-forward-action",
                        value_type: ValueKind::Unknown,
                        in_sections: &["actions"],
                        enum_values: &["allow", "disallow", "no", "terminate", "yes"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "rexec-action",
                        value_type: ValueKind::Unknown,
                        in_sections: &["actions"],
                        enum_values: &["allow", "disallow", "no", "terminate", "yes"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "scp-down-action",
                        value_type: ValueKind::Unknown,
                        in_sections: &["actions"],
                        enum_values: &["allow", "disallow", "no", "terminate", "yes"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "scp-up-action",
                        value_type: ValueKind::Unknown,
                        in_sections: &["actions"],
                        enum_values: &["allow", "disallow", "no", "terminate", "yes"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "sftp-down-action",
                        value_type: ValueKind::Unknown,
                        in_sections: &["actions"],
                        enum_values: &["allow", "disallow", "no", "terminate", "yes"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "sftp-up-action",
                        value_type: ValueKind::Unknown,
                        in_sections: &["actions"],
                        enum_values: &["allow", "disallow", "no", "terminate", "yes"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "shell-action",
                        value_type: ValueKind::Unknown,
                        in_sections: &["actions"],
                        enum_values: &["allow", "disallow", "no", "terminate", "yes"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "sub-system-action",
                        value_type: ValueKind::Unknown,
                        in_sections: &["actions"],
                        enum_values: &["allow", "disallow", "no", "terminate", "yes"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "x11-forward-action",
                        value_type: ValueKind::Unknown,
                        in_sections: &["actions"],
                        enum_values: &["allow", "disallow", "no", "terminate", "yes"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "agent-action",
                value_type: ValueKind::Unknown,
                in_sections: &["actions"],
                enum_values: &["allow", "disallow", "no", "terminate", "yes"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "local-forward-action",
                value_type: ValueKind::Unknown,
                in_sections: &["actions"],
                enum_values: &["allow", "disallow", "no", "terminate", "yes"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "other-action",
                value_type: ValueKind::Unknown,
                in_sections: &["actions"],
                enum_values: &["allow", "disallow", "no", "terminate", "yes"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "remote-forward-action",
                value_type: ValueKind::Unknown,
                in_sections: &["actions"],
                enum_values: &["allow", "disallow", "no", "terminate", "yes"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rexec-action",
                value_type: ValueKind::Unknown,
                in_sections: &["actions"],
                enum_values: &["allow", "disallow", "no", "terminate", "yes"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "scp-down-action",
                value_type: ValueKind::Unknown,
                in_sections: &["actions"],
                enum_values: &["allow", "disallow", "no", "terminate", "yes"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "scp-up-action",
                value_type: ValueKind::Unknown,
                in_sections: &["actions"],
                enum_values: &["allow", "disallow", "no", "terminate", "yes"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sftp-down-action",
                value_type: ValueKind::Unknown,
                in_sections: &["actions"],
                enum_values: &["allow", "disallow", "no", "terminate", "yes"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sftp-up-action",
                value_type: ValueKind::Unknown,
                in_sections: &["actions"],
                enum_values: &["allow", "disallow", "no", "terminate", "yes"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "shell-action",
                value_type: ValueKind::Unknown,
                in_sections: &["actions"],
                enum_values: &["allow", "disallow", "no", "terminate", "yes"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sub-system-action",
                value_type: ValueKind::Unknown,
                in_sections: &["actions"],
                enum_values: &["allow", "disallow", "no", "terminate", "yes"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "x11-forward-action",
                value_type: ValueKind::Unknown,
                in_sections: &["actions"],
                enum_values: &["allow", "disallow", "no", "terminate", "yes"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "auth-info",
                value_type: ValueKind::List,
                list_operators: &["add", "delete", "modify"],
                block: &[
                    BigipPropertySpec {
                        name: "proxy-client-auth",
                        value_type: ValueKind::Unknown,
                        in_sections: &["auth-info"],
                        required: true,
                        shape_kind: Some(ValueKind::Object),
                        usage_flags: &["optional"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "proxy-server-auth",
                        value_type: ValueKind::Unknown,
                        in_sections: &["auth-info"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "real-server-auth",
                        value_type: ValueKind::Unknown,
                        in_sections: &["auth-info"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "proxy-client-auth",
                value_type: ValueKind::Unknown,
                in_sections: &["auth-info"],
                required: true,
                shape_kind: Some(ValueKind::Object),
                usage_flags: &["optional"],
                block: &[
                    BigipPropertySpec {
                        name: "private-key",
                        value_type: ValueKind::String,
                        in_sections: &["auth-info", "proxy-client-auth"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "public-key",
                        value_type: ValueKind::String,
                        in_sections: &["auth-info", "proxy-client-auth"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "private-key",
                value_type: ValueKind::String,
                in_sections: &["auth-info", "proxy-client-auth"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "public-key",
                value_type: ValueKind::String,
                in_sections: &["auth-info", "proxy-client-auth"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "proxy-server-auth",
                value_type: ValueKind::Unknown,
                in_sections: &["auth-info"],
                shape_kind: Some(ValueKind::Object),
                block: &[
                    BigipPropertySpec {
                        name: "private-key",
                        value_type: ValueKind::String,
                        in_sections: &["auth-info", "proxy-server-auth"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "public-key",
                        value_type: ValueKind::String,
                        in_sections: &["auth-info", "proxy-server-auth"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "private-key",
                value_type: ValueKind::String,
                in_sections: &["auth-info", "proxy-server-auth"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "public-key",
                value_type: ValueKind::String,
                in_sections: &["auth-info", "proxy-server-auth"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "real-server-auth",
                value_type: ValueKind::Unknown,
                in_sections: &["auth-info"],
                shape_kind: Some(ValueKind::Object),
                block: &[BigipPropertySpec {
                    name: "public-key",
                    value_type: ValueKind::String,
                    in_sections: &["auth-info", "real-server-auth"],
                    ..BigipPropertySpec::DEFAULT
                }],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "public-key",
                value_type: ValueKind::String,
                in_sections: &["auth-info", "real-server-auth"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "lang-env-tolerance",
                value_type: ValueKind::Enum,
                allow_none: true,
                enum_values: &["any", "common", "default-value", "none"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rules",
                value_type: ValueKind::List,
                allow_none: true,
                references: &[
                    "gtm_rule",
                    "ltm_cipher_rule",
                    "ltm_global_settings_rule",
                    "ltm_rule",
                    "ltm_rule_profiler",
                    "security_firewall_matching_rule",
                    "security_firewall_on_demand_rule_deploy",
                    "security_firewall_rule_list",
                    "security_firewall_rule_stat",
                    "security_packet_filter_rule_stat",
                    "sys_file_rewrite_rule",
                ],
                list_operators: &["add", "delete", "modify", "replace-all-with"],
                block: &[
                    BigipPropertySpec {
                        name: "actions",
                        value_type: ValueKind::List,
                        in_sections: &["rules"],
                        list_operators: &["add", "delete", "modify"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "description",
                        value_type: ValueKind::String,
                        in_sections: &["rules"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "identity-groups",
                        value_type: ValueKind::List,
                        in_sections: &["rules"],
                        list_operators: &["add", "delete", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "identity-users",
                        value_type: ValueKind::List,
                        in_sections: &["rules"],
                        list_operators: &["add", "delete", "replace-all-with"],
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "actions",
                value_type: ValueKind::List,
                in_sections: &["rules"],
                list_operators: &["add", "delete", "modify"],
                block: &[
                    BigipPropertySpec {
                        name: "agent-action",
                        value_type: ValueKind::Unknown,
                        in_sections: &["rules", "actions"],
                        enum_values: &["allow", "disallow", "no", "terminate", "yes"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "local-forward-action",
                        value_type: ValueKind::Unknown,
                        in_sections: &["rules", "actions"],
                        enum_values: &["allow", "disallow", "no", "terminate", "yes"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "other-action",
                        value_type: ValueKind::Unknown,
                        in_sections: &["rules", "actions"],
                        enum_values: &["allow", "disallow", "no", "terminate", "yes"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "remote-forward-action",
                        value_type: ValueKind::Unknown,
                        in_sections: &["rules", "actions"],
                        enum_values: &["allow", "disallow", "no", "terminate", "yes"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "rexec-action",
                        value_type: ValueKind::Unknown,
                        in_sections: &["rules", "actions"],
                        enum_values: &["allow", "disallow", "no", "terminate", "yes"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "scp-down-action",
                        value_type: ValueKind::Unknown,
                        in_sections: &["rules", "actions"],
                        enum_values: &["allow", "disallow", "no", "terminate", "yes"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "scp-up-action",
                        value_type: ValueKind::Unknown,
                        in_sections: &["rules", "actions"],
                        enum_values: &["allow", "disallow", "no", "terminate", "yes"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "sftp-down-action",
                        value_type: ValueKind::Unknown,
                        in_sections: &["rules", "actions"],
                        enum_values: &["allow", "disallow", "no", "terminate", "yes"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "sftp-up-action",
                        value_type: ValueKind::Unknown,
                        in_sections: &["rules", "actions"],
                        enum_values: &["allow", "disallow", "no", "terminate", "yes"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "shell-action",
                        value_type: ValueKind::Unknown,
                        in_sections: &["rules", "actions"],
                        enum_values: &["allow", "disallow", "no", "terminate", "yes"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "sub-system-action",
                        value_type: ValueKind::Unknown,
                        in_sections: &["rules", "actions"],
                        enum_values: &["allow", "disallow", "no", "terminate", "yes"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                    BigipPropertySpec {
                        name: "x11-forward-action",
                        value_type: ValueKind::Unknown,
                        in_sections: &["rules", "actions"],
                        enum_values: &["allow", "disallow", "no", "terminate", "yes"],
                        shape_kind: Some(ValueKind::Object),
                        ..BigipPropertySpec::DEFAULT
                    },
                ],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "agent-action",
                value_type: ValueKind::Unknown,
                in_sections: &["rules", "actions"],
                enum_values: &["allow", "disallow", "no", "terminate", "yes"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "local-forward-action",
                value_type: ValueKind::Unknown,
                in_sections: &["rules", "actions"],
                enum_values: &["allow", "disallow", "no", "terminate", "yes"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "other-action",
                value_type: ValueKind::Unknown,
                in_sections: &["rules", "actions"],
                enum_values: &["allow", "disallow", "no", "terminate", "yes"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "remote-forward-action",
                value_type: ValueKind::Unknown,
                in_sections: &["rules", "actions"],
                enum_values: &["allow", "disallow", "no", "terminate", "yes"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "rexec-action",
                value_type: ValueKind::Unknown,
                in_sections: &["rules", "actions"],
                enum_values: &["allow", "disallow", "no", "terminate", "yes"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "scp-down-action",
                value_type: ValueKind::Unknown,
                in_sections: &["rules", "actions"],
                enum_values: &["allow", "disallow", "no", "terminate", "yes"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "scp-up-action",
                value_type: ValueKind::Unknown,
                in_sections: &["rules", "actions"],
                enum_values: &["allow", "disallow", "no", "terminate", "yes"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sftp-down-action",
                value_type: ValueKind::Unknown,
                in_sections: &["rules", "actions"],
                enum_values: &["allow", "disallow", "no", "terminate", "yes"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sftp-up-action",
                value_type: ValueKind::Unknown,
                in_sections: &["rules", "actions"],
                enum_values: &["allow", "disallow", "no", "terminate", "yes"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "shell-action",
                value_type: ValueKind::Unknown,
                in_sections: &["rules", "actions"],
                enum_values: &["allow", "disallow", "no", "terminate", "yes"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "sub-system-action",
                value_type: ValueKind::Unknown,
                in_sections: &["rules", "actions"],
                enum_values: &["allow", "disallow", "no", "terminate", "yes"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "x11-forward-action",
                value_type: ValueKind::Unknown,
                in_sections: &["rules", "actions"],
                enum_values: &["allow", "disallow", "no", "terminate", "yes"],
                shape_kind: Some(ValueKind::Object),
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "description",
                value_type: ValueKind::String,
                in_sections: &["rules"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "identity-groups",
                value_type: ValueKind::List,
                in_sections: &["rules"],
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "identity-users",
                value_type: ValueKind::List,
                in_sections: &["rules"],
                list_operators: &["add", "delete", "replace-all-with"],
                ..BigipPropertySpec::DEFAULT
            },
            BigipPropertySpec {
                name: "timeout",
                value_type: ValueKind::Integer,
                ..BigipPropertySpec::DEFAULT
            },
        ],
    },
];
