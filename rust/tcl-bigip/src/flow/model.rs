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

//! Data model for pcap flow extraction (`Flow` / `Connection` / `Session`).
//!
//! The per-session
//! explanation types (`SessionExplain` / `ExplainFlowReport`) live with the
//! driver in `f5-cli`, since they depend on the policy-evaluation model.

/// One unique unidirectional L3/L4 flow extracted from a capture.
///
/// Flows are keyed by exact 5-tuple `(src_ip, src_port, dst_ip, dst_port,
/// proto)` — the two halves of a TCP connection occupy two flow entries that
/// [`super::sessions::pair_connections`] later joins into a single
/// [`Connection`].
// The TCP-flag / TLS / HTTP booleans are independent per-flow observations
// pulled straight from the capture, not a state machine to encode as enums.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug, Default)]
pub struct Flow {
    /// Source IP (ipaddress-canonical form).
    pub src_ip: String,
    /// Source L4 port (0 for non-TCP/UDP).
    pub src_port: u16,
    /// Destination IP (ipaddress-canonical form).
    pub dst_ip: String,
    /// Destination L4 port (0 for non-TCP/UDP).
    pub dst_port: u16,
    /// IP protocol number (6 TCP, 17 UDP, 1 ICMP, 58 `ICMPv6`).
    pub proto: u8,
    /// Packet count seen on this direction.
    pub packets: u64,
    /// Total captured bytes seen on this direction.
    pub bytes_total: u64,
    /// A bare SYN was seen.
    pub tcp_syn: bool,
    /// A SYN+ACK was seen.
    pub tcp_synack: bool,
    /// A FIN was seen.
    pub tcp_fin: bool,
    /// A RST was seen.
    pub tcp_rst: bool,
    /// Number of RSTs seen.
    pub tcp_rst_count: u64,
    /// Bytes seen on this side before the first RST.
    pub tcp_rst_after_bytes: u64,
    /// A TLS `ClientHello` record was seen.
    pub tls_clienthello: bool,
    /// SNI from the `ClientHello`.
    pub tls_sni: String,
    /// Legacy version inside `ClientHello`.
    pub tls_version: String,
    /// Version negotiated (from tshark).
    pub tls_chosen_version: String,
    /// Ciphersuite chosen (from tshark).
    pub tls_chosen_cipher: String,
    /// ALPN protocol selected.
    pub tls_alpn: String,
    /// A TLS alert record was seen.
    pub tls_alert_seen: bool,
    /// Decoded TLS alert description (from tshark).
    pub tls_alert_desc: String,
    /// An HTTP request line was seen.
    pub http_request_seen: bool,
    /// HTTP request method.
    pub http_method: String,
    /// HTTP `Host` header.
    pub http_host: String,
    /// HTTP request URI.
    pub http_uri: String,
    /// URI minus the query string.
    pub http_path: String,
    /// Query string portion of the URI.
    pub http_query: String,
    /// e.g. `HTTP/1.1`.
    pub http_request_version: String,
    /// HTTP `User-Agent` header.
    pub http_user_agent: String,
    /// HTTP `Cookie` header.
    pub http_cookie: String,
    /// HTTP `Referer` header.
    pub http_referer: String,
    /// HTTP request `Content-Type` header.
    pub http_request_content_type: String,
    /// HTTP request `Content-Length` header.
    pub http_request_content_length: String,
    /// Lowercase-keyed request headers, in first-seen (insertion) order.
    pub http_request_headers: Vec<(String, String)>,
    /// An HTTP response status line was seen.
    pub http_response_seen: bool,
    /// HTTP response status code.
    pub http_response_code: String,
    /// HTTP response reason phrase.
    pub http_response_phrase: String,
    /// HTTP response `Content-Type` header.
    pub http_response_content_type: String,
    /// HTTP response `Content-Length` header.
    pub http_response_content_length: String,
    /// Lowercase-keyed response headers, in first-seen (insertion) order.
    pub http_response_headers: Vec<(String, String)>,
    /// F5 reset-cause strings pulled from LOW/MED trailer TLVs.
    pub f5_reset_causes: Vec<String>,
    /// TLS server-certificate subject (from tshark).
    pub tls_cert_subject: String,
    /// TLS server-certificate issuer (from tshark).
    pub tls_cert_issuer: String,
    /// Curves / groups.
    pub tls_supported_groups: String,
    /// Offered signature algorithms (from tshark).
    pub tls_signature_algos: String,
    /// F5 trailer peer-side remote IP (populated for `-i <vlan>:np` captures).
    pub peer_remote_ip: String,
    /// F5 trailer peer-side remote port.
    pub peer_remote_port: u16,
    /// F5 trailer peer-side local IP.
    pub peer_local_ip: String,
    /// F5 trailer peer-side local port.
    pub peer_local_port: u16,
}

/// 5-tuple key used to index flows.
pub type FlowKey = (String, u16, String, u16, u8);

impl Flow {
    /// Construct a fresh flow keyed by its 5-tuple, all counters zeroed.
    #[must_use]
    pub fn new(src_ip: String, src_port: u16, dst_ip: String, dst_port: u16, proto: u8) -> Self {
        Self {
            src_ip,
            src_port,
            dst_ip,
            dst_port,
            proto,
            ..Self::default()
        }
    }

