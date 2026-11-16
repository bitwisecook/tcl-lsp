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
#[cfg(test)]
use tcl_lexer::{LexerConfig, SourceSegmentationPurpose, Token};

use crate::lifecycle::{Lifecycle, LifecycleState};
use crate::side_effects::ConnectionSide;

mod data_collection;
mod event_order;
mod event_props_table;
mod flow_chains;
mod top_level_declarations;
mod when_handlers;

pub use data_collection::{
    ASM_PAYLOAD, CACHE_PAYLOAD, CollectionReleaseRequirement, DIAMETER_PAYLOAD,
    DataCollectionAction, DataCollectionBootstrap, DataCollectionOperation, DataCollectionProtocol,
    GTP_PAYLOAD, HTTP_COLLECT, HTTP_PAYLOAD, HTTP_RELEASE, MQTT_COLLECT, MQTT_PAYLOAD,
    MQTT_RELEASE, MR_COLLECT, MR_PAYLOAD, MR_RELEASE, PayloadCollectionRequirement,
    PayloadCollectionRequirementForm, REWRITE_PAYLOAD, RTSP_COLLECT, RTSP_PAYLOAD, RTSP_RELEASE,
    SCTP_COLLECT, SCTP_PAYLOAD, SCTP_RELEASE, SIP_PAYLOAD, SSL_COLLECT, SSL_PAYLOAD, SSL_RELEASE,
    TCP_COLLECT, TCP_PAYLOAD, TCP_RELEASE, UDP_PAYLOAD, WS_COLLECT, WS_PAYLOAD, WS_RELEASE,
    XML_PAYLOAD,
};
use event_order::{master_order, once_per_connection, per_request};
pub(crate) use event_props_table::event_props_table;
use flow_chains::flow_chains;
pub use top_level_declarations::{
    BIGIP_EVENT_HANDLER_PRIORITY, EventHandlerPriority, IRULES_EVENT_CLAUSES,
    IRULES_PRIORITY_CLAUSE, IRULES_TIMING_CLAUSE, IRULES_TIMING_SPELLINGS, IrulesCommandPlacement,
    IrulesDeclarationArguments, IrulesDeclarationSlot, IrulesExecutionContext,
    IrulesTopLevelDeclaration, IrulesTopLevelEffect, closed_braced_argument_words,
};
#[cfg(test)]
pub(crate) use when_handlers::WrittenHeadResolver;
pub use when_handlers::{
    CommandHeadResolver, scan_when_events,
    top_level_when_handler_candidates_with_registry_and_head_resolver, top_level_when_handlers,
    top_level_when_handlers_with_registry_and_head_resolver,
    top_level_when_handlers_with_source_config_and_head_resolver,
};

/// Source flow grammar carried by a selected iRules command descriptor.
/// No variant establishes that a handler ran or an effect completed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrulesSourceFlowSpec {
    /// The source operation can drop or reject the current connection.
    DropsConnection,
    /// The source operation can send the DNS packet after script execution.
    ReturnsDnsPacket,
    /// Effective operands choose which later event handlers are disabled.
    EventControl,
}

/// Conditional source action under a selected descriptor and actual argv.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrulesSourceFlowAction {
    /// A possible connection drop requires a following source guard.
    DropsConnection,
    /// A possible DNS return requires a following script return.
    ReturnsDnsPacket,
    /// The exact source form disables all later event handlers.
    DisablesAllEvents,
}

