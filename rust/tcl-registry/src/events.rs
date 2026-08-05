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

//! iRules event metadata.
//!
//! Static data tables describing the 247 iRules events: protocol
//! layer, connection side, required profiles, flow properties,
//! and canonical firing order.

use rustc_hash::{FxHashMap, FxHashSet};

use crate::lifecycle::{Lifecycle, LifecycleState};
use crate::side_effects::ConnectionSide;

/// Per-event protocol stack properties.
///
/// Describes which connection side an event fires on, what transport
/// it requires, which profiles must be active, its lifecycle on the
/// `BIG-IP` release axis, and classification flags (hot, common).
#[derive(Debug, Clone, PartialEq, Eq)]
// `client_side` / `server_side` are mutually exclusive in
// principle but the static tables include both-sides events; the
// remaining three (`flow`, `hot`, `common`) are
// orthogonal classification facts. A bitflags consolidation is
// possible but the type is part of the registry's public surface
// and the static-data tables encode 247 events as `EventProps {
// client_side: true, ... }` literals.
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
    /// Commonly used event (hot path).
    pub hot: bool,
    /// Commonly used event (general).
    pub common: bool,
    /// Parent event for data events (e.g. `HTTP_REQUEST` for `HTTP_REQUEST_DATA`).
    pub setup_event: Option<&'static str>,
    /// For a collect-gated `*_DATA` event: the protocol collect-command
    /// namespaces (`TCP`, `UDP`, `HTTP`, `SSL`, …) whose `<proto>::collect`
    /// — issued on [`Self::data_collect_side`] — lets the event fire.
    /// Empty when the event is not collect-gated. (`HTTP_REQUEST_DATA` ⇒
    /// `["HTTP"]`: it never fires without a client-side `HTTP::collect`.)
    pub data_collect_protocols: &'static [&'static str],
    /// The connection side that must issue the collect for a collect-gated
    /// `*_DATA` event; `None` when not collect-gated. (Held explicitly
    /// because it is not the event's firing side — `SERVER_DATA` fires on
    /// both sides yet requires a *server*-side collect.)
    pub data_collect_side: Option<crate::side_effects::ConnectionSide>,
    /// Introduction / deprecation / retirement releases of this event on
    /// the `BIG-IP` release axis. An absent introducing release inherits the
    /// axis baseline (declared present since BIG-IP 15.0 —
    /// `VersionKey::BigipVersion.baseline_version()`).
    pub lifecycle: Lifecycle,
}

impl EventProps {
    /// Human-readable connection-side label — one of `"client-side"`,
    /// `"server-side"`, `"client-side and server-side"`, or `"global"`.
    #[must_use]
    pub fn side_label(&self) -> &'static str {
        match (self.client_side, self.server_side) {
            (true, true) => "client-side and server-side",
            (true, false) => "client-side",
            (false, true) => "server-side",
            (false, false) => "global",
        }
    }

    /// Default: not side-specific, no transport, flow = true.
    const DEFAULT: Self = Self {
        client_side: false,
        server_side: false,
        transport: &[],
        implied_profiles: &[],
        flow: true,
        hot: false,
        common: false,
        setup_event: None,
        data_collect_protocols: &[],
        data_collect_side: None,
        lifecycle: Lifecycle::UNSPECIFIED,
    };

    /// The event's lifecycle with the `BIG-IP` axis baseline filled in for an
    /// absent introducing release.
    #[must_use]
    pub fn lifecycle(&self) -> Lifecycle {
        self.lifecycle
            .with_baseline(tcl_dialect::VersionKey::BigipVersion.baseline_version())
    }

    /// Whether the event is deprecated at `bigip_version`, or — with no
    /// resolved release — whether it is ever deprecated. Version-agnostic
    /// surfaces (a plain hover, the catalogue dump) pass `None`.
    #[must_use]
    pub fn is_deprecated(&self, bigip_version: Option<&str>) -> bool {
        match bigip_version {
            Some(_) => self.lifecycle.deprecated_at(bigip_version),
            None => self.lifecycle.is_deprecated_anywhere(),
        }
    }
}

