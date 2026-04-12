//! Side-effect classification for compiler analysis passes.
//!
//! Rich, structured model of what a Tcl/iRules command does to its
//! environment — what it reads, what it writes, where the data lives,
//! what shape it has, and which dialect/connection context applies.
//!
//! The model is layered:
//!
//! 1. **Enums** describe the vocabulary: [`StorageType`] (data shape),
//!    [`StorageScope`] (where data lives), [`ConnectionSide`] (F5 proxy
//!    context), [`SideEffectTarget`] (external resource affected), and
//!    [`EffectRegion`] (coarse bitflags for GVN / interprocedural kill
//!    checks).
//! 2. **Dataclasses** ([`SideEffect`], [`CommandSideEffects`]) compose
//!    those into per-invocation facts — landed in C23b.
//! 3. **Classification functions** resolve registry metadata +
//!    runtime arguments into a [`CommandSideEffects`] — landed in C23d.
//!
//! Consumers include the optimiser (kill safety, CSE), the iRules
//! flow checker (response-commit tracking), the taint engine, and
//! later data-flow analyses that need to know *what* a command
//! touches rather than just *whether* it is pure.
//!
//! Ported from `core/compiler/side_effects.py` in strips:
//!
//! - **C23a** (this file) — enums + `target_to_region`.
//! - **C23b** — `SideEffect`, `CommandSideEffects`, predefined consts.
//! - **C23c** — `scope_from_varname`, `storage_type_for_command`.
//! - **C23d** — `classify_side_effects` entry point.
//!
//! The registry's lightweight `tcl_registry::SideEffect` /
//! `SideEffectTarget` / `StorageType` / `ConnectionSide` types stay
//! unchanged — they are what the command metadata carries; this
//! module holds the richer inferred-by-analysis types.

use bitflags::bitflags;

// ---------------------------------------------------------------------------
// StorageType — data shape of a target
// ---------------------------------------------------------------------------

/// Data shape of the target being read or written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StorageType {
    /// Simple string/integer value (Tcl default representation).
    Scalar,
    /// Tcl list value.
    List,
    /// Tcl dict value (key-value pairs).
    Dict,
    /// Tcl array (associative array of variables, not a value type).
    Array,
    /// Shape cannot be determined statically.
    Unknown,
}

// ---------------------------------------------------------------------------
// StorageScope — where data resides
// ---------------------------------------------------------------------------

/// Where the data resides at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StorageScope {
    // Tcl-universal scopes
    /// Local variable inside the current procedure.
    ProcLocal,
    /// Variable in a named Tcl namespace (`namespace eval`).
    Namespace,
    /// Global namespace variable (`::var` or via `global`).
    Global,
    /// Aliased from a caller's frame via `upvar` / `uplevel`.
    Upvar,

    // F5 iRules-specific scopes
    /// iRules event-scoped state (stable within one `when` handler).
    Event,
    /// iRules connection-scoped state (lives for the connection lifetime).
    Connection,
    /// iRules `static::` variable (system-wide, survives across connections).
    Static,
    /// F5 session table (`table` command). Keyed, with lifetime/timeout.
    SessionTable,
    /// F5 persistence table (`session` / `persist` commands).
    Persistence,
    /// F5 data group / class (read-only at runtime, `class` command).
    DataGroup,

    // External I/O scopes
    /// File on disk (`open` / `puts` / `read` / `close`).
    FileSystem,
    /// Network socket or connection (`socket` / `connect` / `send`).
    NetworkSocket,
    /// Logging destination (`log` / `puts stderr`).
    LogOutput,

    /// Scope cannot be determined statically.
    Unknown,
}

// ---------------------------------------------------------------------------
// ConnectionSide — F5 proxy context
// ---------------------------------------------------------------------------

/// F5 proxy connection context in which a side effect occurs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConnectionSide {
    /// Client-side of the proxy (between client and BIG-IP).
    Client,
    /// Server-side of the proxy (between BIG-IP and pool member).
    Server,
    /// Both sides / the proxy itself (e.g. `table`, connection-wide).
    Both,
    /// No connection context (e.g. `RULE_INIT`, `static::` variables).
    Global,
    /// Not applicable (pure Tcl command, no F5 proxy context).
    None,
}

// ---------------------------------------------------------------------------
// SideEffectTarget — what category of external resource is touched
// ---------------------------------------------------------------------------

