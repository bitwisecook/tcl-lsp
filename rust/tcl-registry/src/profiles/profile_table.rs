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

//! The table of BIG-IP profile types: parents, peers, layers and lifecycles.

use super::{ProfileRelation, ProfileSpec, ProfileTerm};
use crate::lifecycle::Lifecycle;

// AUTO-GENERATED — do not edit manually

pub(super) fn profile_specs() -> Vec<ProfileSpec> {
    let mut out = profile_specs_0();
    out.extend(profile_specs_1());
    out.extend(profile_specs_2());
    out.extend(profile_specs_3());
    out.extend(profile_specs_4());
    out.extend(profile_specs_5());
    out.extend(profile_specs_6());
    out.extend(profile_specs_7());
    out.extend(profile_specs_8());
    out
}

fn profile_specs_0() -> Vec<ProfileSpec> {
    vec![
        ProfileSpec {
            name: "ACCESS",
            layer: "security",
            side: "client",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("ACCESS"),
                    &[ProfileTerm("TCP")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "AIMCP",
            layer: "application",
            side: "both",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("AIMCP"),
                    &[ProfileTerm("HTTP")],
                )]
            },
            lifecycle: Lifecycle::introduced_in("21.1.0"),
        },
        ProfileSpec {
            name: "ANTIFRAUD",
            layer: "security",
            side: "client",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("ANTIFRAUD"),
                    &[ProfileTerm("HTTP")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "ASM",
            layer: "security",
            side: "both",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("ASM"),
                    &[ProfileTerm("HTTP")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "AUTH",
            layer: "security",
            side: "client",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("AUTH"),
                    &[ProfileTerm("TCP")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "AVR",
            layer: "acceleration",
            side: "both",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("AVR"),
                    &[ProfileTerm("HTTP")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "BOTDEFENSE",
            layer: "security",
            side: "client",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("BOTDEFENSE"),
                    &[ProfileTerm("HTTP")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
    ]
}

fn profile_specs_1() -> Vec<ProfileSpec> {
    vec![
        ProfileSpec {
            name: "CACHE",
            layer: "acceleration",
            side: "both",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("CACHE"),
                    &[ProfileTerm("HTTP")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "CATEGORY",
            layer: "acceleration",
            side: "both",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("CATEGORY"),
                    &[ProfileTerm("HTTP")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "CLASSIFICATION",
            layer: "acceleration",
            side: "both",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("CLASSIFICATION"),
                    &[ProfileTerm("TCP")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "CLIENTSSL",
            layer: "tls",
            side: "client",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("CLIENTSSL"),
                    &[ProfileTerm("TCP")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "CONNECTOR",
            layer: "application",
            side: "both",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("CONNECTOR"),
                    &[ProfileTerm("TCP")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "DATAGRAM",
            layer: "application",
            side: "both",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("DATAGRAM"),
                    &[ProfileTerm("UDP")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "DIAMETER",
            layer: "application",
            side: "both",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("DIAMETER"),
                    &[ProfileTerm("TCP")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
    ]
}

fn profile_specs_2() -> Vec<ProfileSpec> {
    vec![
        ProfileSpec {
            name: "DIAMETERSESSION",
            layer: "application",
            side: "both",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("DIAMETERSESSION"),
                    &[ProfileTerm("DIAMETER")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "DIAMETER_ENDPOINT",
            layer: "application",
            side: "both",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("DIAMETER_ENDPOINT"),
                    &[ProfileTerm("DIAMETER")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "DNS",
            layer: "application",
            side: "both",
            relations: &[],
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "DOSL7",
            layer: "security",
            side: "both",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("DOSL7"),
                    &[ProfileTerm("HTTP")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "ECA",
            layer: "security",
            side: "client",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("ECA"),
                    &[ProfileTerm("TCP")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "FASTHTTP",
            layer: "application",
            side: "both",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("FASTHTTP"),
                    &[ProfileTerm("TCP")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "FASTL4",
            layer: "transport",
            side: "both",
            relations: &[],
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "FIX",
            layer: "application",
            side: "both",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("FIX"),
                    &[ProfileTerm("TCP")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
    ]
}

fn profile_specs_3() -> Vec<ProfileSpec> {
    vec![
        ProfileSpec {
            name: "FLOW",
            layer: "application",
            side: "both",
            relations: &[],
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "GENERICMSG",
            layer: "application",
            side: "both",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("GENERICMSG"),
                    &[ProfileTerm("TCP")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "GTP",
            layer: "application",
            side: "both",
            relations: &[],
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "HTML",
            layer: "acceleration",
            side: "both",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("HTML"),
                    &[ProfileTerm("HTTP")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "HTTP",
            layer: "application",
            side: "both",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("HTTP"),
                    &[ProfileTerm("TCP")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "HTTP2",
            layer: "application",
            side: "both",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("HTTP2"),
                    &[ProfileTerm("HTTP")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "HTTP_PROXY_CONNECT",
            layer: "application",
            side: "both",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("HTTP_PROXY_CONNECT"),
                    &[ProfileTerm("HTTP")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "ICAP",
            layer: "acceleration",
            side: "both",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("ICAP"),
                    &[ProfileTerm("TCP")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
    ]
}

fn profile_specs_4() -> Vec<ProfileSpec> {
    vec![
        ProfileSpec {
            name: "IPS",
            layer: "security",
            side: "both",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("IPS"),
                    &[ProfileTerm("PROTOCOL_INSPECTION")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "IVS_ENTRY",
            layer: "application",
            side: "both",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("IVS_ENTRY"),
                    &[ProfileTerm("TCP")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "JSON",
            layer: "acceleration",
            side: "both",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("JSON"),
                    &[ProfileTerm("HTTP")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "L7CHECK",
            layer: "application",
            side: "both",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("L7CHECK"),
                    &[ProfileTerm("TCP")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "LSN",
            layer: "application",
            side: "client",
            relations: &[],
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "MQTT",
            layer: "application",
            side: "both",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("MQTT"),
                    &[ProfileTerm("TCP")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "MR",
            layer: "application",
            side: "both",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("MR"),
                    &[ProfileTerm("TCP")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "MSSQL",
            layer: "application",
            side: "both",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("MSSQL"),
                    &[ProfileTerm("TDS")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
    ]
}

fn profile_specs_5() -> Vec<ProfileSpec> {
    vec![
        ProfileSpec {
            name: "NAME",
            layer: "application",
            side: "both",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("NAME"),
                    &[ProfileTerm("TCP")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "PCP",
            layer: "application",
            side: "both",
            relations: &[],
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "PEM",
            layer: "security",
            side: "client",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("PEM"),
                    &[ProfileTerm("TCP")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "PERSIST",
            // Side-independent (shared) TLS/persistence layer. Stack
            // infrastructure, not an operator-selected profile.
            layer: "tls_shared",
            side: "both",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("PERSIST"),
                    &[ProfileTerm("TCP")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "PROTOCOL_INSPECTION",
            layer: "security",
            side: "both",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("PROTOCOL_INSPECTION"),
                    &[ProfileTerm("TCP")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "QOE",
            layer: "acceleration",
            side: "both",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("QOE"),
                    &[ProfileTerm("HTTP")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "RADIUS",
            layer: "application",
            side: "both",
            relations: &[],
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "RADIUS_AAA",
            layer: "application",
            side: "both",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("RADIUS_AAA"),
                    &[ProfileTerm("RADIUS")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
    ]
}

fn profile_specs_6() -> Vec<ProfileSpec> {
    vec![
        ProfileSpec {
            name: "REQUESTADAPT",
            layer: "acceleration",
            side: "both",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("REQUESTADAPT"),
                    &[ProfileTerm("HTTP")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "RESPONSEADAPT",
            layer: "acceleration",
            side: "both",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("RESPONSEADAPT"),
                    &[ProfileTerm("HTTP")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "REWRITE",
            layer: "acceleration",
            side: "both",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("REWRITE"),
                    &[ProfileTerm("HTTP")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "RTSP",
            layer: "application",
            side: "both",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("RTSP"),
                    &[ProfileTerm("TCP")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "SCTP",
            layer: "transport",
            side: "both",
            relations: &[],
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "SERVERSSL",
            layer: "tls",
            side: "server",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("SERVERSSL"),
                    &[ProfileTerm("TCP")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "SIP",
            layer: "application",
            side: "both",
            relations: &[],
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "SIPROUTER",
            layer: "application",
            side: "both",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("SIPROUTER"),
                    &[ProfileTerm("SIP")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
    ]
}

fn profile_specs_7() -> Vec<ProfileSpec> {
    vec![
        ProfileSpec {
            name: "SIPSESSION",
            layer: "application",
            side: "both",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("SIPSESSION"),
                    &[ProfileTerm("SIP")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "SOCKS",
            layer: "application",
            side: "client",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("SOCKS"),
                    &[ProfileTerm("TCP")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "SSE",
            layer: "acceleration",
            side: "both",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("SSE"),
                    &[ProfileTerm("HTTP")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "SSL_PERSISTENCE",
            // Side-independent (shared) TLS layer. Stack
            // infrastructure, not an operator-selected profile.
            layer: "tls_shared",
            side: "client",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("SSL_PERSISTENCE"),
                    &[ProfileTerm("TCP")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "STREAM",
            layer: "acceleration",
            side: "both",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("STREAM"),
                    &[ProfileTerm("TCP")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "TAP",
            layer: "security",
            side: "client",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("TAP"),
                    &[ProfileTerm("TCP")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "TCP",
            layer: "transport",
            side: "both",
            relations: &[],
            ..ProfileSpec::DEFAULT
        },
    ]
}

fn profile_specs_8() -> Vec<ProfileSpec> {
    vec![
        ProfileSpec {
            name: "TDS",
            layer: "application",
            side: "both",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("TDS"),
                    &[ProfileTerm("TCP")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "UDP",
            layer: "transport",
            side: "both",
            relations: &[],
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "WEBACCELERATION",
            layer: "acceleration",
            side: "both",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("WEBACCELERATION"),
                    &[ProfileTerm("HTTP")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "WS",
            layer: "acceleration",
            side: "both",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("WS"),
                    &[ProfileTerm("HTTP")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
        ProfileSpec {
            name: "XML",
            layer: "acceleration",
            side: "both",
            relations: const {
                &[ProfileRelation::implies(
                    ProfileTerm("XML"),
                    &[ProfileTerm("HTTP")],
                )]
            },
            ..ProfileSpec::DEFAULT
        },
    ]
}
