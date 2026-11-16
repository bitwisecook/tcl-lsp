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

//! Protocol data collection: the `collect`, `release` and `payload` operations of each protocol and the events that bootstrap them.

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

pub(super) const HTTP_BOOTSTRAP_EVENTS: &[DataCollectionBootstrap] = &[
    DataCollectionBootstrap {
        event_prefix: "HTTP_RESPONSE",
        setup_event: "HTTP_RESPONSE",
    },
    DataCollectionBootstrap {
        event_prefix: "HTTP",
        setup_event: "HTTP_REQUEST",
    },
];

pub(super) const SSL_BOOTSTRAP_EVENTS: &[DataCollectionBootstrap] = &[
    DataCollectionBootstrap {
        event_prefix: "SERVERSSL",
        setup_event: "SERVERSSL_HANDSHAKE",
    },
    DataCollectionBootstrap {
        event_prefix: "SSL",
        setup_event: "CLIENTSSL_HANDSHAKE",
    },
];

pub(super) const TCP_BOOTSTRAP_EVENTS: &[DataCollectionBootstrap] = &[
    DataCollectionBootstrap {
        event_prefix: "SERVER",
        setup_event: "SERVER_CONNECTED",
    },
    DataCollectionBootstrap {
        event_prefix: "",
        setup_event: "CLIENT_ACCEPTED",
    },
];

pub(super) const MQTT_BOOTSTRAP_EVENTS: &[DataCollectionBootstrap] = &[
    DataCollectionBootstrap {
        event_prefix: "MQTT_SERVER",
        setup_event: "MQTT_SERVER_INGRESS",
    },
    DataCollectionBootstrap {
        event_prefix: "MQTT",
        setup_event: "MQTT_CLIENT_INGRESS",
    },
];

pub(super) const MR_BOOTSTRAP_EVENTS: &[DataCollectionBootstrap] = &[DataCollectionBootstrap {
    event_prefix: "",
    setup_event: "MR_INGRESS",
}];

pub(super) const RTSP_BOOTSTRAP_EVENTS: &[DataCollectionBootstrap] = &[
    DataCollectionBootstrap {
        event_prefix: "RTSP_RESPONSE",
        setup_event: "RTSP_RESPONSE",
    },
    DataCollectionBootstrap {
        event_prefix: "RTSP",
        setup_event: "RTSP_REQUEST",
    },
];

pub(super) const SCTP_BOOTSTRAP_EVENTS: &[DataCollectionBootstrap] = &[DataCollectionBootstrap {
    event_prefix: "",
    setup_event: "CLIENT_ACCEPTED",
}];

pub(super) const WS_BOOTSTRAP_EVENTS: &[DataCollectionBootstrap] = &[
    DataCollectionBootstrap {
        event_prefix: "WS_SERVER",
        setup_event: "WS_SERVER_FRAME",
    },
    DataCollectionBootstrap {
        event_prefix: "WS",
        setup_event: "WS_CLIENT_FRAME",
    },
];

pub(super) const HTTP_COLLECTION: DataCollectionProtocol = DataCollectionProtocol {
    name: "HTTP",
    payload_requirement: PayloadCollectionRequirement::ExplicitCollect,
    release_requirement: CollectionReleaseRequirement::ImplicitInDataEvent,
    bootstrap_events: HTTP_BOOTSTRAP_EVENTS,
};

pub(super) const TCP_COLLECTION: DataCollectionProtocol = DataCollectionProtocol {
    name: "TCP",
    payload_requirement: PayloadCollectionRequirement::ExplicitCollect,
    release_requirement: CollectionReleaseRequirement::ExplicitRelease,
    bootstrap_events: TCP_BOOTSTRAP_EVENTS,
};