/// What a command requires from the protocol stack.
///
/// Embedded on command specs via `excluded_events` and `EventRequires`.
#[derive(Debug, Clone, PartialEq, Eq)]
// `client_side` / `server_side` / `init_only` / `flow` are
// orthogonal protocol-stack requirements. Same deferral as
// [`EventProps`]: registry public surface, static-data literals.
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
    /// Any caveats or implementation notes.
    pub notes: &'static str,
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
    props: FxHashMap<&'static str, EventProps>,
    order: Vec<OrderEntry>,
    flow_chains: Vec<FlowChain>,
    once_per_connection: FxHashSet<&'static str>,
    per_request: FxHashSet<&'static str>,
    descriptions: FxHashMap<&'static str, &'static str>,
}

impl EventRegistry {
    /// The lifecycle of `event` on the BIG-IP release axis, with an absent
    /// introducing release inheriting the axis baseline (declared present
    /// since BIG-IP 15.0). `None` for an unknown event.
    #[must_use]
    pub fn event_lifecycle(&self, event: &str) -> Option<Lifecycle> {
        Some(self.get_props(event)?.lifecycle())
    }

    /// The lifecycle state of `event` at BIG-IP `version`. `None` for an
    /// unknown event.
    #[must_use]
    pub fn event_lifecycle_state(&self, event: &str, version: &str) -> Option<LifecycleState> {
        Some(self.event_lifecycle(event)?.state_at(Some(version)))
    }

    /// Whether `event` exists at BIG-IP `version` per the declared data
    /// (baseline semantics — see [`Self::event_lifecycle`]). Unknown
    /// events are `false`.
    #[must_use]
    pub fn event_available_at(&self, event: &str, version: &str) -> bool {
        self.event_lifecycle(event)
            .is_some_and(|life| life.available_at(Some(version)))
    }

    /// Build the event registry from static data.
    ///
    /// Called once at startup — the data is compiled into the binary.
    #[must_use]
    pub fn build() -> Self {
        let mut props = FxHashMap::default();
        for (name, ep) in event_props_table() {
            props.insert(name, ep);
        }
        Self {
            props,
            order: master_order(),
            flow_chains: flow_chains(),
            once_per_connection: once_per_connection(),
            per_request: per_request(),
            descriptions: crate::event_descriptions::EVENT_DESCRIPTIONS
                .iter()
                .copied()
                .collect(),
        }
    }

