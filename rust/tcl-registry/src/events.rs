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
use tcl_lexer::{LexerConfig, SourceMap, Span, TokenType, word_span_at};

use crate::lifecycle::{Lifecycle, LifecycleState};
use crate::side_effects::ConnectionSide;

/// How a command participates in an iRules data-collection lifecycle.
///
/// This is attached to the relevant [`crate::spec::CommandSpec`] rather than
/// inferred from a `::collect`, `::release`, or `::payload` spelling.  That
/// keeps the compiler correct for protocol families such as UDP and ASM,
/// which expose a payload but do not require an explicit collect command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataCollectionAction {
    /// Starts collection for a protocol.
    Collect,
    /// Releases previously collected data.
    Release,
    /// Reads or changes a protocol payload.
    Payload,
}

/// Whether a protocol payload is available only after an explicit collect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayloadCollectionRequirement {
    /// A preceding matching `collect` operation is required.
    ExplicitCollect,
    /// The payload is supplied by the protocol or profile without a collect.
    NotRequired,
}

/// How an explicit collection is released.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectionReleaseRequirement {
    /// A matching explicit `release` operation is required.
    ExplicitRelease,
    /// A matching `*_DATA` event releases the data when no new collect starts.
    ImplicitInDataEvent,
    /// This protocol does not have an explicit collection lifecycle.
    NotApplicable,
}

/// Registry data shared by all collection operations in one protocol family.
///
/// The protocol's setup event belongs here because it describes the event
/// surface, not a compiler or editor special case.  `bootstrap_event_for`
/// selects the appropriate setup event for a diagnostic's enclosing event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DataCollectionProtocol {
    /// Upper-case protocol-family name shown in diagnostics.
    pub name: &'static str,
    /// Whether payload access needs a preceding collect.
    pub payload_requirement: PayloadCollectionRequirement,
    /// How a collection is released.
    pub release_requirement: CollectionReleaseRequirement,
    /// Event-prefix-to-setup-event mapping used by the generic quick fix.
    pub bootstrap_events: &'static [DataCollectionBootstrap],
}

impl DataCollectionProtocol {
    /// Return the setup event for a command used in `event`.
    #[must_use]
    pub fn bootstrap_event_for(&self, event: &str) -> Option<&'static str> {
        let event = event.to_ascii_uppercase();
        self.bootstrap_events
            .iter()
            .find(|entry| event.starts_with(entry.event_prefix))
            .or_else(|| self.bootstrap_events.last())
            .map(|entry| entry.setup_event)
    }
}

/// One event-prefix-to-setup-event mapping for a data collection protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DataCollectionBootstrap {
    /// Upper-case event prefix used to select this mapping.
    pub event_prefix: &'static str,
    /// Event where the collect should be placed.
    pub setup_event: &'static str,
}

/// A data-collection fact attached to an individual command spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DataCollectionOperation {
    /// Protocol family whose lifecycle this command participates in.
    pub protocol: DataCollectionProtocol,
    /// The command's operation in that lifecycle.
    pub action: DataCollectionAction,
    /// Argument-prefix overrides for payload availability.
    ///
    /// Most payload commands have one rule for every call form. MQTT is the
    /// important exception: its no-argument and `append` forms read collected
    /// bytes, while `length`, `replace`, and `prepend` can operate on the
    /// current PUBLISH message before collection. Keeping those forms here
    /// prevents the analyser from inferring semantics from subcommand names.
    pub payload_requirement_forms: &'static [PayloadCollectionRequirementForm],
}

/// Payload availability override for one literal command-argument prefix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PayloadCollectionRequirementForm {
    /// Literal leading words after the command name that select this form.
    pub argument_prefix: &'static [&'static str],
    /// Whether this selected form needs a preceding collect.
    pub requirement: PayloadCollectionRequirement,
}

impl PayloadCollectionRequirementForm {
    /// Whether `args` begins with this form's literal selector.
    #[must_use]
    pub fn matches(&self, args: &[&str]) -> bool {
        args.starts_with(self.argument_prefix)
    }
}

