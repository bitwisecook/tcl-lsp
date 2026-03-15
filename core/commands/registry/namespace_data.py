"""Namespace model data tables and pure functions.

This module contains all static data for the namespace model:

- **EVENT_PROPS** (247 entries): Per-event protocol stack properties.
- **EVENT_DESCRIPTIONS** (~200 entries): Human-readable event summaries.
- **MASTER_ORDER** (110 entries): Canonical event firing order.
- **FLOW_CHAINS** (7 chains): Event flow chains per profile combination.
- **ONCE_PER_CONNECTION / PER_REQUEST**: Event multiplicity sets.
- **PROFILE_SPECS**: Profile type metadata with capabilities.
- **PROTOCOL_NAMESPACE_SPECS**: Protocol command namespace availability.
- **MODIFICATION_SPECS**: Commands that change the profile stack.

Also contains pure functions that operate on these data tables:

- Validation helpers: event_satisfies(), missing_requirements_description(),
  profile_info_description()
- Lookup helpers: get_event_props(), events_matching(), get_event_description(),
  get_event_detail()
- Side label helpers: event_side_label(), event_side_label_short()
- Ordering helpers: order_events(), order_events_for_file(), event_index(),
  events_before(), events_after()
- Multiplicity helpers: is_per_request(), is_once_per_connection(),
  variable_scope_note()
- Flow chain helpers: chain_for_profiles(), chain_event_names(),
  compatible_connection_predecessors()
- File scanning: scan_file_events(), parse_profile_directive(),
  infer_profiles_from_events(), compute_file_profiles()

This module imports only from namespace_models (same package) and stdlib.
No registry imports -- safe for command definition files.
"""

from __future__ import annotations

import re

from .namespace_models import (
    EventProps,
    EventRequires,
    FlowChain,
    FlowStep,
    ProfileSpec,
    ProtocolNamespaceSpec,
    StackModification,
)

# EVENT_PROPS table (247 entries)

_TCP = "tcp"
_UDP = "udp"
_TCP_UDP = ("tcp", "udp")

_HP = frozenset({"HTTP", "FASTHTTP"})
_H = frozenset({"HTTP"})
_CS = frozenset({"CLIENTSSL", "SSL_PERSISTENCE"})
_SS = frozenset({"SERVERSSL", "SSL_PERSISTENCE"})
_HTTP_PROXY = frozenset({"HTTP", "HTTP_PROXY_CONNECT"})
_DNS = frozenset({"DNS"})
_SIP = frozenset({"SIP", "SIPROUTER", "SIPSESSION"})
_AUTH = frozenset({"AUTH"})
_ACCESS = frozenset({"ACCESS"})
_STREAM = frozenset({"STREAM"})
_FIX = frozenset({"FIX"})
_DIAMETER = frozenset({"DIAMETER", "DIAMETERSESSION", "DIAMETER_ENDPOINT"})
_DIAMETER_RETRANS = frozenset({"DIAMETER", "DIAMETERSESSION"})
_MQTT = frozenset({"MQTT"})
_ICAP = frozenset({"ICAP"})
_BOTDEFENSE = frozenset({"BOTDEFENSE"})
_ANTIFRAUD = frozenset({"ANTIFRAUD"})
_WEBACCEL = frozenset({"WEBACCELERATION"})
_CLASSIFICATION = frozenset({"CLASSIFICATION"})
_GENERICMSG = frozenset({"GENERICMSG"})
_MR = frozenset({"MR"})
_ADAPT_REQ = frozenset({"REQUESTADAPT"})
_ADAPT_RESP = frozenset({"RESPONSEADAPT"})
_PEM = frozenset({"PEM"})
# Compound profile sets: sub-feature events carry both parent + own profile.
_ASM = frozenset({"ASM", "HTTP", "FASTHTTP"})
_ASM_RESP = frozenset({"ASM", "HTTP", "FASTHTTP"})
_WS = frozenset({"WS", "HTTP"})
_HTML = frozenset({"HTML", "HTTP"})
_CACHE = frozenset({"CACHE", "WEBACCELERATION"})
_DOSL7 = frozenset({"DOSL7", "HTTP", "FASTHTTP"})
_SOCKS = frozenset({"SOCKS"})
_TAP = frozenset({"TAP"})
_CATEGORY = frozenset({"CATEGORY", "HTTP", "ACCESS"})
_PCP = frozenset({"PCP"})
_TDS = frozenset({"TDS", "MSSQL"})
_SSE = frozenset({"SSE"})
_NAME = frozenset({"NAME"})
_PROTO_INSPECT = frozenset({"PROTOCOL_INSPECTION", "IPS"})
_REWRITE = frozenset({"REWRITE", "HTTP"})
_QOE = frozenset({"QOE"})
_L7CHECK = frozenset({"L7CHECK"})
_IVS = frozenset({"IVS_ENTRY"})
_RTSP = frozenset({"RTSP"})
_FLOW = frozenset({"FLOW"})
_DATAGRAM = frozenset({"DATAGRAM"})
_RADIUS = frozenset({"RADIUS", "RADIUS_AAA"})
_CONNECTOR = frozenset({"CONNECTOR"})
_JSON = frozenset({"JSON"})
_XML = frozenset({"XML", "HTTP", "FASTHTTP"})
_GTP = frozenset({"GTP"})
_AVR = frozenset({"AVR"})
_ECA = frozenset({"ECA"})


def _ep(
    client_side: bool = False,
    server_side: bool = False,
    transport: str | None = None,
    profiles: frozenset[str] | None = None,
    *,
    flow: bool = True,
    deprecated: bool = False,
    hot: bool = False,
    common: bool = False,
) -> EventProps:
    return EventProps(
        client_side=client_side,
        server_side=server_side,
        transport=transport,
        implied_profiles=profiles or frozenset(),
        flow=flow,
        deprecated=deprecated,
        hot=hot,
        common=common,
    )


