//! iRules event metadata.
//!
//! Static data tables describing the 247 iRules events: protocol
//! layer, connection side, required profiles, flow properties,
//! and canonical firing order.

use std::collections::{HashMap, HashSet};

/// Per-event protocol stack properties.
///
/// Describes which connection side an event fires on, what transport
/// it requires, which profiles must be active, and classification
/// flags (hot, common, deprecated).
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct EventProps {
    /// Fires on client side.
    pub client_side: bool,
    /// Fires on server side.
    pub server_side: bool,
    /// Required transport: `"tcp"`, `"udp"`, or both.
    pub transport: &'static [&'static str],
    /// Profile types that must be active for this event.
    pub implied_profiles: &'static [&'static str],
    /// Whether there is active traffic flow during this event.
    pub flow: bool,
    /// Event is deprecated.
    pub deprecated: bool,
    /// Commonly used event (hot path).
    pub hot: bool,
    /// Commonly used event (general).
    pub common: bool,
    /// Parent event for data events (e.g. `HTTP_REQUEST` for `HTTP_REQUEST_DATA`).
    pub setup_event: Option<&'static str>,
}

impl EventProps {
    /// Default: not side-specific, no transport, flow = true.
    const DEFAULT: Self = Self {
        client_side: false,
        server_side: false,
        transport: &[],
        implied_profiles: &[],
        flow: true,
        deprecated: false,
        hot: false,
        common: false,
        setup_event: None,
    };
}

/// What a command requires from the protocol stack.
///
/// Embedded on command specs via `excluded_events` and `EventRequires`
/// in the Python registry.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct EventRequires {
    /// Requires client side.
    pub client_side: bool,
    /// Requires server side.
    pub server_side: bool,
    /// Required transport (`"tcp"` or `"udp"`).
    pub transport: Option<&'static str>,
    /// Required profile types.
    pub profiles: &'static [&'static str],
    /// Events where the command is unconditionally valid.
    pub also_in: &'static [&'static str],
    /// Only valid in `RULE_INIT`.
    pub init_only: bool,
    /// Requires active traffic flow.
    pub flow: bool,
    /// Required profile capability (e.g. `"sni"`).
    pub capability: Option<&'static str>,
}

/// A step in an event flow chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowStep {
    /// Event name.
    pub event: &'static str,
    /// Logical phase (`init`, `l4_client`, `tls_client`, `http_request`, etc.).
    pub phase: &'static str,
    /// Whether this event only fires conditionally.
    pub conditional: bool,
    /// Human-readable condition note.
    pub condition_note: &'static str,
}

/// Complete event flow for a profile combination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowChain {
    /// Unique identifier (e.g. `"plain_tcp"`, `"tcp_clientssl_http"`).
    pub chain_id: &'static str,
    /// Human-readable description.
    pub description: &'static str,
    /// Profile types on the virtual server.
    pub profiles: &'static [&'static str],
    /// Ordered steps.
    pub steps: Vec<FlowStep>,
}

/// An entry in the master event firing order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderEntry {
    /// Event name.
    pub event: &'static str,
    /// Profile gates (must be active for event to fire). Empty = always fires.
    pub profile_gates: &'static [&'static str],
}

/// Event registry providing lookup over the static event tables.
pub struct EventRegistry {
    props: HashMap<&'static str, EventProps>,
    order: Vec<OrderEntry>,
    flow_chains: Vec<FlowChain>,
    once_per_connection: HashSet<&'static str>,
    per_request: HashSet<&'static str>,
}

impl EventRegistry {
    /// Build the event registry from static data.
    ///
    /// Called once at startup — the data is compiled into the binary.
    #[must_use]
    pub fn build() -> Self {
        let mut props = HashMap::new();
        for (name, ep) in event_props_table() {
            props.insert(name, ep);
        }
        Self {
            props,
            order: master_order(),
            flow_chains: flow_chains(),
            once_per_connection: once_per_connection(),
            per_request: per_request(),
        }
    }

    /// Look up event properties by name.
    #[must_use]
    pub fn get_props(&self, name: &str) -> Option<&EventProps> {
        self.props.get(name)
    }

    /// Whether an event name is known.
    #[must_use]
    pub fn is_known(&self, name: &str) -> bool {
        self.props.contains_key(name)
    }