impl crate::ResolvedInvocation<'_, '_> {
    /// Project the original source flow grammar over the effective operands.
    /// Availability, dispatch, successful effects and body entry stay separate.
    #[must_use]
    pub fn authored_source_irules_flow_action(&self) -> Option<IrulesSourceFlowAction> {
        match self
            .authored_source_descriptors()
            .command
            .irules_source_flow?
        {
            IrulesSourceFlowSpec::DropsConnection => Some(IrulesSourceFlowAction::DropsConnection),
            IrulesSourceFlowSpec::ReturnsDnsPacket => {
                Some(IrulesSourceFlowAction::ReturnsDnsPacket)
            }
            IrulesSourceFlowSpec::EventControl => {
                let arguments = self
                    .words
                    .arguments()
                    .slice_from(self.semantics.argument_offset);
                let matches = |index, expected: &[u8]| {
                    arguments.get(index).and_then(|word| {
                        word.literal()
                            .map(str::as_bytes)
                            .or_else(|| word.native_bytes())
                    }) == Some(expected)
                };
                (arguments.exact_argv_len() == Some(2)
                    && matches(0, b"disable")
                    && matches(1, b"all"))
                .then_some(IrulesSourceFlowAction::DisablesAllEvents)
            }
        }
    }

    /// Possible response commitment from this selected source descriptor's
    /// actual surface and package floor. This supplies no reached effect.
    #[must_use]
    pub fn authored_source_may_commit_response(&self) -> bool {
        self.semantics
            .side_effects
            .lower_bound()
            .iter()
            .any(|effect| {
                effect.target == crate::side_effects::SideEffectTarget::ResponseCommit
                    && effect.writes
                    && effect.surface.is_none_or(|surface| {
                        tcl_dialect::model::surface_admits(surface, self.dialect.as_ref())
                    })
                    && effect
                        .available_for_version(self.semantics.options.availability.package_version)
            })
    }
}

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
    /// Direction used by side-sensitive commands in this event when the
    /// public firing flags alone are ambiguous.
    pub command_side: ConnectionSide,
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
    /// For a data `*_DATA` event: protocol families that can make it fire.
    ///
    /// A family whose registry descriptor requires an explicit collect needs
    /// that operation on [`Self::data_collect_side`]. A
    /// family such as UDP can supply payload without a collect, so its
    /// presence is a valid non-collect alternative. Empty means this is not a
    /// data-collection event. (`HTTP_REQUEST_DATA` ⇒ `["HTTP"]`: it has no
    /// non-collect alternative.)
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

    /// Effective direction for side-sensitive commands in this event.
    #[must_use]
    pub fn command_side(&self) -> Option<ConnectionSide> {
        match self.command_side {
            ConnectionSide::Client | ConnectionSide::Server => Some(self.command_side),
            _ => match (self.client_side, self.server_side) {
                (true, false) => Some(ConnectionSide::Client),
                (false, true) => Some(ConnectionSide::Server),
                _ => None,
            },
        }
    }

    /// Default: not side-specific, no transport, flow = true.
    const DEFAULT: Self = Self {
        client_side: false,
        server_side: false,
        command_side: ConnectionSide::None,
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
// `client_side` / `server_side` / `flow` are orthogonal
// protocol-stack requirements. Same deferral as
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
    /// Requires active traffic flow.
    pub flow: bool,
}

/// Event-validity override for one literal command-argument prefix.
///
/// A command's top-level [`EventRequires`] describes its ordinary form, but a
/// few Tcl dialect commands deliberately have subforms with different event
/// contracts. `FIX::tag get`, for example, reads a live FIX message while
/// `FIX::tag map set` configures a mapping and is valid outside that event.
/// Keeping the distinction in registry data lets every consumer select the
/// right contract from the call words without knowing the command name.
///
/// The longest matching prefix wins. `requires: None` means that the matching
/// form has no layer/profile requirement; `only_in` can still constrain it to
/// named events when the dialect documentation gives a closed event set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventRequirementForm {
    /// Literal leading words after the command name that select this form.
    pub argument_prefix: &'static [&'static str],
    /// Layer/profile requirements for the selected form.
    pub requires: Option<EventRequires>,
    /// Non-empty means the selected form is legal only in these events.
    pub only_in: &'static [&'static str],
}

impl EventRequirementForm {
    /// Whether `args` begins with this form's literal selector.
    /// An empty selector is the exact no-argument form, not a wildcard.
    #[must_use]
    pub fn matches(&self, args: &[&str]) -> bool {
        if self.argument_prefix.is_empty() {
            args.is_empty()
        } else {
            args.starts_with(self.argument_prefix)
        }
    }
}