# fmt: off
EVENT_PROPS: dict[str, EventProps] = {
    # Lifecycle / no connection
    "RULE_INIT":            _ep(flow=False, common=True),
    "PERSIST_DOWN":         _ep(flow=False),

    # L4 client-side (TCP or UDP)
    "CLIENT_ACCEPTED":      _ep(client_side=True, transport=_TCP_UDP, hot=True, common=True),
    "CLIENT_DATA":          _ep(client_side=True, transport=_TCP_UDP),
    "CLIENT_CLOSED":        _ep(client_side=True, transport=_TCP_UDP, common=True),

    # Load-balancing / server init (TCP or UDP)
    "LB_SELECTED":          _ep(client_side=True, server_side=True, transport=_TCP_UDP, common=True),
    "LB_FAILED":            _ep(client_side=True, transport=_TCP_UDP, common=True),
    "LB_QUEUED":            _ep(client_side=True, server_side=True, transport=_TCP_UDP),
    "SERVER_INIT":          _ep(client_side=True, server_side=True, transport=_TCP_UDP),
    "SA_PICKED":            _ep(client_side=True, transport=_TCP_UDP),

    # L4 server-side (TCP or UDP)
    "SERVER_CONNECTED":     _ep(client_side=True, server_side=True, transport=_TCP_UDP, hot=True, common=True),
    "SERVER_DATA":          _ep(client_side=True, server_side=True, transport=_TCP_UDP),
    "SERVER_CLOSED":        _ep(client_side=True, server_side=True, transport=_TCP_UDP, common=True),

    # HTTP
    "HTTP_REQUEST":             _ep(client_side=True, transport=_TCP, profiles=_HP, hot=True, common=True),
    "HTTP_REQUEST_DATA":        _ep(client_side=True, transport=_TCP, profiles=_H, hot=True, common=True),
    "HTTP_REQUEST_SEND":        _ep(client_side=True, server_side=True, transport=_TCP, profiles=_H),
    "HTTP_REQUEST_RELEASE":     _ep(client_side=True, server_side=True, transport=_TCP, profiles=_H),
    "HTTP_RESPONSE":            _ep(client_side=True, server_side=True, transport=_TCP, profiles=_HP, hot=True, common=True),
    "HTTP_RESPONSE_DATA":       _ep(client_side=True, server_side=True, transport=_TCP, profiles=_H, hot=True, common=True),
    "HTTP_RESPONSE_CONTINUE":   _ep(client_side=True, server_side=True, transport=_TCP, profiles=_H),
    "HTTP_RESPONSE_RELEASE":    _ep(client_side=True, server_side=True, transport=_TCP, profiles=_H),
    "HTTP_DISABLED":            _ep(client_side=True, transport=_TCP, profiles=_H),
    "HTTP_REJECT":              _ep(client_side=True, transport=_TCP, profiles=_H),
    "HTTP_PROXY_REQUEST":       _ep(client_side=True, transport=_TCP, profiles=_HTTP_PROXY),
    "HTTP_PROXY_CONNECT":       _ep(client_side=True, transport=_TCP, profiles=_HTTP_PROXY),
    "HTTP_PROXY_RESPONSE":      _ep(client_side=True, server_side=True, transport=_TCP, profiles=_HTTP_PROXY),
    "HTTP_CLASS_FAILED":        _ep(client_side=True, transport=_TCP, profiles=_H, deprecated=True),
    "HTTP_CLASS_SELECTED":      _ep(client_side=True, transport=_TCP, profiles=_H, deprecated=True),

    # TLS (client-side)
    "CLIENTSSL_CLIENTHELLO":        _ep(client_side=True, transport=_TCP, profiles=_CS),
    "CLIENTSSL_CLIENTCERT":         _ep(client_side=True, transport=_TCP, profiles=_CS),
    "CLIENTSSL_HANDSHAKE":          _ep(client_side=True, transport=_TCP, profiles=_CS, common=True),
    "CLIENTSSL_SERVERHELLO_SEND":   _ep(client_side=True, transport=_TCP, profiles=_CS),
    "CLIENTSSL_DATA":               _ep(client_side=True, transport=_TCP, profiles=_CS),
    "CLIENTSSL_PASSTHROUGH":        _ep(client_side=True, transport=_TCP, profiles=_CS),

    # TLS (server-side)
    "SERVERSSL_CLIENTHELLO_SEND":   _ep(client_side=True, server_side=True, transport=_TCP, profiles=_SS),
    "SERVERSSL_SERVERHELLO":        _ep(client_side=True, server_side=True, transport=_TCP, profiles=_SS),
    "SERVERSSL_SERVERCERT":         _ep(client_side=True, server_side=True, transport=_TCP, profiles=_SS),
    "SERVERSSL_HANDSHAKE":          _ep(client_side=True, server_side=True, transport=_TCP, profiles=_SS, common=True),
    "SERVERSSL_DATA":               _ep(client_side=True, server_side=True, transport=_TCP, profiles=_SS),

    # DNS
    "DNS_REQUEST":      _ep(client_side=True, transport=_UDP, profiles=_DNS, common=True),
    "DNS_RESPONSE":     _ep(client_side=True, server_side=True, transport=_UDP, profiles=_DNS, common=True),

    # SIP
    "SIP_REQUEST":      _ep(client_side=True, profiles=_SIP),
    "SIP_REQUEST_SEND": _ep(client_side=True, server_side=True, profiles=_SIP),
    "SIP_RESPONSE":     _ep(client_side=True, server_side=True, profiles=_SIP),
    "SIP_RESPONSE_SEND":_ep(client_side=True, server_side=True, profiles=_SIP),
    "SIP_REQUEST_DONE": _ep(client_side=True, server_side=True, profiles=_SIP),
    "SIP_RESPONSE_DONE":_ep(client_side=True, server_side=True, profiles=_SIP),

    # WebSocket
    "WS_REQUEST":           _ep(client_side=True, transport=_TCP, profiles=_WS),
    "WS_RESPONSE":          _ep(client_side=True, server_side=True, transport=_TCP, profiles=_WS),
    "WS_CLIENT_FRAME":      _ep(client_side=True, transport=_TCP, profiles=_WS),
    "WS_SERVER_FRAME":      _ep(client_side=True, server_side=True, transport=_TCP, profiles=_WS),
    "WS_CLIENT_FRAME_DONE": _ep(client_side=True, transport=_TCP, profiles=_WS),
    "WS_SERVER_FRAME_DONE": _ep(client_side=True, server_side=True, transport=_TCP, profiles=_WS),
    "WS_CLIENT_DATA":       _ep(client_side=True, transport=_TCP, profiles=_WS),
    "WS_SERVER_DATA":       _ep(client_side=True, server_side=True, transport=_TCP, profiles=_WS),

    # RTSP
    "RTSP_REQUEST":         _ep(client_side=True, transport=_TCP, profiles=_RTSP),
    "RTSP_REQUEST_DATA":    _ep(client_side=True, transport=_TCP, profiles=_RTSP),
    "RTSP_RESPONSE":        _ep(client_side=True, server_side=True, transport=_TCP, profiles=_RTSP),
    "RTSP_RESPONSE_DATA":   _ep(client_side=True, server_side=True, transport=_TCP, profiles=_RTSP),

    # Authentication / Authorization
    "AUTH_RESULT":          _ep(client_side=True, transport=_TCP, profiles=_AUTH),
    "AUTH_ERROR":           _ep(client_side=True, transport=_TCP, profiles=_AUTH, deprecated=True),
    "AUTH_FAILURE":         _ep(client_side=True, transport=_TCP, profiles=_AUTH, deprecated=True),
    "AUTH_SUCCESS":         _ep(client_side=True, transport=_TCP, profiles=_AUTH, deprecated=True),
    "AUTH_WANTCREDENTIAL":  _ep(client_side=True, transport=_TCP, profiles=_AUTH, deprecated=True),

    # Access Policy
    "ACCESS_ACL_ALLOWED":               _ep(client_side=True, transport=_TCP, profiles=_ACCESS),
    "ACCESS_ACL_DENIED":                _ep(client_side=True, transport=_TCP, profiles=_ACCESS),
    "ACCESS_POLICY_AGENT_EVENT":        _ep(client_side=True, transport=_TCP, profiles=_ACCESS),
    "ACCESS_POLICY_COMPLETED":          _ep(client_side=True, transport=_TCP, profiles=_ACCESS),
    "ACCESS_SESSION_CLOSED":            _ep(client_side=True, transport=_TCP, profiles=_ACCESS, flow=False),
    "ACCESS_SESSION_STARTED":           _ep(client_side=True, transport=_TCP, profiles=_ACCESS),
    "ACCESS_PER_REQUEST_AGENT_EVENT":   _ep(client_side=True, transport=_TCP, profiles=_ACCESS),
    "ACCESS_SAML_AUTHN":               _ep(client_side=True, transport=_TCP, profiles=_ACCESS),
    "ACCESS_SAML_ASSERTION":           _ep(client_side=True, transport=_TCP, profiles=_ACCESS),
    "ACCESS_SAML_SLO_REQ":            _ep(client_side=True, transport=_TCP, profiles=_ACCESS),
    "ACCESS_SAML_SLO_RESP":           _ep(client_side=True, transport=_TCP, profiles=_ACCESS),
    "ACCESS2_POLICY_EXPRESSION_EVAL":  _ep(client_side=True, transport=_TCP, profiles=_ACCESS),

    # ASM (Web Application Security)
    "ASM_REQUEST_DONE":         _ep(client_side=True, transport=_TCP, profiles=_ASM),
    "ASM_REQUEST_VIOLATION":    _ep(client_side=True, transport=_TCP, profiles=_ASM, deprecated=True),
    "ASM_REQUEST_BLOCKING":     _ep(client_side=True, transport=_TCP, profiles=_ASM),
    "ASM_RESPONSE_VIOLATION":   _ep(client_side=True, server_side=True, transport=_TCP, profiles=_ASM_RESP),
    "ASM_RESPONSE_LOGIN":       _ep(client_side=True, server_side=True, transport=_TCP, profiles=_ASM_RESP),

    # Bot / Fraud
    "BOTDEFENSE_REQUEST":   _ep(client_side=True, transport=_TCP, profiles=_BOTDEFENSE),
    "BOTDEFENSE_ACTION":    _ep(client_side=True, transport=_TCP, profiles=_BOTDEFENSE),
    "ANTIFRAUD_LOGIN":      _ep(client_side=True, server_side=True, transport=_TCP, profiles=_ANTIFRAUD),
    "ANTIFRAUD_ALERT":      _ep(client_side=True, transport=_TCP, profiles=_ANTIFRAUD),
    "IN_DOSL7_ATTACK":      _ep(client_side=True, transport=_TCP, profiles=_DOSL7),

    # Content inspection / matching
    "CLASSIFICATION_DETECTED":      _ep(client_side=True, transport=_TCP, profiles=_CLASSIFICATION),
    "QOE_PARSE_DONE":               _ep(client_side=True, transport=_TCP, profiles=_QOE, deprecated=True),
    "STREAM_MATCHED":               _ep(client_side=True, transport=_TCP, profiles=_STREAM),
    "CATEGORY_MATCHED":             _ep(client_side=True, transport=_TCP, profiles=_CATEGORY),
    "PROTOCOL_INSPECTION_MATCH":    _ep(client_side=True, transport=_TCP, profiles=_PROTO_INSPECT),
    "HTML_COMMENT_MATCHED":         _ep(client_side=True, transport=_TCP, profiles=_HTML),
    "HTML_TAG_MATCHED":             _ep(client_side=True, transport=_TCP, profiles=_HTML),

    # XML
    "XML_CONTENT_BASED_ROUTING":    _ep(client_side=True, transport=_TCP, profiles=_XML, deprecated=True),
    "XML_BEGIN_DOCUMENT":           _ep(client_side=True, transport=_TCP, profiles=_XML, deprecated=True),
    "XML_BEGIN_ELEMENT":            _ep(client_side=True, transport=_TCP, profiles=_XML, deprecated=True),
    "XML_CDATA":                    _ep(client_side=True, transport=_TCP, profiles=_XML, deprecated=True),
    "XML_END_DOCUMENT":             _ep(client_side=True, transport=_TCP, profiles=_XML, deprecated=True),
    "XML_END_ELEMENT":              _ep(client_side=True, transport=_TCP, profiles=_XML, deprecated=True),
    "XML_EVENT":                    _ep(client_side=True, transport=_TCP, profiles=_XML, deprecated=True),

    # JSON
    "JSON_REQUEST":         _ep(client_side=True, transport=_TCP, profiles=_JSON),
    "JSON_REQUEST_MISSING": _ep(client_side=True, transport=_TCP, profiles=_JSON),
    "JSON_REQUEST_ERROR":   _ep(client_side=True, transport=_TCP, profiles=_JSON),
    "JSON_RESPONSE":        _ep(client_side=True, server_side=True, transport=_TCP, profiles=_JSON),
    "JSON_RESPONSE_MISSING":_ep(client_side=True, server_side=True, transport=_TCP, profiles=_JSON),
    "JSON_RESPONSE_ERROR":  _ep(client_side=True, server_side=True, transport=_TCP, profiles=_JSON),

    # SSE
    "SSE_RESPONSE":     _ep(client_side=True, server_side=True, transport=_TCP, profiles=_SSE),

    # Caching / Web Acceleration
    "CACHE_REQUEST":    _ep(client_side=True, transport=_TCP, profiles=_CACHE),
    "CACHE_RESPONSE":   _ep(client_side=True, server_side=True, transport=_TCP, profiles=_CACHE),
    "CACHE_UPDATE":     _ep(client_side=True, server_side=True, transport=_TCP, profiles=_CACHE),

    # Rewrite
    "REWRITE_REQUEST":      _ep(client_side=True, transport=_TCP, profiles=_REWRITE),
    "REWRITE_REQUEST_DONE": _ep(client_side=True, transport=_TCP, profiles=_REWRITE),
    "REWRITE_RESPONSE":     _ep(client_side=True, server_side=True, transport=_TCP, profiles=_REWRITE),
    "REWRITE_RESPONSE_DONE":_ep(client_side=True, server_side=True, transport=_TCP, profiles=_REWRITE),

    # ICAP (content adaptation)
    "ICAP_REQUEST":     _ep(client_side=True, transport=_TCP, profiles=_ICAP),
    "ICAP_RESPONSE":    _ep(client_side=True, server_side=True, transport=_TCP, profiles=_ICAP),

    # Adaptation
    "ADAPT_REQUEST_RESULT":     _ep(client_side=True, transport=_TCP, profiles=_ADAPT_REQ),
    "ADAPT_REQUEST_HEADERS":    _ep(client_side=True, transport=_TCP, profiles=_ADAPT_REQ),
    "ADAPT_RESPONSE_RESULT":    _ep(client_side=True, server_side=True, transport=_TCP, profiles=_ADAPT_RESP),
    "ADAPT_RESPONSE_HEADERS":   _ep(client_side=True, server_side=True, transport=_TCP, profiles=_ADAPT_RESP),

    # FIX
    "FIX_HEADER":   _ep(client_side=True, transport=_TCP, profiles=_FIX),
    "FIX_MESSAGE":  _ep(client_side=True, transport=_TCP, profiles=_FIX),

    # DIAMETER
    "DIAMETER_INGRESS":         _ep(client_side=True, transport=_TCP, profiles=_DIAMETER),
    "DIAMETER_EGRESS":          _ep(client_side=True, server_side=True, transport=_TCP, profiles=_DIAMETER),
    "DIAMETER_RETRANSMISSION":  _ep(client_side=True, transport=_TCP, profiles=_DIAMETER_RETRANS),

    # MQTT
    "MQTT_CLIENT_INGRESS":  _ep(client_side=True, transport=_TCP, profiles=_MQTT),
    "MQTT_CLIENT_DATA":     _ep(client_side=True, transport=_TCP, profiles=_MQTT),
    "MQTT_CLIENT_EGRESS":   _ep(client_side=True, transport=_TCP, profiles=_MQTT),
    "MQTT_CLIENT_SHUTDOWN": _ep(client_side=True, transport=_TCP, profiles=_MQTT),
    "MQTT_SERVER_INGRESS":  _ep(client_side=True, server_side=True, transport=_TCP, profiles=_MQTT),
    "MQTT_SERVER_DATA":     _ep(client_side=True, server_side=True, transport=_TCP, profiles=_MQTT),
    "MQTT_SERVER_EGRESS":   _ep(client_side=True, server_side=True, transport=_TCP, profiles=_MQTT),

    # Generic message routing
    "GENERICMESSAGE_INGRESS":   _ep(client_side=True, transport=_TCP, profiles=_GENERICMSG),
    "GENERICMESSAGE_EGRESS":    _ep(client_side=True, server_side=True, transport=_TCP, profiles=_GENERICMSG),
    "MR_INGRESS":               _ep(client_side=True, transport=_TCP, profiles=_MR),
    "MR_EGRESS":                _ep(client_side=True, server_side=True, transport=_TCP, profiles=_MR),
    "MR_FAILED":                _ep(client_side=True, transport=_TCP, profiles=_MR),
    "MR_DATA":                  _ep(client_side=True, transport=_TCP, profiles=_MR),

    # GTP
    "GTP_GPDU_INGRESS":         _ep(client_side=True, transport=_UDP, profiles=_GTP),
    "GTP_GPDU_EGRESS":          _ep(client_side=True, server_side=True, transport=_UDP, profiles=_GTP),
    "GTP_PRIME_INGRESS":        _ep(client_side=True, transport=_TCP, profiles=_GTP),
    "GTP_PRIME_EGRESS":         _ep(client_side=True, server_side=True, transport=_TCP, profiles=_GTP),
    "GTP_SIGNALLING_INGRESS":   _ep(client_side=True, transport=_UDP, profiles=_GTP),
    "GTP_SIGNALLING_EGRESS":    _ep(client_side=True, server_side=True, transport=_UDP, profiles=_GTP),

    # RADIUS
    "RADIUS_AAA_AUTH_REQUEST":  _ep(client_side=True, transport=_UDP, profiles=_RADIUS),
    "RADIUS_AAA_AUTH_RESPONSE": _ep(client_side=True, server_side=True, transport=_UDP, profiles=_RADIUS),
    "RADIUS_AAA_ACCT_REQUEST":  _ep(client_side=True, transport=_UDP, profiles=_RADIUS),
    "RADIUS_AAA_ACCT_RESPONSE": _ep(client_side=True, server_side=True, transport=_UDP, profiles=_RADIUS),

    # PCP
    "PCP_REQUEST":  _ep(client_side=True, transport=_UDP, profiles=_PCP),
    "PCP_RESPONSE": _ep(client_side=True, server_side=True, transport=_UDP, profiles=_PCP),

    # SOCKS
    "SOCKS_REQUEST":    _ep(client_side=True, transport=_TCP, profiles=_SOCKS),

    # TDS (Tabular Data Stream / MSSQL)
    "TDS_REQUEST":  _ep(client_side=True, transport=_TCP, profiles=_TDS),
    "TDS_RESPONSE": _ep(client_side=True, server_side=True, transport=_TCP, profiles=_TDS),

    # IVS
    "IVS_ENTRY_REQUEST":    _ep(client_side=True, transport=_TCP, profiles=_IVS),
    "IVS_ENTRY_RESPONSE":   _ep(client_side=True, server_side=True, transport=_TCP, profiles=_IVS),

    # L7 checks / health monitors
    "L7CHECK_CLIENT_DATA":  _ep(client_side=True, transport=_TCP, profiles=_L7CHECK),
    "L7CHECK_SERVER_DATA":  _ep(client_side=True, server_side=True, transport=_TCP, profiles=_L7CHECK),

    # PEM (Policy Enforcement Manager)
    "PEM_POLICY":               _ep(client_side=True, transport=_TCP, profiles=_PEM),
    "PEM_SUBS_SESS_CREATED":    _ep(client_side=True, transport=_TCP, profiles=_PEM),
    "PEM_SUBS_SESS_UPDATED":    _ep(client_side=True, transport=_TCP, profiles=_PEM),
    "PEM_SUBS_SESS_DELETED":    _ep(client_side=True, transport=_TCP, profiles=_PEM),

    # AVR
    "AVR_CSPM_INJECTION":   _ep(client_side=True, server_side=True, transport=_TCP, profiles=_AVR),

    # ECA
    "ECA_REQUEST_ALLOWED":  _ep(client_side=True, transport=_TCP, profiles=_ECA),
    "ECA_REQUEST_DENIED":   _ep(client_side=True, transport=_TCP, profiles=_ECA),

    # Miscellaneous
    "NAME_RESOLVED":            _ep(client_side=True, transport=_TCP, profiles=_NAME),
    "FLOW_INIT":                _ep(client_side=True, transport=_TCP_UDP, profiles=_FLOW),
    "TAP_REQUEST":              _ep(client_side=True, transport=_TCP, profiles=_TAP),
    "CONNECTOR_OPEN":           _ep(client_side=True, transport=_TCP, profiles=_CONNECTOR),
    "PING_REQUEST_READY":       _ep(client_side=True, transport=_TCP, profiles=_H),
    "PING_RESPONSE_READY":      _ep(client_side=True, server_side=True, transport=_TCP, profiles=_H),
    "USER_REQUEST":             _ep(client_side=True, server_side=True, transport=_TCP),
    "USER_RESPONSE":            _ep(client_side=True, server_side=True, transport=_TCP),
    "EPI_NA_CHECK_HTTP_REQUEST":_ep(client_side=True, transport=_TCP),

    # GTM events (not in seed list but used by commands)
    "IP_GTM":   _ep(flow=False),
    "TCP_GTM":  _ep(client_side=True, transport=_TCP),
    "UDP_GTM":  _ep(client_side=True, transport=_UDP),
}
# fmt: on


# EVENT_DESCRIPTIONS (~200 entries)