impl DataCollectionOperation {
    /// Resolve payload availability for one invocation.
    ///
    /// `None` means the call has a dynamic or unknown form and the consumer
    /// must abstain. Non-payload operations do not have a payload requirement.
    #[must_use]
    pub fn payload_requirement_for_args(
        &self,
        args: &[&str],
    ) -> Option<PayloadCollectionRequirement> {
        if self.action != DataCollectionAction::Payload {
            return None;
        }
        if self.payload_requirement_forms.is_empty() || args.is_empty() {
            return Some(self.protocol.payload_requirement);
        }
        self.payload_requirement_forms
            .iter()
            .filter(|form| form.matches(args))
            .max_by_key(|form| form.argument_prefix.len())
            .map(|form| form.requirement)
    }
}

/// How an iRules event-handler command treats an omitted priority clause.
///
/// BIG-IP gives `when` handlers a documented default priority of 500, so an
/// omitted clause is valid and does not itself justify a diagnostic. A
/// dialect can opt into an explicit-priority policy without teaching the
/// analyser a handler command name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventHandlerPriority {
    /// Keyword that introduces an explicit priority value.
    pub keyword: &'static str,
    /// Priority used by the runtime when the keyword is omitted.
    pub default_priority: u16,
    /// Whether an omitted priority is reportable in this dialect.
    pub warn_when_implicit: bool,
}

/// BIG-IP's `when` priority policy: priority defaults to 500.
pub const BIGIP_EVENT_HANDLER_PRIORITY: EventHandlerPriority = EventHandlerPriority {
    keyword: "priority",
    default_priority: 500,
    warn_when_implicit: false,
};

const HTTP_BOOTSTRAP_EVENTS: &[DataCollectionBootstrap] = &[
    DataCollectionBootstrap {
        event_prefix: "HTTP_RESPONSE",
        setup_event: "HTTP_RESPONSE",
    },
    DataCollectionBootstrap {
        event_prefix: "HTTP",
        setup_event: "HTTP_REQUEST",
    },
];

const SSL_BOOTSTRAP_EVENTS: &[DataCollectionBootstrap] = &[
    DataCollectionBootstrap {
        event_prefix: "SERVERSSL",
        setup_event: "SERVERSSL_HANDSHAKE",
    },
    DataCollectionBootstrap {
        event_prefix: "SSL",
        setup_event: "CLIENTSSL_HANDSHAKE",
    },
];

const TCP_BOOTSTRAP_EVENTS: &[DataCollectionBootstrap] = &[
    DataCollectionBootstrap {
        event_prefix: "SERVER",
        setup_event: "SERVER_CONNECTED",
    },
    DataCollectionBootstrap {
        event_prefix: "",
        setup_event: "CLIENT_ACCEPTED",
    },
];

const MQTT_BOOTSTRAP_EVENTS: &[DataCollectionBootstrap] = &[
    DataCollectionBootstrap {
        event_prefix: "MQTT_SERVER",
        setup_event: "MQTT_SERVER_INGRESS",
    },
    DataCollectionBootstrap {
        event_prefix: "MQTT",
        setup_event: "MQTT_CLIENT_INGRESS",
    },
];

const MR_BOOTSTRAP_EVENTS: &[DataCollectionBootstrap] = &[DataCollectionBootstrap {
    event_prefix: "",
    setup_event: "MR_INGRESS",
}];

const RTSP_BOOTSTRAP_EVENTS: &[DataCollectionBootstrap] = &[
    DataCollectionBootstrap {
        event_prefix: "RTSP_RESPONSE",
        setup_event: "RTSP_RESPONSE",
    },
    DataCollectionBootstrap {
        event_prefix: "RTSP",
        setup_event: "RTSP_REQUEST",
    },
];

const SCTP_BOOTSTRAP_EVENTS: &[DataCollectionBootstrap] = &[DataCollectionBootstrap {
    event_prefix: "",
    setup_event: "CLIENT_ACCEPTED",
}];

