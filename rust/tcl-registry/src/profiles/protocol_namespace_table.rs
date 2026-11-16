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

//! The table of protocol command namespaces and the profiles each one needs.

use super::ProtocolNamespaceSpec;

pub(super) fn protocol_namespace_specs() -> Vec<ProtocolNamespaceSpec> {
    let mut out = protocol_namespace_specs_0();
    out.extend(protocol_namespace_specs_1());
    out.extend(protocol_namespace_specs_2());
    out.extend(protocol_namespace_specs_3());
    out.extend(protocol_namespace_specs_4());
    out.extend(protocol_namespace_specs_5());
    out.extend(protocol_namespace_specs_6());
    out.extend(protocol_namespace_specs_7());
    out.extend(protocol_namespace_specs_8());
    out.extend(protocol_namespace_specs_9());
    out
}

fn protocol_namespace_specs_0() -> Vec<ProtocolNamespaceSpec> {
    vec![
        ProtocolNamespaceSpec {
            prefix: "AAA",
            profiles: &[],
            layer: "security",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "ACCESS",
            profiles: &["ACCESS"],
            layer: "security",
            side: "client",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "ACCESS2",
            profiles: &["ACCESS"],
            layer: "security",
            side: "client",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "ACL",
            profiles: &[],
            layer: "security",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "ADAPT",
            profiles: &["HTTP", "REQUESTADAPT", "RESPONSEADAPT"],
            layer: "acceleration",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "AES",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "AM",
            profiles: &[],
            layer: "acceleration",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "ANTIFRAUD",
            profiles: &["ANTIFRAUD"],
            layer: "security",
            side: "client",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "ASM",
            profiles: &["ASM"],
            layer: "security",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "ASN1",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "AUTH",
            profiles: &["AUTH"],
            layer: "security",
            side: "client",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "AVR",
            profiles: &["AVR"],
            layer: "acceleration",
            side: "both",
            side_selectable: false,
        },
    ]
}