    /// The canonical `(src_ip, src_port, dst_ip, dst_port, proto)` key.
    #[must_use]
    pub fn key(&self) -> FlowKey {
        (
            self.src_ip.clone(),
            self.src_port,
            self.dst_ip.clone(),
            self.dst_port,
            self.proto,
        )
    }

    /// Lowercase protocol name (`tcp`/`udp`/`icmp`/`icmpv6`) or the number.
    #[must_use]
    pub fn proto_name(&self) -> String {
        match self.proto {
            6 => "tcp".to_owned(),
            17 => "udp".to_owned(),
            1 => "icmp".to_owned(),
            58 => "icmpv6".to_owned(),
            other => other.to_string(),
        }
    }

    /// Look up a request header by (already lowercase) name.
    #[must_use]
    pub fn request_header(&self, name: &str) -> Option<&str> {
        self.http_request_headers
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    /// Look up a response header by (already lowercase) name.
    #[must_use]
    pub fn response_header(&self, name: &str) -> Option<&str> {
        self.http_response_headers
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }
}

/// A bidirectional TCP/UDP conversation, formed by pairing two flows.
///
/// The `client` side is the SYN-bearer (or, for non-TCP and SYN-less captures,
/// the first-seen direction). `server` is the reverse 5-tuple if the response
/// side appears in the capture, otherwise `None`.
#[derive(Clone, Debug)]
pub struct Connection {
    /// The initiating (SYN-bearer / first-seen) direction.
    pub client: Flow,
    /// The reverse direction, if it appeared in the capture.
    pub server: Option<Flow>,
}

impl Connection {
    /// IP protocol number of the client side.
    #[must_use]
    pub fn proto(&self) -> u8 {
        self.client.proto
    }

    /// Lowercase protocol name of the client side.
    #[must_use]
    pub fn proto_name(&self) -> String {
        self.client.proto_name()
    }

    /// Which side(s) sent a RST: `both`/`client`/`server`/`""`.
    #[must_use]
    pub fn reset_side(&self) -> &'static str {
        let server_rst = self.server.as_ref().is_some_and(|s| s.tcp_rst);
        if self.client.tcp_rst && server_rst {
            "both"
        } else if self.client.tcp_rst {
            "client"
        } else if server_rst {
            "server"
        } else {
            ""
        }
    }

    /// De-duplicated F5 reset causes across both directions, order preserved.
    #[must_use]
    pub fn reset_causes(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        out.extend(self.client.f5_reset_causes.iter().cloned());
        if let Some(server) = &self.server {
            out.extend(server.f5_reset_causes.iter().cloned());
        }
        // De-dup while preserving order.
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        out.retain(|c| seen.insert(c.clone()));
        out
    }

    /// One-line operator summary of the conversation (verbatim report text).
    #[must_use]
    pub fn summary(&self) -> String {
        use std::fmt::Write as _;
        let c = &self.client;
        let mut head = format!(
            "{}:{} <-> {}:{} ({})",
            c.src_ip,
            c.src_port,
            c.dst_ip,
            c.dst_port,
            self.proto_name()
        );
        let c_pkts = c.packets;
        let s_pkts = self.server.as_ref().map_or(0, |s| s.packets);
        let _ = write!(head, " | client\u{2192} {c_pkts} pkt");
        if self.server.is_some() {
            let _ = write!(head, ", server\u{2192} {s_pkts} pkt");
        }
        if !c.tls_sni.is_empty() {
            let _ = write!(head, " | SNI={}", c.tls_sni);
        }
        if !c.tls_chosen_version.is_empty() || !c.tls_version.is_empty() {
            let v = if c.tls_chosen_version.is_empty() {
                &c.tls_version
            } else {
                &c.tls_chosen_version
            };
            let _ = write!(head, " | TLS={v}");
        }
        if !c.http_method.is_empty() {
            let _ = write!(head, " | HTTP {} {}", c.http_method, c.http_uri);
        }
        if let Some(server) = &self.server
            && !server.http_response_code.is_empty()
        {
            let _ = write!(head, " -> {}", server.http_response_code);
        }
        let rst = self.reset_side();
        if !rst.is_empty() {
            let _ = write!(head, " | RST({rst})");
        }
        head
    }
}

/// A logical BIG-IP-mediated conversation: client↔VIP plus VIP↔server.
#[derive(Clone, Debug)]
pub struct Session {
    /// The client↔VIP connection.
    pub front: Connection,
    /// The TMM↔pool-member connection (`:np` captures only).
    pub back: Option<Connection>,
}

impl Session {
    /// IP protocol number of the front connection.
    #[must_use]
    pub fn proto(&self) -> u8 {
        self.front.proto()
    }

    /// Lowercase protocol name of the front connection.
    #[must_use]
    pub fn proto_name(&self) -> String {
        self.front.proto_name()
    }

    /// Human-readable description of which side reset, front or server-side.
    #[must_use]
    pub fn reset_side(&self) -> String {
        if let Some(back) = &self.back {
            let bs = back.reset_side();
            if !bs.is_empty() {
                return format!("server-side ({bs})");
            }
        }
        self.front.reset_side().to_owned()
    }

    /// De-duplicated reset causes across front and back connections.
    #[must_use]
    pub fn reset_causes(&self) -> Vec<String> {
        let mut causes = self.front.reset_causes();
        if let Some(back) = &self.back {
            for c in back.reset_causes() {
                if !causes.contains(&c) {
                    causes.push(c);
                }
            }
        }
        causes
    }
}
