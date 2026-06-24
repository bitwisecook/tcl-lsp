//! Declarative mapping from iRule commands and events to XC constructs.
//!
//! Used by the translator to determine whether each iRule construct has an
//! XC equivalent and how to map it.

use crate::model::XCConstructKind;

/// Describes how an iRule command maps to an XC construct.
#[derive(Debug, Clone, Copy)]
pub struct XCMapping {
    /// The XC construct kind.
    pub kind: XCConstructKind,
    /// XC-side description.
    pub xc_description: &'static str,
    /// Optional note / caveat.
    pub note: &'static str,
}

/// Look up the [`XCMapping`] for an iRule command (the `COMMAND_XC_MAP`
/// table). Returns `None` for commands with no direct mapping.
#[must_use]
pub fn command_xc_map(command: &str) -> Option<XCMapping> {
    use XCConstructKind::{HeaderAction, OriginPool, Route, ServicePolicyRule};
    let m = |kind, xc_description, note| XCMapping {
        kind,
        xc_description,
        note,
    };
    Some(match command {
        "pool" => m(OriginPool, "XC origin pool selection", ""),
        "HTTP::redirect" => m(Route, "XC redirect route", ""),
        "HTTP::respond" => m(Route, "XC direct response route", ""),
        "HTTP::header" => m(HeaderAction, "XC load balancer header processing", ""),
        "HTTP::cookie" => m(
            HeaderAction,
            "XC load balancer cookie processing",
            "Limited to add/remove; complex cookie logic needs manual config",
        ),
        "HTTP::uri" => m(
            Route,
            "XC route path rewrite (setter only)",
            "Getter translates to match criteria; setter translates to route rewrite",
        ),
        "HTTP::path" => m(
            Route,
            "XC route path match / rewrite",
            "Getter translates to match criteria; setter translates to route rewrite",
        ),
        "HTTP::host" => m(
            Route,
            "XC route host match",
            "Getter translates to domain match criteria",
        ),
        "class" => m(
            ServicePolicyRule,
            "XC service policy rule (data-group lookup)",
            "Each data-group entry may map to a separate rule",
        ),
        _ => return None,
    })
}

/// Map an `HTTP::header` subcommand to its XC operation (the
/// `HEADER_SUBCOMMAND_MAP` table): `insert`/`value` → `add`, `replace` →
/// `replace`, `remove` → `remove`.
#[must_use]
pub fn header_subcommand_map(subcommand: &str) -> Option<&'static str> {
    Some(match subcommand {
        "insert" => "add",
        "replace" => "replace",
        "remove" => "remove",
        "value" => "add",
        _ => return None,
    })
}

/// Events with direct XC equivalents (`TRANSLATABLE_EVENTS`). Returns the
/// XC-side description when the event is translatable.
#[must_use]
pub fn translatable_event(event: &str) -> Option<&'static str> {
    Some(match event {
        "HTTP_REQUEST" => "L7 routes + service policies (request processing)",
        "HTTP_RESPONSE" => "Load balancer response header processing",
        _ => return None,
    })
}

/// Events without XC equivalents (`UNTRANSLATABLE_EVENTS`).
#[must_use]
pub fn untranslatable_event(event: &str) -> Option<&'static str> {
    Some(match event {
        "RULE_INIT" => "No XC equivalent. Static configuration replaces init logic.",
        "CLIENT_ACCEPTED" => "L4 event — no XC equivalent for TCP-level logic.",
        "CLIENT_CLOSED" => "L4 event — no XC equivalent.",
        "CLIENT_DATA" => "L4 data event — requires App Stack.",
        "SERVER_CONNECTED" => "Backend connection event — no XC equivalent.",
        "SERVER_CLOSED" => "Backend connection event — no XC equivalent.",
        "SERVER_DATA" => "Backend data event — no XC equivalent.",
        "LB_FAILED" => "Fallback logic — use XC origin pool health checks instead.",
        "LB_SELECTED" => "LB decision point — handled by XC origin pool config.",
        "PERSIST_DOWN" => "Persistence event — no XC equivalent.",
        "FLOW_INIT" => "L4 flow event — no XC equivalent.",
        "HTTP_REQUEST_DATA" => "HTTP payload event — no direct XC equivalent.",
        "HTTP_RESPONSE_DATA" => "HTTP payload event — no direct XC equivalent.",
        "HTTP_REQUEST_SEND" => "Proxy-side event — no XC equivalent.",
        "HTTP_REQUEST_RELEASE" => "Proxy-side event — no XC equivalent.",
        "HTTP_RESPONSE_RELEASE" => "Proxy-side event — no XC equivalent.",
        "HTTP_RESPONSE_CONTINUE" => "100-continue event — no XC equivalent.",
        "TCP_REQUEST" => "L4 TCP event — no XC equivalent.",
        "TCP_RESPONSE" => "L4 TCP event — no XC equivalent.",
        "DNS_REQUEST" => "DNS event — use XC DNS load balancer instead.",
        "DNS_RESPONSE" => "DNS event — use XC DNS load balancer instead.",
        "SIP_REQUEST" => "SIP event — no XC equivalent.",
        "SIP_RESPONSE" => "SIP event — no XC equivalent.",
        _ => return None,
    })
}

/// Events that map to separate XC features (`ADVISORY_EVENTS`).
#[must_use]
pub fn advisory_event(event: &str) -> Option<&'static str> {
    Some(match event {
        "CLIENTSSL_HANDSHAKE" => "TLS — configure via XC TLS settings on the load balancer.",
        "CLIENTSSL_CLIENTCERT" => "Client cert — configure via XC mTLS settings.",
        "SERVERSSL_HANDSHAKE" => "Backend TLS — configure via XC origin pool TLS settings.",
        "ACCESS_POLICY_AGENT_EVENT" => "APM — configure via XC service policy or external IdP.",
        "ACCESS_ACL_ALLOWED" => "ACL — configure via XC service policy.",
        "ACCESS_ACL_DENIED" => "ACL — configure via XC service policy.",
        "ASM_REQUEST_DONE" => "WAF — configure via XC App Firewall.",
        "ASM_REQUEST_BLOCKING" => "WAF — configure via XC App Firewall.",
        "ASM_REQUEST_VIOLATION" => "WAF — configure via XC App Firewall.",
        "BOTDEFENSE_REQUEST" => "Bot defense — configure via XC Bot Defense.",
        "BOTDEFENSE_ACTION" => "Bot defense — configure via XC Bot Defense.",
        "CACHE_REQUEST" => "Caching — configure via XC CDN settings.",
        "CACHE_RESPONSE" => "Caching — configure via XC CDN settings.",
        "DOSL7_ATTACK_DETECTED" => "Rate limiting — configure via XC rate limiting.",
        "DOSL7_ATTACK_STOPPED" => "Rate limiting — configure via XC rate limiting.",
        _ => return None,
    })
}

/// Command prefixes where the entire namespace is untranslatable
/// (`UNTRANSLATABLE_PREFIXES`). A command translatability-override
/// (`is_xc_translatable_override`) can still rescue an individual command.
pub const UNTRANSLATABLE_PREFIXES: &[&str] = &[
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
];
