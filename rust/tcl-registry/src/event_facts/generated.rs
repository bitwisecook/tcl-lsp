// @generated — DO NOT EDIT. Regenerate to refresh.

use super::EventFacts;

/// Per-event structural facts: the protocol categories an event belongs
/// to and the profile types that enable it.
pub static EVENT_FACTS: &[EventFacts] = &[
    EventFacts {
        event: "ACCESS_ACL_ALLOWED",
        categories: &["ACCESS"],
        profiles: &["ACCESS"],
    },
    EventFacts {
        event: "ACCESS_ACL_DENIED",
        categories: &["ACCESS"],
        profiles: &["ACCESS"],
    },
    EventFacts {
        event: "ACCESS_PER_REQUEST_AGENT_EVENT",
        categories: &["HTTP", "IP", "SSL", "TCP"],
        profiles: &[],
    },
    EventFacts {
        event: "ACCESS_POLICY_AGENT_EVENT",
        categories: &["ACCESS"],
        profiles: &["ACCESS"],
    },
    EventFacts {
        event: "ACCESS_POLICY_COMPLETED",
        categories: &["ACCESS"],
        profiles: &["ACCESS"],
    },
    EventFacts {
        event: "ACCESS_SAML_ASSERTION",
        categories: &["ACCESS"],
        profiles: &[],
    },
    EventFacts {
        event: "ACCESS_SAML_AUTHN",
        categories: &["ACCESS"],
        profiles: &[],
    },
    EventFacts {
        event: "ACCESS_SAML_SLO_REQ",
        categories: &["ACCESS"],
        profiles: &[],
    },
    EventFacts {
        event: "ACCESS_SAML_SLO_RESP",
        categories: &["ACCESS"],
        profiles: &[],
    },
    EventFacts {
        event: "ACCESS_SESSION_CLOSED",
        categories: &["ACCESS_GLOBAL"],
        profiles: &[],
    },
    EventFacts {
        event: "ACCESS_SESSION_STARTED",
        categories: &["ACCESS"],
        profiles: &["ACCESS"],
    },
    EventFacts {
        event: "ADAPT_REQUEST_HEADERS",
        categories: &["ADAPT"],
        profiles: &["REQUESTADAPT"],
    },
    EventFacts {
        event: "ADAPT_REQUEST_RESULT",
        categories: &["ADAPT"],
        profiles: &["REQUESTADAPT"],
    },
    EventFacts {
        event: "ADAPT_RESPONSE_HEADERS",
        categories: &["ADAPT"],
        profiles: &["RESPONSEADAPT"],
    },
    EventFacts {
        event: "ADAPT_RESPONSE_RESULT",
        categories: &["ADAPT"],
        profiles: &["RESPONSEADAPT"],
    },
    EventFacts {
        event: "ANTIFRAUD_ALERT",
        categories: &["CLIENTSIDE", "HTTP"],
        profiles: &["ANTIFRAUD"],
    },
    EventFacts {
        event: "ANTIFRAUD_LOGIN",
        categories: &["HTTP", "SERVERSIDE"],
        profiles: &["ANTIFRAUD"],
    },
    EventFacts {
        event: "ASM_REQUEST_BLOCKING",
        categories: &["HTTP"],
        profiles: &["FASTHTTP", "HTTP"],
    },
    EventFacts {
        event: "ASM_REQUEST_DONE",
        categories: &["HTTP"],
        profiles: &["FASTHTTP", "HTTP"],
    },
    EventFacts {
        event: "ASM_REQUEST_VIOLATION",
        categories: &["HTTP"],
        profiles: &["FASTHTTP", "HTTP"],
    },
    EventFacts {
        event: "ASM_RESPONSE_LOGIN",
        categories: &["HTTP"],
        profiles: &["FASTHTTP", "HTTP"],
    },
    EventFacts {
        event: "ASM_RESPONSE_VIOLATION",
        categories: &["HTTP"],
        profiles: &["FASTHTTP", "HTTP"],
    },
    EventFacts {
        event: "AUTH_ERROR",
        categories: &["AUTH"],
        profiles: &["AUTH"],
    },
    EventFacts {
        event: "AUTH_FAILURE",
        categories: &["AUTH"],
        profiles: &["AUTH"],
    },
    EventFacts {
        event: "AUTH_RESULT",
        categories: &["AUTH"],
        profiles: &["AUTH"],
    },
    EventFacts {
        event: "AUTH_SUCCESS",
        categories: &["AUTH"],
        profiles: &["AUTH"],
    },
    EventFacts {
        event: "AUTH_WANTCREDENTIAL",
        categories: &["AUTH"],
        profiles: &["AUTH"],
    },
    EventFacts {
        event: "BOTDEFENSE_ACTION",
        categories: &["BOTDEFENSE", "HTTP"],
        profiles: &["BOTDEFENSE"],
    },
    EventFacts {
        event: "BOTDEFENSE_REQUEST",
        categories: &["BOTDEFENSE", "HTTP"],
        profiles: &["BOTDEFENSE"],
    },
    EventFacts {
        event: "CACHE_REQUEST",
        categories: &["CACHE"],
        profiles: &["WEBACCELERATION"],
    },
    EventFacts {
        event: "CACHE_RESPONSE",
        categories: &["CACHE"],
        profiles: &["WEBACCELERATION"],
    },
    EventFacts {
        event: "CACHE_UPDATE",
        categories: &["CACHE"],
        profiles: &["WEBACCELERATION"],
    },
    EventFacts {
        event: "CATEGORY_MATCHED",
        categories: &["CATEGORY"],
        profiles: &["ACCESS", "HTTP"],
    },
    EventFacts {
        event: "CLASSIFICATION_DETECTED",
        categories: &["CLASSIFICATION"],
        profiles: &["CLASSIFICATION"],
    },
    EventFacts {
        event: "CLIENTSSL_CLIENTCERT",
        categories: &["SSL"],
        profiles: &["CLIENTSSL", "PERSIST"],
    },
    EventFacts {
        event: "CLIENTSSL_CLIENTHELLO",
        categories: &["SSL"],
        profiles: &["CLIENTSSL", "PERSIST"],
    },
    EventFacts {
        event: "CLIENTSSL_DATA",
        categories: &["SSL"],
        profiles: &["CLIENTSSL"],
    },
    EventFacts {
        event: "CLIENTSSL_HANDSHAKE",
        categories: &["SSL"],
        profiles: &["CLIENTSSL"],
    },
    EventFacts {
        event: "CLIENTSSL_PASSTHROUGH",
        categories: &["SSL"],
        profiles: &["CLIENTSSL"],
    },
    EventFacts {
        event: "CLIENTSSL_SERVERHELLO_SEND",
        categories: &["SSL"],
        profiles: &["CLIENTSSL"],
    },
    EventFacts {
        event: "CLIENT_ACCEPTED",
        categories: &["IP", "SCTP", "TCP", "UDP"],
        profiles: &[],
    },
    EventFacts {
        event: "CLIENT_CLOSED",
        categories: &["IP", "SCTP", "TCP", "UDP"],
        profiles: &[],
    },
    EventFacts {
        event: "CLIENT_DATA",
        categories: &["IP", "SCTP", "TCP", "UDP"],
        profiles: &[],
    },
    EventFacts {
        event: "CONNECTOR_OPEN",
        categories: &["CONNECTOR", "IP", "TCP"],
        profiles: &[],
    },
    EventFacts {
        event: "DIAMETER_EGRESS",
        categories: &["DIAMETER"],
        profiles: &["DIAMETER", "DIAMETERSESSION", "DIAMETER_ENDPOINT"],
    },
    EventFacts {
        event: "DIAMETER_INGRESS",
        categories: &["DIAMETER"],
        profiles: &["DIAMETER", "DIAMETERSESSION", "DIAMETER_ENDPOINT"],
    },
    EventFacts {
        event: "DIAMETER_RETRANSMISSION",
        categories: &["DIAMETER"],
        profiles: &["DIAMETERSESSION"],
    },
    EventFacts {
        event: "DNS_REQUEST",
        categories: &["DNS"],
        profiles: &["DNS"],
    },
    EventFacts {
        event: "DNS_RESPONSE",
        categories: &["DNS"],
        profiles: &["DNS"],
    },
    EventFacts {
        event: "FIX_HEADER",
        categories: &["FIX"],
        profiles: &["FIX"],
    },
    EventFacts {
        event: "FIX_MESSAGE",
        categories: &["FIX"],
        profiles: &["FIX"],
    },
    EventFacts {
        event: "FLOW_INIT",
        categories: &["IP", "SCTP", "TCP", "UDP"],
        profiles: &[],
    },
    EventFacts {
        event: "GENERICMESSAGE_EGRESS",
        categories: &["GENERICMESSAGE"],
        profiles: &["GENERICMSG"],
    },
    EventFacts {
        event: "GENERICMESSAGE_INGRESS",
        categories: &["GENERICMESSAGE"],
        profiles: &["GENERICMSG"],
    },
    EventFacts {
        event: "GTP_GPDU_EGRESS",
        categories: &["GTP"],
        profiles: &["GTP"],
    },
    EventFacts {
        event: "GTP_GPDU_INGRESS",
        categories: &["GTP"],
        profiles: &["GTP"],
    },
    EventFacts {
        event: "GTP_PRIME_EGRESS",
        categories: &["GTP"],
        profiles: &["GTP"],
    },
    EventFacts {
        event: "GTP_PRIME_INGRESS",
        categories: &["GTP"],
        profiles: &["GTP"],
    },
    EventFacts {
        event: "GTP_SIGNALLING_EGRESS",
        categories: &["GTP"],
        profiles: &["GTP"],
    },
    EventFacts {
        event: "GTP_SIGNALLING_INGRESS",
        categories: &["GTP"],
        profiles: &["GTP"],
    },
    EventFacts {
        event: "HTML_COMMENT_MATCHED",
        categories: &["CLIENTSIDE", "HTTP"],
        profiles: &["HTTP"],
    },
    EventFacts {
        event: "HTML_TAG_MATCHED",
        categories: &["CLIENTSIDE", "HTTP"],
        profiles: &["HTTP"],
    },
    EventFacts {
        event: "HTTP_DISABLED",
        categories: &["IP", "TCP"],
        profiles: &["HTTP"],
    },
    EventFacts {
        event: "HTTP_PROXY_CONNECT",
        categories: &["HTTPPROXYCONNECT"],
        profiles: &["HTTP_PROXY_CONNECT"],
    },
    EventFacts {
        event: "HTTP_PROXY_REQUEST",
        categories: &["HTTP"],
        profiles: &["HTTP"],
    },
    EventFacts {
        event: "HTTP_PROXY_RESPONSE",
        categories: &["HTTPPROXYCONNECT"],
        profiles: &["HTTP_PROXY_CONNECT"],
    },
    EventFacts {
        event: "HTTP_REJECT",
        categories: &["IP", "TCP"],
        profiles: &[],
    },
    EventFacts {
        event: "HTTP_REQUEST",
        categories: &["HTTP"],
        profiles: &["FASTHTTP", "HTTP"],
    },
    EventFacts {
        event: "HTTP_REQUEST_DATA",
        categories: &["HTTP"],
        profiles: &["HTTP"],
    },
    EventFacts {
        event: "HTTP_REQUEST_RELEASE",
        categories: &["HTTP", "SERVERSIDE"],
        profiles: &["HTTP"],
    },
    EventFacts {
        event: "HTTP_REQUEST_SEND",
        categories: &["HTTP", "SERVERSIDE"],
        profiles: &["HTTP"],
    },
    EventFacts {
        event: "HTTP_RESPONSE",
        categories: &["HTTP", "SERVERSIDE"],
        profiles: &["HTTP"],
    },
    EventFacts {
        event: "HTTP_RESPONSE_CONTINUE",
        categories: &["HTTP", "SERVERSIDE"],
        profiles: &["HTTP"],
    },
    EventFacts {
        event: "HTTP_RESPONSE_DATA",
        categories: &["HTTP", "SERVERSIDE"],
        profiles: &["HTTP"],
    },
    EventFacts {
        event: "HTTP_RESPONSE_RELEASE",
        categories: &["HTTP"],
        profiles: &["HTTP"],
    },
    EventFacts {
        event: "ICAP_REQUEST",
        categories: &["ICAP"],
        profiles: &["ICAP"],
    },
    EventFacts {
        event: "ICAP_RESPONSE",
        categories: &["ICAP"],
        profiles: &["ICAP"],
    },
    EventFacts {
        event: "IN_DOSL7_ATTACK",
        categories: &["HTTP"],
        profiles: &["FASTHTTP", "HTTP"],
    },
    EventFacts {
        event: "IVS_ENTRY_REQUEST",
        categories: &["IVS_ENTRY"],
        profiles: &[],
    },
    EventFacts {
        event: "IVS_ENTRY_RESPONSE",
        categories: &["IVS_ENTRY"],
        profiles: &[],
    },
    EventFacts {
        event: "JSON_REQUEST",
        categories: &["JSON"],
        profiles: &["JSON"],
    },
    EventFacts {
        event: "JSON_REQUEST_ERROR",
        categories: &["JSON"],
        profiles: &["JSON"],
    },
    EventFacts {
        event: "JSON_REQUEST_MISSING",
        categories: &["JSON"],
        profiles: &["JSON"],
    },
    EventFacts {
        event: "JSON_RESPONSE",
        categories: &["JSON"],
        profiles: &["JSON"],
    },
    EventFacts {
        event: "JSON_RESPONSE_ERROR",
        categories: &["JSON"],
        profiles: &["JSON"],
    },
    EventFacts {
        event: "JSON_RESPONSE_MISSING",
        categories: &["JSON"],
        profiles: &["JSON"],
    },
    EventFacts {
        event: "L7CHECK_CLIENT_DATA",
        categories: &["L7CHECK"],
        profiles: &[],
    },
    EventFacts {
        event: "L7CHECK_SERVER_DATA",
        categories: &["L7CHECK"],
        profiles: &[],
    },
    EventFacts {
        event: "LB_FAILED",
        categories: &["GLOBAL"],
        profiles: &[],
    },
    EventFacts {
        event: "LB_QUEUED",
        categories: &["GLOBAL", "SERVERSIDE"],
        profiles: &[],
    },
    EventFacts {
        event: "LB_SELECTED",
        categories: &["GLOBAL", "SERVERSIDE"],
        profiles: &[],
    },
    EventFacts {
        event: "MQTT_CLIENT_DATA",
        categories: &["IP", "MQTT", "TCP"],
        profiles: &[],
    },
    EventFacts {
        event: "MQTT_CLIENT_EGRESS",
        categories: &["IP", "MQTT", "TCP"],
        profiles: &[],
    },
    EventFacts {
        event: "MQTT_CLIENT_INGRESS",
        categories: &["IP", "MQTT", "TCP"],
        profiles: &[],
    },
    EventFacts {
        event: "MQTT_CLIENT_SHUTDOWN",
        categories: &["IP", "MQTT", "TCP"],
        profiles: &[],
    },
    EventFacts {
        event: "MQTT_SERVER_DATA",
        categories: &["IP", "MQTT", "TCP"],
        profiles: &[],
    },
    EventFacts {
        event: "MQTT_SERVER_EGRESS",
        categories: &["IP", "MQTT", "TCP"],
        profiles: &[],
    },
    EventFacts {
        event: "MQTT_SERVER_INGRESS",
        categories: &["IP", "MQTT", "TCP"],
        profiles: &[],
    },
    EventFacts {
        event: "MR_DATA",
        categories: &["MR"],
        profiles: &[],
    },
    EventFacts {
        event: "MR_EGRESS",
        categories: &["MR"],
        profiles: &[],
    },
    EventFacts {
        event: "MR_FAILED",
        categories: &["MR"],
        profiles: &[],
    },
    EventFacts {
        event: "MR_INGRESS",
        categories: &["MR"],
        profiles: &[],
    },
    EventFacts {
        event: "NAME_RESOLVED",
        categories: &["GLOBAL"],
        profiles: &[],
    },
    EventFacts {
        event: "PCP_REQUEST",
        categories: &["PCP"],
        profiles: &[],
    },
    EventFacts {
        event: "PCP_RESPONSE",
        categories: &["PCP"],
        profiles: &[],
    },
    EventFacts {
        event: "PEM_POLICY",
        categories: &["PEM"],
        profiles: &[],
    },
    EventFacts {
        event: "PEM_SUBS_SESS_CREATED",
        categories: &["PEM"],
        profiles: &[],
    },
    EventFacts {
        event: "PEM_SUBS_SESS_DELETED",
        categories: &["PEM"],
        profiles: &[],
    },
    EventFacts {
        event: "PEM_SUBS_SESS_UPDATED",
        categories: &["PEM"],
        profiles: &[],
    },
    EventFacts {
        event: "PERSIST_DOWN",
        categories: &["GLOBAL"],
        profiles: &[],
    },
    EventFacts {
        event: "PING_REQUEST_READY",
        categories: &["PING"],
        profiles: &["HTTP"],
    },
    EventFacts {
        event: "PING_RESPONSE_READY",
        categories: &["PING"],
        profiles: &["HTTP"],
    },
    EventFacts {
        event: "PROTOCOL_INSPECTION_MATCH",
        categories: &["PROTOCOL_INSPECTION"],
        profiles: &["IPS"],
    },
    EventFacts {
        event: "QOE_PARSE_DONE",
        categories: &["QOE"],
        profiles: &["QOE"],
    },
    EventFacts {
        event: "RADIUS_AAA_ACCT_REQUEST",
        categories: &["RADIUS_AAA"],
        profiles: &["RADIUS_AAA"],
    },
    EventFacts {
        event: "RADIUS_AAA_ACCT_RESPONSE",
        categories: &["RADIUS_AAA"],
        profiles: &["RADIUS_AAA"],
    },
    EventFacts {
        event: "RADIUS_AAA_AUTH_REQUEST",
        categories: &["RADIUS_AAA"],
        profiles: &["RADIUS_AAA"],
    },
    EventFacts {
        event: "RADIUS_AAA_AUTH_RESPONSE",
        categories: &["RADIUS_AAA"],
        profiles: &["RADIUS_AAA"],
    },
    EventFacts {
        event: "REWRITE_REQUEST",
        categories: &["CLIENTSIDE", "HTTP"],
        profiles: &["HTTP"],
    },
    EventFacts {
        event: "REWRITE_REQUEST_DONE",
        categories: &["CLIENTSIDE", "HTTP"],
        profiles: &["HTTP"],
    },
    EventFacts {
        event: "REWRITE_RESPONSE",
        categories: &["HTTP", "SERVERSIDE"],
        profiles: &["HTTP"],
    },
    EventFacts {
        event: "REWRITE_RESPONSE_DONE",
        categories: &["HTTP", "SERVERSIDE"],
        profiles: &["HTTP"],
    },
    EventFacts {
        event: "RTSP_REQUEST",
        categories: &["RTSP"],
        profiles: &[],
    },
    EventFacts {
        event: "RTSP_REQUEST_DATA",
        categories: &["RTSP"],
        profiles: &[],
    },
    EventFacts {
        event: "RTSP_RESPONSE",
        categories: &["CLIENTSIDE", "RTSP", "SERVERSIDE"],
        profiles: &[],
    },
    EventFacts {
        event: "RTSP_RESPONSE_DATA",
        categories: &["RTSP"],
        profiles: &[],
    },
    EventFacts {
        event: "RULE_INIT",
        categories: &["RULE_INIT_CATEGORY"],
        profiles: &[],
    },
    EventFacts {
        event: "SA_PICKED",
        categories: &["IP", "SCTP", "TCP", "UDP"],
        profiles: &[],
    },
    EventFacts {
        event: "SERVERSSL_CLIENTHELLO_SEND",
        categories: &["SSL"],
        profiles: &["SERVERSSL"],
    },
    EventFacts {
        event: "SERVERSSL_DATA",
        categories: &["SERVERSIDE", "SSL"],
        profiles: &["SERVERSSL"],
    },
    EventFacts {
        event: "SERVERSSL_HANDSHAKE",
        categories: &["SERVERSIDE", "SSL"],
        profiles: &["PERSIST", "SERVERSSL"],
    },
    EventFacts {
        event: "SERVERSSL_SERVERCERT",
        categories: &["SSL"],
        profiles: &["SERVERSSL"],
    },
    EventFacts {
        event: "SERVERSSL_SERVERHELLO",
        categories: &["SSL"],
        profiles: &["PERSIST", "SERVERSSL"],
    },
    EventFacts {
        event: "SERVER_CLOSED",
        categories: &["IP", "SCTP", "SERVERSIDE", "TCP", "UDP"],
        profiles: &[],
    },
    EventFacts {
        event: "SERVER_CONNECTED",
        categories: &["IP", "SCTP", "SERVERSIDE", "TCP", "UDP"],
        profiles: &[],
    },
    EventFacts {
        event: "SERVER_DATA",
        categories: &["IP", "SCTP", "SERVERSIDE", "TCP", "UDP"],
        profiles: &[],
    },
    EventFacts {
        event: "SERVER_INIT",
        categories: &["SERVERSIDE", "TCP"],
        profiles: &[],
    },
    EventFacts {
        event: "SIP_REQUEST",
        categories: &["SIP", "SIPSESSION"],
        profiles: &["SIP", "SIPROUTER", "SIPSESSION"],
    },
    EventFacts {
        event: "SIP_REQUEST_SEND",
        categories: &["SERVERSIDE", "SIP", "SIPSESSION"],
        profiles: &["SIP", "SIPROUTER", "SIPSESSION"],
    },
    EventFacts {
        event: "SIP_RESPONSE",
        categories: &["SERVERSIDE", "SIP", "SIPSESSION"],
        profiles: &["SIP", "SIPROUTER", "SIPSESSION"],
    },
    EventFacts {
        event: "SIP_RESPONSE_SEND",
        categories: &["SERVERSIDE", "SIP", "SIPSESSION"],
        profiles: &["SIP", "SIPROUTER", "SIPSESSION"],
    },
    EventFacts {
        event: "SOCKS_REQUEST",
        categories: &["SOCKS"],
        profiles: &["SOCKS"],
    },
    EventFacts {
        event: "SSE_RESPONSE",
        categories: &["SSE"],
        profiles: &["SSE"],
    },
    EventFacts {
        event: "STREAM_MATCHED",
        categories: &["STREAM"],
        profiles: &["STREAM"],
    },
    EventFacts {
        event: "TAP_REQUEST",
        categories: &["HTTP"],
        profiles: &[],
    },
    EventFacts {
        event: "TDS_REQUEST",
        categories: &["TDS"],
        profiles: &["MSSQL"],
    },
    EventFacts {
        event: "TDS_RESPONSE",
        categories: &["TDS"],
        profiles: &["MSSQL"],
    },
    EventFacts {
        event: "USER_REQUEST",
        categories: &["SERVERSIDE", "TCP"],
        profiles: &[],
    },
    EventFacts {
        event: "USER_RESPONSE",
        categories: &["TCP"],
        profiles: &[],
    },
    EventFacts {
        event: "WS_CLIENT_DATA",
        categories: &["HTTP", "IP", "TCP", "WEBSOCKET"],
        profiles: &[],
    },
    EventFacts {
        event: "WS_CLIENT_FRAME",
        categories: &["HTTP", "IP", "TCP", "WEBSOCKET"],
        profiles: &[],
    },
    EventFacts {
        event: "WS_CLIENT_FRAME_DONE",
        categories: &["HTTP", "IP", "TCP", "WEBSOCKET"],
        profiles: &[],
    },
    EventFacts {
        event: "WS_REQUEST",
        categories: &["HTTP", "IP", "TCP", "WEBSOCKET"],
        profiles: &[],
    },
    EventFacts {
        event: "WS_RESPONSE",
        categories: &["HTTP", "IP", "TCP", "WEBSOCKET"],
        profiles: &[],
    },
    EventFacts {
        event: "WS_SERVER_DATA",
        categories: &["HTTP", "IP", "TCP", "WEBSOCKET"],
        profiles: &[],
    },
    EventFacts {
        event: "WS_SERVER_FRAME",
        categories: &["HTTP", "IP", "TCP", "WEBSOCKET"],
        profiles: &[],
    },
    EventFacts {
        event: "WS_SERVER_FRAME_DONE",
        categories: &["HTTP", "IP", "TCP", "WEBSOCKET"],
        profiles: &[],
    },
    EventFacts {
        event: "XML_CONTENT_BASED_ROUTING",
        categories: &["HTTP"],
        profiles: &["FASTHTTP", "HTTP"],
    },
];
