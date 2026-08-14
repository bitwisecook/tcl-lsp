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

//! Side-effect metadata for structured effect analysis.

use crate::dialects::DialectSet;
use crate::lifecycle::{Lifecycle, LifecycleState};

/// What kind of external state a command affects.
///
/// Variant names match the consumer's
/// `tcl_compiler::side_effects::SideEffectTarget`. `Process` /
/// `ChannelIo` are registry-only — kept for the existing `exec` /
/// `open` (pipeline form) / `chan` core specs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SideEffectTarget {
    /// Tcl variable read or write.
    Variable,
    /// Session table entry (`table set/add/lookup/delete`).
    SessionTable,
    /// Persistence record (`session add/lookup`, `persist`).
    PersistenceTable,
    /// Data group / class lookup (`class match/search/lookup`).
    DataGroup,
    /// HTTP header read/write (`HTTP::header`).
    HttpHeader,
    /// HTTP payload/body (`HTTP::payload`, `HTTP::collect`).
    HttpBody,
    /// HTTP status code (`HTTP::status`).
    HttpStatus,
    /// HTTP URI components (`HTTP::uri`, `HTTP::path`, `HTTP::query`).
    HttpUri,
    /// HTTP cookie (`HTTP::cookie`).
    HttpCookie,
    /// HTTP method (`HTTP::method`).
    HttpMethod,
    /// HTTP/2 protocol state.
    Http2State,
    /// Commits or sends an HTTP response (`HTTP::respond`, `redirect`).
    ResponseCommit,
    /// Connection-level action: drop, reject, discard, forward.
    ConnectionControl,
    /// TCP connection state (`TCP::close`, `TCP::collect`, …).
    TcpState,
    /// SSL/TLS state (`SSL::disable`, `SSL::cert`, …).
    SslState,
    /// UDP datagram state.
    UdpState,
    /// Pool or pool member selection (`pool`, `LB::select`).
    PoolSelection,
    /// Direct node selection (`node`).
    NodeSelection,
    /// SNAT address selection (`snat`, `snatpool`).
    SnatSelection,
    /// File system I/O.
    FileIo,
    /// Network socket I/O (`socket`, `connect`, `send`).
    NetworkIo,
    /// Logging output (`log`, `puts stderr`).
    LogIo,
    /// Content rewriting via stream profile (`STREAM::`, `REWRITE::`).
    StreamProfile,
    /// DNS message state (`DNS::header`, `DNS::answer`, …).
    DnsState,
    /// Traffic classification state (`CLASSIFY::`, `CLASSIFICATION::`).
    ClassificationState,
    /// Layer-7 denial-of-service protection state (`DOSL7::`).
    Dosl7State,
    /// Flow object state (`FLOW::create_related`, …).
    FlowState,
    /// Large Scale NAT state (`LSN::address`, …).
    LsnState,
    /// FTP protocol state (`FTP::enable`, `FTP::port`, …).
    FtpState,
    /// ICAP protocol state (`ICAP::header`, `ICAP::method`, …).
    IcapState,
    /// Message routing state (`MESSAGE::field`, `MR::message`, …).
    MessageState,
    /// Internal statistics counters (`ISTATS::set/incr`, …).
    IStats,
    /// Access Policy Manager state (`ACCESS::session`, …).
    ApmState,
    /// Application Security Manager state (`ASM::enable/disable`, …).
    AsmState,
    /// BIG-IP configuration change (iApps, `tmsh::` commands).
    BigipConfig,
    /// Defines or removes a procedure (`proc`, `rename`).
    ProcDefinition,
    /// Namespace creation / deletion (`namespace eval/delete`).
    NamespaceState,
    /// Interpreter-level state (`interp`, `package`, `load`).
    InterpState,
    /// Process management (registry-only; `exec`, and `open`'s command-
    /// pipeline form).
    Process,
    /// Channel I/O (registry-only; `chan`).
    ChannelIo,
    /// Event-handler flow control: abandons the remaining script in the
    /// *current* event invocation without affecting the connection, other
    /// events, or other rules (iRules `return` inside a `when` body;
    /// shares the category with `event NAME disable`, a stronger version
    /// of the same idea). Distinct from [`Self::ConnectionControl`], which
    /// is a connection-terminating action (drop/reject/discard) — this
    /// isn't one.
    EventControl,
    /// Unknown or unclassified effect.
    Unknown,
}

