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

//! The master firing order of events and which fire once per connection or once per request.

use super::OrderEntry;
use rustc_hash::FxHashSet;

pub(super) fn master_order() -> Vec<OrderEntry> {
    let mut out = master_order_0();
    out.extend(master_order_1());
    out.extend(master_order_2());
    out.extend(master_order_3());
    out
}

fn master_order_0() -> Vec<OrderEntry> {
    vec![
        OrderEntry {
            event: "RULE_INIT",
            profile_gates: &[],
        },
        OrderEntry {
            event: "FLOW_INIT",
            profile_gates: &["FLOW"],
        },
        OrderEntry {
            event: "CLIENT_ACCEPTED",
            profile_gates: &[],
        },
        OrderEntry {
            event: "CLIENT_DATA",
            profile_gates: &[],
        },
        OrderEntry {
            event: "CLIENTSSL_CLIENTHELLO",
            profile_gates: &["CLIENTSSL", "PERSIST"],
        },
        OrderEntry {
            event: "CLIENTSSL_SERVERHELLO_SEND",
            profile_gates: &["CLIENTSSL"],
        },
        OrderEntry {
            event: "CLIENTSSL_CLIENTCERT",
            profile_gates: &["CLIENTSSL", "PERSIST"],
        },
        OrderEntry {
            event: "CLIENTSSL_HANDSHAKE",
            profile_gates: &["CLIENTSSL"],
        },
        OrderEntry {
            event: "CLIENTSSL_DATA",
            profile_gates: &["CLIENTSSL"],
        },
        OrderEntry {
            event: "CLIENTSSL_PASSTHROUGH",
            profile_gates: &["CLIENTSSL"],
        },
        OrderEntry {
            event: "HTTP_REQUEST",
            profile_gates: &["FASTHTTP", "HTTP"],
        },
        OrderEntry {
            event: "HTTP_REQUEST_DATA",
            profile_gates: &["HTTP"],
        },
        OrderEntry {
            event: "HTTP_PROXY_REQUEST",
            profile_gates: &["HTTP"],
        },
        OrderEntry {
            event: "AUTH_RESULT",
            profile_gates: &["AUTH"],
        },
        OrderEntry {
            event: "AUTH_SUCCESS",
            profile_gates: &["AUTH"],
        },
        OrderEntry {
            event: "AUTH_FAILURE",
            profile_gates: &["AUTH"],
        },
        OrderEntry {
            event: "AUTH_ERROR",
            profile_gates: &["AUTH"],
        },
        OrderEntry {
            event: "AUTH_WANTCREDENTIAL",
            profile_gates: &["AUTH"],
        },
        OrderEntry {
            event: "ACCESS_SESSION_STARTED",
            profile_gates: &["ACCESS"],
        },
        OrderEntry {
            event: "ACCESS_POLICY_AGENT_EVENT",
            profile_gates: &["ACCESS"],
        },
        OrderEntry {
            event: "ACCESS_POLICY_COMPLETED",
            profile_gates: &["ACCESS"],
        },
    ]
}