/// The category of external resource that a command touches.
///
/// This is the *what*, not the *where* — combine with [`StorageScope`]
/// and [`ConnectionSide`] for the full picture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SideEffectTarget {
    // Variable mutation
    /// Tcl variable read or write (`set`, `incr`, `append`, …).
    Variable,

    // F5 iRules data stores
    /// Session table entry (`table set/add/lookup/delete`).
    SessionTable,
    /// Persistence record (`session add/lookup`, `persist`).
    PersistenceTable,
    /// Data group / class lookup (`class match/search/lookup`).
    DataGroup,

    // HTTP state
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

    // Response lifecycle
    /// Commits or sends an HTTP response (`HTTP::respond`, `redirect`).
    ResponseCommit,
    /// Connection-level action: drop, reject, discard, forward.
    ConnectionControl,

    // Transport / TLS
    /// TCP connection state (`TCP::close`, `TCP::collect`, …).
    TcpState,
    /// SSL/TLS state (`SSL::disable`, `SSL::cert`, …).
    SslState,
    /// UDP datagram state.
    UdpState,

    // Load balancing
    /// Pool or pool member selection (`pool`, `LB::select`).
    PoolSelection,
    /// Direct node selection (`node`).
    NodeSelection,
    /// SNAT address selection (`snat`, `snatpool`).
    SnatSelection,

    // External I/O
    /// File system read/write (`open`, `puts`, `read`, `close`).
    FileIo,
    /// Network socket I/O (`socket`, `connect`, `send`).
    NetworkIo,
    /// Logging output (`log local0.`, `puts stderr`).
    LogIo,

    /// Content rewriting via stream profile (`STREAM::`, `REWRITE::`).
    StreamProfile,

    // DNS
    /// DNS message state (`DNS::header`, `DNS::answer`, …).
    DnsState,

    // Classification / DoS
    /// Traffic classification state (`CLASSIFY::`, `CLASSIFICATION::`).
    ClassificationState,
    /// Layer-7 denial-of-service protection state (`DOSL7::`).
    Dosl7State,

    // Flow / connection management
    /// Flow object state (`FLOW::create_related`, `FLOW::idle_timeout`, …).
    FlowState,
    /// Large Scale NAT state (`LSN::address`, `LSN::persistence`, …).
    LsnState,

    // Application protocols
    /// FTP protocol state (`FTP::enable`, `FTP::port`, …).
    FtpState,
    /// ICAP protocol state (`ICAP::header`, `ICAP::method`, …).
    IcapState,
    /// Message routing state (`MESSAGE::field`, `MR::message`, …).
    MessageState,

    // Statistics
    /// Internal statistics counters (`ISTATS::set`, `ISTATS::incr`, …).
    IStats,

    // F5 security / policy
    /// Access Policy Manager state (`ACCESS::session`, `ACCESS::policy`, …).
    ApmState,
    /// Application Security Manager state (`ASM::enable`, `ASM::disable`, …).
    AsmState,

    // F5 configuration (iApps)
    /// BIG-IP configuration change (iApps, `tmsh::` commands).
    BigipConfig,

    // Interpreter state
    /// Defines or removes a procedure (`proc`, `rename`).
    ProcDefinition,
    /// Namespace creation / deletion (`namespace eval`, `namespace delete`).
    NamespaceState,
    /// Interpreter-level state (`interp`, `package`, `load`).
    InterpState,

    /// Target cannot be determined statically.
    Unknown,
}

// ---------------------------------------------------------------------------
// EffectRegion — coarse bitflags for GVN / interprocedural kill checks
// ---------------------------------------------------------------------------

bitflags! {
    /// Abstract mutable regions used for effect invalidation.
    ///
    /// Coarse view consumed by GVN and interprocedural analysis. The
    /// structured [`SideEffectTarget`] model is authoritative; this
    /// enum exists purely for fast bitwise kill checks.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct EffectRegion: u32 {
        /// No region.
        const NONE               = 0;
        /// Any HTTP state (header, body, status, URI, cookie, method, HTTP/2).
        const HTTP_STATE         = 1 << 0;
        /// Response lifecycle (commit / redirect / respond).
        const RESPONSE_LIFECYCLE = 1 << 1;
        /// Global or namespace-scoped variable state.
        const GLOBAL_STATE       = 1 << 2;
        /// Catch-all for unknown effects.
        const UNKNOWN_STATE      = 1 << 3;
    }
}

