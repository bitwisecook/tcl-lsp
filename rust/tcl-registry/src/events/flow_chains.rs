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

//! The event sequence of each modelled traffic flow (plain TCP, HTTP, TLS on either side, DNS).

use super::{FlowChain, FlowStep};

pub(super) fn flow_chains() -> Vec<FlowChain> {
    vec![
        FlowChain {
            chain_id: "plain_tcp",
            notes: "",
            description: "Plain TCP (tcp profile only)",
            profiles: &["TCP"],
            steps: plain_tcp_steps(),
        },
        FlowChain {
            chain_id: "tcp_http",
            notes: "",
            description: "TCP + HTTP",
            profiles: &["HTTP", "TCP"],
            steps: tcp_http_steps(),
        },
        FlowChain {
            chain_id: "tcp_clientssl_http",
            notes: "",
            description: "TCP + ClientSSL + HTTP (client-side TLS termination)",
            profiles: &["CLIENTSSL", "HTTP", "TCP"],
            steps: tcp_clientssl_http_steps(),
        },
        FlowChain {
            chain_id: "tcp_clientssl_serverssl_http",
            notes: "",
            description: "Full HTTPS (ClientSSL + ServerSSL + HTTP)",
            profiles: &["CLIENTSSL", "HTTP", "SERVERSSL", "TCP"],
            steps: tcp_clientssl_serverssl_http_steps(),
        },
        FlowChain {
            chain_id: "tcp_clientssl_serverssl_http_collect",
            notes: "Both HTTP_REQUEST_DATA and HTTP_RESPONSE_DATA fire because \
                    the iRule calls HTTP::collect in the respective request/response \
                    events.  Client sends a POST with body.",
            description: "Full HTTPS with HTTP::collect (request + response body)",
            profiles: &["CLIENTSSL", "HTTP", "SERVERSSL", "TCP"],
            steps: tcp_clientssl_serverssl_http_collect_steps(),
        },
        FlowChain {
            chain_id: "udp_dns",
            notes: "",
            description: "UDP + DNS",
            profiles: &["DNS", "UDP"],
            steps: udp_dns_steps(),
        },
        FlowChain {
            chain_id: "tcp_dns",
            notes: "",
            description: "TCP + DNS",
            profiles: &["DNS", "TCP"],
            steps: tcp_dns_steps(),
        },
    ]
}

fn plain_tcp_steps() -> Vec<FlowStep> {
    vec![
        FlowStep {
            event: "RULE_INIT",
            phase: "init",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "CLIENT_ACCEPTED",
            phase: "l4_client",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "CLIENT_DATA",
            phase: "l4_client",
            conditional: true,
            condition_note: "Requires TCP::collect in CLIENT_ACCEPTED",
        },
        FlowStep {
            event: "LB_SELECTED",
            phase: "lb",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "SA_PICKED",
            phase: "lb",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "SERVER_INIT",
            phase: "lb",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "SERVER_CONNECTED",
            phase: "l4_server",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "SERVER_DATA",
            phase: "l4_server",
            conditional: true,
            condition_note: "Requires TCP::collect in SERVER_CONNECTED",
        },
        FlowStep {
            event: "SERVER_CLOSED",
            phase: "l4_teardown",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "CLIENT_CLOSED",
            phase: "l4_teardown",
            conditional: false,
            condition_note: "",
        },
    ]
}

fn tcp_http_steps() -> Vec<FlowStep> {
    vec![
        FlowStep {
            event: "RULE_INIT",
            phase: "init",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "CLIENT_ACCEPTED",
            phase: "l4_client",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "HTTP_REQUEST",
            phase: "http_request",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "HTTP_REQUEST_DATA",
            phase: "http_request",
            conditional: true,
            condition_note: "Requires HTTP::collect in HTTP_REQUEST",
        },
        FlowStep {
            event: "LB_SELECTED",
            phase: "lb",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "SA_PICKED",
            phase: "lb",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "SERVER_INIT",
            phase: "lb",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "SERVER_CONNECTED",
            phase: "l4_server",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "HTTP_REQUEST_SEND",
            phase: "http_request_server",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "HTTP_REQUEST_RELEASE",
            phase: "http_request_server",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "HTTP_RESPONSE",
            phase: "http_response",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "HTTP_RESPONSE_DATA",
            phase: "http_response",
            conditional: true,
            condition_note: "Requires HTTP::collect in HTTP_RESPONSE",
        },
        FlowStep {
            event: "HTTP_RESPONSE_RELEASE",
            phase: "http_response",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "SERVER_CLOSED",
            phase: "l4_teardown",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "CLIENT_CLOSED",
            phase: "l4_teardown",
            conditional: false,
            condition_note: "",
        },
    ]
}