fn master_order_1() -> Vec<OrderEntry> {
    vec![
        OrderEntry {
            event: "CLASSIFICATION_DETECTED",
            profile_gates: &["CLASSIFICATION"],
        },
        OrderEntry {
            event: "CATEGORY_MATCHED",
            profile_gates: &["ACCESS", "CATEGORY", "HTTP"],
        },
        OrderEntry {
            event: "HTTP_CLASS_SELECTED",
            profile_gates: &["HTTP"],
        },
        OrderEntry {
            event: "HTTP_CLASS_FAILED",
            profile_gates: &["HTTP"],
        },
        OrderEntry {
            event: "CACHE_REQUEST",
            profile_gates: &["CACHE", "WEBACCELERATION"],
        },
        OrderEntry {
            event: "CACHE_RESPONSE",
            profile_gates: &["CACHE", "WEBACCELERATION"],
        },
        OrderEntry {
            event: "IN_DOSL7_ATTACK",
            profile_gates: &["DOSL7", "FASTHTTP", "HTTP"],
        },
        OrderEntry {
            event: "ASM_REQUEST_DONE",
            profile_gates: &["ASM"],
        },
        OrderEntry {
            event: "ASM_REQUEST_VIOLATION",
            profile_gates: &["ASM"],
        },
        OrderEntry {
            event: "ASM_REQUEST_BLOCKING",
            profile_gates: &["ASM"],
        },
        OrderEntry {
            event: "DNS_REQUEST",
            profile_gates: &["DNS"],
        },
        OrderEntry {
            event: "SIP_REQUEST",
            profile_gates: &["SIP"],
        },
        OrderEntry {
            event: "PERSIST_DOWN",
            profile_gates: &[],
        },
        OrderEntry {
            event: "LB_SELECTED",
            profile_gates: &[],
        },
        OrderEntry {
            event: "LB_FAILED",
            profile_gates: &[],
        },
        OrderEntry {
            event: "LB_QUEUED",
            profile_gates: &[],
        },
        OrderEntry {
            event: "SA_PICKED",
            profile_gates: &[],
        },
        OrderEntry {
            event: "SERVER_INIT",
            profile_gates: &[],
        },
        OrderEntry {
            event: "ACCESS_ACL_ALLOWED",
            profile_gates: &["ACCESS"],
        },
        OrderEntry {
            event: "ACCESS_ACL_DENIED",
            profile_gates: &["ACCESS"],
        },
        OrderEntry {
            event: "ACCESS_PER_REQUEST_AGENT_EVENT",
            profile_gates: &["ACCESS"],
        },
    ]
}

fn master_order_2() -> Vec<OrderEntry> {
    vec![
        OrderEntry {
            event: "REWRITE_REQUEST_DONE",
            profile_gates: &["HTTP", "REWRITE"],
        },
        OrderEntry {
            event: "SERVER_CONNECTED",
            profile_gates: &[],
        },
        OrderEntry {
            event: "SERVERSSL_CLIENTHELLO_SEND",
            profile_gates: &["SERVERSSL"],
        },
        OrderEntry {
            event: "SERVERSSL_SERVERHELLO",
            profile_gates: &["PERSIST", "SERVERSSL"],
        },
        OrderEntry {
            event: "SERVERSSL_SERVERCERT",
            profile_gates: &["SERVERSSL"],
        },
        OrderEntry {
            event: "SERVERSSL_HANDSHAKE",
            profile_gates: &["PERSIST", "SERVERSSL"],
        },
        OrderEntry {
            event: "SERVERSSL_DATA",
            profile_gates: &["SERVERSSL"],
        },
        OrderEntry {
            event: "HTTP_REQUEST_SEND",
            profile_gates: &["HTTP"],
        },
        OrderEntry {
            event: "HTTP_REQUEST_RELEASE",
            profile_gates: &["HTTP"],
        },
        OrderEntry {
            event: "SERVER_DATA",
            profile_gates: &[],
        },
        OrderEntry {
            event: "DNS_RESPONSE",
            profile_gates: &["DNS"],
        },
        OrderEntry {
            event: "SIP_REQUEST_SEND",
            profile_gates: &["SIP"],
        },
        OrderEntry {
            event: "SIP_RESPONSE",
            profile_gates: &["SIP"],
        },
        OrderEntry {
            event: "SIP_RESPONSE_SEND",
            profile_gates: &["SIP"],
        },
        OrderEntry {
            event: "HTTP_RESPONSE",
            profile_gates: &["FASTHTTP", "HTTP"],
        },
        OrderEntry {
            event: "HTTP_RESPONSE_DATA",
            profile_gates: &["HTTP"],
        },
        OrderEntry {
            event: "HTTP_RESPONSE_CONTINUE",
            profile_gates: &["HTTP"],
        },
        OrderEntry {
            event: "ASM_RESPONSE_VIOLATION",
            profile_gates: &["ASM", "FASTHTTP", "HTTP"],
        },
        OrderEntry {
            event: "ASM_RESPONSE_LOGIN",
            profile_gates: &["ASM", "FASTHTTP", "HTTP"],
        },
        OrderEntry {
            event: "BOTDEFENSE_REQUEST",
            profile_gates: &["BOTDEFENSE"],
        },
        OrderEntry {
            event: "BOTDEFENSE_ACTION",
            profile_gates: &["BOTDEFENSE"],
        },
    ]
}

