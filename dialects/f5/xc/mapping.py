"""Declarative mapping from iRule commands and events to XC constructs.

Used by the translator to determine whether each iRule construct has an
XC equivalent and how to map it.
"""

from __future__ import annotations

from dataclasses import dataclass

from .xc_model import XCConstructKind


@dataclass(frozen=True, slots=True)
class XCMapping:
    """Describes how an iRule command maps to an XC construct."""

    kind: XCConstructKind
    xc_description: str
    note: str = ""


# Command → XC construct mappings

COMMAND_XC_MAP: dict[str, XCMapping] = {
    "pool": XCMapping(
        kind=XCConstructKind.ORIGIN_POOL,
        xc_description="XC origin pool selection",
    ),
    "HTTP::redirect": XCMapping(
        kind=XCConstructKind.ROUTE,
        xc_description="XC redirect route",
    ),
    "HTTP::respond": XCMapping(
        kind=XCConstructKind.ROUTE,
        xc_description="XC direct response route",
    ),
    "HTTP::header": XCMapping(
        kind=XCConstructKind.HEADER_ACTION,
        xc_description="XC load balancer header processing",
    ),
    "HTTP::cookie": XCMapping(
        kind=XCConstructKind.HEADER_ACTION,
        xc_description="XC load balancer cookie processing",
        note="Limited to add/remove; complex cookie logic needs manual config",
    ),
    "HTTP::uri": XCMapping(
        kind=XCConstructKind.ROUTE,
        xc_description="XC route path rewrite (setter only)",
        note="Getter translates to match criteria; setter translates to route rewrite",
    ),
    "HTTP::path": XCMapping(
        kind=XCConstructKind.ROUTE,
        xc_description="XC route path match / rewrite",
        note="Getter translates to match criteria; setter translates to route rewrite",
    ),
    "HTTP::host": XCMapping(
        kind=XCConstructKind.ROUTE,
        xc_description="XC route host match",
        note="Getter translates to domain match criteria",
    ),
    "class": XCMapping(
        kind=XCConstructKind.SERVICE_POLICY_RULE,
        xc_description="XC service policy rule (data-group lookup)",
        note="Each data-group entry may map to a separate rule",
    ),
}

# Header subcommand → XC operation mapping
HEADER_SUBCOMMAND_MAP: dict[str, str] = {
    "insert": "add",
    "replace": "replace",
    "remove": "remove",
    "value": "add",  # HTTP::header value used as getter, but insert context
}

# Event → XC mapping

# Events with direct XC equivalents
TRANSLATABLE_EVENTS: dict[str, str] = {
    "HTTP_REQUEST": "L7 routes + service policies (request processing)",
    "HTTP_RESPONSE": "Load balancer response header processing",
}

# Events WITHOUT XC equivalents
UNTRANSLATABLE_EVENTS: dict[str, str] = {
    "RULE_INIT": "No XC equivalent. Static configuration replaces init logic.",
    "CLIENT_ACCEPTED": "L4 event — no XC equivalent for TCP-level logic.",
    "CLIENT_CLOSED": "L4 event — no XC equivalent.",
    "CLIENT_DATA": "L4 data event — requires App Stack.",
    "SERVER_CONNECTED": "Backend connection event — no XC equivalent.",
    "SERVER_CLOSED": "Backend connection event — no XC equivalent.",
    "SERVER_DATA": "Backend data event — no XC equivalent.",
    "LB_FAILED": "Fallback logic — use XC origin pool health checks instead.",
    "LB_SELECTED": "LB decision point — handled by XC origin pool config.",
    "PERSIST_DOWN": "Persistence event — no XC equivalent.",
    "FLOW_INIT": "L4 flow event — no XC equivalent.",
    "HTTP_REQUEST_DATA": "HTTP payload event — no direct XC equivalent.",
    "HTTP_RESPONSE_DATA": "HTTP payload event — no direct XC equivalent.",
    "HTTP_REQUEST_SEND": "Proxy-side event — no XC equivalent.",
    "HTTP_REQUEST_RELEASE": "Proxy-side event — no XC equivalent.",
    "HTTP_RESPONSE_RELEASE": "Proxy-side event — no XC equivalent.",
    "HTTP_RESPONSE_CONTINUE": "100-continue event — no XC equivalent.",
    "TCP_REQUEST": "L4 TCP event — no XC equivalent.",
    "TCP_RESPONSE": "L4 TCP event — no XC equivalent.",
    "DNS_REQUEST": "DNS event — use XC DNS load balancer instead.",
    "DNS_RESPONSE": "DNS event — use XC DNS load balancer instead.",
    "SIP_REQUEST": "SIP event — no XC equivalent.",
    "SIP_RESPONSE": "SIP event — no XC equivalent.",
}

# Events that map to separate XC features (not LB routes)
ADVISORY_EVENTS: dict[str, str] = {
    "CLIENTSSL_HANDSHAKE": "TLS — configure via XC TLS settings on the load balancer.",
    "CLIENTSSL_CLIENTCERT": "Client cert — configure via XC mTLS settings.",
    "SERVERSSL_HANDSHAKE": "Backend TLS — configure via XC origin pool TLS settings.",
    "ACCESS_POLICY_AGENT_EVENT": "APM — configure via XC service policy or external IdP.",
    "ACCESS_ACL_ALLOWED": "ACL — configure via XC service policy.",
    "ACCESS_ACL_DENIED": "ACL — configure via XC service policy.",
    "ASM_REQUEST_DONE": "WAF — configure via XC App Firewall.",
    "ASM_REQUEST_BLOCKING": "WAF — configure via XC App Firewall.",
    "ASM_REQUEST_VIOLATION": "WAF — configure via XC App Firewall.",
    "BOTDEFENSE_REQUEST": "Bot defense — configure via XC Bot Defense.",
    "BOTDEFENSE_ACTION": "Bot defense — configure via XC Bot Defense.",
    "CACHE_REQUEST": "Caching — configure via XC CDN settings.",
    "CACHE_RESPONSE": "Caching — configure via XC CDN settings.",
    "DOSL7_ATTACK_DETECTED": "Rate limiting — configure via XC rate limiting.",
    "DOSL7_ATTACK_STOPPED": "Rate limiting — configure via XC rate limiting.",
}

# Commands that are never translatable — sourced from CommandSpec.xc_translatable

# Command prefixes where the entire namespace is untranslatable
UNTRANSLATABLE_PREFIXES: tuple[str, ...] = (
    "TCP::",
    "UDP::",
    "STATS::",
    "PERSIST::",
    "IP::",
    "SSL::",
    "DNS::",
    "SIP::",
    "RTSP::",
    "RADIUS::",
    "DIAMETER::",
    "GTP::",
    "MQTT::",
    "FTP::",
    "SMTPS::",
    "LB::",
    "ACCESS::",
    "APM::",
    "ASM::",
    "BOTDEFENSE::",
    "DOSL7::",
    "WEBSOCKET::",
)

# These prefix commands are translatable despite their prefix being listed
# — sourced from CommandSpec.xc_translatable