const WS_BOOTSTRAP_EVENTS: &[DataCollectionBootstrap] = &[
    DataCollectionBootstrap {
        event_prefix: "WS_SERVER",
        setup_event: "WS_SERVER_FRAME",
    },
    DataCollectionBootstrap {
        event_prefix: "WS",
        setup_event: "WS_CLIENT_FRAME",
    },
];

const HTTP_COLLECTION: DataCollectionProtocol = DataCollectionProtocol {
    name: "HTTP",
    payload_requirement: PayloadCollectionRequirement::ExplicitCollect,
    release_requirement: CollectionReleaseRequirement::ImplicitInDataEvent,
    bootstrap_events: HTTP_BOOTSTRAP_EVENTS,
};

const TCP_COLLECTION: DataCollectionProtocol = DataCollectionProtocol {
    name: "TCP",
    payload_requirement: PayloadCollectionRequirement::ExplicitCollect,
    release_requirement: CollectionReleaseRequirement::ExplicitRelease,
    bootstrap_events: TCP_BOOTSTRAP_EVENTS,
};

const SSL_COLLECTION: DataCollectionProtocol = DataCollectionProtocol {
    name: "SSL",
    payload_requirement: PayloadCollectionRequirement::ExplicitCollect,
    release_requirement: CollectionReleaseRequirement::ExplicitRelease,
    bootstrap_events: SSL_BOOTSTRAP_EVENTS,
};

const UDP_COLLECTION: DataCollectionProtocol = DataCollectionProtocol {
    name: "UDP",
    payload_requirement: PayloadCollectionRequirement::NotRequired,
    release_requirement: CollectionReleaseRequirement::NotApplicable,
    bootstrap_events: &[],
};

const ASM_COLLECTION: DataCollectionProtocol = DataCollectionProtocol {
    name: "ASM",
    payload_requirement: PayloadCollectionRequirement::NotRequired,
    release_requirement: CollectionReleaseRequirement::NotApplicable,
    bootstrap_events: &[],
};

const MQTT_COLLECTION: DataCollectionProtocol = DataCollectionProtocol {
    name: "MQTT",
    payload_requirement: PayloadCollectionRequirement::ExplicitCollect,
    release_requirement: CollectionReleaseRequirement::ExplicitRelease,
    bootstrap_events: MQTT_BOOTSTRAP_EVENTS,
};

const MR_COLLECTION: DataCollectionProtocol = DataCollectionProtocol {
    name: "MR",
    payload_requirement: PayloadCollectionRequirement::ExplicitCollect,
    release_requirement: CollectionReleaseRequirement::ExplicitRelease,
    bootstrap_events: MR_BOOTSTRAP_EVENTS,
};

const RTSP_COLLECTION: DataCollectionProtocol = DataCollectionProtocol {
    name: "RTSP",
    payload_requirement: PayloadCollectionRequirement::ExplicitCollect,
    release_requirement: CollectionReleaseRequirement::ImplicitInDataEvent,
    bootstrap_events: RTSP_BOOTSTRAP_EVENTS,
};

const SCTP_COLLECTION: DataCollectionProtocol = DataCollectionProtocol {
    name: "SCTP",
    payload_requirement: PayloadCollectionRequirement::ExplicitCollect,
    release_requirement: CollectionReleaseRequirement::ExplicitRelease,
    bootstrap_events: SCTP_BOOTSTRAP_EVENTS,
};

const WS_COLLECTION: DataCollectionProtocol = DataCollectionProtocol {
    name: "WS",
    payload_requirement: PayloadCollectionRequirement::ExplicitCollect,
    release_requirement: CollectionReleaseRequirement::ImplicitInDataEvent,
    bootstrap_events: WS_BOOTSTRAP_EVENTS,
};