/// Map a structured target to a coarse [`EffectRegion`].
#[must_use]
pub fn target_to_region(target: SideEffectTarget, scope: StorageScope) -> EffectRegion {
    match target {
        SideEffectTarget::HttpHeader
        | SideEffectTarget::HttpBody
        | SideEffectTarget::HttpStatus
        | SideEffectTarget::HttpUri
        | SideEffectTarget::HttpCookie
        | SideEffectTarget::HttpMethod
        | SideEffectTarget::Http2State => EffectRegion::HTTP_STATE,
        SideEffectTarget::ResponseCommit => {
            EffectRegion::RESPONSE_LIFECYCLE | EffectRegion::HTTP_STATE
        }
        SideEffectTarget::Variable => {
            if matches!(scope, StorageScope::Global | StorageScope::Namespace) {
                EffectRegion::GLOBAL_STATE
            } else {
                EffectRegion::NONE
            }
        }
        // External I/O does not mutate compiler-tracked in-memory state.
        SideEffectTarget::FileIo | SideEffectTarget::NetworkIo | SideEffectTarget::LogIo => {
            EffectRegion::NONE
        }
        _ => EffectRegion::UNKNOWN_STATE,
    }
}

// ---------------------------------------------------------------------------
// Dataclasses — SideEffect, CommandSideEffects (C23b)
// ---------------------------------------------------------------------------

/// One discrete read or write produced by a command invocation.
///
/// A single command may produce multiple [`SideEffect`] instances.
/// For example, `HTTP::header replace Host "example.com"` produces
/// both a read (current header state) and a write (new header
/// value).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SideEffect {
    /// What category of resource is touched.
    pub target: SideEffectTarget,
    /// Whether this effect includes a read from the target.
    pub reads: bool,
    /// Whether this effect includes a write to the target.
    pub writes: bool,
    /// Data shape of the target (scalar, list, dict, array).
    pub storage_type: StorageType,
    /// Where the data resides (proc-local, global, F5 table, …).
    pub scope: StorageScope,
    /// F5 proxy context for this effect.
    pub connection_side: ConnectionSide,
    /// Tcl namespace or F5 protocol namespace (e.g. `"HTTP"`).
    ///
    /// For Tcl variables this is the namespace path (`"::foo::bar"`).
    /// For F5 commands this is the protocol prefix (`"HTTP"`,
    /// `"SSL"`). `None` when not applicable or not determinable.
    pub namespace: Option<String>,
    /// Dialect this effect applies to (`"irules"`, `"tcl"`, …).
    ///
    /// `None` means the effect is dialect-independent.
    pub dialect: Option<String>,
    /// Optional key identifying the specific target.
    ///
    /// For variables: the variable name. For `table` / `session`:
    /// the key expression (if literal). For HTTP headers: the
    /// header name (if literal). `None` when dynamic or not
    /// applicable.
    pub key: Option<String>,
    /// F5 session-table subtable name, if applicable.
    pub subtable: Option<String>,
}

impl SideEffect {
    /// Build a minimal effect with `target`, `reads`, `writes` set
    /// and all other fields at their defaults. Convenient for
    /// classifier impls to chain with struct-update syntax.
    #[must_use]
    pub fn new(target: SideEffectTarget, reads: bool, writes: bool) -> Self {
        Self {
            target,
            reads,
            writes,
            storage_type: StorageType::Unknown,
            scope: StorageScope::Unknown,
            connection_side: ConnectionSide::None,
            namespace: None,
            dialect: None,
            key: None,
            subtable: None,
        }
    }
}

/// Complete side-effect profile for one command invocation.
///
/// Wraps zero or more individual [`SideEffect`] instances plus
/// summary flags for quick consumer queries. Produced by
/// [`classify_side_effects`] (C23d).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CommandSideEffects {
    /// Individual side effects produced by this invocation.
    pub effects: Vec<SideEffect>,
    /// No observable side effects (reads from immutable state OK).
    pub pure: bool,
    /// Same inputs always produce the same outputs.
    pub deterministic: bool,
    /// Contains `eval`/`uplevel`/`call` — effects are unknowable.
    pub dynamic_barrier: bool,
    /// Dialect context in which this classification was made.
    pub dialect: Option<String>,
}

impl CommandSideEffects {
    /// The canonical pure/deterministic result.
    #[must_use]
    pub fn pure() -> Self {
        Self {
            effects: Vec::new(),
            pure: true,
            deterministic: true,
            dynamic_barrier: false,
            dialect: None,
        }
    }

    /// A conservative "writes something unknown" result. Used when
    /// the classifier cannot determine a precise target.
    #[must_use]
    pub fn unknown_write() -> Self {
        Self {
            effects: vec![SideEffect::new(SideEffectTarget::Unknown, false, true)],
            pure: false,
            deterministic: false,
            dynamic_barrier: false,
            dialect: None,
        }
    }

