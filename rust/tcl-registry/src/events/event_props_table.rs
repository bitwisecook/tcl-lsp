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

//! The property row of every iRules event, in event-name order.

use super::EventProps;
use crate::lifecycle::Lifecycle;
use crate::side_effects::ConnectionSide;

// AUTO-GENERATED — do not edit manually

pub(crate) fn event_props_table() -> Vec<(&'static str, EventProps)> {
    let mut out = event_props_table_0();
    out.extend(event_props_table_1());
    out.extend(event_props_table_2());
    out.extend(event_props_table_3());
    out.extend(event_props_table_4());
    out.extend(event_props_table_5());
    out.extend(event_props_table_6());
    out.extend(event_props_table_7());
    out.extend(event_props_table_8());
    out.extend(event_props_table_9());
    out.extend(event_props_table_10());
    out.extend(event_props_table_11());
    out.extend(event_props_table_12());
    out.extend(event_props_table_13());
    out.extend(event_props_table_14());
    out.extend(event_props_table_15());
    out.extend(event_props_table_16());
    out.extend(event_props_table_17());
    out.extend(event_props_table_18());
    out.extend(event_props_table_19());
    out.extend(event_props_table_20());
    out
}

fn event_props_table_0() -> Vec<(&'static str, EventProps)> {
    vec![
        (
            "ACCESS2_POLICY_EXPRESSION_EVAL",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["ACCESS"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "ACCESS_ACL_ALLOWED",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["ACCESS"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "ACCESS_ACL_DENIED",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["ACCESS"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "ACCESS_PER_REQUEST_AGENT_EVENT",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["ACCESS"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "ACCESS_POLICY_AGENT_EVENT",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["ACCESS"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "ACCESS_POLICY_COMPLETED",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["ACCESS"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "ACCESS_SAML_ASSERTION",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["ACCESS"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "ACCESS_SAML_AUTHN",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["ACCESS"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "ACCESS_SAML_SLO_REQ",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["ACCESS"],
                ..EventProps::DEFAULT
            },
        ),
    ]
}

fn event_props_table_1() -> Vec<(&'static str, EventProps)> {
    vec![
        (
            "ACCESS_SAML_SLO_RESP",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["ACCESS"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "ACCESS_SESSION_CLOSED",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["ACCESS"],
                flow: false,
                ..EventProps::DEFAULT
            },
        ),
        (
            "ACCESS_SESSION_STARTED",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["ACCESS"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "ADAPT_REQUEST_HEADERS",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["REQUESTADAPT"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "ADAPT_REQUEST_RESULT",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["REQUESTADAPT"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "ADAPT_RESPONSE_HEADERS",
            EventProps {
                client_side: true,
                server_side: true,
                transport: &["tcp"],
                implied_profiles: &["RESPONSEADAPT"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "ADAPT_RESPONSE_RESULT",
            EventProps {
                client_side: true,
                server_side: true,
                transport: &["tcp"],
                implied_profiles: &["RESPONSEADAPT"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "ANTIFRAUD_ALERT",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["ANTIFRAUD"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "ANTIFRAUD_LOGIN",
            EventProps {
                client_side: true,
                server_side: true,
                transport: &["tcp"],
                implied_profiles: &["ANTIFRAUD"],
                ..EventProps::DEFAULT
            },
        ),
    ]
}

fn event_props_table_2() -> Vec<(&'static str, EventProps)> {
    vec![
        (
            "ASM_REQUEST_BLOCKING",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["ASM", "FASTHTTP", "HTTP"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "ASM_REQUEST_DONE",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["ASM", "FASTHTTP", "HTTP"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "ASM_REQUEST_VIOLATION",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["ASM", "FASTHTTP", "HTTP"],
                lifecycle: Lifecycle::deprecated_in("11.3.0"),
                ..EventProps::DEFAULT
            },
        ),
        (
            "ASM_RESPONSE_LOGIN",
            EventProps {
                client_side: true,
                server_side: true,
                transport: &["tcp"],
                implied_profiles: &["ASM", "FASTHTTP", "HTTP"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "ASM_RESPONSE_VIOLATION",
            EventProps {
                client_side: true,
                server_side: true,
                transport: &["tcp"],
                implied_profiles: &["ASM", "FASTHTTP", "HTTP"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "AUTH_ERROR",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["AUTH"],
                lifecycle: Lifecycle::introduced_in("9.0.0").deprecated_from("9.4.0"),
                ..EventProps::DEFAULT
            },
        ),
        (
            "AUTH_FAILURE",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["AUTH"],
                lifecycle: Lifecycle::introduced_in("9.0.0").deprecated_from("9.4.0"),
                ..EventProps::DEFAULT
            },
        ),
        (
            "AUTH_RESULT",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["AUTH"],
                ..EventProps::DEFAULT
            },
        ),
    ]
}

fn event_props_table_3() -> Vec<(&'static str, EventProps)> {
    vec![
        (
            "AUTH_SUCCESS",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["AUTH"],
                lifecycle: Lifecycle::introduced_in("9.0.0").deprecated_from("9.4.0"),
                ..EventProps::DEFAULT
            },
        ),
        (
            "AUTH_WANTCREDENTIAL",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["AUTH"],
                lifecycle: Lifecycle::introduced_in("9.0.0").deprecated_from("9.4.0"),
                ..EventProps::DEFAULT
            },
        ),
        (
            "AVR_CSPM_INJECTION",
            EventProps {
                client_side: true,
                server_side: true,
                transport: &["tcp"],
                implied_profiles: &["AVR"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "BOTDEFENSE_ACTION",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["BOTDEFENSE"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "BOTDEFENSE_REQUEST",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["BOTDEFENSE"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "CACHE_REQUEST",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["CACHE", "WEBACCELERATION"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "CACHE_RESPONSE",
            EventProps {
                client_side: true,
                server_side: true,
                transport: &["tcp"],
                implied_profiles: &["CACHE", "WEBACCELERATION"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "CACHE_UPDATE",
            EventProps {
                client_side: true,
                server_side: true,
                transport: &["tcp"],
                implied_profiles: &["CACHE", "WEBACCELERATION"],
                ..EventProps::DEFAULT
            },
        ),
    ]
}

fn event_props_table_4() -> Vec<(&'static str, EventProps)> {
    vec![
        (
            "CATEGORY_MATCHED",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["ACCESS", "CATEGORY", "HTTP"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "CLASSIFICATION_DETECTED",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["CLASSIFICATION"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "CLIENTSSL_CLIENTCERT",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["CLIENTSSL", "PERSIST", "SSL_PERSISTENCE"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "CLIENTSSL_CLIENTHELLO",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["CLIENTSSL", "PERSIST", "SSL_PERSISTENCE"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "CLIENTSSL_DATA",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["CLIENTSSL"],
                setup_event: Some("CLIENTSSL_HANDSHAKE"),
                data_collect_protocols: &["SSL"],
                data_collect_side: Some(ConnectionSide::Client),
                ..EventProps::DEFAULT
            },
        ),
        (
            "CLIENTSSL_HANDSHAKE",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["CLIENTSSL"],
                common: true,
                ..EventProps::DEFAULT
            },
        ),
        (
            "CLIENTSSL_PASSTHROUGH",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["CLIENTSSL"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "CLIENTSSL_SERVERHELLO_SEND",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["CLIENTSSL"],
                ..EventProps::DEFAULT
            },
        ),
    ]
}

fn event_props_table_5() -> Vec<(&'static str, EventProps)> {
    vec![
        (
            "CLIENT_ACCEPTED",
            EventProps {
                client_side: true,
                transport: &["tcp", "udp"],
                hot: true,
                common: true,
                ..EventProps::DEFAULT
            },
        ),
        (
            "CLIENT_CLOSED",
            EventProps {
                client_side: true,
                transport: &["tcp", "udp"],
                common: true,
                ..EventProps::DEFAULT
            },
        ),
        (
            "CLIENT_DATA",
            EventProps {
                client_side: true,
                transport: &["tcp", "udp"],
                setup_event: Some("CLIENT_ACCEPTED"),
                data_collect_protocols: &["TCP", "UDP", "SCTP"],
                data_collect_side: Some(ConnectionSide::Client),
                ..EventProps::DEFAULT
            },
        ),
        (
            "CONNECTOR_OPEN",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["CONNECTOR"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "DIAMETER_EGRESS",
            EventProps {
                client_side: true,
                server_side: true,
                transport: &["tcp"],
                implied_profiles: &["DIAMETER", "DIAMETERSESSION", "DIAMETER_ENDPOINT"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "DIAMETER_INGRESS",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["DIAMETER", "DIAMETERSESSION", "DIAMETER_ENDPOINT"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "DIAMETER_RETRANSMISSION",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["DIAMETER", "DIAMETERSESSION"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "DNS_REQUEST",
            EventProps {
                client_side: true,
                transport: &["udp"],
                implied_profiles: &["DNS"],
                common: true,
                ..EventProps::DEFAULT
            },
        ),
    ]
}

fn event_props_table_6() -> Vec<(&'static str, EventProps)> {
    vec![
        (
            "DNS_RESPONSE",
            EventProps {
                client_side: true,
                server_side: true,
                transport: &["udp"],
                implied_profiles: &["DNS"],
                common: true,
                ..EventProps::DEFAULT
            },
        ),
        (
            "ECA_REQUEST_ALLOWED",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["ECA"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "ECA_REQUEST_DENIED",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["ECA"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "EPI_NA_CHECK_HTTP_REQUEST",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "FIX_HEADER",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["FIX"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "FIX_MESSAGE",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["FIX"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "FLOW_INIT",
            EventProps {
                client_side: true,
                transport: &["tcp", "udp"],
                implied_profiles: &["FLOW"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "GENERICMESSAGE_EGRESS",
            EventProps {
                client_side: true,
                server_side: true,
                transport: &["tcp"],
                implied_profiles: &["GENERICMSG"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "GENERICMESSAGE_INGRESS",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["GENERICMSG"],
                ..EventProps::DEFAULT
            },
        ),
    ]
}

fn event_props_table_7() -> Vec<(&'static str, EventProps)> {
    vec![
        (
            "GTP_GPDU_EGRESS",
            EventProps {
                client_side: true,
                server_side: true,
                transport: &["udp"],
                implied_profiles: &["GTP"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "GTP_GPDU_INGRESS",
            EventProps {
                client_side: true,
                transport: &["udp"],
                implied_profiles: &["GTP"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "GTP_PRIME_EGRESS",
            EventProps {
                client_side: true,
                server_side: true,
                transport: &["tcp"],
                implied_profiles: &["GTP"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "GTP_PRIME_INGRESS",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["GTP"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "GTP_SIGNALLING_EGRESS",
            EventProps {
                client_side: true,
                server_side: true,
                transport: &["udp"],
                implied_profiles: &["GTP"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "GTP_SIGNALLING_INGRESS",
            EventProps {
                client_side: true,
                transport: &["udp"],
                implied_profiles: &["GTP"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "HTML_COMMENT_MATCHED",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["HTML", "HTTP"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "HTML_TAG_MATCHED",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["HTML", "HTTP"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "HTTP_CLASS_FAILED",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["HTTP"],
                lifecycle: Lifecycle::deprecated_in("11.4.0"),
                ..EventProps::DEFAULT
            },
        ),
    ]
}

fn event_props_table_8() -> Vec<(&'static str, EventProps)> {
    vec![
        (
            "HTTP_CLASS_SELECTED",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["HTTP"],
                lifecycle: Lifecycle::deprecated_in("11.4.0"),
                ..EventProps::DEFAULT
            },
        ),
        (
            "HTTP_DISABLED",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["HTTP"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "HTTP_PROXY_CONNECT",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["HTTP", "HTTP_PROXY_CONNECT"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "HTTP_PROXY_REQUEST",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["HTTP", "HTTP_PROXY_CONNECT"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "HTTP_PROXY_RESPONSE",
            EventProps {
                client_side: true,
                server_side: true,
                transport: &["tcp"],
                implied_profiles: &["HTTP", "HTTP_PROXY_CONNECT"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "HTTP_REJECT",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["HTTP"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "HTTP_REQUEST",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["FASTHTTP", "HTTP"],
                lifecycle: Lifecycle::introduced_in("9.0.0"),
                hot: true,
                common: true,
                ..EventProps::DEFAULT
            },
        ),
        (
            "HTTP_REQUEST_DATA",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["HTTP"],
                hot: true,
                common: true,
                setup_event: Some("HTTP_REQUEST"),
                data_collect_protocols: &["HTTP"],
                data_collect_side: Some(ConnectionSide::Client),
                ..EventProps::DEFAULT
            },
        ),
    ]
}

fn event_props_table_9() -> Vec<(&'static str, EventProps)> {
    vec![
        (
            "HTTP_REQUEST_RELEASE",
            EventProps {
                client_side: true,
                server_side: true,
                transport: &["tcp"],
                implied_profiles: &["HTTP"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "HTTP_REQUEST_SEND",
            EventProps {
                client_side: true,
                server_side: true,
                transport: &["tcp"],
                implied_profiles: &["HTTP"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "HTTP_RESPONSE",
            EventProps {
                client_side: true,
                server_side: true,
                command_side: ConnectionSide::Server,
                transport: &["tcp"],
                implied_profiles: &["FASTHTTP", "HTTP"],
                hot: true,
                common: true,
                ..EventProps::DEFAULT
            },
        ),
        (
            "HTTP_RESPONSE_CONTINUE",
            EventProps {
                client_side: true,
                server_side: true,
                transport: &["tcp"],
                implied_profiles: &["HTTP"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "HTTP_RESPONSE_DATA",
            EventProps {
                client_side: true,
                server_side: true,
                command_side: ConnectionSide::Server,
                transport: &["tcp"],
                implied_profiles: &["HTTP"],
                hot: true,
                common: true,
                setup_event: Some("HTTP_RESPONSE"),
                data_collect_protocols: &["HTTP"],
                data_collect_side: Some(ConnectionSide::Server),
                ..EventProps::DEFAULT
            },
        ),
        (
            "HTTP_RESPONSE_RELEASE",
            EventProps {
                client_side: true,
                server_side: true,
                transport: &["tcp"],
                implied_profiles: &["HTTP"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "ICAP_REQUEST",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["ICAP"],
                ..EventProps::DEFAULT
            },
        ),
    ]
}

fn event_props_table_10() -> Vec<(&'static str, EventProps)> {
    vec![
        (
            "ICAP_RESPONSE",
            EventProps {
                client_side: true,
                server_side: true,
                transport: &["tcp"],
                implied_profiles: &["ICAP"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "IN_DOSL7_ATTACK",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["DOSL7", "FASTHTTP", "HTTP"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "IP_GTM",
            EventProps {
                flow: false,
                ..EventProps::DEFAULT
            },
        ),
        (
            "IVS_ENTRY_REQUEST",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["IVS_ENTRY"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "IVS_ENTRY_RESPONSE",
            EventProps {
                client_side: true,
                server_side: true,
                transport: &["tcp"],
                implied_profiles: &["IVS_ENTRY"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "JSON_REQUEST",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["JSON"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "JSON_REQUEST_ERROR",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["JSON"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "JSON_REQUEST_MISSING",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["JSON"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "JSON_RESPONSE",
            EventProps {
                client_side: true,
                server_side: true,
                transport: &["tcp"],
                implied_profiles: &["JSON"],
                ..EventProps::DEFAULT
            },
        ),
    ]
}

fn event_props_table_11() -> Vec<(&'static str, EventProps)> {
    vec![
        (
            "JSON_RESPONSE_ERROR",
            EventProps {
                client_side: true,
                server_side: true,
                transport: &["tcp"],
                implied_profiles: &["JSON"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "JSON_RESPONSE_MISSING",
            EventProps {
                client_side: true,
                server_side: true,
                transport: &["tcp"],
                implied_profiles: &["JSON"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "L7CHECK_CLIENT_DATA",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["L7CHECK"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "L7CHECK_SERVER_DATA",
            EventProps {
                client_side: true,
                server_side: true,
                transport: &["tcp"],
                implied_profiles: &["L7CHECK"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "LB_FAILED",
            EventProps {
                client_side: true,
                transport: &["tcp", "udp"],
                common: true,
                ..EventProps::DEFAULT
            },
        ),
        (
            "LB_QUEUED",
            EventProps {
                client_side: true,
                server_side: true,
                transport: &["tcp", "udp"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "LB_SELECTED",
            EventProps {
                client_side: true,
                server_side: true,
                transport: &["tcp", "udp"],
                common: true,
                ..EventProps::DEFAULT
            },
        ),
        (
            "MQTT_CLIENT_DATA",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["MQTT"],
                setup_event: Some("MQTT_CLIENT_INGRESS"),
                data_collect_protocols: &["MQTT"],
                data_collect_side: Some(ConnectionSide::Client),
                ..EventProps::DEFAULT
            },
        ),
        (
            "MQTT_CLIENT_EGRESS",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["MQTT"],
                ..EventProps::DEFAULT
            },
        ),
    ]
}

fn event_props_table_12() -> Vec<(&'static str, EventProps)> {
    vec![
        (
            "MQTT_CLIENT_INGRESS",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["MQTT"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "MQTT_CLIENT_SHUTDOWN",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["MQTT"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "MQTT_SERVER_DATA",
            EventProps {
                client_side: true,
                server_side: true,
                command_side: ConnectionSide::Server,
                transport: &["tcp"],
                implied_profiles: &["MQTT"],
                setup_event: Some("MQTT_SERVER_INGRESS"),
                data_collect_protocols: &["MQTT"],
                data_collect_side: Some(ConnectionSide::Server),
                ..EventProps::DEFAULT
            },
        ),
        (
            "MQTT_SERVER_EGRESS",
            EventProps {
                client_side: true,
                server_side: true,
                command_side: ConnectionSide::Server,
                transport: &["tcp"],
                implied_profiles: &["MQTT"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "MQTT_SERVER_INGRESS",
            EventProps {
                client_side: true,
                server_side: true,
                command_side: ConnectionSide::Server,
                transport: &["tcp"],
                implied_profiles: &["MQTT"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "MR_DATA",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["MR"],
                setup_event: Some("MR_INGRESS"),
                data_collect_protocols: &["MR"],
                data_collect_side: Some(ConnectionSide::Client),
                ..EventProps::DEFAULT
            },
        ),
        (
            "MR_EGRESS",
            EventProps {
                client_side: true,
                server_side: true,
                transport: &["tcp"],
                implied_profiles: &["MR"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "MR_FAILED",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["MR"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "MR_INGRESS",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["MR"],
                ..EventProps::DEFAULT
            },
        ),
    ]
}

fn event_props_table_13() -> Vec<(&'static str, EventProps)> {
    vec![
        (
            "NAME_RESOLVED",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["NAME"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "PCP_REQUEST",
            EventProps {
                client_side: true,
                transport: &["udp"],
                implied_profiles: &["PCP"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "PCP_RESPONSE",
            EventProps {
                client_side: true,
                server_side: true,
                transport: &["udp"],
                implied_profiles: &["PCP"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "PEM_POLICY",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["PEM"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "PEM_SUBS_SESS_CREATED",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["PEM"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "PEM_SUBS_SESS_DELETED",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["PEM"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "PEM_SUBS_SESS_UPDATED",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["PEM"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "PERSIST_DOWN",
            EventProps {
                flow: false,
                ..EventProps::DEFAULT
            },
        ),
        (
            "PING_REQUEST_READY",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["HTTP"],
                ..EventProps::DEFAULT
            },
        ),
    ]
}

fn event_props_table_14() -> Vec<(&'static str, EventProps)> {
    vec![
        (
            "PING_RESPONSE_READY",
            EventProps {
                client_side: true,
                server_side: true,
                transport: &["tcp"],
                implied_profiles: &["HTTP"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "PROTOCOL_INSPECTION_MATCH",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["IPS", "PROTOCOL_INSPECTION"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "QOE_PARSE_DONE",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["QOE"],
                lifecycle: Lifecycle::introduced_in("11.5.0").deprecated_from("15.0.0"),
                ..EventProps::DEFAULT
            },
        ),
        (
            "RADIUS_AAA_ACCT_REQUEST",
            EventProps {
                client_side: true,
                transport: &["udp"],
                implied_profiles: &["RADIUS", "RADIUS_AAA"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "RADIUS_AAA_ACCT_RESPONSE",
            EventProps {
                client_side: true,
                server_side: true,
                transport: &["udp"],
                implied_profiles: &["RADIUS", "RADIUS_AAA"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "RADIUS_AAA_AUTH_REQUEST",
            EventProps {
                client_side: true,
                transport: &["udp"],
                implied_profiles: &["RADIUS", "RADIUS_AAA"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "RADIUS_AAA_AUTH_RESPONSE",
            EventProps {
                client_side: true,
                server_side: true,
                transport: &["udp"],
                implied_profiles: &["RADIUS", "RADIUS_AAA"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "REWRITE_REQUEST",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["HTTP", "REWRITE"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "REWRITE_REQUEST_DONE",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["HTTP", "REWRITE"],
                ..EventProps::DEFAULT
            },
        ),
    ]
}

fn event_props_table_15() -> Vec<(&'static str, EventProps)> {
    vec![
        (
            "REWRITE_RESPONSE",
            EventProps {
                client_side: true,
                server_side: true,
                transport: &["tcp"],
                implied_profiles: &["HTTP", "REWRITE"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "REWRITE_RESPONSE_DONE",
            EventProps {
                client_side: true,
                server_side: true,
                transport: &["tcp"],
                implied_profiles: &["HTTP", "REWRITE"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "RTSP_REQUEST",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["RTSP"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "RTSP_REQUEST_DATA",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["RTSP"],
                setup_event: Some("RTSP_REQUEST"),
                data_collect_protocols: &["RTSP"],
                data_collect_side: Some(ConnectionSide::Client),
                ..EventProps::DEFAULT
            },
        ),
        (
            "RTSP_RESPONSE",
            EventProps {
                client_side: true,
                server_side: true,
                command_side: ConnectionSide::Server,
                transport: &["tcp"],
                implied_profiles: &["RTSP"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "RTSP_RESPONSE_DATA",
            EventProps {
                client_side: true,
                server_side: true,
                command_side: ConnectionSide::Server,
                transport: &["tcp"],
                implied_profiles: &["RTSP"],
                setup_event: Some("RTSP_RESPONSE"),
                data_collect_protocols: &["RTSP"],
                data_collect_side: Some(ConnectionSide::Server),
                ..EventProps::DEFAULT
            },
        ),
        (
            "RULE_INIT",
            EventProps {
                flow: false,
                common: true,
                ..EventProps::DEFAULT
            },
        ),
        (
            "SA_PICKED",
            EventProps {
                client_side: true,
                transport: &["tcp", "udp"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "SERVERSSL_CLIENTHELLO_SEND",
            EventProps {
                client_side: true,
                server_side: true,
                transport: &["tcp"],
                implied_profiles: &["SERVERSSL"],
                ..EventProps::DEFAULT
            },
        ),
    ]
}

fn event_props_table_16() -> Vec<(&'static str, EventProps)> {
    vec![
        (
            "SERVERSSL_DATA",
            EventProps {
                client_side: true,
                server_side: true,
                command_side: ConnectionSide::Server,
                transport: &["tcp"],
                implied_profiles: &["SERVERSSL"],
                setup_event: Some("SERVERSSL_HANDSHAKE"),
                data_collect_protocols: &["SSL"],
                data_collect_side: Some(ConnectionSide::Server),
                ..EventProps::DEFAULT
            },
        ),
        (
            "SERVERSSL_HANDSHAKE",
            EventProps {
                client_side: true,
                server_side: true,
                command_side: ConnectionSide::Server,
                transport: &["tcp"],
                implied_profiles: &["PERSIST", "SERVERSSL", "SSL_PERSISTENCE"],
                common: true,
                ..EventProps::DEFAULT
            },
        ),
        (
            "SERVERSSL_SERVERCERT",
            EventProps {
                client_side: true,
                server_side: true,
                transport: &["tcp"],
                implied_profiles: &["SERVERSSL"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "SERVERSSL_SERVERHELLO",
            EventProps {
                client_side: true,
                server_side: true,
                transport: &["tcp"],
                implied_profiles: &["PERSIST", "SERVERSSL", "SSL_PERSISTENCE"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "SERVER_CLOSED",
            EventProps {
                client_side: true,
                server_side: true,
                transport: &["tcp", "udp"],
                common: true,
                ..EventProps::DEFAULT
            },
        ),
        (
            "SERVER_CONNECTED",
            EventProps {
                client_side: true,
                server_side: true,
                command_side: ConnectionSide::Server,
                transport: &["tcp", "udp"],
                hot: true,
                common: true,
                ..EventProps::DEFAULT
            },
        ),
        (
            "SERVER_DATA",
            EventProps {
                client_side: true,
                server_side: true,
                command_side: ConnectionSide::Server,
                transport: &["tcp", "udp"],
                setup_event: Some("SERVER_CONNECTED"),
                data_collect_protocols: &["TCP", "UDP"],
                data_collect_side: Some(ConnectionSide::Server),
                ..EventProps::DEFAULT
            },
        ),
    ]
}

fn event_props_table_17() -> Vec<(&'static str, EventProps)> {
    vec![
        (
            "SERVER_INIT",
            EventProps {
                client_side: true,
                server_side: true,
                transport: &["tcp", "udp"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "SIP_REQUEST",
            EventProps {
                client_side: true,
                implied_profiles: &["SIP", "SIPROUTER", "SIPSESSION"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "SIP_REQUEST_DONE",
            EventProps {
                client_side: true,
                server_side: true,
                implied_profiles: &["SIP", "SIPROUTER", "SIPSESSION"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "SIP_REQUEST_SEND",
            EventProps {
                client_side: true,
                server_side: true,
                implied_profiles: &["SIP", "SIPROUTER", "SIPSESSION"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "SIP_RESPONSE",
            EventProps {
                client_side: true,
                server_side: true,
                implied_profiles: &["SIP", "SIPROUTER", "SIPSESSION"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "SIP_RESPONSE_DONE",
            EventProps {
                client_side: true,
                server_side: true,
                implied_profiles: &["SIP", "SIPROUTER", "SIPSESSION"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "SIP_RESPONSE_SEND",
            EventProps {
                client_side: true,
                server_side: true,
                implied_profiles: &["SIP", "SIPROUTER", "SIPSESSION"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "SOCKS_REQUEST",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["SOCKS"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "SSE_RESPONSE",
            EventProps {
                client_side: true,
                server_side: true,
                transport: &["tcp"],
                implied_profiles: &["SSE"],
                ..EventProps::DEFAULT
            },
        ),
    ]
}

fn event_props_table_18() -> Vec<(&'static str, EventProps)> {
    vec![
        (
            "STREAM_MATCHED",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["STREAM"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "TAP_REQUEST",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["TAP"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "TCP_GTM",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "TDS_REQUEST",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["MSSQL", "TDS"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "TDS_RESPONSE",
            EventProps {
                client_side: true,
                server_side: true,
                transport: &["tcp"],
                implied_profiles: &["MSSQL", "TDS"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "UDP_GTM",
            EventProps {
                client_side: true,
                transport: &["udp"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "USER_REQUEST",
            EventProps {
                client_side: true,
                server_side: true,
                transport: &["tcp"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "USER_RESPONSE",
            EventProps {
                client_side: true,
                server_side: true,
                transport: &["tcp"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "WS_CLIENT_DATA",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["HTTP", "WS"],
                setup_event: Some("WS_CLIENT_FRAME"),
                data_collect_protocols: &["WS"],
                data_collect_side: Some(ConnectionSide::Client),
                ..EventProps::DEFAULT
            },
        ),
    ]
}

fn event_props_table_19() -> Vec<(&'static str, EventProps)> {
    vec![
        (
            "WS_CLIENT_FRAME",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["HTTP", "WS"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "WS_CLIENT_FRAME_DONE",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["HTTP", "WS"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "WS_REQUEST",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["HTTP", "WS"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "WS_RESPONSE",
            EventProps {
                client_side: true,
                server_side: true,
                transport: &["tcp"],
                implied_profiles: &["HTTP", "WS"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "WS_SERVER_DATA",
            EventProps {
                client_side: true,
                server_side: true,
                command_side: ConnectionSide::Server,
                transport: &["tcp"],
                implied_profiles: &["HTTP", "WS"],
                setup_event: Some("WS_SERVER_FRAME"),
                data_collect_protocols: &["WS"],
                data_collect_side: Some(ConnectionSide::Server),
                ..EventProps::DEFAULT
            },
        ),
        (
            "WS_SERVER_FRAME",
            EventProps {
                client_side: true,
                server_side: true,
                command_side: ConnectionSide::Server,
                transport: &["tcp"],
                implied_profiles: &["HTTP", "WS"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "WS_SERVER_FRAME_DONE",
            EventProps {
                client_side: true,
                server_side: true,
                command_side: ConnectionSide::Server,
                transport: &["tcp"],
                implied_profiles: &["HTTP", "WS"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "XML_BEGIN_DOCUMENT",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["FASTHTTP", "HTTP", "XML"],
                lifecycle: Lifecycle::introduced_in("9.0.3")
                    .deprecated_from("10.0.0")
                    .retired_from("10.0.0"),
                ..EventProps::DEFAULT
            },
        ),
    ]
}

fn event_props_table_20() -> Vec<(&'static str, EventProps)> {
    vec![
        (
            "XML_BEGIN_ELEMENT",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["FASTHTTP", "HTTP", "XML"],
                lifecycle: Lifecycle::introduced_in("9.0.3")
                    .deprecated_from("10.0.0")
                    .retired_from("10.0.0"),
                ..EventProps::DEFAULT
            },
        ),
        (
            "XML_CDATA",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["FASTHTTP", "HTTP", "XML"],
                lifecycle: Lifecycle::introduced_in("9.0.3")
                    .deprecated_from("10.0.0")
                    .retired_from("10.0.0"),
                ..EventProps::DEFAULT
            },
        ),
        (
            "XML_CONTENT_BASED_ROUTING",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["FASTHTTP", "HTTP", "XML"],
                lifecycle: Lifecycle::introduced_in("10.2.0"),
                ..EventProps::DEFAULT
            },
        ),
        (
            "XML_END_DOCUMENT",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["FASTHTTP", "HTTP", "XML"],
                lifecycle: Lifecycle::introduced_in("9.0.3")
                    .deprecated_from("10.0.0")
                    .retired_from("10.0.0"),
                ..EventProps::DEFAULT
            },
        ),
        (
            "XML_END_ELEMENT",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["FASTHTTP", "HTTP", "XML"],
                lifecycle: Lifecycle::introduced_in("9.0.3")
                    .deprecated_from("10.0.0")
                    .retired_from("10.0.0"),
                ..EventProps::DEFAULT
            },
        ),
        (
            "XML_EVENT",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["FASTHTTP", "HTTP", "XML"],
                lifecycle: Lifecycle::introduced_in("9.0.3")
                    .deprecated_from("10.0.0")
                    .retired_from("10.0.0"),
                ..EventProps::DEFAULT
            },
        ),
    ]
}