    /// All known event names.
    #[must_use]
    pub fn all_event_names(&self) -> Vec<&str> {
        self.props.keys().copied().collect()
    }

    /// Number of registered events.
    #[must_use]
    pub fn len(&self) -> usize {
        self.props.len()
    }

    /// Whether the registry has no events.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.props.is_empty()
    }

    /// Canonical event firing order.
    #[must_use]
    pub fn master_order(&self) -> &[OrderEntry] {
        &self.order
    }

    /// Available flow chains.
    #[must_use]
    pub fn flow_chains(&self) -> &[FlowChain] {
        &self.flow_chains
    }

    /// Whether an event fires at most once per connection.
    #[must_use]
    pub fn is_once_per_connection(&self, event: &str) -> bool {
        self.once_per_connection.contains(event)
    }

    /// Whether an event fires per request/transaction.
    #[must_use]
    pub fn is_per_request(&self, event: &str) -> bool {
        self.per_request.contains(event)
    }
}

// Static data — populated by the Python codegen or manually.
// For now, provide the framework with a few representative entries.
// The full 247-entry table will be generated from Python.

#[allow(clippy::too_many_lines)]
fn event_props_table() -> Vec<(&'static str, EventProps)> {
    vec![
        (
            "RULE_INIT",
            EventProps {
                flow: false,
                common: true,
                ..EventProps::DEFAULT
            },
        ),
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
            "HTTP_REQUEST",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["HTTP", "FASTHTTP"],
                hot: true,
                common: true,
                ..EventProps::DEFAULT
            },
        ),
        (
            "HTTP_RESPONSE",
            EventProps {
                server_side: true,
                transport: &["tcp"],
                implied_profiles: &["HTTP", "FASTHTTP"],
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
                common: true,
                setup_event: Some("HTTP_REQUEST"),
                ..EventProps::DEFAULT
            },
        ),
        (
            "HTTP_RESPONSE_DATA",
            EventProps {
                server_side: true,
                transport: &["tcp"],
                implied_profiles: &["HTTP"],
                common: true,
                setup_event: Some("HTTP_RESPONSE"),
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
            "SERVERSSL_HANDSHAKE",
            EventProps {
                server_side: true,
                transport: &["tcp"],
                implied_profiles: &["SERVERSSL"],
                common: true,
                ..EventProps::DEFAULT
            },
        ),
        (
            "CLIENT_DATA",
            EventProps {
                client_side: true,
                transport: &["tcp", "udp"],
                common: true,
                ..EventProps::DEFAULT
            },
        ),
        (
            "SERVER_DATA",
            EventProps {
                server_side: true,
                transport: &["tcp", "udp"],
                common: true,
                ..EventProps::DEFAULT
            },
        ),
        (
            "LB_SELECTED",
            EventProps {
                common: true,
                ..EventProps::DEFAULT
            },
        ),
        (
            "LB_FAILED",
            EventProps {
                common: true,
                ..EventProps::DEFAULT
            },
        ),
        (
            "SERVER_CONNECTED",
            EventProps {
                server_side: true,
                transport: &["tcp"],
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
            "DNS_REQUEST",
            EventProps {
                client_side: true,
                implied_profiles: &["DNS"],
                common: true,
                ..EventProps::DEFAULT
            },
        ),
        (
            "DNS_RESPONSE",
            EventProps {
                server_side: true,
                implied_profiles: &["DNS"],
                common: true,
                ..EventProps::DEFAULT
            },
        ),
    ]
}