/// How sure a command's declared event emission is.
///
/// Kept explicit rather than collapsed to a boolean because the three cases
/// need different treatment by a reachability model: only [`Self::Definite`]
/// licenses "this event will run", and [`Self::Asynchronous`] must never be
/// read as same-frame fall-through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventEmissionCertainty {
    /// The form always raises the event.
    Definite,
    /// The form may raise the event; something outside the call decides.
    ///
    /// `TCP::notify request` raises `USER_REQUEST` *unless* an mblb
    /// message-boundary context consumes it instead, and nothing at the call
    /// site distinguishes the two.
    Possible,
    /// The form raises the event later, on another stack.
    ///
    /// `NAME::lookup` returns immediately and `NAME_RESOLVED` fires when the
    /// DNS answer arrives, so the handler is reachable but is not a
    /// continuation of the caller.
    Asynchronous,
}

/// The iRules events one command form can raise.
///
/// Separate from [`EventRequires`], which answers the opposite question:
/// whether a command is legal *inside* the event it is written in.
///
/// An empty [`Self::events`] is a positive fact — "this form raises nothing" —
/// not an absence. `TCP::notify eom` declares it so that the form cannot
/// inherit a broader form's emission, which is exactly the mistake a
/// consumer-side command-name table would make.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventEmission {
    /// Target event names. Every name must exist in the event registry; a
    /// drift test enforces that, so a typo cannot silently emit nothing.
    pub events: &'static [&'static str],
    /// How sure the emission is.
    pub certainty: EventEmissionCertainty,
}

/// Event emission for one literal command-argument prefix.
///
/// The same selection rule as [`EventRequirementForm`]: longest matching
/// literal prefix wins, and a command whose forms differ (`TCP::notify
/// request` / `response` / `eom`) describes each in registry data rather than
/// leaving consumers to match the subcommand by name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventEmissionForm {
    /// Literal leading words after the command name that select this form.
    pub argument_prefix: &'static [&'static str],
    /// What the selected form raises.
    pub emission: EventEmission,
}

impl EventEmissionForm {
    /// Whether `args` begins with this form's literal selector.
    /// An empty selector is the exact no-argument form, not a wildcard —
    /// the whole-command fact is [`crate::spec::CommandSpec::event_emits`].
    #[must_use]
    pub fn matches(&self, args: &[&str]) -> bool {
        if self.argument_prefix.is_empty() {
            args.is_empty()
        } else {
            args.starts_with(self.argument_prefix)
        }
    }
}

/// The effective event contract for one resolved command call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedEventRequirements<'a> {
    /// Layer/profile requirements, if any.
    pub requires: Option<&'a EventRequires>,
    /// A closed event set, when the form has one.
    pub only_in: &'a [&'a str],
    /// Whether an argument-prefix form selected this contract. A matching
    /// unrestricted form is intentional and suppresses the command-name
    /// namespace fallback used by callers for legacy whole-command specs.
    pub matched_form: bool,
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

/// The variable frame an event supplies to its executable body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventVariableFrame {
    /// Rule initialisation has an interpreter namespace frame, no connection.
    InitialisationNamespace,
    /// A traffic event executes in the current flow's persistent frame.
    Connection,
    /// The registry does not establish the variable frame.
    Unknown,
}

/// Invocation multiplicity selected by the host event contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventExecutionMultiplicity {
    /// Successful initialisation executes once in each worker's reload epoch.
    OncePerWorkerEpoch,
    /// Each connection may enter the handler once; other connections can repeat it.
    OncePerConnection,
    /// Reactive invocations may repeat, including within a connection.
    MayRepeat,
    /// No supported event contract establishes the invocation envelope.
    Unknown,
}