impl SideEffectTarget {
    /// Stable registry vocabulary for serialised effect and world-state views.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Variable => "variable",
            Self::SessionTable => "session-table",
            Self::PersistenceTable => "persistence-table",
            Self::DataGroup => "data-group",
            Self::HttpHeader => "http-header",
            Self::HttpBody => "http-body",
            Self::HttpStatus => "http-status",
            Self::HttpUri => "http-uri",
            Self::HttpCookie => "http-cookie",
            Self::HttpMethod => "http-method",
            Self::Http2State => "http2-state",
            Self::ResponseCommit => "response-commit",
            Self::ConnectionControl => "connection-control",
            Self::TcpState => "tcp-state",
            Self::SslState => "ssl-state",
            Self::UdpState => "udp-state",
            Self::PoolSelection => "pool-selection",
            Self::NodeSelection => "node-selection",
            Self::SnatSelection => "snat-selection",
            Self::FileIo => "file-io",
            Self::NetworkIo => "network-io",
            Self::LogIo => "log-io",
            Self::StreamProfile => "stream-profile",
            Self::DnsState => "dns-state",
            Self::ClassificationState => "classification-state",
            Self::Dosl7State => "dosl7-state",
            Self::FlowState => "flow-state",
            Self::LsnState => "lsn-state",
            Self::FtpState => "ftp-state",
            Self::IcapState => "icap-state",
            Self::MessageState => "message-state",
            Self::IStats => "istats",
            Self::ApmState => "apm-state",
            Self::AsmState => "asm-state",
            Self::BigipConfig => "bigip-config",
            Self::ProcDefinition => "proc-definition",
            Self::NamespaceState => "namespace-state",
            Self::InterpState => "interp-state",
            Self::Process => "process",
            Self::ChannelIo => "channel-io",
            Self::EventControl => "event-control",
            Self::Unknown => "unknown",
        }
    }
}

/// Which connection side a command operates on (iRules).
///
/// Variant names match the consumer's
/// `tcl_compiler::side_effects::ConnectionSide`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConnectionSide {
    /// No connection side (not iRules or side-neutral).
    None,
    /// Client side.
    Client,
    /// Server side.
    Server,
    /// Both client and server sides.
    Both,
    /// Global / connection-independent.
    Global,
}

/// The side context a nesting-script command establishes.
///
/// This is separate from [`ConnectionSide`]: `Peer` is a direction relative
/// to the enclosing event rather than a fixed side.  It is attached to the
/// command spec so consumers can descend into `clientside`, `serverside`, or
/// future side-switch commands without matching command names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SideSwitchTarget {
    /// Evaluate the body as client-side traffic.
    Client,
    /// Evaluate the body as server-side traffic.
    Server,
    /// Evaluate the body on the opposite side of the enclosing event.
    Peer,
}

impl SideSwitchTarget {
    /// Resolve the execution side selected by this registered command.
    ///
    /// `peer` runs on the side opposite the current event; the other two
    /// commands select their named side. Consumers use this method instead of
    /// interpreting command spellings or enum variants themselves.
    #[must_use]
    pub fn execution_side(self, current_side: &str) -> &'static str {
        if self == Self::Server || (self == Self::Peer && current_side == "client") {
            "server"
        } else {
            "client"
        }
    }
}

/// Structured side-effect declaration for a command or subcommand.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SideEffect {
    /// What kind of state is affected.
    pub target: SideEffectTarget,
    /// Whether the command reads from the target.
    pub reads: bool,
    /// Whether the command writes to the target.
    pub writes: bool,
    /// Connection side (iRules).
    pub connection_side: ConnectionSide,
    /// Dialects *this effect* applies in, when narrower than the
    /// command's own availability — e.g. `return`'s `EventControl` effect
    /// only exists in iRules, even though `return` itself is universal
    /// Tcl (`CommandSpec::dialects: None`). `None` = inherits the
    /// command's own dialect gating, so every effect declared before this
    /// field existed keeps its meaning unchanged.
    pub dialects: Option<DialectSet>,
    /// Introduction / deprecation / retirement releases of *this effect* on
    /// the owning command's package version axis — an effect a later release
    /// added or stopped having. [`Lifecycle::UNSPECIFIED`] means the effect
    /// holds in every package version; orthogonal to [`Self::dialects`],
    /// which gates on the Tcl *core* version.
    pub lifecycle: Lifecycle,
}

impl SideEffect {
    /// Baseline: [`SideEffectTarget::Unknown`], no reads/writes, no
    /// connection side, no extra dialect restriction, no lifecycle — used
    /// with `..SideEffect::DEFAULT`.
    pub const DEFAULT: Self = Self {
        target: SideEffectTarget::Unknown,
        reads: false,
        writes: false,
        connection_side: ConnectionSide::None,
        dialects: None,
        lifecycle: Lifecycle::UNSPECIFIED,
    };

    /// Whether this effect applies given the resolved *`package_version`*.
    ///
    /// *`package_version`* is the guaranteed-available floor from a
    /// `package require` (see [`crate::version::requirement_lower_bound`]).
    /// `None` is permissive.
    #[must_use]
    pub fn available_for_version(&self, package_version: Option<&str>) -> bool {
        self.lifecycle.available_at(package_version)
    }

    /// This effect's lifecycle state at the resolved *`package_version`*.
    #[must_use]
    pub fn lifecycle_state(&self, package_version: Option<&str>) -> LifecycleState {
        self.lifecycle.state_at(package_version)
    }
}

/// Inferred storage type for a command's target variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StorageType {
    /// Dictionary.
    Dict,
    /// List.
    List,
    /// Array.
    Array,
}