# fmt: off
EVENT_DESCRIPTIONS: dict[str, str] = {

    # Lifecycle / global
    "RULE_INIT":
        "Fires once when the iRule is loaded (device boot, config change,"
        " or iRule save). Use for initialising static:: variables only.",
    "PERSIST_DOWN":
        "Fires pre-LB when persistence would direct a connection to a node"
        " or pool member that has been marked down.",
    "FLOW_INIT":
        "Fires once per TCP or unique UDP/IP flow, after packet"
        " filters but before AFM and TMM processing. Runs before"
        " CLIENT_ACCEPTED in the processing pipeline.",

    # L4 client-side (TCP/UDP)
    "CLIENT_ACCEPTED":
        "Fires when a client connection is accepted (TCP: after 3-way"
        " handshake; UDP: on the first datagram). On UDP virtual servers,"
        " packet data is available via UDP::payload in this event.",
    "CLIENT_DATA":
        "Fires when client data is received while the connection is in"
        " collect state (requires TCP::collect or UDP::collect). On UDP"
        " virtual servers, fires on every datagram including the first.",
    "CLIENT_CLOSED":
        "Fires when the client-side connection closes (TCP: FIN/RST;"
        " UDP: idle timeout). Fires once per connection.",

    # Load-balancing / server init
    "LB_SELECTED":
        "Fires when the system selects a pool member. On HTTP virtual"
        " servers, fires after HTTP_REQUEST (per-request). On UDP (9.4+),"
        " fires after CLIENT_ACCEPTED but before CLIENT_DATA.",
    "LB_FAILED":
        "Fires when the system fails to select a pool or pool"
        " member, or when a selected resource is unreachable."
        " Alternative to LB_SELECTED at the same logical point.",
    "LB_QUEUED":
        "Fires when a connection limit is hit at the pool or pool"
        " member level. Alternative to LB_SELECTED.",
    "SERVER_INIT":
        "Fires after LB_SELECTED when the server-side TCP SYN is about"
        " to be sent. Server address/port are available for inspection.",
    "SA_PICKED":
        "Fires after source address translation is complete but the"
        " server-side flow is not yet set up. Runs between LB_SELECTED"
        " and SERVER_CONNECTED.",

    # L4 server-side (TCP/UDP)
    "SERVER_CONNECTED":
        "Fires when a server-side connection is established (TCP: after"
        " handshake with the pool member). On UDP, fires if and when a"
        " server is selected, before the datagram is forwarded.",
    "SERVER_DATA":
        "Fires when new data is received from the server while"
        " the connection is in collect state (requires TCP::collect"
        " in SERVER_CONNECTED). On UDP, fires on every server datagram.",
    "SERVER_CLOSED":
        "Fires when the server-side connection closes. On UDP,"
        " fires when the connection table entry times out.",

    # HTTP
    "HTTP_REQUEST":
        "Fires when request headers are fully parsed (pre-LB). On"
        " keep-alive connections, fires once per HTTP transaction."
        " Pipeline: L7 iRules layer.",
    "HTTP_REQUEST_DATA":
        "Fires when HTTP::collect has gathered the specified amount of"
        " request body data. Only fires if HTTP::collect was called in"
        " HTTP_REQUEST.",
    "HTTP_REQUEST_SEND":
        "Fires immediately before the request is forwarded to the"
        " server-side TCP stack (post-LB, after SERVER_CONNECTED).",
    "HTTP_REQUEST_RELEASE":
        "Fires when HTTP data is about to be released on the server-side."
        " Last chance to modify request data before it reaches the server.",
    "HTTP_RESPONSE":
        "Fires when response status and headers are fully parsed from"
        " the server. Fires once per HTTP transaction on keep-alive.",
    "HTTP_RESPONSE_DATA":
        "Fires when HTTP::collect has gathered the specified amount of"
        " response body data. Only fires if HTTP::collect was called in"
        " HTTP_RESPONSE.",
    "HTTP_RESPONSE_CONTINUE":
        "Fires when the system receives a 100 Continue interim response"
        " from the server.",
    "HTTP_RESPONSE_RELEASE":
        "Fires when HTTP data is about to be released on the client-side."
        " Last chance to modify response data before it reaches the client.",
    "HTTP_DISABLED":
        "Fires when HTTP processing is disabled on the connection"
        " (e.g. WebSocket upgrade or protocol switch).",
    "HTTP_REJECT":
        "Fires when HTTP encounters a parsing error and aborts the"
        " connection.",
    "HTTP_PROXY_REQUEST":
        "Fires when a virtual server is configured with explicit proxy"
        " mode.",
    "HTTP_PROXY_CONNECT":
        "Fires when proxy chaining via the HTTP_PROXY_CONNECT"
        " profile.",
    "HTTP_PROXY_RESPONSE":
        "Fires when a response from the remote HTTP proxy is"
        " received.",
    "HTTP_CLASS_FAILED":
        "Fires when an HTTP request does not match any HTTP class"
        " filter (deprecated; pre-LB classification).",
    "HTTP_CLASS_SELECTED":
        "Fires when an HTTP request matches an HTTP class filter"
        " (deprecated; pre-LB classification).",

    # TLS (client-side)
    "CLIENTSSL_CLIENTHELLO":
        "Fires when the client's SSL/TLS ClientHello message is received."
        " Fires after CLIENT_ACCEPTED, before CLIENTSSL_HANDSHAKE."
        " Allows SNI inspection and certificate selection.",
    "CLIENTSSL_CLIENTCERT":
        "Fires when a client certificate is received (mutual TLS only)."
        " Only fires when the ClientSSL profile requires or requests a"
        " client certificate.",
    "CLIENTSSL_HANDSHAKE":
        "Fires when the client-side SSL/TLS handshake completes."
        " Fires once per connection. After this, HTTP events can fire.",
    "CLIENTSSL_SERVERHELLO_SEND":
        "Fires when BIG-IP is about to send its ServerHello on the"
        " client-side connection. Allows cipher and protocol modification.",
    "CLIENTSSL_DATA":
        "Fires when SSL data is received from the client while in"
        " collect state (requires SSL::collect).",
    "CLIENTSSL_PASSTHROUGH":
        "Fires when the ClientSSL profile receives plaintext (non-TLS)"
        " data and enters passthrough mode.",

    # TLS (server-side)
    "SERVERSSL_CLIENTHELLO_SEND":
        "Fires when BIG-IP is about to send its ClientHello on the"
        " server-side connection (post-LB, after SERVER_CONNECTED).",
    "SERVERSSL_SERVERHELLO":
        "Fires when the server's ServerHello message is received on"
        " the server-side connection.",
    "SERVERSSL_SERVERCERT":
        "Fires when the server's certificate is received and"
        " verification completes on the server-side connection.",
    "SERVERSSL_HANDSHAKE":
        "Fires when the server-side SSL/TLS handshake completes."
        " After this, HTTP_REQUEST_SEND can fire.",
    "SERVERSSL_DATA":
        "Fires when SSL data is received from the server while in"
        " collect state (requires SSL::collect).",

    # DNS
    "DNS_REQUEST":
        "Fires when a DNS query is received (pre-LB). On UDP virtual"
        " servers, this is the first event (no CLIENT_ACCEPTED). On TCP"
        " virtual servers, fires after CLIENT_ACCEPTED.",
    "DNS_RESPONSE":
        "Fires when a DNS response is ready to be sent to the client"
        " (post-LB).",

    # SIP
    "SIP_REQUEST":
        "Triggered when the system fully parses a complete client SIP"
        " request header.",
    "SIP_REQUEST_SEND":
        "Triggered immediately before a SIP request is sent to the"
        " server-side TCP stack.",
    "SIP_RESPONSE":
        "Triggered when the system parses all response status and"
        " header lines from the server SIP response.",
    "SIP_RESPONSE_SEND":
        "Triggered immediately before a SIP response is sent.",
    "SIP_REQUEST_DONE":
        "Raised when a SIP request message is received from the proxy"
        " after routing.",
    "SIP_RESPONSE_DONE":
        "Raised when a SIP response message is received from the proxy"
        " after routing.",

    # WebSocket
    "WS_REQUEST":
        "Raised when WebSocket upgrade headers are present in the"
        " client request.",
    "WS_RESPONSE":
        "Raised when WebSocket upgrade headers are present in the"
        " server response.",
    "WS_CLIENT_FRAME":
        "Raised at the start of a WebSocket frame received from the"
        " client.",
    "WS_SERVER_FRAME":
        "Raised at the start of a WebSocket frame received from the"
        " server.",
    "WS_CLIENT_FRAME_DONE":
        "Raised at the end of a WebSocket frame received from the"
        " client.",
    "WS_SERVER_FRAME_DONE":
        "Raised at the end of a WebSocket frame received from the"
        " server.",
    "WS_CLIENT_DATA":
        "Raised when the system collects the specified amount of"
        " WebSocket data from the client via WS::collect.",
    "WS_SERVER_DATA":
        "Raised when the system collects the specified amount of"
        " WebSocket data from the server via WS::collect.",

    # RTSP
    "RTSP_REQUEST":
        "Triggered after a complete RTSP request has been received.",
    "RTSP_REQUEST_DATA":
        "Triggered when an RTSP::collect command finishes processing.",
    "RTSP_RESPONSE":
        "Triggered after a complete RTSP response has been received.",
    "RTSP_RESPONSE_DATA":
        "Triggered when collection of RTSP response data is finished.",

    # Authentication / Authorization
    "AUTH_RESULT":
        "Replaces AUTH_SUCCESS, AUTH_FAILURE, AUTH_ERROR, and"
        " AUTH_WANTCREDENTIAL events.",
    "AUTH_ERROR":
        "Triggered when an error occurs during authorization"
        " (deprecated; use AUTH_RESULT).",
    "AUTH_FAILURE":
        "Triggered when an unsuccessful authorization operation"
        " completes (deprecated; use AUTH_RESULT).",
    "AUTH_SUCCESS":
        "Triggered when a successful authorization completes"
        " (deprecated; use AUTH_RESULT).",
    "AUTH_WANTCREDENTIAL":
        "Triggered when an authorization operation needs an additional"
        " credential (deprecated; use AUTH_RESULT).",

    # Access Policy
    "ACCESS_ACL_ALLOWED":
        "Fires when a resource request passes ACL checks (post-LB)."
        " Per-request event on keep-alive connections.",
    "ACCESS_ACL_DENIED":
        "Fires when a resource request fails ACL checks (post-LB)."
        " Per-request event.",
    "ACCESS_POLICY_AGENT_EVENT":
        "Fires during access policy execution to allow iRule logic"
        " (pre-LB, per-session).",
    "ACCESS_POLICY_COMPLETED":
        "Fires when access policy evaluation completes for a user"
        " session (pre-LB, once per session).",
    "ACCESS_SESSION_CLOSED":
        "Fires when a user session is removed (logout, timeout, or"
        " admin action). Fires after CLIENT_CLOSED.",
    "ACCESS_SESSION_STARTED":
        "Fires when a new APM user session is created"
        " (pre-LB, once per session).",
    "ACCESS_PER_REQUEST_AGENT_EVENT":
        "Allows iRule logic execution at a desired point in per-request"
        " access policy execution.",
    "ACCESS_SAML_AUTHN":
        "Triggered when the SAML authentication request payload is"
        " generated for a user session.",
    "ACCESS_SAML_ASSERTION":
        "Triggered when the SAML assertion payload is generated for a"
        " user session.",
    "ACCESS_SAML_SLO_REQ":
        "Triggered when the SAML single logout request payload is"
        " generated for a user session.",
    "ACCESS_SAML_SLO_RESP":
        "Triggered when the SAML single logout response payload is"
        " generated for a user session.",
    "ACCESS2_POLICY_EXPRESSION_EVAL":
        "Triggered when per-request policy branch expressions are"
        " evaluated.",

    # ASM (Web Application Security)
    # BIG-IP 14.1+ processing pipeline:
    #   IP Intelligence → AFM/L2-L4 DoS → L2-L4 policies → L2-L4 iRules
    #   → L7 policies → ASM → L7 iRules → L7 DoS → Bot Defense
    # ASM request-side events fire pre-LB (between CACHE and LB_SELECTED).
    "ASM_REQUEST_DONE":
        "Fires after ASM finishes processing a request (pre-LB)."
        " Normal mode: fires after every request."
        " Compatibility mode: does not fire (use ASM_REQUEST_VIOLATION).",
    "ASM_REQUEST_VIOLATION":
        "Fires when ASM detects a request policy violation (pre-LB)."
        " Compatibility mode: fires only when a violation occurs."
        " Normal mode: violations reported via ASM_REQUEST_DONE instead."
        " Deprecated event.",
    "ASM_REQUEST_BLOCKING":
        "Fires when ASM is generating a blocking response (pre-LB)."
        " Allows modification of the reject page before it is sent."
        " Fires in both Normal and Compatibility modes.",
    "ASM_RESPONSE_VIOLATION":
        "Fires when ASM detects a response policy violation."
        " Runs post-response (after HTTP_RESPONSE_DATA).",
    "ASM_RESPONSE_LOGIN":
        "Fires on an ASM response login event."
        " Runs post-response.",

    # Bot / Fraud / DoS
    "BOTDEFENSE_REQUEST":
        "Fires on an HTTP request after Bot Defense processing completes,"
        " before a blocking decision is made. Pipeline: runs after L7"
        " DoS in the BIG-IP 14.1+ processing order.",
    "BOTDEFENSE_ACTION":
        "Fires immediately before Bot Defense takes action on a"
        " transaction (block, challenge, allow).",
    "ANTIFRAUD_LOGIN":
        "Fires on an antifraud login event.",
    "ANTIFRAUD_ALERT":
        "Fires when an antifraud alert is received or generated.",
    "IN_DOSL7_ATTACK":
        "Fires when the L7 DoS profile detects an attack condition"
        " (pre-LB). Pipeline: runs after CACHE events and before ASM"
        " request-side events in the BIG-IP 14.1+ processing order.",

    # Content inspection / matching
    "CLASSIFICATION_DETECTED":
        "Triggered when a flow is classified.",
    "QOE_PARSE_DONE":
        "Triggered when the system finishes parsing static video"
        " parameters from the video header.",
    "STREAM_MATCHED":
        "Triggered when a stream expression matches data-stream"
        " octets.",
    "CATEGORY_MATCHED":
        "Triggered when a custom category match is found during URL"
        " filtering.",
    "PROTOCOL_INSPECTION_MATCH":
        "Triggered when protocol inspection is matched for the flow.",
    "HTML_COMMENT_MATCHED":
        "Raised when an HTML comment is encountered.",
    "HTML_TAG_MATCHED":
        "Raised when an HTML tag is encountered.",

    # XML
    "XML_CONTENT_BASED_ROUTING":
        "Triggered when a match is found in the XML profile.",
    "XML_BEGIN_DOCUMENT":
        "Triggered before the XML document gets parsed.",
    "XML_BEGIN_ELEMENT":
        "Triggered when the parser encounters the start of an element.",
    "XML_CDATA":
        "Triggered when the parser encounters character data (CDATA).",
    "XML_END_DOCUMENT":
        "Triggered when an XML document is completely parsed.",
    "XML_END_ELEMENT":
        "Triggered when the parser encounters the end of an element.",
    "XML_EVENT":
        "Generic catch-all event triggered for all XML events.",

    # JSON
    "JSON_REQUEST":
        "Triggered upon successful parsing of valid JSON content in an"
        " HTTP request body.",
    "JSON_REQUEST_ERROR":
        "Triggered when an HTTP request body should contain JSON but"
        " could not be parsed.",
    "JSON_REQUEST_MISSING":
        "Triggered when an HTTP request has no body or does not contain"
        " JSON content.",
    "JSON_RESPONSE":
        "Triggered upon successful parsing of valid JSON content in an"
        " HTTP response body.",
    "JSON_RESPONSE_ERROR":
        "Triggered when an HTTP response body should contain JSON but"
        " could not be parsed.",
    "JSON_RESPONSE_MISSING":
        "Triggered when an HTTP response has no body or does not"
        " contain JSON content.",

    # SSE
    "SSE_RESPONSE":
        "Triggered when an SSE server response message has been"
        " received.",

    # Caching / Web Acceleration
    "CACHE_REQUEST":
        "Fires when a cacheable request is received (pre-LB). Allows"
        " cache key manipulation before the cache lookup.",
    "CACHE_RESPONSE":
        "Fires when a cached response is about to be served directly"
        " (pre-LB cache hit). Bypasses LB_SELECTED and server-side"
        " events when the response is served from cache.",
    "CACHE_UPDATE":
        "Fires when a new cache entry is inserted or an expired object"
        " is refreshed (post-response).",

    # Rewrite
    "REWRITE_REQUEST":
        "Triggered on a rewrite request event.",
    "REWRITE_REQUEST_DONE":
        "Triggered after ACCESS_ACL_ALLOWED when a Portal Access"
        " resource is accessed.",
    "REWRITE_RESPONSE":
        "Triggered on a rewrite response event.",
    "REWRITE_RESPONSE_DONE":
        "Triggered when REWRITE_REQUEST_DONE calls"
        " REWRITE::post_process on.",

    # ICAP (content adaptation)
    "ICAP_REQUEST":
        "Raised after an ICAP command has been created but before it is"
        " sent to an ICAP server.",
    "ICAP_RESPONSE":
        "Raised after an ICAP response has been processed but before"
        " the result is sent to the adaptation virtual server.",

    # Adaptation
    "ADAPT_REQUEST_RESULT":
        "Raised after the internal virtual server returns the result of"
        " request modification.",
    "ADAPT_REQUEST_HEADERS":
        "Raised as soon as any HTTP request headers have been returned"
        " from the IVS.",
    "ADAPT_RESPONSE_RESULT":
        "Raised after the internal virtual server returns the result of"
        " response modification.",
    "ADAPT_RESPONSE_HEADERS":
        "Raised as soon as any HTTP response headers have been returned"
        " from the IVS.",

    # FIX
    "FIX_HEADER":
        "Triggered when the system finishes parsing a new FIX header.",
    "FIX_MESSAGE":
        "Triggered when the system finishes parsing a new FIX message.",

    # DIAMETER
    "DIAMETER_INGRESS":
        "Triggered when the system receives a DIAMETER message.",
    "DIAMETER_EGRESS":
        "Triggered when the system is ready to send a DIAMETER message.",
    "DIAMETER_RETRANSMISSION":
        "Triggered when the system generates a retransmitted DIAMETER"
        " request or answer message.",

    # MQTT
    "MQTT_CLIENT_INGRESS":
        "Triggered when an MQTT message is received from the"
        " client-side.",
    "MQTT_CLIENT_DATA":
        "Triggered when a prior MQTT::collect command finishes on the"
        " client-side.",
    "MQTT_CLIENT_EGRESS":
        "Triggered when an MQTT message is sent to the client-side.",
    "MQTT_CLIENT_SHUTDOWN":
        "Triggered when the MQTT client closes the TCP connection.",
    "MQTT_SERVER_INGRESS":
        "Triggered when an MQTT message is received from the"
        " server-side.",
    "MQTT_SERVER_DATA":
        "Triggered when server-side payload data collection via"
        " MQTT::collect finishes.",
    "MQTT_SERVER_EGRESS":
        "Triggered when an MQTT message is sent to the server-side.",

    # Generic message routing
    "GENERICMESSAGE_INGRESS":
        "Raised when a message is received by the generic message"
        " filter.",
    "GENERICMESSAGE_EGRESS":
        "Raised when a message is received from the proxy.",
    "MR_INGRESS":
        "Raised when a message is received by the message proxy before"
        " route lookup.",
    "MR_EGRESS":
        "Raised after the route has been selected and the message is"
        " delivered for forwarding.",
    "MR_FAILED":
        "Raised when a message has been returned to the originating"
        " flow due to a routing failure.",
    "MR_DATA":
        "Raised when message routing data is received.",

    # GTP
    "GTP_GPDU_INGRESS":
        "Triggered for a GTP G-PDU message on the connection that"
        " accepted the message.",
    "GTP_GPDU_EGRESS":
        "Triggered for a GTP G-PDU message on the connection that"
        " forwards the message.",
    "GTP_PRIME_INGRESS":
        "Triggered for GTP prime messages on the connection that"
        " accepted the message.",
    "GTP_PRIME_EGRESS":
        "Triggered for GTP prime messages on the connection that"
        " forwards the message.",
    "GTP_SIGNALLING_INGRESS":
        "Triggered for any GTP signalling message (except G-PDU) on"
        " the accepting connection.",
    "GTP_SIGNALLING_EGRESS":
        "Triggered for any GTP signalling message (except G-PDU) on"
        " the forwarding connection.",

    # RADIUS
    "RADIUS_AAA_AUTH_REQUEST":
        "Triggered when a RADIUS authentication request is received.",
    "RADIUS_AAA_AUTH_RESPONSE":
        "Triggered when a RADIUS authentication response is received.",
    "RADIUS_AAA_ACCT_REQUEST":
        "Triggered when a RADIUS accounting request is received.",
    "RADIUS_AAA_ACCT_RESPONSE":
        "Triggered when a RADIUS accounting response is received.",

    # PCP
    "PCP_REQUEST":
        "Triggered on receipt of a valid PCP request from a client.",
    "PCP_RESPONSE":
        "Triggered when a PCP response is returned to the client.",

    # SOCKS
    "SOCKS_REQUEST":
        "Triggered upon receipt of a SOCKS command on a SOCKS"
        " connection, before authentication.",

    # TDS (Tabular Data Stream / MSSQL)
    "TDS_REQUEST":
        "Triggered when a TDS request message is received.",
    "TDS_RESPONSE":
        "Triggered when a TDS response message is received.",

    # IVS
    "IVS_ENTRY_REQUEST":
        "Triggered when the internal virtual server receives a request"
        " from the parent virtual server.",
    "IVS_ENTRY_RESPONSE":
        "Triggered when the internal virtual server receives a response"
        " from the parent virtual server.",

    # L7 checks / health monitors
    "L7CHECK_CLIENT_DATA":
        "Triggered each time new ingress data is received from the"
        " client.",
    "L7CHECK_SERVER_DATA":
        "Triggered each time new ingress data is received from the"
        " server.",

    # PEM (Policy Enforcement Manager)
    "PEM_POLICY":
        "Triggered for PEM policy evaluation.",
    "PEM_SUBS_SESS_CREATED":
        "Triggered when a subscriber session is created.",
    "PEM_SUBS_SESS_UPDATED":
        "Triggered when a subscriber session attribute is updated.",
    "PEM_SUBS_SESS_DELETED":
        "Triggered when a subscriber session is deleted.",

    # AVR
    "AVR_CSPM_INJECTION":
        "Triggered when the AVR profile is about to insert CSPM"
        " javascript into the response.",

    # ECA
    "ECA_REQUEST_ALLOWED":
        "Triggered when the ECA plugin successfully authenticates and"
        " is about to forward the request.",
    "ECA_REQUEST_DENIED":
        "Triggered when the ECA plugin cannot verify user credentials.",

    # Miscellaneous
    "NAME_RESOLVED":
        "Triggered after a NAME::lookup command has been issued and a"
        " response received.",
    "TAP_REQUEST":
        "Triggered once a security token is obtained for certain HTTP"
        " transactions.",
    "CONNECTOR_OPEN":
        "Triggered when the connector is about to raise the service"
        " connect.",
    "PING_REQUEST_READY":
        "Triggered when TMM has assembled an HTTP request to the"
        " PingAccess policy server.",
    "PING_RESPONSE_READY":
        "Triggered when TMM has received an HTTP response from the"
        " PingAccess policy server.",
    "USER_REQUEST":
        "Triggered by the TCP::notify request command; executes in"
        " server-side context.",
    "USER_RESPONSE":
        "Triggered by the TCP::notify response command; executes in"
        " client-side context.",
    "EPI_NA_CHECK_HTTP_REQUEST":
        "Internal event for Network Access and Endpoint Inspector"
        " client applications (requires APM).",

    # GTM
    "IP_GTM":
        "GTM IP event.",
    "TCP_GTM":
        "GTM TCP event.",
    "UDP_GTM":
        "GTM UDP event.",
}
# fmt: on