const CACHE_PAYLOAD_AVAILABLE: DataCollectionProtocol = immediate_payload("CACHE");
const DIAMETER_PAYLOAD_AVAILABLE: DataCollectionProtocol = immediate_payload("DIAMETER");
const GTP_PAYLOAD_AVAILABLE: DataCollectionProtocol = immediate_payload("GTP");
const REWRITE_PAYLOAD_AVAILABLE: DataCollectionProtocol = immediate_payload("REWRITE");
const SIP_PAYLOAD_AVAILABLE: DataCollectionProtocol = immediate_payload("SIP");
const XML_PAYLOAD_AVAILABLE: DataCollectionProtocol = immediate_payload("XML");

const fn immediate_payload(name: &'static str) -> DataCollectionProtocol {
    DataCollectionProtocol {
        name,
        payload_requirement: PayloadCollectionRequirement::NotRequired,
        release_requirement: CollectionReleaseRequirement::NotApplicable,
        bootstrap_events: &[],
    }
}

const fn operation(
    protocol: DataCollectionProtocol,
    action: DataCollectionAction,
) -> DataCollectionOperation {
    DataCollectionOperation {
        protocol,
        action,
        payload_requirement_forms: &[],
    }
}

/// `HTTP::collect` data-collection metadata.
pub const HTTP_COLLECT: DataCollectionOperation =
    operation(HTTP_COLLECTION, DataCollectionAction::Collect);
/// `HTTP::release` data-collection metadata.
pub const HTTP_RELEASE: DataCollectionOperation =
    operation(HTTP_COLLECTION, DataCollectionAction::Release);
/// `HTTP::payload` data-collection metadata.
pub const HTTP_PAYLOAD: DataCollectionOperation =
    operation(HTTP_COLLECTION, DataCollectionAction::Payload);
/// `TCP::collect` data-collection metadata.
pub const TCP_COLLECT: DataCollectionOperation =
    operation(TCP_COLLECTION, DataCollectionAction::Collect);
/// `TCP::release` data-collection metadata.
pub const TCP_RELEASE: DataCollectionOperation =
    operation(TCP_COLLECTION, DataCollectionAction::Release);
/// `TCP::payload` data-collection metadata.
pub const TCP_PAYLOAD: DataCollectionOperation =
    operation(TCP_COLLECTION, DataCollectionAction::Payload);
/// `SSL::collect` data-collection metadata.
pub const SSL_COLLECT: DataCollectionOperation =
    operation(SSL_COLLECTION, DataCollectionAction::Collect);
/// `SSL::release` data-collection metadata.
pub const SSL_RELEASE: DataCollectionOperation =
    operation(SSL_COLLECTION, DataCollectionAction::Release);
/// `SSL::payload` data-collection metadata.
pub const SSL_PAYLOAD: DataCollectionOperation =
    operation(SSL_COLLECTION, DataCollectionAction::Payload);
/// `UDP::payload` data-collection metadata.
pub const UDP_PAYLOAD: DataCollectionOperation =
    operation(UDP_COLLECTION, DataCollectionAction::Payload);
/// `ASM::payload` data-collection metadata.
pub const ASM_PAYLOAD: DataCollectionOperation =
    operation(ASM_COLLECTION, DataCollectionAction::Payload);

/// `MQTT::collect` data-collection metadata.
pub const MQTT_COLLECT: DataCollectionOperation =
    operation(MQTT_COLLECTION, DataCollectionAction::Collect);
/// `MQTT::release` data-collection metadata.
pub const MQTT_RELEASE: DataCollectionOperation =
    operation(MQTT_COLLECTION, DataCollectionAction::Release);
const MQTT_PAYLOAD_REQUIREMENT_FORMS: &[PayloadCollectionRequirementForm] = &[
    PayloadCollectionRequirementForm {
        argument_prefix: &["length"],
        requirement: PayloadCollectionRequirement::NotRequired,
    },
    PayloadCollectionRequirementForm {
        argument_prefix: &["replace"],
        requirement: PayloadCollectionRequirement::NotRequired,
    },
    PayloadCollectionRequirementForm {
        argument_prefix: &["prepend"],
        requirement: PayloadCollectionRequirement::NotRequired,
    },
    PayloadCollectionRequirementForm {
        argument_prefix: &["append"],
        requirement: PayloadCollectionRequirement::ExplicitCollect,
    },
];
/// `MQTT::payload` data-collection metadata, including call-form differences.
pub const MQTT_PAYLOAD: DataCollectionOperation = DataCollectionOperation {
    protocol: MQTT_COLLECTION,
    action: DataCollectionAction::Payload,
    payload_requirement_forms: MQTT_PAYLOAD_REQUIREMENT_FORMS,
};

