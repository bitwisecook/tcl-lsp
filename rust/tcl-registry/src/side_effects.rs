//! Side-effect metadata for structured effect analysis.

/// What kind of external state a command affects.
///
/// Mirrors Python `SideEffectTarget` in `core/compiler/side_effects.py`
/// (the reference standard); variant names match the consumer's
/// `tcl_compiler::side_effects::SideEffectTarget` so the two can be
/// unified later. `Process` / `ChannelIo` are registry-only (no Python
/// counterpart) — kept for the existing `exec` / `chan` core specs.
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
    /// Process management (registry-only; `exec`).
    Process,
    /// Channel I/O (registry-only; `chan`).
    ChannelIo,
    /// Unknown or unclassified effect.
    Unknown,
}

/// Which connection side a command operates on (iRules).
///
/// Mirrors Python `ConnectionSide`; variant names match the consumer's
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