# MASTER_ORDER (110 entries) — canonical event firing order

MASTER_ORDER: tuple[tuple[str, frozenset[str]], ...] = (
    # Initialisation
    ("RULE_INIT", frozenset()),
    # Per-flow init (FLOW profile, before TMM processing)
    ("FLOW_INIT", frozenset({"FLOW"})),
    # L4 client-side
    ("CLIENT_ACCEPTED", frozenset()),
    ("CLIENT_DATA", frozenset()),  # conditional: TCP::collect
    # Client-side TLS
    ("CLIENTSSL_CLIENTHELLO", frozenset({"CLIENTSSL"})),
    ("CLIENTSSL_SERVERHELLO_SEND", frozenset({"CLIENTSSL"})),
    ("CLIENTSSL_CLIENTCERT", frozenset({"CLIENTSSL"})),  # conditional: mutual TLS
    ("CLIENTSSL_HANDSHAKE", frozenset({"CLIENTSSL"})),
    ("CLIENTSSL_DATA", frozenset({"CLIENTSSL"})),  # conditional: SSL::collect
    ("CLIENTSSL_PASSTHROUGH", frozenset({"CLIENTSSL"})),  # conditional: plaintext
    # HTTP request (client-side)
    ("HTTP_REQUEST", frozenset({"HTTP", "FASTHTTP"})),
    ("HTTP_REQUEST_DATA", frozenset({"HTTP"})),  # conditional: HTTP::collect
    ("HTTP_PROXY_REQUEST", frozenset({"HTTP"})),  # conditional: explicit proxy
    # Authentication / authorisation
    ("AUTH_RESULT", frozenset({"AUTH"})),
    ("AUTH_SUCCESS", frozenset({"AUTH"})),
    ("AUTH_FAILURE", frozenset({"AUTH"})),
    ("AUTH_ERROR", frozenset({"AUTH"})),
    ("AUTH_WANTCREDENTIAL", frozenset({"AUTH"})),
    # Access Policy (APM -- per-session)
    ("ACCESS_SESSION_STARTED", frozenset({"ACCESS"})),
    ("ACCESS_POLICY_AGENT_EVENT", frozenset({"ACCESS"})),
    ("ACCESS_POLICY_COMPLETED", frozenset({"ACCESS"})),
    # Classification / content matching (pre-LB)
    ("CLASSIFICATION_DETECTED", frozenset({"CLASSIFICATION"})),
    ("CATEGORY_MATCHED", frozenset({"CATEGORY", "ACCESS", "HTTP"})),
    ("HTTP_CLASS_SELECTED", frozenset({"HTTP"})),
    ("HTTP_CLASS_FAILED", frozenset({"HTTP"})),
    # Caching (WebAcceleration, pre-LB)
    ("CACHE_REQUEST", frozenset({"CACHE", "WEBACCELERATION"})),
    ("CACHE_RESPONSE", frozenset({"CACHE", "WEBACCELERATION"})),  # cache hit shortcut
    # L7 DoS (pre-LB, BIG-IP 14.1+)
    ("IN_DOSL7_ATTACK", frozenset({"DOSL7", "HTTP", "FASTHTTP"})),
    # ASM / WAF request-side (pre-LB, BIG-IP 14.1+)
    # Normal mode:        ASM_REQUEST_DONE fires after every request
    # Compatibility mode: ASM_REQUEST_VIOLATION fires only for violations
    # Both modes:         ASM_REQUEST_BLOCKING fires when blocking
    ("ASM_REQUEST_DONE", frozenset({"ASM"})),
    ("ASM_REQUEST_VIOLATION", frozenset({"ASM"})),
    ("ASM_REQUEST_BLOCKING", frozenset({"ASM"})),
    # DNS (pre-LB)
    ("DNS_REQUEST", frozenset({"DNS"})),
    # SIP (pre-LB)
    ("SIP_REQUEST", frozenset({"SIP"})),
    # Persistence
    ("PERSIST_DOWN", frozenset()),
    # Load-balancing
    ("LB_SELECTED", frozenset()),
    ("LB_FAILED", frozenset()),  # alternative to LB_SELECTED
    ("LB_QUEUED", frozenset()),  # connection-limit hit
    # SNAT / server init
    ("SA_PICKED", frozenset()),
    ("SERVER_INIT", frozenset()),
    # Access Policy (APM -- per-request)
    ("ACCESS_ACL_ALLOWED", frozenset({"ACCESS"})),
    ("ACCESS_ACL_DENIED", frozenset({"ACCESS"})),
    ("ACCESS_PER_REQUEST_AGENT_EVENT", frozenset({"ACCESS"})),
    ("REWRITE_REQUEST_DONE", frozenset({"REWRITE", "HTTP"})),
    # L4 server-side
    ("SERVER_CONNECTED", frozenset()),
    # Server-side TLS
    ("SERVERSSL_CLIENTHELLO_SEND", frozenset({"SERVERSSL"})),
    ("SERVERSSL_SERVERHELLO", frozenset({"SERVERSSL"})),
    ("SERVERSSL_SERVERCERT", frozenset({"SERVERSSL"})),
    ("SERVERSSL_HANDSHAKE", frozenset({"SERVERSSL"})),
    ("SERVERSSL_DATA", frozenset({"SERVERSSL"})),  # conditional: SSL::collect
    # HTTP request (server-side)
    ("HTTP_REQUEST_SEND", frozenset({"HTTP"})),
    ("HTTP_REQUEST_RELEASE", frozenset({"HTTP"})),
    # Server data
    ("SERVER_DATA", frozenset()),  # conditional: TCP::collect
    # DNS (response)
    ("DNS_RESPONSE", frozenset({"DNS"})),
    # SIP (server-side + response)
    ("SIP_REQUEST_SEND", frozenset({"SIP"})),
    ("SIP_RESPONSE", frozenset({"SIP"})),
    ("SIP_RESPONSE_SEND", frozenset({"SIP"})),
    # HTTP response
    ("HTTP_RESPONSE", frozenset({"HTTP", "FASTHTTP"})),
    ("HTTP_RESPONSE_DATA", frozenset({"HTTP"})),  # conditional: HTTP::collect
    ("HTTP_RESPONSE_CONTINUE", frozenset({"HTTP"})),  # conditional: 100-continue
    # ASM / WAF response-side
    ("ASM_RESPONSE_VIOLATION", frozenset({"ASM", "HTTP", "FASTHTTP"})),
    ("ASM_RESPONSE_LOGIN", frozenset({"ASM", "HTTP", "FASTHTTP"})),
    # Bot Defense
    ("BOTDEFENSE_REQUEST", frozenset({"BOTDEFENSE"})),
    ("BOTDEFENSE_ACTION", frozenset({"BOTDEFENSE"})),
    # Caching (response update)
    ("CACHE_UPDATE", frozenset({"CACHE", "WEBACCELERATION"})),
    # Content matching (response)
    ("STREAM_MATCHED", frozenset({"STREAM"})),
    ("HTML_TAG_MATCHED", frozenset({"HTML"})),
    ("HTML_COMMENT_MATCHED", frozenset({"HTML"})),
    # Rewrite (response)
    ("REWRITE_RESPONSE_DONE", frozenset({"REWRITE", "HTTP"})),
    # HTTP response release
    ("HTTP_RESPONSE_RELEASE", frozenset({"HTTP"})),
    # HTTP disabled / reject (can fire at various points)
    ("HTTP_DISABLED", frozenset({"HTTP"})),
    ("HTTP_REJECT", frozenset({"HTTP"})),
    # Teardown
    ("SERVER_CLOSED", frozenset()),
    ("CLIENT_CLOSED", frozenset()),
    # Session teardown (APM)
    ("ACCESS_SESSION_CLOSED", frozenset({"ACCESS"})),
)