    /// A dynamic-dispatch barrier (`eval` / `uplevel` / `call`).
    #[must_use]
    pub fn dynamic_barrier() -> Self {
        Self {
            effects: Vec::new(),
            pure: false,
            deterministic: false,
            dynamic_barrier: true,
            dialect: None,
        }
    }

    /// Whether any effect includes a read.
    #[must_use]
    pub fn reads_any(&self) -> bool {
        self.effects.iter().any(|e| e.reads)
    }

    /// Whether any effect includes a write.
    #[must_use]
    pub fn writes_any(&self) -> bool {
        self.effects.iter().any(|e| e.writes)
    }

    /// Whether this invocation touches `target` (read or write).
    #[must_use]
    pub fn affects_target(&self, target: SideEffectTarget) -> bool {
        self.effects.iter().any(|e| e.target == target)
    }

    /// Whether this invocation writes to `target`.
    #[must_use]
    pub fn writes_target(&self, target: SideEffectTarget) -> bool {
        self.effects.iter().any(|e| e.target == target && e.writes)
    }

    /// Whether this invocation reads from `target`.
    #[must_use]
    pub fn reads_target(&self, target: SideEffectTarget) -> bool {
        self.effects.iter().any(|e| e.target == target && e.reads)
    }

    /// Effects restricted to a specific storage scope.
    #[must_use]
    pub fn effects_in_scope(&self, scope: StorageScope) -> Vec<&SideEffect> {
        self.effects.iter().filter(|e| e.scope == scope).collect()
    }

    /// Effects restricted to a specific connection side.
    #[must_use]
    pub fn effects_on_side(&self, side: ConnectionSide) -> Vec<&SideEffect> {
        self.effects
            .iter()
            .filter(|e| e.connection_side == side)
            .collect()
    }