pub(super) const SSL_COLLECTION: DataCollectionProtocol = DataCollectionProtocol {
    name: "SSL",
    payload_requirement: PayloadCollectionRequirement::ExplicitCollect,
    release_requirement: CollectionReleaseRequirement::ExplicitRelease,
    bootstrap_events: SSL_BOOTSTRAP_EVENTS,
};

pub(super) const UDP_COLLECTION: DataCollectionProtocol = DataCollectionProtocol {
    name: "UDP",
    payload_requirement: PayloadCollectionRequirement::NotRequired,
    release_requirement: CollectionReleaseRequirement::NotApplicable,
    bootstrap_events: &[],
};

pub(super) const ASM_COLLECTION: DataCollectionProtocol = DataCollectionProtocol {
    name: "ASM",
    payload_requirement: PayloadCollectionRequirement::NotRequired,
    release_requirement: CollectionReleaseRequirement::NotApplicable,
    bootstrap_events: &[],
};

pub(super) const MQTT_COLLECTION: DataCollectionProtocol = DataCollectionProtocol {
    name: "MQTT",
    payload_requirement: PayloadCollectionRequirement::ExplicitCollect,
    release_requirement: CollectionReleaseRequirement::ExplicitRelease,
    bootstrap_events: MQTT_BOOTSTRAP_EVENTS,
};

pub(super) const MR_COLLECTION: DataCollectionProtocol = DataCollectionProtocol {
    name: "MR",
    payload_requirement: PayloadCollectionRequirement::ExplicitCollect,
    release_requirement: CollectionReleaseRequirement::ExplicitRelease,
    bootstrap_events: MR_BOOTSTRAP_EVENTS,
};

pub(super) const RTSP_COLLECTION: DataCollectionProtocol = DataCollectionProtocol {
    name: "RTSP",
    payload_requirement: PayloadCollectionRequirement::ExplicitCollect,
    release_requirement: CollectionReleaseRequirement::ImplicitInDataEvent,
    bootstrap_events: RTSP_BOOTSTRAP_EVENTS,
};

pub(super) const SCTP_COLLECTION: DataCollectionProtocol = DataCollectionProtocol {
    name: "SCTP",
    payload_requirement: PayloadCollectionRequirement::ExplicitCollect,
    release_requirement: CollectionReleaseRequirement::ExplicitRelease,
    bootstrap_events: SCTP_BOOTSTRAP_EVENTS,
};

pub(super) const WS_COLLECTION: DataCollectionProtocol = DataCollectionProtocol {
    name: "WS",
    payload_requirement: PayloadCollectionRequirement::ExplicitCollect,
    release_requirement: CollectionReleaseRequirement::ImplicitInDataEvent,
    bootstrap_events: WS_BOOTSTRAP_EVENTS,
};

pub(super) const CACHE_PAYLOAD_AVAILABLE: DataCollectionProtocol = immediate_payload("CACHE");

pub(super) const DIAMETER_PAYLOAD_AVAILABLE: DataCollectionProtocol = immediate_payload("DIAMETER");

pub(super) const GTP_PAYLOAD_AVAILABLE: DataCollectionProtocol = immediate_payload("GTP");

pub(super) const REWRITE_PAYLOAD_AVAILABLE: DataCollectionProtocol = immediate_payload("REWRITE");

pub(super) const SIP_PAYLOAD_AVAILABLE: DataCollectionProtocol = immediate_payload("SIP");

pub(super) const XML_PAYLOAD_AVAILABLE: DataCollectionProtocol = immediate_payload("XML");

pub(super) const fn immediate_payload(name: &'static str) -> DataCollectionProtocol {
    DataCollectionProtocol {
        name,
        payload_requirement: PayloadCollectionRequirement::NotRequired,
        release_requirement: CollectionReleaseRequirement::NotApplicable,
        bootstrap_events: &[],
    }
}

pub(super) const fn operation(
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

pub(super) const MQTT_PAYLOAD_REQUIREMENT_FORMS: &[PayloadCollectionRequirementForm] = &[
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
