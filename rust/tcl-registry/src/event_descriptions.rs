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

//! Per-event description prose for iRules events.
//!
//! Generated from the Python registry's `EVENT_DESCRIPTIONS`
//! (`compiler/registry/namespace_data.py`) by
//! `scripts/registry-audit/gen_event_descriptions.py`. Drives the
//! `description:` line of `f5 irule event-info`.

/// `(event_name, description)` for every iRules event that carries
/// descriptive prose, in Python dict (insertion) order.
pub(crate) const EVENT_DESCRIPTIONS: &[(&str, &str)] = &[
    (
        "RULE_INIT",
        "Fires once when the iRule is loaded (device boot, config change, or iRule save). Use for initialising static:: variables only.",
    ),
    (
        "PERSIST_DOWN",
        "Fires pre-LB when persistence would direct a connection to a node or pool member that has been marked down.",
    ),
    (
        "FLOW_INIT",
        "Fires once per TCP or unique UDP/IP flow, after packet filters but before AFM and TMM processing. Runs before CLIENT_ACCEPTED in the processing pipeline.",
    ),
    (
        "CLIENT_ACCEPTED",
        "Fires when a client connection is accepted (TCP: after 3-way handshake; UDP: on the first datagram). On UDP virtual servers, packet data is available via UDP::payload in this event.",
    ),
    (
        "CLIENT_DATA",
        "Fires when client data is received while TCP is in collect state (requires TCP::collect). On UDP virtual servers, it fires on every datagram including the first; UDP payload is available without a collect command.",
    ),
    (
        "CLIENT_CLOSED",
        "Fires when the client-side connection closes (TCP: FIN/RST; UDP: idle timeout). Fires once per connection.",
    ),
    (
        "LB_SELECTED",
        "Fires when the system selects a pool member. On HTTP virtual servers, fires after HTTP_REQUEST (per-request). On UDP (9.4+), fires after CLIENT_ACCEPTED but before CLIENT_DATA.",
    ),
    (
        "LB_FAILED",
        "Fires when the system fails to select a pool or pool member, or when a selected resource is unreachable. Alternative to LB_SELECTED at the same logical point.",
    ),
    (
        "LB_QUEUED",
        "Fires when a connection limit is hit at the pool or pool member level. Alternative to LB_SELECTED.",
    ),
    (
        "SERVER_INIT",
        "Fires after LB_SELECTED when the server-side TCP SYN is about to be sent. Server address/port are available for inspection.",
    ),
    (
        "SA_PICKED",
        "Fires after source address translation is complete but the server-side flow is not yet set up. Runs between LB_SELECTED and SERVER_CONNECTED.",
    ),
    (
        "SERVER_CONNECTED",
        "Fires when a server-side connection is established (TCP: after handshake with the pool member). On UDP, fires if and when a server is selected, before the datagram is forwarded.",
    ),
    (
        "SERVER_DATA",
        "Fires when new data is received from the server while TCP is in collect state (requires TCP::collect in SERVER_CONNECTED). On UDP virtual servers, it fires on every server datagram and payload is available without a collect command.",
    ),
    (
        "SERVER_CLOSED",
        "Fires when the server-side connection closes. On UDP, fires when the connection table entry times out.",
    ),
    (
        "HTTP_REQUEST",
        "Fires when request headers are fully parsed (pre-LB). On keep-alive connections, fires once per HTTP transaction. Pipeline: L7 iRules layer.",
    ),
    (
        "HTTP_REQUEST_DATA",
        "Fires when HTTP::collect has gathered the specified amount of request body data. Only fires if HTTP::collect was called in HTTP_REQUEST.",
    ),
    (
        "HTTP_REQUEST_SEND",
        "Fires immediately before the request is forwarded to the server-side TCP stack (post-LB, after SERVER_CONNECTED).",
    ),
    (
        "HTTP_REQUEST_RELEASE",
        "Fires when HTTP data is about to be released on the server-side. Last chance to modify request data before it reaches the server.",
    ),
    (
        "HTTP_RESPONSE",
        "Fires when response status and headers are fully parsed from the server. Fires once per HTTP transaction on keep-alive.",
    ),
    (
        "HTTP_RESPONSE_DATA",
        "Fires when HTTP::collect has gathered the specified amount of response body data. Only fires if HTTP::collect was called in HTTP_RESPONSE.",
    ),
    (
        "HTTP_RESPONSE_CONTINUE",
        "Fires when the system receives a 100 Continue interim response from the server.",
    ),
    (
        "HTTP_RESPONSE_RELEASE",
        "Fires when HTTP data is about to be released on the client-side. Last chance to modify response data before it reaches the client.",
    ),
    (
        "HTTP_DISABLED",
        "Fires when HTTP processing is disabled on the connection (e.g. WebSocket upgrade or protocol switch).",
    ),
    (
        "HTTP_REJECT",
        "Fires when HTTP encounters a parsing error and aborts the connection.",
    ),
    (
        "HTTP_PROXY_REQUEST",
        "Fires when a virtual server is configured with explicit proxy mode.",
    ),
    (
        "HTTP_PROXY_CONNECT",
        "Fires when proxy chaining via the HTTP_PROXY_CONNECT profile.",
    ),
    (
        "HTTP_PROXY_RESPONSE",
        "Fires when a response from the remote HTTP proxy is received.",
    ),
    (
        "HTTP_CLASS_FAILED",
        "Fires when an HTTP request does not match any HTTP class filter (deprecated; pre-LB classification).",
    ),
    (
        "HTTP_CLASS_SELECTED",
        "Fires when an HTTP request matches an HTTP class filter (deprecated; pre-LB classification).",
    ),
    (
        "CLIENTSSL_CLIENTHELLO",
        "Fires when the client's SSL/TLS ClientHello message is received. Fires after CLIENT_ACCEPTED, before CLIENTSSL_HANDSHAKE. Allows SNI inspection and certificate selection.",
    ),
    (
        "CLIENTSSL_CLIENTCERT",
        "Fires when a client certificate is received (mutual TLS only). Only fires when the ClientSSL profile requires or requests a client certificate.",
    ),
    (
        "CLIENTSSL_HANDSHAKE",
        "Fires when the client-side SSL/TLS handshake completes. Fires once per connection. After this, HTTP events can fire.",
    ),
    (
        "CLIENTSSL_SERVERHELLO_SEND",
        "Fires when BIG-IP is about to send its ServerHello on the client-side connection. Allows cipher and protocol modification.",
    ),
    (
        "CLIENTSSL_DATA",
        "Fires when SSL data is received from the client while in collect state (requires SSL::collect).",
    ),
    (
        "CLIENTSSL_PASSTHROUGH",
        "Fires when the ClientSSL profile receives plaintext (non-TLS) data and enters passthrough mode.",
    ),
    (
        "SERVERSSL_CLIENTHELLO_SEND",
        "Fires when BIG-IP is about to send its ClientHello on the server-side connection (post-LB, after SERVER_CONNECTED).",
    ),
    (
        "SERVERSSL_SERVERHELLO",
        "Fires when the server's ServerHello message is received on the server-side connection.",
    ),
    (
        "SERVERSSL_SERVERCERT",
        "Fires when the server's certificate is received and verification completes on the server-side connection.",
    ),
    (
        "SERVERSSL_HANDSHAKE",
        "Fires when the server-side SSL/TLS handshake completes. After this, HTTP_REQUEST_SEND can fire.",
    ),
    (
        "SERVERSSL_DATA",
        "Fires when SSL data is received from the server while in collect state (requires SSL::collect).",
    ),
    (
        "DNS_REQUEST",
        "Fires when a DNS query is received (pre-LB). On UDP virtual servers, this is the first event (no CLIENT_ACCEPTED). On TCP virtual servers, fires after CLIENT_ACCEPTED.",
    ),
    (
        "DNS_RESPONSE",
        "Fires when a DNS response is ready to be sent to the client (post-LB).",
    ),
    (
        "SIP_REQUEST",
        "Triggered when the system fully parses a complete client SIP request header.",
    ),
    (
        "SIP_REQUEST_SEND",
        "Triggered immediately before a SIP request is sent to the server-side TCP stack.",
    ),
    (
        "SIP_RESPONSE",
        "Triggered when the system parses all response status and header lines from the server SIP response.",
    ),
    (
        "SIP_RESPONSE_SEND",
        "Triggered immediately before a SIP response is sent.",
    ),
    (
        "SIP_REQUEST_DONE",
        "Raised when a SIP request message is received from the proxy after routing.",
    ),
    (
        "SIP_RESPONSE_DONE",
        "Raised when a SIP response message is received from the proxy after routing.",
    ),
    (
        "WS_REQUEST",
        "Raised when WebSocket upgrade headers are present in the client request.",
    ),
    (
        "WS_RESPONSE",
        "Raised when WebSocket upgrade headers are present in the server response.",
    ),
    (
        "WS_CLIENT_FRAME",
        "Raised at the start of a WebSocket frame received from the client.",
    ),
    (
        "WS_SERVER_FRAME",
        "Raised at the start of a WebSocket frame received from the server.",
    ),
    (
        "WS_CLIENT_FRAME_DONE",
        "Raised at the end of a WebSocket frame received from the client.",
    ),
    (
        "WS_SERVER_FRAME_DONE",
        "Raised at the end of a WebSocket frame received from the server.",
    ),
    (
        "WS_CLIENT_DATA",
        "Raised when the system collects the specified amount of WebSocket data from the client via WS::collect.",
    ),
    (
        "WS_SERVER_DATA",
        "Raised when the system collects the specified amount of WebSocket data from the server via WS::collect.",
    ),
    (
        "RTSP_REQUEST",
        "Triggered after a complete RTSP request has been received.",
    ),
    (
        "RTSP_REQUEST_DATA",
        "Triggered when an RTSP::collect command finishes processing.",
    ),
    (
        "RTSP_RESPONSE",
        "Triggered after a complete RTSP response has been received.",
    ),
    (
        "RTSP_RESPONSE_DATA",
        "Triggered when collection of RTSP response data is finished.",
    ),
    (
        "AUTH_RESULT",
        "Replaces AUTH_SUCCESS, AUTH_FAILURE, AUTH_ERROR, and AUTH_WANTCREDENTIAL events.",
    ),
    (
        "AUTH_ERROR",
        "Triggered when an error occurs during authorization (deprecated; use AUTH_RESULT).",
    ),
    (
        "AUTH_FAILURE",
        "Triggered when an unsuccessful authorization operation completes (deprecated; use AUTH_RESULT).",
    ),
    (
        "AUTH_SUCCESS",
        "Triggered when a successful authorization completes (deprecated; use AUTH_RESULT).",
    ),
    (
        "AUTH_WANTCREDENTIAL",
        "Triggered when an authorization operation needs an additional credential (deprecated; use AUTH_RESULT).",
    ),
    (
        "ACCESS_ACL_ALLOWED",
        "Fires when a resource request passes ACL checks (post-LB). Per-request event on keep-alive connections.",
    ),
    (
        "ACCESS_ACL_DENIED",
        "Fires when a resource request fails ACL checks (post-LB). Per-request event.",
    ),
    (
        "ACCESS_POLICY_AGENT_EVENT",
        "Fires during access policy execution to allow iRule logic (pre-LB, per-session).",
    ),
    (
        "ACCESS_POLICY_COMPLETED",
        "Fires when access policy evaluation completes for a user session (pre-LB, once per session).",
    ),
    (
        "ACCESS_SESSION_CLOSED",
        "Fires when a user session is removed (logout, timeout, or admin action). Fires after CLIENT_CLOSED.",
    ),
    (
        "ACCESS_SESSION_STARTED",
        "Fires when a new APM user session is created (pre-LB, once per session).",
    ),
    (
        "ACCESS_PER_REQUEST_AGENT_EVENT",
        "Allows iRule logic execution at a desired point in per-request access policy execution.",
    ),
    (
        "ACCESS_SAML_AUTHN",
        "Triggered when the SAML authentication request payload is generated for a user session.",
    ),
    (
        "ACCESS_SAML_ASSERTION",
        "Triggered when the SAML assertion payload is generated for a user session.",
    ),
    (
        "ACCESS_SAML_SLO_REQ",
        "Triggered when the SAML single logout request payload is generated for a user session.",
    ),
    (
        "ACCESS_SAML_SLO_RESP",
        "Triggered when the SAML single logout response payload is generated for a user session.",
    ),
    (
        "ACCESS2_POLICY_EXPRESSION_EVAL",
        "Triggered when per-request policy branch expressions are evaluated.",
    ),
    (
        "ASM_REQUEST_DONE",
        "Fires after ASM finishes processing a request (pre-LB). Normal mode: fires after every request. Compatibility mode: does not fire (use ASM_REQUEST_VIOLATION).",
    ),
    (
        "ASM_REQUEST_VIOLATION",
        "Fires when ASM detects a request policy violation (pre-LB). Compatibility mode: fires only when a violation occurs. Normal mode: violations reported via ASM_REQUEST_DONE instead. Deprecated event.",
    ),
    (
        "ASM_REQUEST_BLOCKING",
        "Fires when ASM is generating a blocking response (pre-LB). Allows modification of the reject page before it is sent. Fires in both Normal and Compatibility modes.",
    ),
    (
        "ASM_RESPONSE_VIOLATION",
        "Fires when ASM detects a response policy violation. Runs post-response (after HTTP_RESPONSE_DATA).",
    ),
    (
        "ASM_RESPONSE_LOGIN",
        "Fires on an ASM response login event. Runs post-response.",
    ),
    (
        "BOTDEFENSE_REQUEST",
        "Fires on an HTTP request after Bot Defense processing completes, before a blocking decision is made. Pipeline: runs after L7 DoS in the BIG-IP 14.1+ processing order.",
    ),
    (
        "BOTDEFENSE_ACTION",
        "Fires immediately before Bot Defense takes action on a transaction (block, challenge, allow).",
    ),
    ("ANTIFRAUD_LOGIN", "Fires on an antifraud login event."),
    (
        "ANTIFRAUD_ALERT",
        "Fires when an antifraud alert is received or generated.",
    ),
    (
        "IN_DOSL7_ATTACK",
        "Fires when the L7 DoS profile detects an attack condition (pre-LB). Pipeline: runs after CACHE events and before ASM request-side events in the BIG-IP 14.1+ processing order.",
    ),
    (
        "CLASSIFICATION_DETECTED",
        "Triggered when a flow is classified.",
    ),
    (
        "QOE_PARSE_DONE",
        "Triggered when the system finishes parsing static video parameters from the video header.",
    ),
    (
        "STREAM_MATCHED",
        "Triggered when a stream expression matches data-stream octets.",
    ),
    (
        "CATEGORY_MATCHED",
        "Triggered when a custom category match is found during URL filtering.",
    ),
    (
        "PROTOCOL_INSPECTION_MATCH",
        "Triggered when protocol inspection is matched for the flow.",
    ),
    (
        "HTML_COMMENT_MATCHED",
        "Raised when an HTML comment is encountered.",
    ),
    (
        "HTML_TAG_MATCHED",
        "Raised when an HTML tag is encountered.",
    ),
    (
        "XML_CONTENT_BASED_ROUTING",
        "Triggered when a match is found in the XML profile.",
    ),
    (
        "XML_BEGIN_DOCUMENT",
        "Triggered before the XML document gets parsed.",
    ),
    (
        "XML_BEGIN_ELEMENT",
        "Triggered when the parser encounters the start of an element.",
    ),
    (
        "XML_CDATA",
        "Triggered when the parser encounters character data (CDATA).",
    ),
    (
        "XML_END_DOCUMENT",
        "Triggered when an XML document is completely parsed.",
    ),
    (
        "XML_END_ELEMENT",
        "Triggered when the parser encounters the end of an element.",
    ),
    (
        "XML_EVENT",
        "Generic catch-all event triggered for all XML events.",
    ),
    (
        "JSON_REQUEST",
        "Triggered upon successful parsing of valid JSON content in an HTTP request body.",
    ),
    (
        "JSON_REQUEST_ERROR",
        "Triggered when an HTTP request body should contain JSON but could not be parsed.",
    ),
    (
        "JSON_REQUEST_MISSING",
        "Triggered when an HTTP request has no body or does not contain JSON content.",
    ),
    (
        "JSON_RESPONSE",
        "Triggered upon successful parsing of valid JSON content in an HTTP response body.",
    ),
    (
        "JSON_RESPONSE_ERROR",
        "Triggered when an HTTP response body should contain JSON but could not be parsed.",
    ),
    (
        "JSON_RESPONSE_MISSING",
        "Triggered when an HTTP response has no body or does not contain JSON content.",
    ),
    (
        "SSE_RESPONSE",
        "Triggered when an SSE server response message has been received.",
    ),
    (
        "CACHE_REQUEST",
        "Fires when a cacheable request is received (pre-LB). Allows cache key manipulation before the cache lookup.",
    ),
    (
        "CACHE_RESPONSE",
        "Fires when a cached response is about to be served directly (pre-LB cache hit). Bypasses LB_SELECTED and server-side events when the response is served from cache.",
    ),
    (
        "CACHE_UPDATE",
        "Fires when a new cache entry is inserted or an expired object is refreshed (post-response).",
    ),
    ("REWRITE_REQUEST", "Triggered on a rewrite request event."),
    (
        "REWRITE_REQUEST_DONE",
        "Triggered after ACCESS_ACL_ALLOWED when a Portal Access resource is accessed.",
    ),
    ("REWRITE_RESPONSE", "Triggered on a rewrite response event."),
    (
        "REWRITE_RESPONSE_DONE",
        "Triggered when REWRITE_REQUEST_DONE calls REWRITE::post_process on.",
    ),
    (
        "ICAP_REQUEST",
        "Raised after an ICAP command has been created but before it is sent to an ICAP server.",
    ),
    (
        "ICAP_RESPONSE",
        "Raised after an ICAP response has been processed but before the result is sent to the adaptation virtual server.",
    ),
    (
        "ADAPT_REQUEST_RESULT",
        "Raised after the internal virtual server returns the result of request modification.",
    ),
    (
        "ADAPT_REQUEST_HEADERS",
        "Raised as soon as any HTTP request headers have been returned from the IVS.",
    ),
    (
        "ADAPT_RESPONSE_RESULT",
        "Raised after the internal virtual server returns the result of response modification.",
    ),
    (
        "ADAPT_RESPONSE_HEADERS",
        "Raised as soon as any HTTP response headers have been returned from the IVS.",
    ),
    (
        "FIX_HEADER",
        "Triggered when the system finishes parsing a new FIX header.",
    ),
    (
        "FIX_MESSAGE",
        "Triggered when the system finishes parsing a new FIX message.",
    ),
    (
        "DIAMETER_INGRESS",
        "Triggered when the system receives a DIAMETER message.",
    ),
    (
        "DIAMETER_EGRESS",
        "Triggered when the system is ready to send a DIAMETER message.",
    ),
    (
        "DIAMETER_RETRANSMISSION",
        "Triggered when the system generates a retransmitted DIAMETER request or answer message.",
    ),
    (
        "MQTT_CLIENT_INGRESS",
        "Triggered when an MQTT message is received from the client-side.",
    ),
    (
        "MQTT_CLIENT_DATA",
        "Triggered when a prior MQTT::collect command finishes on the client-side.",
    ),
    (
        "MQTT_CLIENT_EGRESS",
        "Triggered when an MQTT message is sent to the client-side.",
    ),
    (
        "MQTT_CLIENT_SHUTDOWN",
        "Triggered when the MQTT client closes the TCP connection.",
    ),
    (
        "MQTT_SERVER_INGRESS",
        "Triggered when an MQTT message is received from the server-side.",
    ),
    (
        "MQTT_SERVER_DATA",
        "Triggered when server-side payload data collection via MQTT::collect finishes.",
    ),
    (
        "MQTT_SERVER_EGRESS",
        "Triggered when an MQTT message is sent to the server-side.",
    ),
    (
        "GENERICMESSAGE_INGRESS",
        "Raised when a message is received by the generic message filter.",
    ),
    (
        "GENERICMESSAGE_EGRESS",
        "Raised when a message is received from the proxy.",
    ),
    (
        "MR_INGRESS",
        "Raised when a message is received by the message proxy before route lookup.",
    ),
    (
        "MR_EGRESS",
        "Raised after the route has been selected and the message is delivered for forwarding.",
    ),
    (
        "MR_FAILED",
        "Raised when a message has been returned to the originating flow due to a routing failure.",
    ),
    ("MR_DATA", "Raised when message routing data is received."),
    (
        "GTP_GPDU_INGRESS",
        "Triggered for a GTP G-PDU message on the connection that accepted the message.",
    ),
    (
        "GTP_GPDU_EGRESS",
        "Triggered for a GTP G-PDU message on the connection that forwards the message.",
    ),
    (
        "GTP_PRIME_INGRESS",
        "Triggered for GTP prime messages on the connection that accepted the message.",
    ),
    (
        "GTP_PRIME_EGRESS",
        "Triggered for GTP prime messages on the connection that forwards the message.",
    ),
    (
        "GTP_SIGNALLING_INGRESS",
        "Triggered for any GTP signalling message (except G-PDU) on the accepting connection.",
    ),
    (
        "GTP_SIGNALLING_EGRESS",
        "Triggered for any GTP signalling message (except G-PDU) on the forwarding connection.",
    ),
    (
        "RADIUS_AAA_AUTH_REQUEST",
        "Triggered when a RADIUS authentication request is received.",
    ),
    (
        "RADIUS_AAA_AUTH_RESPONSE",
        "Triggered when a RADIUS authentication response is received.",
    ),
    (
        "RADIUS_AAA_ACCT_REQUEST",
        "Triggered when a RADIUS accounting request is received.",
    ),
    (
        "RADIUS_AAA_ACCT_RESPONSE",
        "Triggered when a RADIUS accounting response is received.",
    ),
    (
        "PCP_REQUEST",
        "Triggered on receipt of a valid PCP request from a client.",
    ),
    (
        "PCP_RESPONSE",
        "Triggered when a PCP response is returned to the client.",
    ),
    (
        "SOCKS_REQUEST",
        "Triggered upon receipt of a SOCKS command on a SOCKS connection, before authentication.",
    ),
    (
        "TDS_REQUEST",
        "Triggered when a TDS request message is received.",
    ),
    (
        "TDS_RESPONSE",
        "Triggered when a TDS response message is received.",
    ),
    (
        "IVS_ENTRY_REQUEST",
        "Triggered when the internal virtual server receives a request from the parent virtual server.",
    ),
    (
        "IVS_ENTRY_RESPONSE",
        "Triggered when the internal virtual server receives a response from the parent virtual server.",
    ),
    (
        "L7CHECK_CLIENT_DATA",
        "Triggered each time new ingress data is received from the client.",
    ),
    (
        "L7CHECK_SERVER_DATA",
        "Triggered each time new ingress data is received from the server.",
    ),
    ("PEM_POLICY", "Triggered for PEM policy evaluation."),
    (
        "PEM_SUBS_SESS_CREATED",
        "Triggered when a subscriber session is created.",
    ),
    (
        "PEM_SUBS_SESS_UPDATED",
        "Triggered when a subscriber session attribute is updated.",
    ),
    (
        "PEM_SUBS_SESS_DELETED",
        "Triggered when a subscriber session is deleted.",
    ),
    (
        "AVR_CSPM_INJECTION",
        "Triggered when the AVR profile is about to insert CSPM javascript into the response.",
    ),
    (
        "ECA_REQUEST_ALLOWED",
        "Triggered when the ECA plugin successfully authenticates and is about to forward the request.",
    ),
    (
        "ECA_REQUEST_DENIED",
        "Triggered when the ECA plugin cannot verify user credentials.",
    ),
    (
        "NAME_RESOLVED",
        "Triggered after a NAME::lookup command has been issued and a response received.",
    ),
    (
        "TAP_REQUEST",
        "Triggered once a security token is obtained for certain HTTP transactions.",
    ),
    (
        "CONNECTOR_OPEN",
        "Triggered when the connector is about to raise the service connect.",
    ),
    (
        "PING_REQUEST_READY",
        "Triggered when TMM has assembled an HTTP request to the PingAccess policy server.",
    ),
    (
        "PING_RESPONSE_READY",
        "Triggered when TMM has received an HTTP response from the PingAccess policy server.",
    ),
    (
        "USER_REQUEST",
        "Triggered by the TCP::notify request command; executes in server-side context.",
    ),
    (
        "USER_RESPONSE",
        "Triggered by the TCP::notify response command; executes in client-side context.",
    ),
    (
        "EPI_NA_CHECK_HTTP_REQUEST",
        "Internal event for Network Access and Endpoint Inspector client applications (requires APM).",
    ),
    ("IP_GTM", "GTM IP event."),
    ("TCP_GTM", "GTM TCP event."),
    ("UDP_GTM", "GTM UDP event."),
];