/// `MR::collect` data-collection metadata.
pub const MR_COLLECT: DataCollectionOperation =
    operation(MR_COLLECTION, DataCollectionAction::Collect);
/// `MR::release` data-collection metadata.
pub const MR_RELEASE: DataCollectionOperation =
    operation(MR_COLLECTION, DataCollectionAction::Release);
/// `MR::payload` data-collection metadata.
pub const MR_PAYLOAD: DataCollectionOperation =
    operation(MR_COLLECTION, DataCollectionAction::Payload);

/// `RTSP::collect` data-collection metadata.
pub const RTSP_COLLECT: DataCollectionOperation =
    operation(RTSP_COLLECTION, DataCollectionAction::Collect);
/// `RTSP::release` data-collection metadata.
pub const RTSP_RELEASE: DataCollectionOperation =
    operation(RTSP_COLLECTION, DataCollectionAction::Release);
/// `RTSP::payload` data-collection metadata.
pub const RTSP_PAYLOAD: DataCollectionOperation =
    operation(RTSP_COLLECTION, DataCollectionAction::Payload);

/// `SCTP::collect` data-collection metadata.
pub const SCTP_COLLECT: DataCollectionOperation =
    operation(SCTP_COLLECTION, DataCollectionAction::Collect);
/// `SCTP::release` data-collection metadata.
pub const SCTP_RELEASE: DataCollectionOperation =
    operation(SCTP_COLLECTION, DataCollectionAction::Release);
/// `SCTP::payload` data-collection metadata.
pub const SCTP_PAYLOAD: DataCollectionOperation =
    operation(SCTP_COLLECTION, DataCollectionAction::Payload);

/// `WS::collect` data-collection metadata.
pub const WS_COLLECT: DataCollectionOperation =
    operation(WS_COLLECTION, DataCollectionAction::Collect);
/// `WS::release` data-collection metadata.
pub const WS_RELEASE: DataCollectionOperation =
    operation(WS_COLLECTION, DataCollectionAction::Release);
/// `WS::payload` data-collection metadata.
pub const WS_PAYLOAD: DataCollectionOperation =
    operation(WS_COLLECTION, DataCollectionAction::Payload);

/// `CACHE::payload` event-owned payload metadata.
pub const CACHE_PAYLOAD: DataCollectionOperation =
    operation(CACHE_PAYLOAD_AVAILABLE, DataCollectionAction::Payload);
/// `DIAMETER::payload` event-owned payload metadata.
pub const DIAMETER_PAYLOAD: DataCollectionOperation =
    operation(DIAMETER_PAYLOAD_AVAILABLE, DataCollectionAction::Payload);
/// `GTP::payload` event-owned payload metadata.
pub const GTP_PAYLOAD: DataCollectionOperation =
    operation(GTP_PAYLOAD_AVAILABLE, DataCollectionAction::Payload);
/// `REWRITE::payload` event-owned payload metadata.
pub const REWRITE_PAYLOAD: DataCollectionOperation =
    operation(REWRITE_PAYLOAD_AVAILABLE, DataCollectionAction::Payload);
/// `SIP::payload` event-owned payload metadata.
pub const SIP_PAYLOAD: DataCollectionOperation =
    operation(SIP_PAYLOAD_AVAILABLE, DataCollectionAction::Payload);
/// `XML::payload` event-owned payload metadata.
pub const XML_PAYLOAD: DataCollectionOperation =
    operation(XML_PAYLOAD_AVAILABLE, DataCollectionAction::Payload);

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