fn protocol_namespace_specs_1() -> Vec<ProtocolNamespaceSpec> {
    vec![
        ProtocolNamespaceSpec {
            prefix: "BIGPROTO",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "BIGTCP",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "BOTDEFENSE",
            profiles: &["BOTDEFENSE"],
            layer: "security",
            side: "client",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "BWC",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "CACHE",
            profiles: &["CACHE", "WEBACCELERATION"],
            layer: "acceleration",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "CATEGORY",
            profiles: &["CATEGORY"],
            layer: "acceleration",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "CLASSIFICATION",
            profiles: &["CLASSIFICATION"],
            layer: "acceleration",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "CLASSIFY",
            profiles: &["FASTHTTP"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "COMPRESS",
            profiles: &["FASTHTTP", "HTTP"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "CONNECTOR",
            profiles: &["CONNECTOR"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "CRYPTO",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "DATAGRAM",
            profiles: &["DATAGRAM"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
    ]
}

fn protocol_namespace_specs_2() -> Vec<ProtocolNamespaceSpec> {
    vec![
        ProtocolNamespaceSpec {
            prefix: "DECOMPRESS",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "DEMANGLE",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "DHCP",
            profiles: &[],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "DHCPv4",
            profiles: &[],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "DHCPv6",
            profiles: &[],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "DIAG",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "DIAMETER",
            profiles: &["DIAMETER", "DIAMETERSESSION", "DIAMETER_ENDPOINT"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "DNS",
            profiles: &["DNS"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "DNSMSG",
            profiles: &[],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "DOSL7",
            profiles: &["DOSL7"],
            layer: "security",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "DSLITE",
            profiles: &[],
            layer: "application",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "ECA",
            profiles: &["ECA"],
            layer: "security",
            side: "client",
            side_selectable: false,
        },
    ]
}

fn protocol_namespace_specs_3() -> Vec<ProtocolNamespaceSpec> {
    vec![
        ProtocolNamespaceSpec {
            prefix: "FIX",
            profiles: &["FIX"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "FLOW",
            profiles: &["FLOW"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "FLOWTABLE",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "FTP",
            profiles: &[],
            layer: "application",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "GENERICMESSAGE",
            profiles: &["GENERICMSG"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "GTP",
            profiles: &["GTP"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "HA",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "HSL",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "HTML",
            profiles: &["HTML"],
            layer: "acceleration",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "HTTP",
            profiles: &["FASTHTTP", "HTTP", "HTTP_PROXY_CONNECT"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "HTTP2",
            profiles: &["HTTP2"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "HTTPLOG",
            profiles: &["HTTP"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
    ]
}

fn protocol_namespace_specs_4() -> Vec<ProtocolNamespaceSpec> {
    vec![
        ProtocolNamespaceSpec {
            prefix: "ICAP",
            profiles: &["ICAP"],
            layer: "acceleration",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "IKE",
            profiles: &[],
            layer: "application",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "ILX",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "IMAP",
            profiles: &[],
            layer: "application",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "IP",
            profiles: &[],
            layer: "transport",
            side: "both",
            side_selectable: true,
        },
        ProtocolNamespaceSpec {
            prefix: "IPFIX",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "ISESSION",
            profiles: &[],
            layer: "application",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "ISTATS",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "IVS_ENTRY",
            profiles: &["IVS_ENTRY"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "JSON",
            profiles: &["JSON"],
            layer: "acceleration",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "L7CHECK",
            profiles: &["L7CHECK"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "LB",
            profiles: &[],
            layer: "load_balance",
            side: "global",
            side_selectable: false,
        },
    ]
}

fn protocol_namespace_specs_5() -> Vec<ProtocolNamespaceSpec> {
    vec![
        ProtocolNamespaceSpec {
            prefix: "LDAP",
            profiles: &[],
            layer: "application",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "LINE",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "LINK",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "LSN",
            profiles: &["LSN"],
            layer: "application",
            side: "client",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "MESSAGE",
            profiles: &["MR"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "MQTT",
            profiles: &["MQTT"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "MR",
            profiles: &["MR"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "NAME",
            profiles: &["NAME"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "NSH",
            profiles: &[],
            layer: "application",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "NTLM",
            profiles: &[],
            layer: "application",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "OFFBOX",
            profiles: &[],
            layer: "application",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "ONECONNECT",
            profiles: &[],
            layer: "application",
            side: "global",
            side_selectable: false,
        },
    ]
}

fn protocol_namespace_specs_6() -> Vec<ProtocolNamespaceSpec> {
    vec![
        ProtocolNamespaceSpec {
            prefix: "PCP",
            profiles: &["PCP"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "PEM",
            profiles: &["PEM"],
            layer: "security",
            side: "client",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "PLUGIN",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "POLICY",
            profiles: &[],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "POP3",
            profiles: &[],
            layer: "application",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "PROFILE",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "PROTOCOL_INSPECTION",
            profiles: &["IPS", "PROTOCOL_INSPECTION"],
            layer: "security",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "PSC",
            profiles: &[],
            layer: "security",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "PSM",
            profiles: &["HTTP"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "QOE",
            profiles: &["QOE"],
            layer: "acceleration",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "RADIUS",
            profiles: &["RADIUS", "RADIUS_AAA"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "RESOLV",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
    ]
}

fn protocol_namespace_specs_7() -> Vec<ProtocolNamespaceSpec> {
    vec![
        ProtocolNamespaceSpec {
            prefix: "RESOLVER",
            profiles: &[],
            layer: "application",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "REST",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "REWRITE",
            profiles: &["REWRITE"],
            layer: "acceleration",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "ROUTE",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "RTSP",
            profiles: &["RTSP"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "SCTP",
            profiles: &["SCTP"],
            layer: "transport",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "SDP",
            profiles: &[],
            layer: "application",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "SIP",
            profiles: &["SIP", "SIPROUTER", "SIPSESSION"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "SIPALG",
            profiles: &["MR", "SIP"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "SMTPS",
            profiles: &[],
            layer: "application",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "SOCKS",
            profiles: &["SOCKS"],
            layer: "application",
            side: "client",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "SSE",
            profiles: &["SSE"],
            layer: "acceleration",
            side: "both",
            side_selectable: false,
        },
    ]
}

fn protocol_namespace_specs_8() -> Vec<ProtocolNamespaceSpec> {
    vec![
        ProtocolNamespaceSpec {
            prefix: "SSL",
            profiles: &["CLIENTSSL", "PERSIST", "SERVERSSL", "SSL_PERSISTENCE"],
            layer: "tls",
            side: "both",
            side_selectable: true,
        },
        ProtocolNamespaceSpec {
            prefix: "STATS",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "STREAM",
            profiles: &["STREAM"],
            layer: "acceleration",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "TAP",
            profiles: &["TAP"],
            layer: "security",
            side: "client",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "TCP",
            profiles: &["TCP"],
            layer: "transport",
            side: "both",
            side_selectable: true,
        },
        ProtocolNamespaceSpec {
            prefix: "TDS",
            profiles: &["MSSQL", "TDS"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "TMM",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "UDP",
            profiles: &["UDP"],
            layer: "transport",
            side: "both",
            side_selectable: true,
        },
        ProtocolNamespaceSpec {
            prefix: "URI",
            profiles: &["HTTP"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "VALIDATE",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "VDI",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "WAM",
            profiles: &["HTTP"],
            layer: "application",
            side: "both",
            side_selectable: false,
        },
    ]
}

fn protocol_namespace_specs_9() -> Vec<ProtocolNamespaceSpec> {
    vec![
        ProtocolNamespaceSpec {
            prefix: "WEBSSO",
            profiles: &["ACCESS", "HTTP"],
            layer: "security",
            side: "client",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "WS",
            profiles: &["WS"],
            layer: "acceleration",
            side: "both",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "X509",
            profiles: &[],
            layer: "utility",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "XLAT",
            profiles: &[],
            layer: "application",
            side: "global",
            side_selectable: false,
        },
        ProtocolNamespaceSpec {
            prefix: "XML",
            profiles: &["XML"],
            layer: "acceleration",
            side: "both",
            side_selectable: false,
        },
    ]
}