    /// Description prose for `event`, or `None` when none is recorded.
    #[must_use]
    pub fn description(&self, event: &str) -> Option<&'static str> {
        self.descriptions.get(event).copied()
    }

    /// Multiplicity category for `event` — one of `"init"`,
    /// `"once_per_connection"`, `"per_request"`, or `"unknown"`.
    #[must_use]
    pub fn multiplicity(&self, event: &str) -> &'static str {
        if event == "RULE_INIT" {
            "init"
        } else if self.is_once_per_connection(event) {
            "once_per_connection"
        } else if self.is_per_request(event) {
            "per_request"
        } else {
            "unknown"
        }
    }

    /// Look up event properties by name.
    #[must_use]
    pub fn get_props(&self, name: &str) -> Option<&EventProps> {
        self.props.get(name)
    }

    /// The collect requirement of a collect-gated `*_DATA` event: the
    /// protocol collect namespaces and the side (`"client"` / `"server"`)
    /// whose `<proto>::collect` lets the event fire. `None` when `event`
    /// is unknown or not collect-gated.
    ///
    /// Drives the IRULE1005 "data event never fires" check from the
    /// registry rather than a hardcoded table in the compiler.
    #[must_use]
    pub fn data_collect_requirement(
        &self,
        event: &str,
    ) -> Option<(&'static [&'static str], &'static str)> {
        let props = self.get_props(event)?;
        let side = match props.data_collect_side? {
            ConnectionSide::Client => "client",
            ConnectionSide::Server => "server",
            _ => return None,
        };
        Some((props.data_collect_protocols, side))
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

    /// The multiplicity category for an event: `"init"`,
    /// `"once_per_connection"`, `"per_request"`, or `"unknown"`.
    #[must_use]
    pub fn event_multiplicity(&self, event: &str) -> &'static str {
        if event == "RULE_INIT" {
            "init"
        } else if self.once_per_connection.contains(event) {
            "once_per_connection"
        } else if self.per_request.contains(event) {
            "per_request"
        } else {
            "unknown"
        }
    }

    /// Sort `events` into canonical firing order. Events absent from the
    /// master-order table are appended in sorted order, so none are dropped.
    ///
    /// Callers pass a unique
    /// set of names (duplicates would each be kept).
    #[must_use]
    pub fn order_events(&self, events: &[String]) -> Vec<String> {
        let mut known: Vec<(usize, String)> = Vec::new();
        let mut unknown: Vec<String> = Vec::new();
        for evt in events {
            match self.master_order_index(evt) {
                Some(idx) => known.push((idx, evt.clone())),
                None => unknown.push(evt.clone()),
            }
        }
        known.sort();
        unknown.sort();
        known.into_iter().map(|(_, e)| e).chain(unknown).collect()
    }

    /// Scan the `when EVENT` handler names from `source` and return them in
    /// canonical firing order (order the scanned events).
    #[must_use]
    pub fn order_events_for_file(&self, source: &str) -> Vec<String> {
        self.order_events(&scan_when_events(source))
    }

    /// Return a note when a variable set in `set_event` has
    /// scoping concerns when read in `read_event`.
    ///
    /// Returns `None` when there's no concern — the caller's
    /// cross-event analysis (`ConnectionScope::build`) treats
    /// a `None` result as "valid cross-event flow" and records
    /// the variable in the `cross_event_defs` /
    /// `cross_event_imports` sets.
    #[must_use]
    pub fn variable_scope_note(&self, set_event: &str, read_event: &str) -> Option<String> {
        if set_event == "RULE_INIT" {
            return None;
        }
        let set_idx = self.master_order_index(set_event)?;
        let read_idx = self.master_order_index(read_event)?;
        if read_idx < set_idx {
            return Some(format!(
                "variable set in {set_event} is not yet available in {read_event} (fires earlier)"
            ));
        }
        if self.per_request.contains(set_event) && self.once_per_connection.contains(read_event) {
            return Some(format!(
                "variable set in {set_event} (per-request) may not \
                 be set yet in {read_event} (per-connection)"
            ));
        }
        None
    }

    /// Position of `event` in [`Self::master_order`], or `None`
    /// when the event isn't in the firing-order table.
    fn master_order_index(&self, event: &str) -> Option<usize> {
        self.order.iter().position(|e| e.event == event)
    }

    /// Position of `event` in the master firing order, or `None` when the
    /// event isn't in it.
    #[must_use]
    pub fn event_index(&self, event: &str) -> Option<usize> {
        self.master_order_index(event)
    }
}