# Build a lookup: event_name → position in master ordering.
_MASTER_INDEX: dict[str, int] = {evt: idx for idx, (evt, _gates) in enumerate(MASTER_ORDER)}


# Event multiplicity

ONCE_PER_CONNECTION: frozenset[str] = frozenset(
    {
        "RULE_INIT",
        "FLOW_INIT",
        "CLIENT_ACCEPTED",
        "CLIENTSSL_CLIENTHELLO",
        "CLIENTSSL_SERVERHELLO_SEND",
        "CLIENTSSL_CLIENTCERT",
        "CLIENTSSL_HANDSHAKE",
        "CLIENTSSL_PASSTHROUGH",
        "ACCESS_SESSION_STARTED",
        "ACCESS_POLICY_AGENT_EVENT",
        "ACCESS_POLICY_COMPLETED",
        "ACCESS_SESSION_CLOSED",
        "CLIENT_CLOSED",
    }
)

PER_REQUEST: frozenset[str] = frozenset(
    {
        # HTTP request cycle — repeats on keep-alive
        "HTTP_REQUEST",
        "HTTP_REQUEST_DATA",
        "HTTP_PROXY_REQUEST",
        "HTTP_CLASS_SELECTED",
        "HTTP_CLASS_FAILED",
        "CACHE_REQUEST",
        "IN_DOSL7_ATTACK",
        "LB_SELECTED",
        "LB_FAILED",
        "LB_QUEUED",
        "SA_PICKED",
        "SERVER_INIT",
        "SERVER_CONNECTED",
        "SERVERSSL_CLIENTHELLO_SEND",
        "SERVERSSL_SERVERHELLO",
        "SERVERSSL_SERVERCERT",
        "SERVERSSL_HANDSHAKE",
        "HTTP_REQUEST_SEND",
        "HTTP_REQUEST_RELEASE",
        # HTTP response cycle
        "HTTP_RESPONSE",
        "HTTP_RESPONSE_DATA",
        "HTTP_RESPONSE_CONTINUE",
        "HTTP_RESPONSE_RELEASE",
        "CACHE_RESPONSE",
        "CACHE_UPDATE",
        "STREAM_MATCHED",
        "SERVER_CLOSED",
        # ASM / Bot defense — per-request
        "ASM_REQUEST_DONE",
        "ASM_REQUEST_VIOLATION",
        "ASM_REQUEST_BLOCKING",
        "ASM_RESPONSE_VIOLATION",
        "BOTDEFENSE_REQUEST",
        "BOTDEFENSE_ACTION",
        # DNS — per-query
        "DNS_REQUEST",
        "DNS_RESPONSE",
        # SIP — per-message
        "SIP_REQUEST",
        "SIP_REQUEST_SEND",
        "SIP_RESPONSE",
        "SIP_RESPONSE_SEND",
        # Access per-request flow
        "ACCESS_ACL_ALLOWED",
        "ACCESS_ACL_DENIED",
        "ACCESS_PER_REQUEST_AGENT_EVENT",
    }
)


# Flow chains (7 chains)


def _s(event: str, phase: str, **kw: object) -> FlowStep:
    """Shorthand for FlowStep construction."""
    return FlowStep(event=event, phase=phase, **kw)  # type: ignore[arg-type]


# fmt: off
FLOW_CHAINS: dict[str, FlowChain] = {

    # 1. Plain TCP
    "plain_tcp": FlowChain(
        chain_id="plain_tcp",
        description="Plain TCP (tcp profile only)",
        profiles=frozenset({"TCP"}),
        steps=(
            _s("RULE_INIT",         "init"),
            _s("CLIENT_ACCEPTED",   "l4_client"),
            _s("CLIENT_DATA",       "l4_client", conditional=True,
               condition_note="Requires TCP::collect in CLIENT_ACCEPTED"),
            _s("LB_SELECTED",       "lb"),
            _s("SA_PICKED",         "lb"),
            _s("SERVER_INIT",       "lb"),
            _s("SERVER_CONNECTED",  "l4_server"),
            _s("SERVER_DATA",       "l4_server", conditional=True,
               condition_note="Requires TCP::collect in SERVER_CONNECTED"),
            _s("SERVER_CLOSED",     "l4_teardown"),
            _s("CLIENT_CLOSED",     "l4_teardown"),
        ),
    ),

    # 2. TCP + HTTP
    "tcp_http": FlowChain(
        chain_id="tcp_http",
        description="TCP + HTTP",
        profiles=frozenset({"TCP", "HTTP"}),
        steps=(
            _s("RULE_INIT",              "init"),
            _s("CLIENT_ACCEPTED",        "l4_client"),
            _s("HTTP_REQUEST",           "http_request"),
            _s("HTTP_REQUEST_DATA",      "http_request", conditional=True,
               condition_note="Requires HTTP::collect in HTTP_REQUEST"),
            _s("LB_SELECTED",            "lb"),
            _s("SA_PICKED",              "lb"),
            _s("SERVER_INIT",            "lb"),
            _s("SERVER_CONNECTED",       "l4_server"),
            _s("HTTP_REQUEST_SEND",      "http_request_server"),
            _s("HTTP_REQUEST_RELEASE",   "http_request_server"),
            _s("HTTP_RESPONSE",          "http_response"),
            _s("HTTP_RESPONSE_DATA",     "http_response", conditional=True,
               condition_note="Requires HTTP::collect in HTTP_RESPONSE"),
            _s("HTTP_RESPONSE_RELEASE",  "http_response"),
            _s("SERVER_CLOSED",          "l4_teardown"),
            _s("CLIENT_CLOSED",          "l4_teardown"),
        ),
    ),

    # 3. TCP + ClientSSL + HTTP
    "tcp_clientssl_http": FlowChain(
        chain_id="tcp_clientssl_http",
        description="TCP + ClientSSL + HTTP (client-side TLS termination)",
        profiles=frozenset({"TCP", "CLIENTSSL", "HTTP"}),
        steps=(
            _s("RULE_INIT",                  "init"),
            _s("CLIENT_ACCEPTED",            "l4_client"),
            _s("CLIENTSSL_CLIENTHELLO",      "tls_client"),
            _s("CLIENTSSL_SERVERHELLO_SEND", "tls_client"),
            _s("CLIENTSSL_CLIENTCERT",       "tls_client", conditional=True,
               condition_note="Only with mutual TLS (client certificate required)"),
            _s("CLIENTSSL_HANDSHAKE",        "tls_client"),
            _s("HTTP_REQUEST",               "http_request"),
            _s("HTTP_REQUEST_DATA",          "http_request", conditional=True,
               condition_note="Requires HTTP::collect in HTTP_REQUEST"),
            _s("LB_SELECTED",               "lb"),
            _s("SA_PICKED",                 "lb"),
            _s("SERVER_INIT",               "lb"),
            _s("SERVER_CONNECTED",           "l4_server"),
            _s("HTTP_REQUEST_SEND",          "http_request_server"),
            _s("HTTP_REQUEST_RELEASE",       "http_request_server"),
            _s("HTTP_RESPONSE",              "http_response"),
            _s("HTTP_RESPONSE_DATA",         "http_response", conditional=True,
               condition_note="Requires HTTP::collect in HTTP_RESPONSE"),
            _s("HTTP_RESPONSE_RELEASE",      "http_response"),
            _s("SERVER_CLOSED",              "l4_teardown"),
            _s("CLIENT_CLOSED",              "l4_teardown"),
        ),
    ),

    # 4. TCP + ClientSSL + ServerSSL + HTTP (full HTTPS)
    "tcp_clientssl_serverssl_http": FlowChain(
        chain_id="tcp_clientssl_serverssl_http",
        description="Full HTTPS (ClientSSL + ServerSSL + HTTP)",
        profiles=frozenset({"TCP", "CLIENTSSL", "SERVERSSL", "HTTP"}),
        steps=(
            _s("RULE_INIT",                    "init"),
            _s("CLIENT_ACCEPTED",              "l4_client"),
            _s("CLIENTSSL_CLIENTHELLO",        "tls_client"),
            _s("CLIENTSSL_SERVERHELLO_SEND",   "tls_client"),
            _s("CLIENTSSL_CLIENTCERT",         "tls_client", conditional=True,
               condition_note="Only with mutual TLS (client certificate required)"),
            _s("CLIENTSSL_HANDSHAKE",          "tls_client"),
            _s("HTTP_REQUEST",                 "http_request"),
            _s("HTTP_REQUEST_DATA",            "http_request", conditional=True,
               condition_note="Requires HTTP::collect in HTTP_REQUEST"),
            _s("LB_SELECTED",                 "lb"),
            _s("SA_PICKED",                   "lb"),
            _s("SERVER_INIT",                  "lb"),
            _s("SERVER_CONNECTED",             "l4_server"),
            _s("SERVERSSL_CLIENTHELLO_SEND",   "tls_server"),
            _s("SERVERSSL_SERVERHELLO",        "tls_server"),
            _s("SERVERSSL_SERVERCERT",         "tls_server"),
            _s("SERVERSSL_HANDSHAKE",          "tls_server"),
            _s("HTTP_REQUEST_SEND",            "http_request_server"),
            _s("HTTP_REQUEST_RELEASE",         "http_request_server"),
            _s("HTTP_RESPONSE",                "http_response"),
            _s("HTTP_RESPONSE_DATA",           "http_response", conditional=True,
               condition_note="Requires HTTP::collect in HTTP_RESPONSE"),
            _s("HTTP_RESPONSE_RELEASE",        "http_response"),
            _s("SERVER_CLOSED",                "l4_teardown"),
            _s("CLIENT_CLOSED",                "l4_teardown"),
        ),
    ),

    # 5. Full HTTPS + HTTP::collect
    "tcp_clientssl_serverssl_http_collect": FlowChain(
        chain_id="tcp_clientssl_serverssl_http_collect",
        description="Full HTTPS with HTTP::collect (request + response body)",
        profiles=frozenset({"TCP", "CLIENTSSL", "SERVERSSL", "HTTP"}),
        steps=(
            _s("RULE_INIT",                    "init"),
            _s("CLIENT_ACCEPTED",              "l4_client"),
            _s("CLIENTSSL_CLIENTHELLO",        "tls_client"),
            _s("CLIENTSSL_SERVERHELLO_SEND",   "tls_client"),
            _s("CLIENTSSL_HANDSHAKE",          "tls_client"),
            _s("HTTP_REQUEST",                 "http_request"),
            _s("HTTP_REQUEST_DATA",            "http_request",
               condition_note="HTTP::collect called in HTTP_REQUEST"),
            _s("LB_SELECTED",                 "lb"),
            _s("SA_PICKED",                   "lb"),
            _s("SERVER_INIT",                  "lb"),
            _s("SERVER_CONNECTED",             "l4_server"),
            _s("SERVERSSL_CLIENTHELLO_SEND",   "tls_server"),
            _s("SERVERSSL_SERVERHELLO",        "tls_server"),
            _s("SERVERSSL_SERVERCERT",         "tls_server"),
            _s("SERVERSSL_HANDSHAKE",          "tls_server"),
            _s("HTTP_REQUEST_SEND",            "http_request_server"),
            _s("HTTP_REQUEST_RELEASE",         "http_request_server"),
            _s("HTTP_RESPONSE",                "http_response"),
            _s("HTTP_RESPONSE_DATA",           "http_response",
               condition_note="HTTP::collect called in HTTP_RESPONSE"),
            _s("HTTP_RESPONSE_RELEASE",        "http_response"),
            _s("SERVER_CLOSED",                "l4_teardown"),
            _s("CLIENT_CLOSED",                "l4_teardown"),
        ),
        notes="Both HTTP_REQUEST_DATA and HTTP_RESPONSE_DATA fire because "
              "the iRule calls HTTP::collect in the respective request/response "
              "events.  Client sends a POST with body.",
    ),

    # 6. UDP + DNS
    # UDP DNS: the DNS profile intercepts the query before L4 events.
    # DNS_REQUEST fires on the first datagram (the L4 CLIENT_ACCEPTED
    # and CLIENT_DATA events are not surfaced by the DNS profile).
    # A "UDP connection" is a connection-table entry for the client
    # IP:port that times out after idle (default 60s).
    "udp_dns": FlowChain(
        chain_id="udp_dns",
        description="UDP + DNS",
        profiles=frozenset({"UDP", "DNS"}),
        steps=(
            _s("RULE_INIT",      "init"),
            _s("DNS_REQUEST",    "dns_request"),
            _s("LB_SELECTED",   "lb"),
            _s("DNS_RESPONSE",   "dns_response"),
        ),
    ),

    # 7. TCP + DNS
    # DNS over TCP (zone transfers, large responses, DNS-over-TLS):
    # The TCP connection lifecycle wraps the DNS events.
    "tcp_dns": FlowChain(
        chain_id="tcp_dns",
        description="TCP + DNS",
        profiles=frozenset({"TCP", "DNS"}),
        steps=(
            _s("RULE_INIT",          "init"),
            _s("CLIENT_ACCEPTED",    "l4_client"),
            _s("CLIENT_DATA",        "l4_client", conditional=True,
               condition_note="Requires TCP::collect in CLIENT_ACCEPTED"),
            _s("DNS_REQUEST",        "dns_request"),
            _s("LB_SELECTED",       "lb"),
            _s("SA_PICKED",         "lb"),
            _s("SERVER_INIT",       "lb"),
            _s("SERVER_CONNECTED",   "l4_server"),
            _s("DNS_RESPONSE",       "dns_response"),
            _s("SERVER_CLOSED",      "l4_teardown"),
            _s("CLIENT_CLOSED",      "l4_teardown"),
        ),
    ),
}
# fmt: on


