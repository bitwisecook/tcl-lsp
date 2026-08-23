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
use crate::documentation::{DocumentationAnnotation, DocumentationExample};
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
    /// Every declared effect target, in stable author-facing order.
    pub const ALL: &'static [Self] = &[
        Self::Variable,
        Self::SessionTable,
        Self::PersistenceTable,
        Self::DataGroup,
        Self::HttpHeader,
        Self::HttpBody,
        Self::HttpStatus,
        Self::HttpUri,
        Self::HttpCookie,
        Self::HttpMethod,
        Self::Http2State,
        Self::ResponseCommit,
        Self::ConnectionControl,
        Self::TcpState,
        Self::SslState,
        Self::UdpState,
        Self::PoolSelection,
        Self::NodeSelection,
        Self::SnatSelection,
        Self::FileIo,
        Self::NetworkIo,
        Self::LogIo,
        Self::StreamProfile,
        Self::DnsState,
        Self::ClassificationState,
        Self::Dosl7State,
        Self::FlowState,
        Self::LsnState,
        Self::FtpState,
        Self::IcapState,
        Self::MessageState,
        Self::IStats,
        Self::ApmState,
        Self::AsmState,
        Self::BigipConfig,
        Self::ProcDefinition,
        Self::NamespaceState,
        Self::InterpState,
        Self::Process,
        Self::ChannelIo,
        Self::EventControl,
        Self::Unknown,
    ];

    /// Rust spelling used in authoring formats.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Variable => "Variable",
            Self::SessionTable => "SessionTable",
            Self::PersistenceTable => "PersistenceTable",
            Self::DataGroup => "DataGroup",
            Self::HttpHeader => "HttpHeader",
            Self::HttpBody => "HttpBody",
            Self::HttpStatus => "HttpStatus",
            Self::HttpUri => "HttpUri",
            Self::HttpCookie => "HttpCookie",
            Self::HttpMethod => "HttpMethod",
            Self::Http2State => "Http2State",
            Self::ResponseCommit => "ResponseCommit",
            Self::ConnectionControl => "ConnectionControl",
            Self::TcpState => "TcpState",
            Self::SslState => "SslState",
            Self::UdpState => "UdpState",
            Self::PoolSelection => "PoolSelection",
            Self::NodeSelection => "NodeSelection",
            Self::SnatSelection => "SnatSelection",
            Self::FileIo => "FileIo",
            Self::NetworkIo => "NetworkIo",
            Self::LogIo => "LogIo",
            Self::StreamProfile => "StreamProfile",
            Self::DnsState => "DnsState",
            Self::ClassificationState => "ClassificationState",
            Self::Dosl7State => "Dosl7State",
            Self::FlowState => "FlowState",
            Self::LsnState => "LsnState",
            Self::FtpState => "FtpState",
            Self::IcapState => "IcapState",
            Self::MessageState => "MessageState",
            Self::IStats => "IStats",
            Self::ApmState => "ApmState",
            Self::AsmState => "AsmState",
            Self::BigipConfig => "BigipConfig",
            Self::ProcDefinition => "ProcDefinition",
            Self::NamespaceState => "NamespaceState",
            Self::InterpState => "InterpState",
            Self::Process => "Process",
            Self::ChannelIo => "ChannelIo",
            Self::EventControl => "EventControl",
            Self::Unknown => "Unknown",
        }
    }

    /// Resolve an authoring-format spelling.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|item| item.name() == name)
    }

    /// Short author-facing description, owned with the effect declaration.
    #[must_use]
    pub const fn summary(self) -> &'static str {
        match self {
            Self::Variable => "Tcl variable read or write",
            Self::SessionTable => "session table entry",
            Self::PersistenceTable => "persistence record",
            Self::DataGroup => "data group / class lookup",
            Self::HttpHeader => "HTTP header read or write",
            Self::HttpBody => "HTTP payload / body",
            Self::HttpStatus => "HTTP status code",
            Self::HttpUri => "HTTP URI components",
            Self::HttpCookie => "HTTP cookie",
            Self::HttpMethod => "HTTP method",
            Self::Http2State => "HTTP/2 protocol state",
            Self::ResponseCommit => "commits or sends an HTTP response",
            Self::ConnectionControl => "drop / reject / discard / forward",
            Self::TcpState => "TCP connection state",
            Self::SslState => "SSL/TLS state",
            Self::UdpState => "UDP state",
            Self::PoolSelection => "pool selection",
            Self::NodeSelection => "node selection",
            Self::SnatSelection => "SNAT selection",
            Self::FileIo => "filesystem I/O",
            Self::NetworkIo => "network I/O",
            Self::LogIo => "logging output",
            Self::StreamProfile => "stream profile state",
            Self::DnsState => "DNS state",
            Self::ClassificationState => "classification state",
            Self::Dosl7State => "L7 DoS state",
            Self::FlowState => "flow state",
            Self::LsnState => "LSN state",
            Self::FtpState => "FTP state",
            Self::IcapState => "ICAP state",
            Self::MessageState => "message-routing state",
            Self::IStats => "iStats counters",
            Self::ApmState => "APM state",
            Self::AsmState => "ASM state",
            Self::BigipConfig => "BIG-IP configuration",
            Self::ProcDefinition => "procedure definition table",
            Self::NamespaceState => "namespace state",
            Self::InterpState => "interpreter state",
            Self::Process => "process creation / control",
            Self::ChannelIo => "channel I/O",
            Self::EventControl => "iRules event control flow",
            Self::Unknown => "unclassified effect",
        }
    }

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

    /// Registry-owned program showing the state before the operation and the
    /// observable read or write afterward. This exhaustive match is the
    /// compile gate for side-effect documentation.
    #[must_use]
    #[expect(
        clippy::too_many_lines,
        reason = "one exhaustive closed-vocabulary documentation table; splitting it would weaken the compile gate"
    )]
    pub const fn example(self) -> DocumentationExample {
        macro_rules! effect {
            ($code:literal; $(($line:literal, $needle:literal, $label:literal)),+ $(,)?) => {
                {
                    const ANNOTATIONS: &[DocumentationAnnotation] =
                        &[$(DocumentationAnnotation::new($line, $needle, $label)),+];
                    DocumentationExample::new($code, ANNOTATIONS)
                }
            };
        }
        match self {
            Self::Variable => {
                effect!("set counter 1\nincr counter\nputs $counter"; (0, "counter 1", "creates variable state"), (1, "incr counter", "reads and writes it"), (2, "$counter", "observes 2"))
            }
            Self::SessionTable => {
                effect!("table set session:key old\ntable set session:key new\nlog local0. [table lookup session:key]"; (0, "table set", "creates session state"), (1, "table set", "writes it"), (2, "table lookup", "reads new"))
            }
            Self::PersistenceTable => {
                effect!("persist add uie $key 600\nset record [persist lookup uie $key]\nlog local0. $record"; (0, "persist add", "writes persistence state"), (1, "persist lookup", "reads it"), (2, "$record", "carries the result"))
            }
            Self::DataGroup => {
                effect!("set key [HTTP::host]\nset route [class lookup $key routes]\npool $route"; (0, "HTTP::host", "supplies a lookup key"), (1, "class lookup", "reads the data group"), (2, "$route", "selects the pool"))
            }
            Self::HttpHeader => {
                effect!("set host [HTTP::header value Host]\nHTTP::header replace X-Upstream $host\nlog local0. [HTTP::header value X-Upstream]"; (0, "HTTP::header value Host", "reads header state"), (1, "HTTP::header replace", "writes header state"), (2, "HTTP::header value X-Upstream", "observes it"))
            }
            Self::HttpBody => {
                effect!("HTTP::collect\nset body [HTTP::payload]\nlog local0. $body"; (0, "HTTP::collect", "requests body state"), (1, "HTTP::payload", "reads the collected body"), (2, "$body", "flows to output"))
            }
            Self::HttpStatus => {
                effect!("HTTP::respond 404 content missing\nset status [HTTP::status]\nlog local0. $status"; (0, "HTTP::respond 404", "writes response status"), (1, "HTTP::status", "reads it"), (2, "$status", "observes 404"))
            }
            Self::HttpUri => {
                effect!("set original [HTTP::uri]\nHTTP::uri /rewritten\nlog local0. [HTTP::uri]"; (0, "HTTP::uri", "reads URI state"), (1, "HTTP::uri /rewritten", "writes it"), (2, "[HTTP::uri]", "observes the rewrite"))
            }
            Self::HttpCookie => {
                effect!("set token [HTTP::cookie value session]\nHTTP::cookie remove session\nlog local0. $token"; (0, "HTTP::cookie value", "reads cookie state"), (1, "HTTP::cookie remove", "writes it"), (2, "$token", "retains the prior value"))
            }
            Self::HttpMethod => {
                effect!("set original [HTTP::method]\nHTTP::method POST\nlog local0. [HTTP::method]"; (0, "HTTP::method", "reads method state"), (1, "HTTP::method POST", "writes it"), (2, "[HTTP::method]", "observes POST"))
            }
            Self::Http2State => {
                effect!("set enabled [HTTP2::active]\nHTTP2::disable\nlog local0. $enabled"; (0, "HTTP2::active", "reads HTTP/2 state"), (1, "HTTP2::disable", "writes it"), (2, "$enabled", "retains the prior result"))
            }
            Self::ResponseCommit => {
                effect!("set user [HTTP::uri]\nHTTP::respond 200 content $user\nHTTP::header value Host"; (0, "HTTP::uri", "supplies request data"), (1, "HTTP::respond", "commits it to the response"), (2, "HTTP::header", "is invalid after commit"))
            }
            Self::ConnectionControl => {
                effect!("set blocked 1\nif {$blocked} { reject }\nlog local0. continued"; (0, "blocked 1", "supplies the action decision"), (1, "$blocked", "selects the action"), (1, "reject", "terminates connection flow"), (2, "continued", "is unreachable on that path"))
            }
            Self::TcpState => {
                effect!("TCP::collect 1024\nset bytes [TCP::payload]\nTCP::release"; (0, "TCP::collect", "writes TCP collection state"), (1, "TCP::payload", "reads it"), (2, "TCP::release", "changes it again"))
            }
            Self::SslState => {
                effect!("set before [SSL::enabled]\nSSL::disable\nlog local0. $before"; (0, "SSL::enabled", "reads TLS state"), (1, "SSL::disable", "writes it"), (2, "$before", "retains the prior value"))
            }
            Self::UdpState => {
                effect!("set payload [UDP::payload]\nUDP::drop\nlog local0. $payload"; (0, "UDP::payload", "reads datagram state"), (1, "UDP::drop", "writes its disposition"), (2, "$payload", "contains the prior payload"))
            }
            Self::PoolSelection => {
                effect!("set before [LB::server pool]\npool application_pool\nlog local0. [LB::server pool]"; (0, "LB::server pool", "reads selection state"), (1, "pool application_pool", "writes it"), (2, "[LB::server pool]", "observes the new pool"))
            }
            Self::NodeSelection => {
                effect!("set address 192.0.2.10\nnode $address 443\nlog local0. [LB::server addr]"; (0, "192.0.2.10", "supplies the target"), (1, "node $address 443", "writes node selection"), (2, "LB::server addr", "observes it"))
            }
            Self::SnatSelection => {
                effect!("set address 192.0.2.20\nsnat $address\nlog local0. selected"; (0, "192.0.2.20", "supplies the translated address"), (1, "snat $address", "writes SNAT selection"), (2, "selected", "runs with that selection"))
            }
            Self::FileIo => {
                effect!("set path ./data.txt\nset channel [open $path w]\nputs $channel hello"; (0, "./data.txt", "selects a filesystem object"), (1, "open $path w", "creates file I/O state"), (2, "puts $channel", "writes external data"))
            }
            Self::NetworkIo => {
                effect!("set host example.test\nset channel [socket $host 443]\nputs $channel request"; (0, "example.test", "selects a remote endpoint"), (1, "socket $host 443", "creates network state"), (2, "puts $channel", "writes to it"))
            }
            Self::LogIo => {
                effect!("set user [gets stdin]\nlog local0. $user\nputs logged"; (0, "[gets stdin]", "supplies external data"), (1, "log local0. $user", "writes it to logging output"), (2, "puts logged", "continues after the effect"))
            }
            Self::StreamProfile => {
                effect!("STREAM::expression {@old@new@}\nSTREAM::enable\nlog local0. enabled"; (0, "STREAM::expression", "writes rewrite rules"), (1, "STREAM::enable", "activates stream state"), (2, "enabled", "runs with rewriting active"))
            }
            Self::DnsState => {
                effect!("set name [DNS::question name]\nDNS::answer insert \"$name. 60 IN A 192.0.2.1\"\nlog local0. $name"; (0, "DNS::question name", "reads DNS state"), (1, "DNS::answer insert", "writes it"), (2, "$name", "identifies the affected record"))
            }
            Self::ClassificationState => {
                effect!("CLASSIFY::application set web\nset app [CLASSIFY::application]\nlog local0. $app"; (0, "CLASSIFY::application set", "writes classification state"), (1, "CLASSIFY::application", "reads it"), (2, "$app", "observes web"))
            }
            Self::Dosl7State => {
                effect!("DOSL7::enable\nset state [DOSL7::profile]\nlog local0. $state"; (0, "DOSL7::enable", "writes DoS protection state"), (1, "DOSL7::profile", "reads it"), (2, "$state", "reports the active profile"))
            }
            Self::FlowState => {
                effect!("set related [FLOW::create_related]\nFLOW::forward $related\nlog local0. $related"; (0, "FLOW::create_related", "creates flow state"), (1, "FLOW::forward", "changes its disposition"), (2, "$related", "identifies the flow"))
            }
            Self::LsnState => {
                effect!("set address [LSN::address]\nLSN::disable\nlog local0. $address"; (0, "LSN::address", "reads LSN state"), (1, "LSN::disable", "writes it"), (2, "$address", "retains the prior value"))
            }
            Self::FtpState => {
                effect!("set port [FTP::port]\nFTP::disable\nlog local0. $port"; (0, "FTP::port", "reads FTP state"), (1, "FTP::disable", "writes it"), (2, "$port", "retains the prior value"))
            }
            Self::IcapState => {
                effect!("set method [ICAP::method]\nICAP::header insert X-Method $method\nlog local0. $method"; (0, "ICAP::method", "reads ICAP state"), (1, "ICAP::header insert", "writes it"), (2, "$method", "identifies the update"))
            }
            Self::MessageState => {
                effect!("set id [MESSAGE::id]\nMESSAGE::field set route primary\nlog local0. $id"; (0, "MESSAGE::id", "reads message state"), (1, "MESSAGE::field set", "writes it"), (2, "$id", "identifies the message"))
            }
            Self::IStats => {
                effect!("ISTATS::set app.requests 1\nISTATS::incr app.requests\nlog local0. [ISTATS::get app.requests]"; (0, "ISTATS::set", "creates counter state"), (1, "ISTATS::incr", "reads and writes it"), (2, "ISTATS::get", "observes 2"))
            }
            Self::ApmState => {
                effect!("ACCESS::session data set session.custom.note ready\nset note [ACCESS::session data get session.custom.note]\nlog local0. $note"; (0, "ACCESS::session data set", "writes APM state"), (1, "ACCESS::session data get", "reads it"), (2, "$note", "observes ready"))
            }
            Self::AsmState => {
                effect!("set before [ASM::status]\nASM::disable\nlog local0. $before"; (0, "ASM::status", "reads ASM state"), (1, "ASM::disable", "writes it"), (2, "$before", "retains the prior result"))
            }
            Self::BigipConfig => {
                effect!("set old [tmsh::get_config /ltm/pool/app]\ntmsh::modify /ltm/pool/app members add {node:80}\nlog local0. $old"; (0, "tmsh::get_config", "reads BIG-IP configuration"), (1, "tmsh::modify", "writes it"), (2, "$old", "contains the previous configuration"))
            }
            Self::ProcDefinition => {
                effect!("proc greet {} {return hello}\nrename greet welcome\nputs [welcome]"; (0, "proc greet", "creates command-table state"), (1, "rename greet welcome", "rewrites it"), (2, "welcome", "uses the new definition"))
            }
            Self::NamespaceState => {
                effect!("namespace eval ::demo { set value 1 }\nnamespace delete ::demo\nputs [namespace exists ::demo]"; (0, "namespace eval ::demo", "creates namespace state"), (1, "namespace delete ::demo", "destroys it"), (2, "namespace exists", "observes false"))
            }
            Self::InterpState => {
                effect!("set child [interp create]\ninterp eval $child {set value 1}\ninterp delete $child"; (0, "interp create", "creates interpreter state"), (1, "interp eval", "writes within it"), (2, "interp delete", "destroys it"))
            }
            Self::Process => {
                effect!("set input [gets stdin]\nset output [exec helper -- $input]\nputs $output"; (0, "[gets stdin]", "supplies external data"), (1, "exec helper -- $input", "creates a process and passes the value"), (2, "$output", "observes process output"))
            }
            Self::ChannelIo => {
                effect!("set channel [open data.txt r]\nset data [chan read $channel]\nputs $data"; (0, "open data.txt r", "creates a channel"), (1, "chan read $channel", "reads channel state"), (2, "$data", "carries the external bytes"))
            }
            Self::EventControl => {
                effect!("when HTTP_REQUEST {\n    event disable all\n    log local0. unreachable\n}"; (0, "HTTP_REQUEST", "starts event execution"), (1, "event disable all", "changes event-control state"), (2, "unreachable", "is skipped for disabled flow"))
            }
            Self::Unknown => {
                effect!("set before [opaque state]\nunknown_effect $before\nputs [opaque state]"; (0, "opaque state", "reads unclassified state"), (1, "unknown_effect", "may read or write any external domain"), (2, "opaque state", "must be treated as changed"))
            }
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