/// Scan the distinct `when EVENT` handler names from `source` — a
/// regex-free scan matching the shape `\bwhen\s+([A-Z_][A-Z0-9_]*)`.
/// Names are returned in first-seen order (deduplicated); the caller orders
/// them canonically via [`EventRegistry::order_events`].
#[must_use]
pub fn scan_when_events(source: &str) -> Vec<String> {
    fn is_word(b: u8) -> bool {
        b.is_ascii_alphanumeric() || b == b'_'
    }
    fn is_upper_start(b: u8) -> bool {
        b.is_ascii_uppercase() || b == b'_'
    }
    fn is_upper_rest(b: u8) -> bool {
        b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_'
    }

    let bytes = source.as_bytes();
    let mut out: Vec<String> = Vec::new();
    let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let mut i = 0usize;
    while i + 4 <= bytes.len() {
        // `\bwhen`: the four chars "when" with a word boundary before.
        if &bytes[i..i + 4] == b"when" && (i == 0 || !is_word(bytes[i - 1])) {
            let mut j = i + 4;
            // `\s+`
            let ws_start = j;
            while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            if j > ws_start && j < bytes.len() && is_upper_start(bytes[j]) {
                let name_start = j;
                j += 1;
                while j < bytes.len() && is_upper_rest(bytes[j]) {
                    j += 1;
                }
                let name = &source[name_start..j];
                if seen.insert(name) {
                    out.push(name.to_owned());
                }
                i = j;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// True when an event's [`EventProps`] satisfy a command's [`EventRequires`].
///
/// The single canonical implementation of the event-validity test.  Used both
/// by the per-command "valid events" hover list and by
/// [`crate::CommandRegistry::valid_irules_commands_for_event`].
#[must_use]
pub fn event_satisfies(
    props: &EventProps,
    requires: &EventRequires,
    event_name: &str,
    profiles: &crate::profiles::ProfileRegistry,
) -> bool {
    if requires.init_only {
        return event_name == "RULE_INIT";
    }
    if requires.also_in.contains(&event_name) {
        return true;
    }
    if requires.flow && !props.flow {
        return false;
    }
    if requires.client_side && !props.client_side {
        return false;
    }
    if requires.server_side && !props.server_side {
        return false;
    }
    if let Some(t) = requires.transport
        && !props.transport.contains(&t)
    {
        return false;
    }
    if !requires.profiles.is_empty()
        && !profiles.stack_satisfies(requires.profiles, props.implied_profiles)
    {
        return false;
    }
    true
}

// Full static data — auto-generated.

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
                implied_profiles: &["CLIENTSSL", "PERSIST", "SSL_PERSISTENCE"],
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
                implied_profiles: &["CLIENTSSL", "PERSIST", "SSL_PERSISTENCE"],
                common: true,
                ..EventProps::DEFAULT
            },
        ),
        (
            "CLIENTSSL_PASSTHROUGH",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["CLIENTSSL", "PERSIST", "SSL_PERSISTENCE"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "CLIENTSSL_SERVERHELLO_SEND",
            EventProps {
                client_side: true,
                transport: &["tcp"],
                implied_profiles: &["CLIENTSSL", "PERSIST", "SSL_PERSISTENCE"],
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
                data_collect_protocols: &["TCP", "UDP"],
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
                transport: &["tcp"],
                implied_profiles: &["MQTT"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "MQTT_SERVER_EGRESS",
            EventProps {
                client_side: true,
                server_side: true,
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
                ..EventProps::DEFAULT
            },
        ),
        (
            "RTSP_RESPONSE",
            EventProps {
                client_side: true,
                server_side: true,
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
                transport: &["tcp"],
                implied_profiles: &["RTSP"],
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
                implied_profiles: &["PERSIST", "SERVERSSL", "SSL_PERSISTENCE"],
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
                transport: &["tcp"],
                implied_profiles: &["PERSIST", "SERVERSSL", "SSL_PERSISTENCE"],
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
                implied_profiles: &["PERSIST", "SERVERSSL", "SSL_PERSISTENCE"],
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
                transport: &["tcp"],
                implied_profiles: &["HTTP", "WS"],
                ..EventProps::DEFAULT
            },
        ),
        (
            "WS_SERVER_FRAME",
            EventProps {
                client_side: true,
                server_side: true,
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

fn master_order() -> Vec<OrderEntry> {
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

fn once_per_connection() -> FxHashSet<&'static str> {
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

fn per_request() -> FxHashSet<&'static str> {
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

fn flow_chains() -> Vec<FlowChain> {
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