# PROFILE_SPECS — profile type metadata with capabilities

# fmt: off
PROFILE_SPECS: dict[str, ProfileSpec] = {
    "TCP":         ProfileSpec("TCP",       layer="transport",    side="both"),
    "UDP":         ProfileSpec("UDP",       layer="transport",    side="both"),
    "FASTL4":      ProfileSpec("FASTL4",    layer="transport",    side="both"),
    "SCTP":        ProfileSpec("SCTP",      layer="transport",    side="both"),
    "CLIENTSSL":   ProfileSpec("CLIENTSSL", layer="tls",          side="client",
                               requires=frozenset({"TCP"}),
                               capabilities=frozenset({
                                   "sni", "extensions", "sessionid",
                                   "cipher", "cert", "tls_data", "tls_control",
                               })),
    "SERVERSSL":   ProfileSpec("SERVERSSL", layer="tls",          side="server",
                               requires=frozenset({"TCP"}),
                               capabilities=frozenset({
                                   "sni", "extensions", "sessionid",
                                   "cipher", "cert", "tls_data", "tls_control",
                               })),
    "SSL_PERSISTENCE": ProfileSpec("SSL_PERSISTENCE", layer="tls", side="client",
                               requires=frozenset({"TCP"}),
                               capabilities=frozenset({
                                   "sni", "extensions", "sessionid",
                               })),
    "PERSIST":     ProfileSpec("PERSIST",   layer="tls",          side="both",
                               requires=frozenset({"TCP"})),
    "HTTP":        ProfileSpec("HTTP",      layer="application",  side="both",
                               requires=frozenset({"TCP"})),
    "FASTHTTP":    ProfileSpec("FASTHTTP",  layer="application",  side="both",
                               requires=frozenset({"TCP"})),
    "HTTP_PROXY_CONNECT": ProfileSpec("HTTP_PROXY_CONNECT", layer="application", side="both",
                               requires=frozenset({"HTTP"})),
    "HTTP2":       ProfileSpec("HTTP2",     layer="application",  side="both",
                               requires=frozenset({"HTTP"})),
    "DNS":         ProfileSpec("DNS",       layer="application",  side="both"),
    "SIP":         ProfileSpec("SIP",       layer="application",  side="both"),
    "SIPROUTER":   ProfileSpec("SIPROUTER", layer="application",  side="both",
                               requires=frozenset({"SIP"})),
    "SIPSESSION":  ProfileSpec("SIPSESSION",layer="application",  side="both",
                               requires=frozenset({"SIP"})),
    "FIX":         ProfileSpec("FIX",       layer="application",  side="both",
                               requires=frozenset({"TCP"})),
    "DIAMETER":    ProfileSpec("DIAMETER",  layer="application",  side="both",
                               requires=frozenset({"TCP"})),
    "DIAMETERSESSION": ProfileSpec("DIAMETERSESSION", layer="application", side="both",
                               requires=frozenset({"DIAMETER"})),
    "DIAMETER_ENDPOINT": ProfileSpec("DIAMETER_ENDPOINT", layer="application", side="both",
                               requires=frozenset({"DIAMETER"})),
    "MQTT":        ProfileSpec("MQTT",      layer="application",  side="both",
                               requires=frozenset({"TCP"})),
    "RTSP":        ProfileSpec("RTSP",      layer="application",  side="both",
                               requires=frozenset({"TCP"})),
    "GENERICMSG":  ProfileSpec("GENERICMSG",layer="application",  side="both",
                               requires=frozenset({"TCP"})),
    "MR":          ProfileSpec("MR",        layer="application",  side="both",
                               requires=frozenset({"TCP"})),
    "GTP":         ProfileSpec("GTP",       layer="application",  side="both"),
    "RADIUS":      ProfileSpec("RADIUS",    layer="application",  side="both"),
    "RADIUS_AAA":  ProfileSpec("RADIUS_AAA",layer="application",  side="both",
                               requires=frozenset({"RADIUS"})),
    "PCP":         ProfileSpec("PCP",       layer="application",  side="both"),
    "SOCKS":       ProfileSpec("SOCKS",     layer="application",  side="client",
                               requires=frozenset({"TCP"})),
    "TDS":         ProfileSpec("TDS",       layer="application",  side="both",
                               requires=frozenset({"TCP"})),
    "MSSQL":       ProfileSpec("MSSQL",     layer="application",  side="both",
                               requires=frozenset({"TDS"})),
    "IVS_ENTRY":   ProfileSpec("IVS_ENTRY", layer="application",  side="both",
                               requires=frozenset({"TCP"})),
    "ASM":         ProfileSpec("ASM",       layer="security",     side="both",
                               requires=frozenset({"HTTP"})),
    "ACCESS":      ProfileSpec("ACCESS",    layer="security",     side="client",
                               requires=frozenset({"TCP"})),
    "BOTDEFENSE":  ProfileSpec("BOTDEFENSE",layer="security",     side="client",
                               requires=frozenset({"HTTP"})),
    "ANTIFRAUD":   ProfileSpec("ANTIFRAUD", layer="security",     side="client",
                               requires=frozenset({"HTTP"})),
    "DOSL7":       ProfileSpec("DOSL7",     layer="security",     side="both",
                               requires=frozenset({"HTTP"})),
    "AUTH":        ProfileSpec("AUTH",      layer="security",     side="client",
                               requires=frozenset({"TCP"})),
    "ECA":         ProfileSpec("ECA",       layer="security",     side="client",
                               requires=frozenset({"TCP"})),
    "PROTOCOL_INSPECTION": ProfileSpec("PROTOCOL_INSPECTION", layer="security", side="both",
                               requires=frozenset({"TCP"})),
    "IPS":         ProfileSpec("IPS",       layer="security",     side="both",
                               requires=frozenset({"PROTOCOL_INSPECTION"})),
    "PEM":         ProfileSpec("PEM",       layer="security",     side="client",
                               requires=frozenset({"TCP"})),
    "TAP":         ProfileSpec("TAP",       layer="security",     side="client",
                               requires=frozenset({"TCP"})),
    "STREAM":      ProfileSpec("STREAM",    layer="acceleration", side="both",
                               requires=frozenset({"TCP"})),
    "WEBACCELERATION": ProfileSpec("WEBACCELERATION", layer="acceleration", side="both",
                               requires=frozenset({"HTTP"})),
    "CACHE":       ProfileSpec("CACHE",     layer="acceleration", side="both",
                               requires=frozenset({"HTTP"})),
    "CATEGORY":    ProfileSpec("CATEGORY",  layer="acceleration", side="both",
                               requires=frozenset({"HTTP"})),
    "CLASSIFICATION": ProfileSpec("CLASSIFICATION", layer="acceleration", side="both",
                               requires=frozenset({"TCP"})),
    "HTML":        ProfileSpec("HTML",      layer="acceleration", side="both",
                               requires=frozenset({"HTTP"})),
    "REWRITE":     ProfileSpec("REWRITE",   layer="acceleration", side="both",
                               requires=frozenset({"HTTP"})),
    "QOE":         ProfileSpec("QOE",       layer="acceleration", side="both",
                               requires=frozenset({"HTTP"})),
    "XML":         ProfileSpec("XML",       layer="acceleration", side="both",
                               requires=frozenset({"HTTP"})),
    "JSON":        ProfileSpec("JSON",      layer="acceleration", side="both",
                               requires=frozenset({"HTTP"})),
    "SSE":         ProfileSpec("SSE",       layer="acceleration", side="both",
                               requires=frozenset({"HTTP"})),
    "WS":          ProfileSpec("WS",        layer="acceleration", side="both",
                               requires=frozenset({"HTTP"})),
    "ICAP":        ProfileSpec("ICAP",      layer="acceleration", side="both",
                               requires=frozenset({"TCP"})),
    "REQUESTADAPT":  ProfileSpec("REQUESTADAPT",  layer="acceleration", side="both",
                               requires=frozenset({"HTTP"})),
    "RESPONSEADAPT": ProfileSpec("RESPONSEADAPT", layer="acceleration", side="both",
                               requires=frozenset({"HTTP"})),
    "AVR":         ProfileSpec("AVR",       layer="acceleration", side="both",
                               requires=frozenset({"HTTP"})),
    "NAME":        ProfileSpec("NAME",      layer="application",  side="both",
                               requires=frozenset({"TCP"})),
    "FLOW":        ProfileSpec("FLOW",      layer="application",  side="both"),
    "CONNECTOR":   ProfileSpec("CONNECTOR", layer="application",  side="both",
                               requires=frozenset({"TCP"})),
    "L7CHECK":     ProfileSpec("L7CHECK",   layer="application",  side="both",
                               requires=frozenset({"TCP"})),
    "DATAGRAM":    ProfileSpec("DATAGRAM",  layer="application",  side="both",
                               requires=frozenset({"UDP"})),
    "LSN":         ProfileSpec("LSN",       layer="application",  side="client"),
}
# fmt: on

# PROTOCOL_NAMESPACE_SPECS — command namespace availability