/// Lifecycle knowledge between two handlers, without assuming event dominance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventLifecycleRelation {
    /// Successful initialisation precedes traffic on the same worker/epoch.
    /// This establishes no value equality across workers or transfer across reloads.
    InitialisationBeforeTraffic,
    /// Multiple handlers of one event require priority and source ordering.
    SameEvent,
    /// A permitted lifecycle path exists; execution is not guaranteed.
    MayPrecede,
    /// The canonical lifecycle places the reader earlier than the writer.
    ReaderPrecedesWriter,
    /// No registry proof relates these events.
    Unknown,
}

impl EventRegistry {
    /// Host invocation envelope for executable source-body entry analysis.
    /// Interpreter namespace and command effects persist across connections,
    /// including handlers which execute only once for each connection.
    #[must_use]
    pub fn execution_multiplicity(&self, event: &str) -> EventExecutionMultiplicity {
        if self.get_props(event).is_none() {
            EventExecutionMultiplicity::Unknown
        } else if self.variable_frame(event) == EventVariableFrame::InitialisationNamespace {
            EventExecutionMultiplicity::OncePerWorkerEpoch
        } else if self.is_once_per_connection(event) {
            EventExecutionMultiplicity::OncePerConnection
        } else {
            EventExecutionMultiplicity::MayRepeat
        }
    }
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

    /// The data-collection alternatives of a `*_DATA` event: protocol
    /// families and the side (`"client"` / `"server"`) where an explicit
    /// collection would run. `None` when `event` is unknown or does not have
    /// a data-collection lifecycle.
    ///
    /// Drives IRULE1005 from the registry rather than a hardcoded compiler
    /// table. Consumers must also consult the protocol descriptor: UDP, for
    /// example, is an implicit alternative rather than a nonexistent
    /// `UDP::collect` command.
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

    /// Whether `data_event` is the matching implicit-release event for a
    /// collection started in `setup_event` on `side`.
    #[must_use]
    pub fn data_event_releases_collection(
        &self,
        data_event: &str,
        setup_event: &str,
        protocol: &str,
        side: ConnectionSide,
    ) -> bool {
        let Some(props) = self.get_props(data_event) else {
            return false;
        };
        props.setup_event == Some(setup_event)
            && props.data_collect_protocols.contains(&protocol)
            && props.data_collect_side == Some(side)
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

    /// Variable-frame declaration shared by analysis and runtime adapters.
    #[must_use]
    pub fn variable_frame(&self, event: &str) -> EventVariableFrame {
        if event == "RULE_INIT" {
            EventVariableFrame::InitialisationNamespace
        } else if self.get_props(event).is_some_and(|props| props.flow) {
            EventVariableFrame::Connection
        } else {
            EventVariableFrame::Unknown
        }
    }

    /// A lifecycle relation is possibility knowledge, never a reaching-def proof.
    #[must_use]
    pub fn lifecycle_relation(&self, writer: &str, reader: &str) -> EventLifecycleRelation {
        if self.get_props(writer).is_none() || self.get_props(reader).is_none() {
            return EventLifecycleRelation::Unknown;
        }
        if writer == reader {
            return EventLifecycleRelation::SameEvent;
        }
        if writer == "RULE_INIT" && self.variable_frame(reader) == EventVariableFrame::Connection {
            return EventLifecycleRelation::InitialisationBeforeTraffic;
        }
        let (Some(writer_index), Some(reader_index)) = (
            self.master_order_index(writer),
            self.master_order_index(reader),
        ) else {
            return EventLifecycleRelation::Unknown;
        };
        if reader_index < writer_index {
            EventLifecycleRelation::ReaderPrecedesWriter
        } else {
            EventLifecycleRelation::MayPrecede
        }
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
        let (Some(set_idx), Some(read_idx)) = (
            self.master_order_index(set_event),
            self.master_order_index(read_event),
        ) else {
            return Some("event lifecycle is not established by the registry".to_owned());
        };
        if read_idx < set_idx {
            return Some(format!(
                "variable set in {set_event} may not yet be available in {read_event} (fires earlier)"
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
    !(!requires.profiles.is_empty()
        && !profiles.stack_satisfies(requires.profiles, props.implied_profiles))
}

// Full static data — auto-generated.

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_event_source_config_tests;