fn tcp_clientssl_http_steps() -> Vec<FlowStep> {
    let mut out = tcp_clientssl_http_steps_0();
    out.extend(tcp_clientssl_http_steps_1());
    out
}

fn tcp_clientssl_http_steps_0() -> Vec<FlowStep> {
    vec![
        FlowStep {
            event: "RULE_INIT",
            phase: "init",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "CLIENT_ACCEPTED",
            phase: "l4_client",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "CLIENTSSL_CLIENTHELLO",
            phase: "tls_client",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "CLIENTSSL_SERVERHELLO_SEND",
            phase: "tls_client",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "CLIENTSSL_CLIENTCERT",
            phase: "tls_client",
            conditional: true,
            condition_note: "Only with mutual TLS (client certificate required)",
        },
        FlowStep {
            event: "CLIENTSSL_HANDSHAKE",
            phase: "tls_client",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "HTTP_REQUEST",
            phase: "http_request",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "HTTP_REQUEST_DATA",
            phase: "http_request",
            conditional: true,
            condition_note: "Requires HTTP::collect in HTTP_REQUEST",
        },
        FlowStep {
            event: "LB_SELECTED",
            phase: "lb",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "SA_PICKED",
            phase: "lb",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "SERVER_INIT",
            phase: "lb",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "SERVER_CONNECTED",
            phase: "l4_server",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "HTTP_REQUEST_SEND",
            phase: "http_request_server",
            conditional: false,
            condition_note: "",
        },
    ]
}

fn tcp_clientssl_http_steps_1() -> Vec<FlowStep> {
    vec![
        FlowStep {
            event: "HTTP_REQUEST_RELEASE",
            phase: "http_request_server",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "HTTP_RESPONSE",
            phase: "http_response",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "HTTP_RESPONSE_DATA",
            phase: "http_response",
            conditional: true,
            condition_note: "Requires HTTP::collect in HTTP_RESPONSE",
        },
        FlowStep {
            event: "HTTP_RESPONSE_RELEASE",
            phase: "http_response",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "SERVER_CLOSED",
            phase: "l4_teardown",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "CLIENT_CLOSED",
            phase: "l4_teardown",
            conditional: false,
            condition_note: "",
        },
    ]
}

fn tcp_clientssl_serverssl_http_steps() -> Vec<FlowStep> {
    let mut out = tcp_clientssl_serverssl_http_steps_0();
    out.extend(tcp_clientssl_serverssl_http_steps_1());
    out
}

fn tcp_clientssl_serverssl_http_steps_0() -> Vec<FlowStep> {
    vec![
        FlowStep {
            event: "RULE_INIT",
            phase: "init",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "CLIENT_ACCEPTED",
            phase: "l4_client",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "CLIENTSSL_CLIENTHELLO",
            phase: "tls_client",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "CLIENTSSL_SERVERHELLO_SEND",
            phase: "tls_client",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "CLIENTSSL_CLIENTCERT",
            phase: "tls_client",
            conditional: true,
            condition_note: "Only with mutual TLS (client certificate required)",
        },
        FlowStep {
            event: "CLIENTSSL_HANDSHAKE",
            phase: "tls_client",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "HTTP_REQUEST",
            phase: "http_request",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "HTTP_REQUEST_DATA",
            phase: "http_request",
            conditional: true,
            condition_note: "Requires HTTP::collect in HTTP_REQUEST",
        },
        FlowStep {
            event: "LB_SELECTED",
            phase: "lb",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "SA_PICKED",
            phase: "lb",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "SERVER_INIT",
            phase: "lb",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "SERVER_CONNECTED",
            phase: "l4_server",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "SERVERSSL_CLIENTHELLO_SEND",
            phase: "tls_server",
            conditional: false,
            condition_note: "",
        },
    ]
}