# fmt: off
PROTOCOL_NAMESPACE_SPECS: dict[str, ProtocolNamespaceSpec] = {
    "HTTP":   ProtocolNamespaceSpec("HTTP",  profiles=frozenset({"HTTP", "FASTHTTP", "HTTP_PROXY_CONNECT"}),
                                    layer="application", side="both"),
    "HTTP2":  ProtocolNamespaceSpec("HTTP2", profiles=frozenset({"HTTP2"}),
                                    layer="application", side="both"),
    "SSL":    ProtocolNamespaceSpec("SSL",   profiles=frozenset({"CLIENTSSL", "SERVERSSL", "SSL_PERSISTENCE"}),
                                    layer="tls", side="both", side_selectable=True),
    "TCP":    ProtocolNamespaceSpec("TCP",   profiles=frozenset({"TCP"}),
                                    layer="transport", side="both", side_selectable=True),
    "UDP":    ProtocolNamespaceSpec("UDP",   profiles=frozenset({"UDP"}),
                                    layer="transport", side="both", side_selectable=True),
    "SCTP":   ProtocolNamespaceSpec("SCTP",  profiles=frozenset({"SCTP"}),
                                    layer="transport", side="both"),
    "IP":     ProtocolNamespaceSpec("IP",    profiles=frozenset(),
                                    layer="transport", side="both", side_selectable=True),
    "LB":     ProtocolNamespaceSpec("LB",    profiles=frozenset(),
                                    layer="load_balance", side="global"),
    # Application protocols
    "DNS":    ProtocolNamespaceSpec("DNS",   profiles=frozenset({"DNS"}),
                                    layer="application", side="both"),
    "SIP":    ProtocolNamespaceSpec("SIP",   profiles=frozenset({"SIP", "SIPROUTER", "SIPSESSION"}),
                                    layer="application", side="both"),
    "FIX":    ProtocolNamespaceSpec("FIX",   profiles=frozenset({"FIX"}),
                                    layer="application", side="both"),
    "DIAMETER": ProtocolNamespaceSpec("DIAMETER", profiles=frozenset({"DIAMETER", "DIAMETERSESSION", "DIAMETER_ENDPOINT"}),
                                    layer="application", side="both"),
    "MQTT":   ProtocolNamespaceSpec("MQTT",  profiles=frozenset({"MQTT"}),
                                    layer="application", side="both"),
    "MR":     ProtocolNamespaceSpec("MR",    profiles=frozenset({"MR"}),
                                    layer="application", side="both"),
    "GENERICMESSAGE": ProtocolNamespaceSpec("GENERICMESSAGE", profiles=frozenset({"GENERICMSG"}),
                                    layer="application", side="both"),
    "RTSP":   ProtocolNamespaceSpec("RTSP",  profiles=frozenset({"RTSP"}),
                                    layer="application", side="both"),
    "GTP":    ProtocolNamespaceSpec("GTP",   profiles=frozenset({"GTP"}),
                                    layer="application", side="both"),
    "RADIUS": ProtocolNamespaceSpec("RADIUS",profiles=frozenset({"RADIUS", "RADIUS_AAA"}),
                                    layer="application", side="both"),
    "PCP":    ProtocolNamespaceSpec("PCP",   profiles=frozenset({"PCP"}),
                                    layer="application", side="both"),
    "SOCKS":  ProtocolNamespaceSpec("SOCKS", profiles=frozenset({"SOCKS"}),
                                    layer="application", side="client"),
    "TDS":    ProtocolNamespaceSpec("TDS",   profiles=frozenset({"TDS", "MSSQL"}),
                                    layer="application", side="both"),
    "CONNECTOR": ProtocolNamespaceSpec("CONNECTOR", profiles=frozenset({"CONNECTOR"}),
                                    layer="application", side="both"),
    "DATAGRAM": ProtocolNamespaceSpec("DATAGRAM", profiles=frozenset({"DATAGRAM"}),
                                    layer="application", side="both"),
    "FLOW":   ProtocolNamespaceSpec("FLOW",  profiles=frozenset({"FLOW"}),
                                    layer="application", side="both"),
    "NAME":   ProtocolNamespaceSpec("NAME",  profiles=frozenset({"NAME"}),
                                    layer="application", side="both"),
    "LSN":    ProtocolNamespaceSpec("LSN",   profiles=frozenset({"LSN"}),
                                    layer="application", side="client"),
    "MESSAGE": ProtocolNamespaceSpec("MESSAGE", profiles=frozenset({"MR"}),
                                    layer="application", side="both"),
    # Security
    "ACCESS": ProtocolNamespaceSpec("ACCESS",profiles=frozenset({"ACCESS"}),
                                    layer="security", side="client"),
    "ASM":    ProtocolNamespaceSpec("ASM",   profiles=frozenset({"ASM"}),
                                    layer="security", side="both"),
    "AUTH":   ProtocolNamespaceSpec("AUTH",  profiles=frozenset({"AUTH"}),
                                    layer="security", side="client"),
    "ANTIFRAUD": ProtocolNamespaceSpec("ANTIFRAUD", profiles=frozenset({"ANTIFRAUD"}),
                                    layer="security", side="client"),
    "AVR":    ProtocolNamespaceSpec("AVR",   profiles=frozenset({"AVR"}),
                                    layer="acceleration", side="both"),
    "BOTDEFENSE": ProtocolNamespaceSpec("BOTDEFENSE", profiles=frozenset({"BOTDEFENSE"}),
                                    layer="security", side="client"),
    "DOSL7":  ProtocolNamespaceSpec("DOSL7", profiles=frozenset({"DOSL7"}),
                                    layer="security", side="both"),
    "ECA":    ProtocolNamespaceSpec("ECA",   profiles=frozenset({"ECA"}),
                                    layer="security", side="client"),
    "PEM":    ProtocolNamespaceSpec("PEM",   profiles=frozenset({"PEM"}),
                                    layer="security", side="client"),
    "PROTOCOL_INSPECTION": ProtocolNamespaceSpec("PROTOCOL_INSPECTION", profiles=frozenset({"PROTOCOL_INSPECTION", "IPS"}),
                                    layer="security", side="both"),
    "TAP":    ProtocolNamespaceSpec("TAP",   profiles=frozenset({"TAP"}),
                                    layer="security", side="client"),
    # Acceleration / content
    "STREAM": ProtocolNamespaceSpec("STREAM",profiles=frozenset({"STREAM"}),
                                    layer="acceleration", side="both"),
    "CACHE":  ProtocolNamespaceSpec("CACHE", profiles=frozenset({"CACHE", "WEBACCELERATION"}),
                                    layer="acceleration", side="both"),
    "CATEGORY": ProtocolNamespaceSpec("CATEGORY", profiles=frozenset({"CATEGORY"}),
                                    layer="acceleration", side="both"),
    "CLASSIFICATION": ProtocolNamespaceSpec("CLASSIFICATION", profiles=frozenset({"CLASSIFICATION"}),
                                    layer="acceleration", side="both"),
    "HTML":   ProtocolNamespaceSpec("HTML",  profiles=frozenset({"HTML"}),
                                    layer="acceleration", side="both"),
    "ICAP":   ProtocolNamespaceSpec("ICAP",  profiles=frozenset({"ICAP"}),
                                    layer="acceleration", side="both"),
    "JSON":   ProtocolNamespaceSpec("JSON",  profiles=frozenset({"JSON"}),
                                    layer="acceleration", side="both"),
    "L7CHECK": ProtocolNamespaceSpec("L7CHECK", profiles=frozenset({"L7CHECK"}),
                                    layer="application", side="both"),
    "IVS_ENTRY": ProtocolNamespaceSpec("IVS_ENTRY", profiles=frozenset({"IVS_ENTRY"}),
                                    layer="application", side="both"),
    "QOE":    ProtocolNamespaceSpec("QOE",   profiles=frozenset({"QOE"}),
                                    layer="acceleration", side="both"),
    "REWRITE":ProtocolNamespaceSpec("REWRITE",profiles=frozenset({"REWRITE"}),
                                    layer="acceleration", side="both"),
    "SSE":    ProtocolNamespaceSpec("SSE",   profiles=frozenset({"SSE"}),
                                    layer="acceleration", side="both"),
    "WS":     ProtocolNamespaceSpec("WS",    profiles=frozenset({"WS"}),
                                    layer="acceleration", side="both"),
    "XML":    ProtocolNamespaceSpec("XML",   profiles=frozenset({"XML"}),
                                    layer="acceleration", side="both"),
    # Utility / global namespaces (no profile requirement)
    "AAA":    ProtocolNamespaceSpec("AAA",   profiles=frozenset(),
                                    layer="security", side="global"),
    "ACL":    ProtocolNamespaceSpec("ACL",   profiles=frozenset(),
                                    layer="security", side="global"),
    "ADAPT":  ProtocolNamespaceSpec("ADAPT", profiles=frozenset({"HTTP", "REQUESTADAPT", "RESPONSEADAPT"}),
                                    layer="acceleration", side="both"),
    "AES":    ProtocolNamespaceSpec("AES",   profiles=frozenset(),
                                    layer="utility", side="global"),
    "ASN1":   ProtocolNamespaceSpec("ASN1",  profiles=frozenset(),
                                    layer="utility", side="global"),
    "BIGPROTO": ProtocolNamespaceSpec("BIGPROTO", profiles=frozenset(),
                                    layer="utility", side="global"),
    "BIGTCP": ProtocolNamespaceSpec("BIGTCP", profiles=frozenset(),
                                    layer="utility", side="global"),
    "BWC":    ProtocolNamespaceSpec("BWC",   profiles=frozenset(),
                                    layer="utility", side="global"),
    "CLASSIFY": ProtocolNamespaceSpec("CLASSIFY", profiles=frozenset({"FASTHTTP"}),
                                    layer="application", side="both"),
    "COMPRESS": ProtocolNamespaceSpec("COMPRESS", profiles=frozenset({"HTTP", "FASTHTTP"}),
                                    layer="application", side="both"),
    "CRYPTO": ProtocolNamespaceSpec("CRYPTO", profiles=frozenset(),
                                    layer="utility", side="global"),
    "DECOMPRESS": ProtocolNamespaceSpec("DECOMPRESS", profiles=frozenset(),
                                    layer="utility", side="global"),
    "DHCP":   ProtocolNamespaceSpec("DHCP",  profiles=frozenset(),
                                    layer="application", side="both"),
    "DHCPv4": ProtocolNamespaceSpec("DHCPv4", profiles=frozenset(),
                                    layer="application", side="both"),
    "DHCPv6": ProtocolNamespaceSpec("DHCPv6", profiles=frozenset(),
                                    layer="application", side="both"),
    "DNSMSG": ProtocolNamespaceSpec("DNSMSG", profiles=frozenset(),
                                    layer="application", side="both"),
    "DSLITE": ProtocolNamespaceSpec("DSLITE", profiles=frozenset(),
                                    layer="application", side="global"),
    "FLOWTABLE": ProtocolNamespaceSpec("FLOWTABLE", profiles=frozenset(),
                                    layer="utility", side="global"),
    "FTP":    ProtocolNamespaceSpec("FTP",   profiles=frozenset(),
                                    layer="application", side="global"),
    "HA":     ProtocolNamespaceSpec("HA",    profiles=frozenset(),
                                    layer="utility", side="global"),
    "HSL":    ProtocolNamespaceSpec("HSL",   profiles=frozenset(),
                                    layer="utility", side="global"),
    "IKE":    ProtocolNamespaceSpec("IKE",   profiles=frozenset(),
                                    layer="application", side="global"),
    "ILX":    ProtocolNamespaceSpec("ILX",   profiles=frozenset(),
                                    layer="utility", side="global"),
    "IMAP":   ProtocolNamespaceSpec("IMAP",  profiles=frozenset(),
                                    layer="application", side="global"),
    "IPFIX":  ProtocolNamespaceSpec("IPFIX", profiles=frozenset(),
                                    layer="utility", side="global"),
    "ISESSION": ProtocolNamespaceSpec("ISESSION", profiles=frozenset(),
                                    layer="application", side="global"),
    "ISTATS": ProtocolNamespaceSpec("ISTATS", profiles=frozenset(),
                                    layer="utility", side="global"),
    "LDAP":   ProtocolNamespaceSpec("LDAP",  profiles=frozenset(),
                                    layer="application", side="global"),
    "LINK":   ProtocolNamespaceSpec("LINK",  profiles=frozenset(),
                                    layer="utility", side="global"),
    "NSH":    ProtocolNamespaceSpec("NSH",   profiles=frozenset(),
                                    layer="application", side="global"),
    "NTLM":   ProtocolNamespaceSpec("NTLM",  profiles=frozenset(),
                                    layer="application", side="global"),
    "OFFBOX": ProtocolNamespaceSpec("OFFBOX", profiles=frozenset(),
                                    layer="application", side="global"),
    "ONECONNECT": ProtocolNamespaceSpec("ONECONNECT", profiles=frozenset(),
                                    layer="application", side="global"),
    "POLICY": ProtocolNamespaceSpec("POLICY", profiles=frozenset(),
                                    layer="application", side="both"),
    "POP3":   ProtocolNamespaceSpec("POP3",  profiles=frozenset(),
                                    layer="application", side="global"),
    "PROFILE": ProtocolNamespaceSpec("PROFILE", profiles=frozenset(),
                                    layer="utility", side="global"),
    "PSC":    ProtocolNamespaceSpec("PSC",   profiles=frozenset(),
                                    layer="security", side="global"),
    "PSM":    ProtocolNamespaceSpec("PSM",   profiles=frozenset({"HTTP"}),
                                    layer="application", side="both"),
    "RESOLVER": ProtocolNamespaceSpec("RESOLVER", profiles=frozenset(),
                                    layer="application", side="global"),
    "ROUTE":  ProtocolNamespaceSpec("ROUTE", profiles=frozenset(),
                                    layer="utility", side="global"),
    "SDP":    ProtocolNamespaceSpec("SDP",   profiles=frozenset(),
                                    layer="application", side="global"),
    "SIPALG": ProtocolNamespaceSpec("SIPALG", profiles=frozenset({"MR", "SIP"}),
                                    layer="application", side="both"),
    "SMTPS":  ProtocolNamespaceSpec("SMTPS", profiles=frozenset(),
                                    layer="application", side="global"),
    "STATS":  ProtocolNamespaceSpec("STATS", profiles=frozenset(),
                                    layer="utility", side="global"),
    "TMM":    ProtocolNamespaceSpec("TMM",   profiles=frozenset(),
                                    layer="utility", side="global"),
    "URI":    ProtocolNamespaceSpec("URI",   profiles=frozenset({"HTTP"}),
                                    layer="application", side="both"),
    "VALIDATE": ProtocolNamespaceSpec("VALIDATE", profiles=frozenset(),
                                    layer="utility", side="global"),
    "WAM":    ProtocolNamespaceSpec("WAM",   profiles=frozenset({"HTTP"}),
                                    layer="application", side="both"),
    "WEBSSO": ProtocolNamespaceSpec("WEBSSO", profiles=frozenset({"ACCESS", "HTTP"}),
                                    layer="security", side="client"),
    "X509":   ProtocolNamespaceSpec("X509",  profiles=frozenset(),
                                    layer="utility", side="global"),
    "XLAT":   ProtocolNamespaceSpec("XLAT",  profiles=frozenset(),
                                    layer="application", side="global"),
}
# fmt: on