/// Parse top-level iRules event handlers through the shared syntax owner.
#[must_use]
pub fn top_level_when_handlers(source: &str) -> Vec<tcl_syntax::event_handler::EventHandler> {
    tcl_syntax::event_handler::event_handlers(source, LexerConfig::for_file_dialect("f5-irules"))
}

/// Parse iRules event handlers recursively through registry-declared script
/// surfaces.
///
/// The fixed iRules registry is the resolved semantic baseline for this
/// iRules-only API. Arbitrary braced values are data unless a command spec
/// declares them as [`crate::ArgRole::Body`]; command substitutions execute
/// regardless of their containing argument role.
#[must_use]
pub fn recursive_when_handlers(source: &str) -> Vec<tcl_syntax::event_handler::EventHandler> {
    recursive_when_handlers_with_registry(source, crate::registry_for_dialect("f5-irules"))
}

/// Registry-resolved recursive event discovery.
#[must_use]
pub fn recursive_when_handlers_with_registry(
    source: &str,
    registry: &crate::CommandRegistry,
) -> Vec<tcl_syntax::event_handler::EventHandler> {
    let config = registry.profile().map_or_else(
        || LexerConfig::for_file_dialect("f5-irules"),
        |profile| LexerConfig::from_grammar(profile.grammar),
    );
    let mut out = Vec::new();
    let mut pending = vec![(0_usize, source.len(), 0_u32, true)];
    let mut seen = FxHashSet::default();
    while let Some((start, end, depth, inspect_handlers)) = pending.pop() {
        if depth > 256 || !seen.insert((start, end)) {
            continue;
        }
        let Some(text) = source.get(start..end) else {
            continue;
        };
        let Ok(base) = u32::try_from(start) else {
            continue;
        };
        if inspect_handlers {
            out.extend(
                tcl_syntax::event_handler::event_handlers(text, config.at_depth(depth))
                    .into_iter()
                    .map(|handler| shift_handler(handler, base)),
            );
        }
        let sm = SourceMap::new(text);
        for command in tcl_syntax::event_handler::script_commands(text, config.at_depth(depth)) {
            if !inspect_handlers {
                for token in command
                    .words
                    .iter()
                    .flatten()
                    .filter(|token| token.kind == TokenType::Cmd)
                {
                    let whole = word_span_at(text, token.span);
                    let inner_start = whole.start() as usize + token.content_offset as usize;
                    let inner_end = whole.end().saturating_sub(1) as usize;
                    if inner_start <= inner_end {
                        pending.push((start + inner_start, start + inner_end, depth + 1, true));
                    }
                }
                continue;
            }
            let Some(head_word) = command.words.first() else {
                continue;
            };
            let [head_token] = head_word.as_slice() else {
                continue;
            };
            let head = tcl_syntax::naming::canonical_written_command(sm.token_text(*head_token));
            let head = head.trim_start_matches("::");
            let args: Vec<&str> = command
                .words
                .iter()
                .skip(1)
                .map(|word| match word.as_slice() {
                    [token] => sm.token_text(*token),
                    _ => "",
                })
                .collect();
            for index in registry.arg_indices_for_role(head, &args, crate::ArgRole::Body) {
                if let Some([token]) = command.words.get(index + 1).map(Vec::as_slice)
                    && token.kind == TokenType::Str
                    && let Some((inner_start, inner_end)) = braced_inner(text, *token)
                {
                    pending.push((start + inner_start, start + inner_end, depth + 1, true));
                }
            }
            // Expression words are not scripts. Only their live `[…]`
            // substitutions are executable regions, so inspect the word for
            // substitutions without recognising a bare `when` in expression
            // string data.
            for index in registry.arg_indices_for_role(head, &args, crate::ArgRole::Expr) {
                if let Some([token]) = command.words.get(index + 1).map(Vec::as_slice)
                    && matches!(token.kind, TokenType::Str | TokenType::Esc)
                {
                    let whole = word_span_at(text, token.span);
                    let inner_start = whole.start() as usize + token.content_offset as usize;
                    let inner_end = whole
                        .end()
                        .saturating_sub(u32::from(token.content_offset > 0))
                        as usize;
                    if inner_start <= inner_end {
                        pending.push((start + inner_start, start + inner_end, depth + 1, false));
                    }
                }
            }
            for token in command
                .words
                .iter()
                .flatten()
                .filter(|token| token.kind == TokenType::Cmd)
            {
                let whole = word_span_at(text, token.span);
                let inner_start = whole.start() as usize + token.content_offset as usize;
                let inner_end = whole.end().saturating_sub(1) as usize;
                if inner_start <= inner_end {
                    pending.push((start + inner_start, start + inner_end, depth + 1, true));
                }
            }
        }
    }
    out.sort_by_key(|handler| handler.span.start());
    out
}