    /// Map structured effects to coarse `(reads, writes)`
    /// [`EffectRegion`] bitflags. Suitable for GVN /
    /// interprocedural kill checks.
    #[must_use]
    pub fn to_effect_regions(&self) -> (EffectRegion, EffectRegion) {
        let mut reads = EffectRegion::NONE;
        let mut writes = EffectRegion::NONE;
        for e in &self.effects {
            let region = target_to_region(e.target, e.scope);
            if e.reads {
                reads |= region;
            }
            if e.writes {
                writes |= region;
            }
        }
        if self.dynamic_barrier {
            writes |= EffectRegion::UNKNOWN_STATE;
        }
        (reads, writes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_targets_map_to_http_state_region() {
        for target in [
            SideEffectTarget::HttpHeader,
            SideEffectTarget::HttpBody,
            SideEffectTarget::HttpStatus,
            SideEffectTarget::HttpUri,
            SideEffectTarget::HttpCookie,
            SideEffectTarget::HttpMethod,
            SideEffectTarget::Http2State,
        ] {
            assert_eq!(
                target_to_region(target, StorageScope::Unknown),
                EffectRegion::HTTP_STATE,
                "{target:?}"
            );
        }
    }

    #[test]
    fn response_commit_is_both_response_and_http_state() {
        let r = target_to_region(SideEffectTarget::ResponseCommit, StorageScope::Unknown);
        assert!(r.contains(EffectRegion::RESPONSE_LIFECYCLE));
        assert!(r.contains(EffectRegion::HTTP_STATE));
    }

    #[test]
    fn variable_scope_distinguishes_global_vs_local() {
        assert_eq!(
            target_to_region(SideEffectTarget::Variable, StorageScope::Global),
            EffectRegion::GLOBAL_STATE
        );
        assert_eq!(
            target_to_region(SideEffectTarget::Variable, StorageScope::Namespace),
            EffectRegion::GLOBAL_STATE
        );
        assert_eq!(
            target_to_region(SideEffectTarget::Variable, StorageScope::ProcLocal),
            EffectRegion::NONE
        );
    }

    #[test]
    fn external_io_is_not_compiler_tracked() {
        assert_eq!(
            target_to_region(SideEffectTarget::FileIo, StorageScope::FileSystem),
            EffectRegion::NONE
        );
        assert_eq!(
            target_to_region(SideEffectTarget::NetworkIo, StorageScope::NetworkSocket),
            EffectRegion::NONE
        );
        assert_eq!(
            target_to_region(SideEffectTarget::LogIo, StorageScope::LogOutput),
            EffectRegion::NONE
        );
    }

    #[test]
    fn unknown_targets_fall_through() {
        assert_eq!(
            target_to_region(SideEffectTarget::Unknown, StorageScope::Unknown),
            EffectRegion::UNKNOWN_STATE
        );
        assert_eq!(
            target_to_region(SideEffectTarget::SessionTable, StorageScope::Unknown),
            EffectRegion::UNKNOWN_STATE
        );
    }

    #[test]
    fn effect_region_supports_bitwise_combine() {
        let combined = EffectRegion::HTTP_STATE | EffectRegion::GLOBAL_STATE;
        assert!(combined.contains(EffectRegion::HTTP_STATE));
        assert!(combined.contains(EffectRegion::GLOBAL_STATE));
        assert!(!combined.contains(EffectRegion::UNKNOWN_STATE));
    }

    // -- C23b: SideEffect / CommandSideEffects --

    #[test]
    fn side_effect_new_defaults() {
        let e = SideEffect::new(SideEffectTarget::Variable, true, false);
        assert_eq!(e.target, SideEffectTarget::Variable);
        assert!(e.reads);
        assert!(!e.writes);
        assert_eq!(e.storage_type, StorageType::Unknown);
        assert_eq!(e.scope, StorageScope::Unknown);
        assert_eq!(e.connection_side, ConnectionSide::None);
        assert!(e.namespace.is_none());
        assert!(e.dialect.is_none());
        assert!(e.key.is_none());
        assert!(e.subtable.is_none());
    }

    #[test]
    fn command_side_effects_pure_constant() {
        let cse = CommandSideEffects::pure();
        assert!(cse.pure);
        assert!(cse.deterministic);
        assert!(cse.effects.is_empty());
        assert!(!cse.dynamic_barrier);
        assert!(!cse.reads_any());
        assert!(!cse.writes_any());
    }

    #[test]
    fn command_side_effects_unknown_write_constant() {
        let cse = CommandSideEffects::unknown_write();
        assert!(!cse.pure);
        assert_eq!(cse.effects.len(), 1);
        assert_eq!(cse.effects[0].target, SideEffectTarget::Unknown);
        assert!(cse.effects[0].writes);
        assert!(cse.writes_any());
    }

    #[test]
    fn command_side_effects_dynamic_barrier_flag() {
        let cse = CommandSideEffects::dynamic_barrier();
        assert!(cse.dynamic_barrier);
        let (_, w) = cse.to_effect_regions();
        assert!(w.contains(EffectRegion::UNKNOWN_STATE));
    }

    #[test]
    fn affects_reads_writes_target_helpers() {
        let cse = CommandSideEffects {
            effects: vec![
                SideEffect::new(SideEffectTarget::HttpHeader, true, false),
                SideEffect::new(SideEffectTarget::Variable, false, true),
            ],
            ..CommandSideEffects::default()
        };
        assert!(cse.affects_target(SideEffectTarget::HttpHeader));
        assert!(cse.affects_target(SideEffectTarget::Variable));
        assert!(!cse.affects_target(SideEffectTarget::FileIo));
        assert!(cse.reads_target(SideEffectTarget::HttpHeader));
        assert!(!cse.reads_target(SideEffectTarget::Variable));
        assert!(cse.writes_target(SideEffectTarget::Variable));
    }

    #[test]
    fn to_effect_regions_unions_reads_and_writes() {
        let mut global_var = SideEffect::new(SideEffectTarget::Variable, false, true);
        global_var.scope = StorageScope::Global;
        let cse = CommandSideEffects {
            effects: vec![
                SideEffect::new(SideEffectTarget::HttpHeader, true, false),
                global_var,
            ],
            ..CommandSideEffects::default()
        };
        let (r, w) = cse.to_effect_regions();
        assert!(r.contains(EffectRegion::HTTP_STATE));
        assert!(w.contains(EffectRegion::GLOBAL_STATE));
        assert!(!w.contains(EffectRegion::HTTP_STATE));
    }

    #[test]
    fn effects_filtered_by_scope_and_side() {
        let mut client_hdr = SideEffect::new(SideEffectTarget::HttpHeader, true, false);
        client_hdr.connection_side = ConnectionSide::Client;
        let mut server_hdr = SideEffect::new(SideEffectTarget::HttpHeader, true, false);
        server_hdr.connection_side = ConnectionSide::Server;
        let cse = CommandSideEffects {
            effects: vec![client_hdr, server_hdr],
            ..CommandSideEffects::default()
        };
        assert_eq!(cse.effects_on_side(ConnectionSide::Client).len(), 1);
        assert_eq!(cse.effects_on_side(ConnectionSide::Server).len(), 1);
        assert_eq!(cse.effects_in_scope(StorageScope::Global).len(), 0);
    }
}
