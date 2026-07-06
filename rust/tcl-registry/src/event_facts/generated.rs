// @generated — DO NOT EDIT. Regenerate to refresh.

use super::EventFacts;

/// Per-event structural facts: the protocol categories an event
/// belongs to. Enabling-profile facts are NOT recorded here — they
/// live in the hand-curated `EventProps::implied_profiles` in the
/// tcl-registry `events` module (the single source of truth), which
/// `events_for_profile` reads directly.
pub static EVENT_FACTS: &[EventFacts] = &[
    EventFacts {
        event: "ACCESS_ACL_ALLOWED",
        categories: &["ACCESS"],
    },
    EventFacts {
        event: "ACCESS_ACL_DENIED",
        categories: &["ACCESS"],
    },
    EventFacts {
        event: "ACCESS_PER_REQUEST_AGENT_EVENT",
        categories: &["HTTP", "IP", "SSL", "TCP"],
    },
    EventFacts {
        event: "ACCESS_POLICY_AGENT_EVENT",
        categories: &["ACCESS"],
    },
    EventFacts {
        event: "ACCESS_POLICY_COMPLETED",
        categories: &["ACCESS"],
    },
    EventFacts {
        event: "ACCESS_SAML_ASSERTION",
        categories: &["ACCESS"],
    },
    EventFacts {
        event: "ACCESS_SAML_AUTHN",
        categories: &["ACCESS"],
    },
    EventFacts {
        event: "ACCESS_SAML_SLO_REQ",
        categories: &["ACCESS"],
    },
    EventFacts {
        event: "ACCESS_SAML_SLO_RESP",
        categories: &["ACCESS"],
    },
    EventFacts {
        event: "ACCESS_SESSION_CLOSED",
        categories: &["ACCESS_GLOBAL"],
    },
    EventFacts {
        event: "ACCESS_SESSION_STARTED",
        categories: &["ACCESS"],
    },
    EventFacts {
        event: "ADAPT_REQUEST_HEADERS",
        categories: &["ADAPT"],
    },
    EventFacts {
        event: "ADAPT_REQUEST_RESULT",
        categories: &["ADAPT"],
    },
    EventFacts {
        event: "ADAPT_RESPONSE_HEADERS",
        categories: &["ADAPT"],
    },
    EventFacts {
        event: "ADAPT_RESPONSE_RESULT",
        categories: &["ADAPT"],
    },
    EventFacts {
        event: "ANTIFRAUD_ALERT",
        categories: &["CLIENTSIDE", "HTTP"],
    },
    EventFacts {
        event: "ANTIFRAUD_LOGIN",
        categories: &["HTTP", "SERVERSIDE"],
    },
    EventFacts {
        event: "ASM_REQUEST_BLOCKING",
        categories: &["HTTP"],
    },
    EventFacts {
        event: "ASM_REQUEST_DONE",
        categories: &["HTTP"],
    },
    EventFacts {
        event: "ASM_REQUEST_VIOLATION",
        categories: &["HTTP"],
    },
    EventFacts {
        event: "ASM_RESPONSE_LOGIN",
        categories: &["HTTP"],
    },
    EventFacts {
        event: "ASM_RESPONSE_VIOLATION",
        categories: &["HTTP"],
    },
    EventFacts {
        event: "AUTH_ERROR",
        categories: &["AUTH"],
    },
    EventFacts {
        event: "AUTH_FAILURE",
        categories: &["AUTH"],
    },
    EventFacts {
        event: "AUTH_RESULT",
        categories: &["AUTH"],
    },
    EventFacts {
        event: "AUTH_SUCCESS",
        categories: &["AUTH"],
    },
    EventFacts {
        event: "AUTH_WANTCREDENTIAL",
        categories: &["AUTH"],
    },
    EventFacts {
        event: "BOTDEFENSE_ACTION",
        categories: &["BOTDEFENSE", "HTTP"],
    },
    EventFacts {
        event: "BOTDEFENSE_REQUEST",
        categories: &["BOTDEFENSE", "HTTP"],
    },
    EventFacts {
        event: "CACHE_REQUEST",
        categories: &["CACHE"],
    },
    EventFacts {
        event: "CACHE_RESPONSE",
        categories: &["CACHE"],
    },
    EventFacts {
        event: "CACHE_UPDATE",
        categories: &["CACHE"],
    },
    EventFacts {
        event: "CATEGORY_MATCHED",
        categories: &["CATEGORY"],
    },
    EventFacts {
        event: "CLASSIFICATION_DETECTED",
        categories: &["CLASSIFICATION"],
    },
    EventFacts {
        event: "CLIENTSSL_CLIENTCERT",
        categories: &["SSL"],
    },
    EventFacts {
        event: "CLIENTSSL_CLIENTHELLO",
        categories: &["SSL"],
    },
    EventFacts {
        event: "CLIENTSSL_DATA",
        categories: &["SSL"],
    },
    EventFacts {
        event: "CLIENTSSL_HANDSHAKE",
        categories: &["SSL"],
    },
    EventFacts {
        event: "CLIENTSSL_PASSTHROUGH",
        categories: &["SSL"],
    },
    EventFacts {
        event: "CLIENTSSL_SERVERHELLO_SEND",
        categories: &["SSL"],
    },
    EventFacts {
        event: "CLIENT_ACCEPTED",
        categories: &["IP", "SCTP", "TCP", "UDP"],
    },
    EventFacts {
        event: "CLIENT_CLOSED",
        categories: &["IP", "SCTP", "TCP", "UDP"],
    },
    EventFacts {
        event: "CLIENT_DATA",
        categories: &["IP", "SCTP", "TCP", "UDP"],
    },
    EventFacts {
        event: "CONNECTOR_OPEN",
        categories: &["CONNECTOR", "IP", "TCP"],
    },
    EventFacts {
        event: "DIAMETER_EGRESS",
        categories: &["DIAMETER"],
    },
    EventFacts {
        event: "DIAMETER_INGRESS",
        categories: &["DIAMETER"],
    },
    EventFacts {
        event: "DIAMETER_RETRANSMISSION",
        categories: &["DIAMETER"],
    },
    EventFacts {
        event: "DNS_REQUEST",
        categories: &["DNS"],
    },
    EventFacts {
        event: "DNS_RESPONSE",
        categories: &["DNS"],
    },
    EventFacts {
        event: "FIX_HEADER",
        categories: &["FIX"],
    },
    EventFacts {
        event: "FIX_MESSAGE",
        categories: &["FIX"],
    },
    EventFacts {
        event: "FLOW_INIT",
        categories: &["IP", "SCTP", "TCP", "UDP"],
    },
    EventFacts {
        event: "GENERICMESSAGE_EGRESS",
        categories: &["GENERICMESSAGE"],
    },
    EventFacts {
        event: "GENERICMESSAGE_INGRESS",
        categories: &["GENERICMESSAGE"],
    },
    EventFacts {
        event: "GTP_GPDU_EGRESS",
        categories: &["GTP"],
    },
    EventFacts {
        event: "GTP_GPDU_INGRESS",
        categories: &["GTP"],
    },
    EventFacts {
        event: "GTP_PRIME_EGRESS",
        categories: &["GTP"],
    },
    EventFacts {
        event: "GTP_PRIME_INGRESS",
        categories: &["GTP"],
    },
    EventFacts {
        event: "GTP_SIGNALLING_EGRESS",
        categories: &["GTP"],
    },
    EventFacts {
        event: "GTP_SIGNALLING_INGRESS",
        categories: &["GTP"],
    },
    EventFacts {
        event: "HTML_COMMENT_MATCHED",
        categories: &["CLIENTSIDE", "HTTP"],
    },
    EventFacts {
        event: "HTML_TAG_MATCHED",
        categories: &["CLIENTSIDE", "HTTP"],
    },
    EventFacts {
        event: "HTTP_DISABLED",
        categories: &["IP", "TCP"],
    },
    EventFacts {
        event: "HTTP_PROXY_CONNECT",
        categories: &["HTTPPROXYCONNECT"],
    },
    EventFacts {
        event: "HTTP_PROXY_REQUEST",
        categories: &["HTTP"],
    },
    EventFacts {
        event: "HTTP_PROXY_RESPONSE",
        categories: &["HTTPPROXYCONNECT"],
    },
    EventFacts {
        event: "HTTP_REJECT",
        categories: &["IP", "TCP"],
    },
    EventFacts {
        event: "HTTP_REQUEST",
        categories: &["HTTP"],
    },
    EventFacts {
        event: "HTTP_REQUEST_DATA",
        categories: &["HTTP"],
    },
    EventFacts {
        event: "HTTP_REQUEST_RELEASE",
        categories: &["HTTP", "SERVERSIDE"],
    },
    EventFacts {
        event: "HTTP_REQUEST_SEND",
        categories: &["HTTP", "SERVERSIDE"],
    },
    EventFacts {
        event: "HTTP_RESPONSE",
        categories: &["HTTP", "SERVERSIDE"],
    },
    EventFacts {
        event: "HTTP_RESPONSE_CONTINUE",
        categories: &["HTTP", "SERVERSIDE"],
    },
    EventFacts {
        event: "HTTP_RESPONSE_DATA",
        categories: &["HTTP", "SERVERSIDE"],
    },
    EventFacts {
        event: "HTTP_RESPONSE_RELEASE",
        categories: &["HTTP"],
    },
    EventFacts {
        event: "ICAP_REQUEST",
        categories: &["ICAP"],
    },
    EventFacts {
        event: "ICAP_RESPONSE",
        categories: &["ICAP"],
    },
    EventFacts {
        event: "IN_DOSL7_ATTACK",
        categories: &["HTTP"],
    },
    EventFacts {
        event: "IVS_ENTRY_REQUEST",
        categories: &["IVS_ENTRY"],
    },
    EventFacts {
        event: "IVS_ENTRY_RESPONSE",
        categories: &["IVS_ENTRY"],
    },
    EventFacts {
        event: "JSON_REQUEST",
        categories: &["JSON"],
    },
    EventFacts {
        event: "JSON_REQUEST_ERROR",
        categories: &["JSON"],
    },
    EventFacts {
        event: "JSON_REQUEST_MISSING",
        categories: &["JSON"],
    },
    EventFacts {
        event: "JSON_RESPONSE",
        categories: &["JSON"],
    },
    EventFacts {
        event: "JSON_RESPONSE_ERROR",
        categories: &["JSON"],
    },
    EventFacts {
        event: "JSON_RESPONSE_MISSING",
        categories: &["JSON"],
    },
    EventFacts {
        event: "L7CHECK_CLIENT_DATA",
        categories: &["L7CHECK"],
    },
    EventFacts {
        event: "L7CHECK_SERVER_DATA",
        categories: &["L7CHECK"],
    },
    EventFacts {
        event: "LB_FAILED",
        categories: &["GLOBAL"],
    },
    EventFacts {
        event: "LB_QUEUED",
        categories: &["GLOBAL", "SERVERSIDE"],
    },
    EventFacts {
        event: "LB_SELECTED",
        categories: &["GLOBAL", "SERVERSIDE"],
    },
    EventFacts {
        event: "MQTT_CLIENT_DATA",
        categories: &["IP", "MQTT", "TCP"],
    },
    EventFacts {
        event: "MQTT_CLIENT_EGRESS",
        categories: &["IP", "MQTT", "TCP"],
    },
    EventFacts {
        event: "MQTT_CLIENT_INGRESS",
        categories: &["IP", "MQTT", "TCP"],
    },
    EventFacts {
        event: "MQTT_CLIENT_SHUTDOWN",
        categories: &["IP", "MQTT", "TCP"],
    },
    EventFacts {
        event: "MQTT_SERVER_DATA",
        categories: &["IP", "MQTT", "TCP"],
    },
    EventFacts {
        event: "MQTT_SERVER_EGRESS",
        categories: &["IP", "MQTT", "TCP"],
    },
    EventFacts {
        event: "MQTT_SERVER_INGRESS",
        categories: &["IP", "MQTT", "TCP"],
    },
    EventFacts {
        event: "MR_DATA",
        categories: &["MR"],
    },
    EventFacts {
        event: "MR_EGRESS",
        categories: &["MR"],
    },
    EventFacts {
        event: "MR_FAILED",
        categories: &["MR"],
    },
    EventFacts {
        event: "MR_INGRESS",
        categories: &["MR"],
    },
    EventFacts {
        event: "NAME_RESOLVED",
        categories: &["GLOBAL"],
    },
    EventFacts {
        event: "PCP_REQUEST",
        categories: &["PCP"],
    },
    EventFacts {
        event: "PCP_RESPONSE",
        categories: &["PCP"],
    },
    EventFacts {
        event: "PEM_POLICY",
        categories: &["PEM"],
    },
    EventFacts {
        event: "PEM_SUBS_SESS_CREATED",
        categories: &["PEM"],
    },
    EventFacts {
        event: "PEM_SUBS_SESS_DELETED",
        categories: &["PEM"],
    },
    EventFacts {
        event: "PEM_SUBS_SESS_UPDATED",
        categories: &["PEM"],
    },
    EventFacts {
        event: "PERSIST_DOWN",
        categories: &["GLOBAL"],
    },
    EventFacts {
        event: "PING_REQUEST_READY",
        categories: &["PING"],
    },
    EventFacts {
        event: "PING_RESPONSE_READY",
        categories: &["PING"],
    },
    EventFacts {
        event: "PROTOCOL_INSPECTION_MATCH",
        categories: &["PROTOCOL_INSPECTION"],
    },
    EventFacts {
        event: "QOE_PARSE_DONE",
        categories: &["QOE"],
    },
    EventFacts {
        event: "RADIUS_AAA_ACCT_REQUEST",
        categories: &["RADIUS_AAA"],
    },
    EventFacts {
        event: "RADIUS_AAA_ACCT_RESPONSE",
        categories: &["RADIUS_AAA"],
    },
    EventFacts {
        event: "RADIUS_AAA_AUTH_REQUEST",
        categories: &["RADIUS_AAA"],
    },
    EventFacts {
        event: "RADIUS_AAA_AUTH_RESPONSE",
        categories: &["RADIUS_AAA"],
    },
    EventFacts {
        event: "REWRITE_REQUEST",
        categories: &["CLIENTSIDE", "HTTP"],
    },
    EventFacts {
        event: "REWRITE_REQUEST_DONE",
        categories: &["CLIENTSIDE", "HTTP"],
    },
    EventFacts {
        event: "REWRITE_RESPONSE",
        categories: &["HTTP", "SERVERSIDE"],
    },
    EventFacts {
        event: "REWRITE_RESPONSE_DONE",
        categories: &["HTTP", "SERVERSIDE"],
    },
    EventFacts {
        event: "RTSP_REQUEST",
        categories: &["RTSP"],
    },
    EventFacts {
        event: "RTSP_REQUEST_DATA",
        categories: &["RTSP"],
    },
    EventFacts {
        event: "RTSP_RESPONSE",
        categories: &["CLIENTSIDE", "RTSP", "SERVERSIDE"],
    },
    EventFacts {
        event: "RTSP_RESPONSE_DATA",
        categories: &["RTSP"],
    },
    EventFacts {
        event: "RULE_INIT",
        categories: &["RULE_INIT_CATEGORY"],
    },
    EventFacts {
        event: "SA_PICKED",
        categories: &["IP", "SCTP", "TCP", "UDP"],
    },
    EventFacts {
        event: "SERVERSSL_CLIENTHELLO_SEND",
        categories: &["SSL"],
    },
    EventFacts {
        event: "SERVERSSL_DATA",
        categories: &["SERVERSIDE", "SSL"],
    },
    EventFacts {
        event: "SERVERSSL_HANDSHAKE",
        categories: &["SERVERSIDE", "SSL"],
    },
    EventFacts {
        event: "SERVERSSL_SERVERCERT",
        categories: &["SSL"],
    },
    EventFacts {
        event: "SERVERSSL_SERVERHELLO",
        categories: &["SSL"],
    },
    EventFacts {
        event: "SERVER_CLOSED",
        categories: &["IP", "SCTP", "SERVERSIDE", "TCP", "UDP"],
    },
    EventFacts {
        event: "SERVER_CONNECTED",
        categories: &["IP", "SCTP", "SERVERSIDE", "TCP", "UDP"],
    },
    EventFacts {
        event: "SERVER_DATA",
        categories: &["IP", "SCTP", "SERVERSIDE", "TCP", "UDP"],
    },
    EventFacts {
        event: "SERVER_INIT",
        categories: &["SERVERSIDE", "TCP"],
    },
    EventFacts {
        event: "SIP_REQUEST",
        categories: &["SIP", "SIPSESSION"],
    },
    EventFacts {
        event: "SIP_REQUEST_SEND",
        categories: &["SERVERSIDE", "SIP", "SIPSESSION"],
    },
    EventFacts {
        event: "SIP_RESPONSE",
        categories: &["SERVERSIDE", "SIP", "SIPSESSION"],
    },
    EventFacts {
        event: "SIP_RESPONSE_SEND",
        categories: &["SERVERSIDE", "SIP", "SIPSESSION"],
    },
    EventFacts {
        event: "SOCKS_REQUEST",
        categories: &["SOCKS"],
    },
    EventFacts {
        event: "SSE_RESPONSE",
        categories: &["SSE"],
    },
    EventFacts {
        event: "STREAM_MATCHED",
        categories: &["STREAM"],
    },
    EventFacts {
        event: "TAP_REQUEST",
        categories: &["HTTP"],
    },
    EventFacts {
        event: "TDS_REQUEST",
        categories: &["TDS"],
    },
    EventFacts {
        event: "TDS_RESPONSE",
        categories: &["TDS"],
    },
    EventFacts {
        event: "USER_REQUEST",
        categories: &["SERVERSIDE", "TCP"],
    },
    EventFacts {
        event: "USER_RESPONSE",
        categories: &["TCP"],
    },
    EventFacts {
        event: "WS_CLIENT_DATA",
        categories: &["HTTP", "IP", "TCP", "WEBSOCKET"],
    },
    EventFacts {
        event: "WS_CLIENT_FRAME",
        categories: &["HTTP", "IP", "TCP", "WEBSOCKET"],
    },
    EventFacts {
        event: "WS_CLIENT_FRAME_DONE",
        categories: &["HTTP", "IP", "TCP", "WEBSOCKET"],
    },
    EventFacts {
        event: "WS_REQUEST",
        categories: &["HTTP", "IP", "TCP", "WEBSOCKET"],
    },
    EventFacts {
        event: "WS_RESPONSE",
        categories: &["HTTP", "IP", "TCP", "WEBSOCKET"],
    },
    EventFacts {
        event: "WS_SERVER_DATA",
        categories: &["HTTP", "IP", "TCP", "WEBSOCKET"],
    },
    EventFacts {
        event: "WS_SERVER_FRAME",
        categories: &["HTTP", "IP", "TCP", "WEBSOCKET"],
    },
    EventFacts {
        event: "WS_SERVER_FRAME_DONE",
        categories: &["HTTP", "IP", "TCP", "WEBSOCKET"],
    },
    EventFacts {
        event: "XML_CONTENT_BASED_ROUTING",
        categories: &["HTTP"],
    },
];