fn braced_inner(source: &str, token: tcl_lexer::Token) -> Option<(usize, usize)> {
    let whole = word_span_at(source, token.span);
    let start = whole.start() as usize + 1;
    let end = whole.end().checked_sub(1)? as usize;
    (source.as_bytes().get(whole.end().checked_sub(1)? as usize) == Some(&b'}') && start <= end)
        .then_some((start, end))
}

fn shift_handler(
    mut handler: tcl_syntax::event_handler::EventHandler,
    base: u32,
) -> tcl_syntax::event_handler::EventHandler {
    let shift = |span: Span| Span::new(base + span.start(), base + span.end());
    handler.span = shift(handler.span);
    handler.event_span = shift(handler.event_span);
    handler.body_span = shift(handler.body_span);
    handler
}

/// Scan distinct event handlers recursively through the shared Tcl syntax
/// owner. Names are normalised to upper case and returned in first-seen order;
/// the caller orders them canonically via [`EventRegistry::order_events`].
#[must_use]
pub fn scan_when_events(source: &str) -> Vec<String> {
    let mut seen = FxHashSet::default();
    let mut out = Vec::new();
    for handler in recursive_when_handlers(source) {
        if seen.insert(handler.event.clone()) {
            out.push(handler.event);
        }
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
                command_side: ConnectionSide::Server,
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
    fn command_side_distinguishes_ambiguous_response_events() {
        let reg = EventRegistry::build();
        assert_eq!(
            reg.get_props("HTTP_REQUEST")
                .and_then(EventProps::command_side),
            Some(ConnectionSide::Client)
        );
        assert_eq!(
            reg.get_props("HTTP_RESPONSE")
                .and_then(EventProps::command_side),
            Some(ConnectionSide::Server)
        );
        assert_eq!(
            reg.get_props("HTTP_PROXY_RESPONSE")
                .and_then(EventProps::command_side),
            None,
            "an ambiguous event without an explicit command side must abstain"
        );
    }

    #[test]
    fn implicit_release_requires_the_matching_setup_event_and_side() {
        let reg = EventRegistry::build();
        assert!(reg.data_event_releases_collection(
            "HTTP_RESPONSE_DATA",
            "HTTP_RESPONSE",
            "HTTP",
            ConnectionSide::Server,
        ));
        assert!(!reg.data_event_releases_collection(
            "HTTP_RESPONSE_DATA",
            "HTTP_REQUEST",
            "HTTP",
            ConnectionSide::Server,
        ));
        assert!(!reg.data_event_releases_collection(
            "HTTP_RESPONSE_DATA",
            "HTTP_RESPONSE",
            "HTTP",
            ConnectionSide::Client,
        ));
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

    #[test]
    fn event_scan_uses_recursive_rooted_normalised_owner() {
        assert_eq!(
            scan_when_events("::when http_request { if {1} { :::when client_data {} } }"),
            ["HTTP_REQUEST", "CLIENT_DATA"]
        );
    }

    #[test]
    fn event_scan_recurses_only_through_registry_script_surfaces() {
        let source = r#"
            set payload {when CLIENT_DATA {}}
            set quoted "when SERVER_DATA {}"
            # when RULE_INIT {}
            ::when http_request { if {1} { :::when client_data {} } }
        "#;
        assert_eq!(scan_when_events(source), ["HTTP_REQUEST", "CLIENT_DATA"]);
    }
}