# MODIFICATION_SPECS — commands that change the profile stack

# fmt: off
MODIFICATION_SPECS: dict[str, StackModification] = {
    "SSL::disable": StackModification(
        command="SSL::disable",
        side=None,          # resolved from arg: clientside/serverside
        removes_profile=None,  # resolved: CLIENTSSL or SERVERSSL per side
    ),
    "SSL::enable": StackModification(
        command="SSL::enable",
        side=None,
        adds_profile=None,  # resolved per side arg
    ),
    "HTTP::disable": StackModification(
        command="HTTP::disable",
        side=None,
        removes_profile="HTTP",
    ),
    "HTTP::enable": StackModification(
        command="HTTP::enable",
        side=None,
        adds_profile="HTTP",
    ),
}
# fmt: on


def event_satisfies(
    event: EventProps,
    requires: EventRequires,
    event_name: str | None = None,
) -> bool:
    """Return True if *event* properties satisfy the command *requires*.

    When *event_name* is provided, it is also checked against
    ``requires.also_in`` for an unconditional match.
    """
    if requires.init_only:
        return event_name == "RULE_INIT"
    if event_name and requires.also_in and event_name in requires.also_in:
        return True
    if requires.flow and not event.flow:
        return False
    if requires.client_side and not event.client_side:
        return False
    if requires.server_side and not event.server_side:
        return False
    if requires.transport is not None:
        et = event.transport
        if isinstance(et, tuple):
            if requires.transport not in et:
                return False
        elif et != requires.transport:
            return False
    if requires.profiles and not profile_stack_satisfies(requires.profiles, event.implied_profiles):
        return False
    return True


def missing_requirements_description(
    event_name: str,
    event: EventProps,
    requires: EventRequires,
) -> str:
    """Build a human-readable description of unmet requirements."""
    if requires.init_only:
        return "only valid in RULE_INIT"
    reasons: list[str] = []
    if requires.flow and not event.flow:
        reasons.append("no active traffic flow (non-flow event)")
    if requires.client_side and not event.client_side:
        reasons.append("no client-side connection")
    if requires.server_side and not event.server_side:
        reasons.append("no server-side connection")
    if requires.transport is not None:
        et = event.transport
        mismatch = (
            (isinstance(et, tuple) and requires.transport not in et)
            or (not isinstance(et, tuple) and et != requires.transport)
        )
        if mismatch:
            actual = "/".join(et) if isinstance(et, tuple) else (et or "none")
            reasons.append(f"transport is {actual}, needs {requires.transport}")
    if requires.profiles and not profile_stack_satisfies(requires.profiles, event.implied_profiles):
        reasons.append(f"requires profile {' or '.join(sorted(requires.profiles))}")
    return "; ".join(reasons)


def expand_profile_stack(profiles: frozenset[str]) -> frozenset[str]:
    """Return *profiles* plus all transitive ``ProfileSpec.requires`` parents."""
    expanded: set[str] = {p.upper() for p in profiles}
    pending: list[str] = list(expanded)
    while pending:
        current = pending.pop()
        spec = PROFILE_SPECS.get(current)
        if spec is None:
            continue
        for required in spec.requires:
            name = required.upper()
            if name in expanded:
                continue
            expanded.add(name)
            pending.append(name)
    return frozenset(expanded)


def profile_stack_satisfies(
    required_profiles: frozenset[str],
    active_profiles: frozenset[str],
) -> bool:
    """Return True if *active_profiles* satisfies any required profile stack.

    ``required_profiles`` is treated as OR semantics (same as existing
    ``EventRequires.profiles`` behaviour).  For each candidate profile,
    its transitive dependency stack must be present.
    """
    if not required_profiles:
        return True
    active_expanded = expand_profile_stack(active_profiles)
    for candidate in required_profiles:
        required_stack = expand_profile_stack(frozenset({candidate}))
        if required_stack <= active_expanded:
            return True
    return False


def profile_info_description(
    requires: EventRequires,
    file_profiles: frozenset[str],
) -> str | None:
    """Return an informational string if required profiles are unconfirmed.

    Returns ``None`` when no profile requirement exists or when all
    required profiles are covered by *file_profiles*.
    """
    if not requires.profiles:
        return None
    if profile_stack_satisfies(requires.profiles, file_profiles):
        return None
    return f"assumes profile {' or '.join(sorted(requires.profiles))} on the virtual server"


def deprecated_events() -> frozenset[str]:
    """Return all event names flagged as deprecated."""
    return frozenset(name for name, props in EVENT_PROPS.items() if props.deprecated)


def hot_events() -> frozenset[str]:
    """Return all event names flagged as high-frequency."""
    return frozenset(name for name, props in EVENT_PROPS.items() if props.hot)


def common_events() -> frozenset[str]:
    """Return all event names flagged as commonly used."""
    return frozenset(name for name, props in EVENT_PROPS.items() if props.common)


def get_event_props(event_name: str) -> EventProps | None:
    """Look up event properties, returning ``None`` for unknown events."""
    return EVENT_PROPS.get(event_name)


def events_matching(requires: EventRequires) -> list[str]:
    """Return sorted event names whose properties satisfy *requires*."""
    return sorted(
        name
        for name, props in EVENT_PROPS.items()
        if event_satisfies(props, requires, event_name=name)
    )


def event_side_label(props: EventProps) -> str:
    """Derive a human-readable side label from *props*.

    Returns one of ``"client-side"``, ``"server-side"``,
    ``"client-side and server-side"``, or ``"global"``.
    """
    if props.client_side and props.server_side:
        return "client-side and server-side"
    if props.client_side:
        return "client-side"
    if props.server_side:
        return "server-side"
    return "global"


def event_side_label_short(props: EventProps) -> str:
    """Short form for completion detail strings."""
    if props.client_side and props.server_side:
        return "both sides"
    if props.client_side:
        return "client-side"
    if props.server_side:
        return "server-side"
    return "global"


def get_event_description(event_name: str) -> str | None:
    """Return the description for *event_name*, or ``None`` if unknown."""
    return EVENT_DESCRIPTIONS.get(event_name)


def get_event_detail(event_name: str) -> str:
    """Build a short detail string for completion items.

    Combines the side label with ``"iRules event"`` for use in
    ``ArgumentValueSpec.detail``.
    """
    props = EVENT_PROPS.get(event_name)
    if props is not None:
        return f"iRules event ({event_side_label_short(props)})"
    return "iRules event"


# File-level profile support — regex patterns built from registry data

_PROFILE_DIRECTIVE_RE = re.compile(
    r"^\s*#\s*profiles?\s*:\s*(.+)",
    re.IGNORECASE,
)

# Build a regex that matches all known event names from EVENT_PROPS.
# This is derived from the registry data at module load time rather than
# being hardcoded separately.
_event_names: list[str] = sorted(EVENT_PROPS, key=lambda n: len(n), reverse=True)
_ALL_EVENT_NAMES_PATTERN = "|".join(_event_names)
_WHEN_EVENT_RE = re.compile(r"\bwhen\s+([A-Z_][A-Z0-9_]*)")


def parse_profile_directive(source: str) -> frozenset[str]:
    """Scan leading comment lines for ``# profiles: HTTP, CLIENTSSL ...``.

    Accepts both comma-separated and space-separated profile names.
    Only checks the first 20 lines.  Stops at the first non-comment,
    non-blank line.
    """
    profiles: set[str] = set()
    for line in source.split("\n", 20)[:20]:
        stripped = line.strip()
        if not stripped:
            continue
        if not stripped.startswith("#"):
            break
        m = _PROFILE_DIRECTIVE_RE.match(stripped)
        if m:
            for token in re.split(r"[,\s]+", m.group(1)):
                if token:
                    profiles.add(token.upper())
    return frozenset(profiles)


def scan_file_events(source: str) -> frozenset[str]:
    """Return the set of event names from all ``when EVENT`` blocks."""
    return frozenset(m.group(1) for m in _WHEN_EVENT_RE.finditer(source))


def infer_profiles_from_events(event_names: frozenset[str]) -> frozenset[str]:
    """Infer attached profiles from the set of events present in a file.

    For example, a ``CLIENTSSL_HANDSHAKE`` handler implies CLIENTSSL is
    attached; an ``HTTP_REQUEST`` handler implies HTTP.
    """
    inferred: set[str] = set()
    for event_name in event_names:
        props = EVENT_PROPS.get(event_name)
        if props is not None:
            inferred.update(props.implied_profiles)
    return frozenset(inferred)


def compute_file_profiles(source: str) -> frozenset[str]:
    """Compute effective profiles for a file.

    Combines explicit ``# profiles: ...`` directive with profiles
    inferred from ``when`` event handlers present in the file.
    """
    return expand_profile_stack(
        parse_profile_directive(source) | infer_profiles_from_events(scan_file_events(source))
    )


def order_events(file_events: frozenset[str]) -> list[str]:
    """Return the subset of *file_events* in canonical firing order.

    Events not present in ``MASTER_ORDER`` are appended at the end
    in sorted order (so no event is silently dropped).
    """
    known: list[tuple[int, str]] = []
    unknown: list[str] = []
    for evt in file_events:
        idx = _MASTER_INDEX.get(evt)
        if idx is not None:
            known.append((idx, evt))
        else:
            unknown.append(evt)
    known.sort()
    return [evt for _idx, evt in known] + sorted(unknown)


def order_events_for_file(source: str) -> list[str]:
    """Scan ``when`` blocks in *source* and return events in firing order."""
    return order_events(scan_file_events(source))


def event_index(event_name: str) -> int | None:
    """Return the position of *event_name* in the master ordering.

    Returns ``None`` for unknown events.
    """
    return _MASTER_INDEX.get(event_name)


def events_before(event_name: str, file_events: frozenset[str]) -> list[str]:
    """Return events from *file_events* that fire before *event_name*."""
    target = _MASTER_INDEX.get(event_name)
    if target is None:
        return []
    pairs: list[tuple[int, str]] = []
    for e in file_events:
        i = _MASTER_INDEX.get(e)
        if i is not None and i < target:
            pairs.append((i, e))
    pairs.sort()
    return [evt for _i, evt in pairs]


def events_after(event_name: str, file_events: frozenset[str]) -> list[str]:
    """Return events from *file_events* that fire after *event_name*."""
    target = _MASTER_INDEX.get(event_name)
    if target is None:
        return []
    pairs: list[tuple[int, str]] = []
    for e in file_events:
        i = _MASTER_INDEX.get(e)
        if i is not None and i > target:
            pairs.append((i, e))
    pairs.sort()
    return [evt for _i, evt in pairs]


def is_per_request(event_name: str) -> bool:
    """Return True if *event_name* can fire multiple times per connection."""
    return event_name in PER_REQUEST


def is_once_per_connection(event_name: str) -> bool:
    """Return True if *event_name* fires at most once per connection."""
    return event_name in ONCE_PER_CONNECTION


def event_multiplicity(event_name: str) -> str:
    """Return the multiplicity category for a single event name.

    Returns one of ``"init"``, ``"once_per_connection"``,
    ``"per_request"``, or ``"unknown"``.
    """
    if event_name == "RULE_INIT":
        return "init"
    if event_name in ONCE_PER_CONNECTION:
        return "once_per_connection"
    if event_name in PER_REQUEST:
        return "per_request"
    return "unknown"


def variable_scope_note(set_event: str, read_event: str) -> str | None:
    """Return a note if a variable set in *set_event* has scoping concerns
    when read in *read_event*.

    Returns ``None`` when there is no concern.
    """
    if set_event == "RULE_INIT":
        return None  # static vars are always available
    set_idx = _MASTER_INDEX.get(set_event)
    read_idx = _MASTER_INDEX.get(read_event)
    if set_idx is None or read_idx is None:
        return None
    if read_idx < set_idx:
        return f"variable set in {set_event} is not yet available in {read_event} (fires earlier)"
    if set_event in PER_REQUEST and read_event in ONCE_PER_CONNECTION:
        return (
            f"variable set in {set_event} (per-request) may not "
            f"be set yet in {read_event} (per-connection)"
        )
    return None


def chain_for_profiles(profiles: frozenset[str]) -> FlowChain | None:
    """Find the best-matching flow chain for a set of profile types.

    Returns the chain whose ``profiles`` field has the largest
    intersection with *profiles*, breaking ties by fewest extra profiles
    (i.e. most specific match).  Returns ``None`` when no chain matches.
    """
    upper = frozenset(p.upper() for p in profiles)
    best: FlowChain | None = None
    best_score = (-1, 999)  # (intersection_size, -extra_count)
    for chain in FLOW_CHAINS.values():
        overlap = len(chain.profiles & upper)
        extra = len(chain.profiles - upper)
        score = (overlap, -extra)
        if overlap > 0 and score > best_score:
            best = chain
            best_score = score
    return best


def chain_event_names(chain: FlowChain) -> list[str]:
    """Return the ordered event names from a flow chain."""
    return [step.event for step in chain.steps]


def compatible_connection_predecessors(event: str) -> frozenset[str]:
    """Return ``ONCE_PER_CONNECTION`` events that precede *event* in at least
    one :data:`FLOW_CHAINS` entry.

    ``RULE_INIT`` is excluded (it is device-scoped, not connection-scoped).
    Returns an empty set when *event* does not appear in any flow chain.

    This prevents IRULE4004 from suggesting hoisting targets that belong to
    incompatible profile stacks (e.g. ``ACCESS_POLICY_COMPLETED`` for a DNS
    virtual server).
    """
    result: set[str] = set()
    for chain in FLOW_CHAINS.values():
        chain_events = [s.event for s in chain.steps]
        try:
            idx = chain_events.index(event)
        except ValueError:
            continue
        for i in range(idx):
            e = chain_events[i]
            if e in ONCE_PER_CONNECTION and e != "RULE_INIT":
                result.add(e)
    return frozenset(result)