fn master_order_3() -> Vec<OrderEntry> {
    vec![
        OrderEntry {
            event: "CACHE_UPDATE",
            profile_gates: &["CACHE", "WEBACCELERATION"],
        },
        OrderEntry {
            event: "STREAM_MATCHED",
            profile_gates: &["STREAM"],
        },
        OrderEntry {
            event: "HTML_TAG_MATCHED",
            profile_gates: &["HTML"],
        },
        OrderEntry {
            event: "HTML_COMMENT_MATCHED",
            profile_gates: &["HTML"],
        },
        OrderEntry {
            event: "REWRITE_RESPONSE_DONE",
            profile_gates: &["HTTP", "REWRITE"],
        },
        OrderEntry {
            event: "HTTP_RESPONSE_RELEASE",
            profile_gates: &["HTTP"],
        },
        OrderEntry {
            event: "HTTP_DISABLED",
            profile_gates: &["HTTP"],
        },
        OrderEntry {
            event: "HTTP_REJECT",
            profile_gates: &["HTTP"],
        },
        OrderEntry {
            event: "SERVER_CLOSED",
            profile_gates: &[],
        },
        OrderEntry {
            event: "CLIENT_CLOSED",
            profile_gates: &[],
        },
        OrderEntry {
            event: "ACCESS_SESSION_CLOSED",
            profile_gates: &["ACCESS"],
        },
    ]
}

pub(super) fn once_per_connection() -> FxHashSet<&'static str> {
    [
        "ACCESS_POLICY_AGENT_EVENT",
        "ACCESS_POLICY_COMPLETED",
        "ACCESS_SESSION_CLOSED",
        "ACCESS_SESSION_STARTED",
        "CLIENTSSL_CLIENTCERT",
        "CLIENTSSL_CLIENTHELLO",
        "CLIENTSSL_HANDSHAKE",
        "CLIENTSSL_PASSTHROUGH",
        "CLIENTSSL_SERVERHELLO_SEND",
        "CLIENT_ACCEPTED",
        "CLIENT_CLOSED",
        "FLOW_INIT",
        "RULE_INIT",
    ]
    .into_iter()
    .collect()
}

pub(super) fn per_request() -> FxHashSet<&'static str> {
    [
        "ACCESS_ACL_ALLOWED",
        "ACCESS_ACL_DENIED",
        "ACCESS_PER_REQUEST_AGENT_EVENT",
        "ASM_REQUEST_BLOCKING",
        "ASM_REQUEST_DONE",
        "ASM_REQUEST_VIOLATION",
        "ASM_RESPONSE_VIOLATION",
        "BOTDEFENSE_ACTION",
        "BOTDEFENSE_REQUEST",
        "CACHE_REQUEST",
        "CACHE_RESPONSE",
        "CACHE_UPDATE",
        "DNS_REQUEST",
        "DNS_RESPONSE",
        "HTTP_CLASS_FAILED",
        "HTTP_CLASS_SELECTED",
        "HTTP_PROXY_REQUEST",
        "HTTP_REQUEST",
        "HTTP_REQUEST_DATA",
        "HTTP_REQUEST_RELEASE",
        "HTTP_REQUEST_SEND",
        "HTTP_RESPONSE",
        "HTTP_RESPONSE_CONTINUE",
        "HTTP_RESPONSE_DATA",
        "HTTP_RESPONSE_RELEASE",
        "IN_DOSL7_ATTACK",
        "LB_FAILED",
        "LB_QUEUED",
        "LB_SELECTED",
        "SA_PICKED",
        "SERVERSSL_CLIENTHELLO_SEND",
        "SERVERSSL_HANDSHAKE",
        "SERVERSSL_SERVERCERT",
        "SERVERSSL_SERVERHELLO",
        "SERVER_CLOSED",
        "SERVER_CONNECTED",
        "SERVER_INIT",
        "SIP_REQUEST",
        "SIP_REQUEST_SEND",
        "SIP_RESPONSE",
        "SIP_RESPONSE_SEND",
        "STREAM_MATCHED",
    ]
    .into_iter()
    .collect()
}