fn tcp_clientssl_serverssl_http_steps_1() -> Vec<FlowStep> {
    vec![
        FlowStep {
            event: "SERVERSSL_SERVERHELLO",
            phase: "tls_server",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "SERVERSSL_SERVERCERT",
            phase: "tls_server",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "SERVERSSL_HANDSHAKE",
            phase: "tls_server",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "HTTP_REQUEST_SEND",
            phase: "http_request_server",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "HTTP_REQUEST_RELEASE",
            phase: "http_request_server",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "HTTP_RESPONSE",
            phase: "http_response",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "HTTP_RESPONSE_DATA",
            phase: "http_response",
            conditional: true,
            condition_note: "Requires HTTP::collect in HTTP_RESPONSE",
        },
        FlowStep {
            event: "HTTP_RESPONSE_RELEASE",
            phase: "http_response",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "SERVER_CLOSED",
            phase: "l4_teardown",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "CLIENT_CLOSED",
            phase: "l4_teardown",
            conditional: false,
            condition_note: "",
        },
    ]
}

fn tcp_clientssl_serverssl_http_collect_steps() -> Vec<FlowStep> {
    let mut out = tcp_clientssl_serverssl_http_collect_steps_0();
    out.extend(tcp_clientssl_serverssl_http_collect_steps_1());
    out
}

fn tcp_clientssl_serverssl_http_collect_steps_0() -> Vec<FlowStep> {
    vec![
        FlowStep {
            event: "RULE_INIT",
            phase: "init",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "CLIENT_ACCEPTED",
            phase: "l4_client",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "CLIENTSSL_CLIENTHELLO",
            phase: "tls_client",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "CLIENTSSL_SERVERHELLO_SEND",
            phase: "tls_client",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "CLIENTSSL_HANDSHAKE",
            phase: "tls_client",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "HTTP_REQUEST",
            phase: "http_request",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "HTTP_REQUEST_DATA",
            phase: "http_request",
            conditional: false,
            condition_note: "HTTP::collect called in HTTP_REQUEST",
        },
        FlowStep {
            event: "LB_SELECTED",
            phase: "lb",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "SA_PICKED",
            phase: "lb",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "SERVER_INIT",
            phase: "lb",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "SERVER_CONNECTED",
            phase: "l4_server",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "SERVERSSL_CLIENTHELLO_SEND",
            phase: "tls_server",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "SERVERSSL_SERVERHELLO",
            phase: "tls_server",
            conditional: false,
            condition_note: "",
        },
    ]
}

fn tcp_clientssl_serverssl_http_collect_steps_1() -> Vec<FlowStep> {
    vec![
        FlowStep {
            event: "SERVERSSL_SERVERCERT",
            phase: "tls_server",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "SERVERSSL_HANDSHAKE",
            phase: "tls_server",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "HTTP_REQUEST_SEND",
            phase: "http_request_server",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "HTTP_REQUEST_RELEASE",
            phase: "http_request_server",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "HTTP_RESPONSE",
            phase: "http_response",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "HTTP_RESPONSE_DATA",
            phase: "http_response",
            conditional: false,
            condition_note: "HTTP::collect called in HTTP_RESPONSE",
        },
        FlowStep {
            event: "HTTP_RESPONSE_RELEASE",
            phase: "http_response",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "SERVER_CLOSED",
            phase: "l4_teardown",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "CLIENT_CLOSED",
            phase: "l4_teardown",
            conditional: false,
            condition_note: "",
        },
    ]
}

fn udp_dns_steps() -> Vec<FlowStep> {
    vec![
        FlowStep {
            event: "RULE_INIT",
            phase: "init",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "DNS_REQUEST",
            phase: "dns_request",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "LB_SELECTED",
            phase: "lb",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "DNS_RESPONSE",
            phase: "dns_response",
            conditional: false,
            condition_note: "",
        },
    ]
}

fn tcp_dns_steps() -> Vec<FlowStep> {
    vec![
        FlowStep {
            event: "RULE_INIT",
            phase: "init",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "CLIENT_ACCEPTED",
            phase: "l4_client",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "CLIENT_DATA",
            phase: "l4_client",
            conditional: true,
            condition_note: "Requires TCP::collect in CLIENT_ACCEPTED",
        },
        FlowStep {
            event: "DNS_REQUEST",
            phase: "dns_request",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "LB_SELECTED",
            phase: "lb",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "SA_PICKED",
            phase: "lb",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "SERVER_INIT",
            phase: "lb",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "SERVER_CONNECTED",
            phase: "l4_server",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "DNS_RESPONSE",
            phase: "dns_response",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "SERVER_CLOSED",
            phase: "l4_teardown",
            conditional: false,
            condition_note: "",
        },
        FlowStep {
            event: "CLIENT_CLOSED",
            phase: "l4_teardown",
            conditional: false,
            condition_note: "",
        },
    ]
}