fn master_order() -> Vec<OrderEntry> {
    vec![
        OrderEntry {
            event: "RULE_INIT",
            profile_gates: &[],
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
            event: "CLIENTSSL_HANDSHAKE",
            profile_gates: &["CLIENTSSL"],
        },
        OrderEntry {
            event: "HTTP_REQUEST",
            profile_gates: &["HTTP"],
        },
        OrderEntry {
            event: "HTTP_REQUEST_DATA",
            profile_gates: &["HTTP"],
        },
        OrderEntry {
            event: "LB_SELECTED",
            profile_gates: &[],
        },
        OrderEntry {
            event: "SERVER_CONNECTED",
            profile_gates: &[],
        },
        OrderEntry {
            event: "SERVERSSL_HANDSHAKE",
            profile_gates: &["SERVERSSL"],
        },
        OrderEntry {
            event: "HTTP_RESPONSE",
            profile_gates: &["HTTP"],
        },
        OrderEntry {
            event: "HTTP_RESPONSE_DATA",
            profile_gates: &["HTTP"],
        },
        OrderEntry {
            event: "CLIENT_CLOSED",
            profile_gates: &[],
        },
        OrderEntry {
            event: "SERVER_CLOSED",
            profile_gates: &[],
        },
        OrderEntry {
            event: "DNS_REQUEST",
            profile_gates: &["DNS"],
        },
        OrderEntry {
            event: "DNS_RESPONSE",
            profile_gates: &["DNS"],
        },
    ]
}

#[allow(clippy::too_many_lines)]
fn flow_chains() -> Vec<FlowChain> {
    vec![
        FlowChain {
            chain_id: "plain_tcp",
            description: "Plain TCP connection",
            profiles: &["TCP"],
            steps: vec![
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
                    condition_note: "if data received",
                },
                FlowStep {
                    event: "LB_SELECTED",
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
                    condition_note: "if data received",
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
            ],
        },
        FlowChain {
            chain_id: "tcp_http",
            description: "TCP + HTTP",
            profiles: &["TCP", "HTTP"],
            steps: vec![
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
                    event: "LB_SELECTED",
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
                    event: "HTTP_RESPONSE",
                    phase: "http_response",
                    conditional: false,
                    condition_note: "",
                },
                FlowStep {
                    event: "CLIENT_CLOSED",
                    phase: "l4_teardown",
                    conditional: false,
                    condition_note: "",
                },
            ],
        },
    ]
}

fn once_per_connection() -> HashSet<&'static str> {
    [
        "RULE_INIT",
        "FLOW_INIT",
        "CLIENT_ACCEPTED",
        "CLIENTSSL_CLIENTHELLO",
        "CLIENTSSL_SERVERHELLO_SEND",
        "CLIENTSSL_CLIENTCERT",
        "CLIENTSSL_HANDSHAKE",
        "CLIENTSSL_PASSTHROUGH",
        "CLIENT_CLOSED",
    ]
    .into_iter()
    .collect()
}

fn per_request() -> HashSet<&'static str> {
    [
        "HTTP_REQUEST",
        "HTTP_REQUEST_DATA",
        "HTTP_REQUEST_SEND",
        "HTTP_RESPONSE",
        "HTTP_RESPONSE_CONTINUE",
        "HTTP_RESPONSE_DATA",
        "LB_SELECTED",
        "LB_FAILED",
        "SERVER_CONNECTED",
        "DNS_REQUEST",
        "DNS_RESPONSE",
    ]
    .into_iter()
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_event_registry() {
        let reg = EventRegistry::build();
        assert!(reg.len() > 10);
        assert!(reg.is_known("HTTP_REQUEST"));
        assert!(!reg.is_known("NONEXISTENT_EVENT"));
    }

    #[test]
    fn event_props_lookup() {
        let reg = EventRegistry::build();
        let props = reg.get_props("HTTP_REQUEST").unwrap();
        assert!(props.client_side);
        assert!(props.hot);
        assert!(props.implied_profiles.contains(&"HTTP"));
    }

    #[test]
    fn once_per_connection_check() {
        let reg = EventRegistry::build();
        assert!(reg.is_once_per_connection("CLIENT_ACCEPTED"));
        assert!(!reg.is_once_per_connection("HTTP_REQUEST"));
    }

    #[test]
    fn per_request_check() {
        let reg = EventRegistry::build();
        assert!(reg.is_per_request("HTTP_REQUEST"));
        assert!(!reg.is_per_request("CLIENT_ACCEPTED"));
    }

    #[test]
    fn master_order_starts_with_rule_init() {
        let reg = EventRegistry::build();
        assert_eq!(reg.master_order()[0].event, "RULE_INIT");
    }

    #[test]
    fn flow_chains_exist() {
        let reg = EventRegistry::build();
        assert!(!reg.flow_chains().is_empty());
        assert_eq!(reg.flow_chains()[0].chain_id, "plain_tcp");
    }
}
