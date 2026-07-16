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
//!    those into per-invocation facts.
//! 3. **Classification functions** resolve registry metadata +
//!    runtime arguments into a [`CommandSideEffects`].
//!
//! Consumers include the optimiser (kill safety, CSE), the iRules
//! flow checker (response-commit tracking), the taint engine, and
//! later data-flow analyses that need to know *what* a command
//! touches rather than just *whether* it is pure.
//!
//! The registry's lightweight `tcl_registry::SideEffect` /
//! `SideEffectTarget` / `StorageType` / `ConnectionSide` types stay
//! unchanged — they are what the command metadata carries; this
//! module holds the richer inferred-by-analysis types.

use bitflags::bitflags;

use tcl_dialect::DialectSet;
use tcl_registry::prelude::StorageType as RegistryStorageType;
use tcl_registry::side_effects::{
    ConnectionSide as RegistryConnectionSide, SideEffect as RegistrySideEffect,
    SideEffectTarget as RegistryTarget,
};
use tcl_registry::{CommandRegistry, CommandSpec, Traits};

// StorageType — data shape of a target

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

// StorageScope — where data resides

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

// ConnectionSide — F5 proxy context

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

// SideEffectTarget — what category of external resource is touched

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

// EffectRegion — coarse bitflags for GVN / interprocedural kill checks

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

// Scope / storage-type inference helpers

/// Infer storage scope and namespace from a variable-name prefix.
///
/// Returns `(scope, namespace)` where `namespace` is `Some("::…")`
/// for qualified names (populated only for [`StorageScope::Namespace`]
/// and [`StorageScope::Global`]) or `None` otherwise.
///
/// Conventions:
/// - `static::NAME` → iRules [`StorageScope::Static`] (namespace
///   left as `None` — the `static::` prefix is the scope marker,
///   not a Tcl namespace).
/// - `::NAME` with no further `::` → [`StorageScope::Global`] with
///   `Some("::")`.
/// - `::NS::…::VAR` → [`StorageScope::Namespace`] with
///   `Some("::NS::…")` (the last `::` segment is the variable
///   name itself and is stripped).
/// - Everything else → [`StorageScope::ProcLocal`] with `None`.
#[must_use]
pub fn scope_from_varname(name: &str) -> (StorageScope, Option<String>) {
    if let Some(rest) = name.strip_prefix("static::") {
        // Only treat as iRules static if the rest doesn't itself
        // contain further qualification — but this does not distinguish
        // there, treating any `static::` prefix as static scope.
        let _ = rest;
        return (StorageScope::Static, None);
    }
    if let Some(rest) = name.strip_prefix("::") {
        // Peel all "::"-separated segments except the last (the
        // variable name itself). Empty leading segments from "::"
        // are ignored in the rebuild.
        let parts: Vec<&str> = rest.split("::").collect();
        if parts.len() <= 1 {
            // Pure "::VAR" — global scope, namespace is root.
            return (StorageScope::Global, Some("::".into()));
        }
        let ns_parts: Vec<&str> = parts[..parts.len() - 1]
            .iter()
            .copied()
            .filter(|p| !p.is_empty())
            .collect();
        let ns = if ns_parts.is_empty() {
            "::".to_string()
        } else {
            format!("::{}", ns_parts.join("::"))
        };
        let scope = if ns == "::" {
            StorageScope::Global
        } else {
            StorageScope::Namespace
        };
        return (scope, Some(ns));
    }
    (StorageScope::ProcLocal, None)
}

/// Lift a registry `StorageType` value into the richer
/// compiler-side [`StorageType`].
fn lift_registry_storage_type(rt: RegistryStorageType) -> StorageType {
    match rt {
        RegistryStorageType::Dict => StorageType::Dict,
        RegistryStorageType::List => StorageType::List,
        RegistryStorageType::Array => StorageType::Array,
    }
}

/// Infer the storage type for a command invocation by querying the
/// registry's `inferred_storage_type` metadata. Falls back to
/// [`StorageType::Scalar`] for commands without a declared shape, so
/// `set`, `incr`, `append`, and similar scalar commands default to
/// the scalar shape.
///
/// `args` is accepted for API symmetry but currently ignored;
/// argument-dependent shape inference is not implemented.
#[must_use]
pub fn storage_type_for_command(
    registry: &CommandRegistry,
    command: &str,
    _args: &[String],
) -> StorageType {
    if let Some(spec) = registry.get(command)
        && let Some(rt) = spec.inferred_storage_type
    {
        return lift_registry_storage_type(rt);
    }
    StorageType::Scalar
}

// SideEffect, CommandSideEffects

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
/// [`classify_side_effects`].
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

// Classification

/// Optional summary of a procedure's side effects, provided by
/// interprocedural analysis. When supplied to
/// [`classify_side_effects`], it bypasses registry-based classification
/// and the summary's effect regions are translated directly into
/// [`CommandSideEffects`].
#[derive(Debug, Clone, Copy)]
pub struct CalleeSummary {
    /// Coarse regions read by the callee.
    pub effect_reads: EffectRegion,
    /// Coarse regions written by the callee.
    pub effect_writes: EffectRegion,
    /// Whether the callee is pure overall.
    pub pure: bool,
}

impl Default for CalleeSummary {
    fn default() -> Self {
        Self {
            effect_reads: EffectRegion::NONE,
            effect_writes: EffectRegion::NONE,
            pure: false,
        }
    }
}

/// Classify the side effects of a command invocation.
///
/// Combines registry metadata with argument inspection to produce a
/// structured [`CommandSideEffects`] describing what the command
/// reads and writes, in what scope, and on which connection side.
///
/// When `callee_summary` is `Some`, registry-based classification is
/// skipped and the summary's effect regions are translated directly
/// into effects — see [`classify_from_callee_summary`].
///
/// The registry
/// traits consulted today are `PURE`, `PURE_EVALUATION`,
/// `EVALUATES_CODE`, `CREATES_BARRIER`, `DEFINES_PROCEDURE`, and
/// `DESTROYS_VARIABLE`, plus the spec-level `assigns_variable_at`
/// field, and the spec's structured `side_effects`. A `PURE` command
/// with a mutator subcommand
/// (`HTTP::header insert …`) is downgraded to impure and a still-pure
/// command surfaces its hint read-only. Full arity-based form
/// resolution and the per-subcommand protocol-namespace write modelling
/// are not implemented.
#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "sequential registry-trait dispatch; splitting hurts readability"
)]
pub fn classify_side_effects(
    registry: &CommandRegistry,
    command: &str,
    args: &[String],
    dialect: Option<&str>,
    callee_summary: Option<&CalleeSummary>,
) -> CommandSideEffects {
    if let Some(summary) = callee_summary {
        return classify_from_callee_summary(summary, dialect);
    }

    let Some(spec) = registry.get(command) else {
        return fallback_unknown_write(dialect);
    };

    // Dynamic-dispatch commands are barriers.
    if spec
        .traits
        .intersects(Traits::EVALUATES_CODE | Traits::CREATES_BARRIER)
    {
        return CommandSideEffects {
            effects: vec![SideEffect::new(SideEffectTarget::Unknown, true, true)],
            dynamic_barrier: true,
            dialect: dialect.map(str::to_owned),
            ..CommandSideEffects::default()
        };
    }

    // Pure evaluation (`expr` when braced) is always pure — checked first
    // and unconditionally, mirroring `is_pure_evaluation` short-
    // circuit (no subcommand / hint handling applies).
    if spec.traits.contains(Traits::PURE_EVALUATION) {
        return CommandSideEffects {
            pure: true,
            deterministic: true,
            dialect: dialect.map(str::to_owned),
            ..CommandSideEffects::default()
        };
    }

    // Command-level `PURE` commands. Two refinements on the pure
    // branch:
    //   1. A *mutator* subcommand downgrades purity — e.g. `HTTP::header`
    //      is `PURE` but `HTTP::header insert …` mutates header state, so
    //      it must fall through to the structured-hints classification
    //      below (which yields the read+write effect). Without this a
    //      header mutation reads as pure with no effects.
    //   2. A still-pure command may carry a *read-only* hint — `HTTP::header`
    //      (getter) reads header state. Return the structured hint
    //      with `reads=true, writes=false` by forcing the
    //      spec's hints read-only.
    if spec.traits.contains(Traits::PURE) {
        let effective_sub = args.first().map(String::as_str);
        let sub_is_mutator = effective_sub
            .and_then(|s| spec.resolve_subcommand(s))
            .is_some_and(|sc| sc.mutator);
        if !sub_is_mutator {
            let effects = dialect_side_effect_hints(registry, command, effective_sub, dialect)
                .map(|hints| {
                    hints
                        .into_iter()
                        .map(|mut e| {
                            e.reads = true;
                            e.writes = false;
                            e
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            return CommandSideEffects {
                effects,
                pure: true,
                deterministic: true,
                dialect: dialect.map(str::to_owned),
                ..CommandSideEffects::default()
            };
        }
        // Mutator subcommand → not pure; fall through to the hints block.
    }

    // Subcommand-level purity: ensemble commands (`string`, `dict`, `array`,
    // …) are not pure at the command level but most of their subcommands are
    // (`string length`, `dict get`, `array names`). A command that is not
    // pure inherits purity from a pure subcommand spec, with a `mutator`
    // subcommand overriding back to impure.
    // Without this the side-effect classifier treats every ensemble call as a
    // conservative unknown-write, which (e.g.) hides redundant `string length`
    // computations from GVN.
    if let Some(sub) = args.first().and_then(|a| spec.resolve_subcommand(a))
        && sub.pure
        && !sub.mutator
    {
        return CommandSideEffects {
            pure: true,
            deterministic: true,
            dialect: dialect.map(str::to_owned),
            ..CommandSideEffects::default()
        };
    }

    // Variable-assigning commands (`set`, `incr`, `append`, `lappend`, …).
    if let Some(idx) = spec.assigns_variable_at {
        let idx = usize::from(idx);
        if idx < args.len() {
            return classify_variable_assignment(
                registry,
                command,
                args,
                idx,
                spec.traits,
                dialect,
            );
        }
        return CommandSideEffects {
            effects: vec![SideEffect::new(SideEffectTarget::Variable, true, true)],
            dialect: dialect.map(str::to_owned),
            ..CommandSideEffects::default()
        };
    }

    // Procedure definition (`proc`).
    if spec.traits.contains(Traits::DEFINES_PROCEDURE) {
        let mut e = SideEffect::new(SideEffectTarget::ProcDefinition, false, true);
        e.scope = StorageScope::Namespace;
        e.dialect = dialect.map(str::to_owned);
        return CommandSideEffects {
            effects: vec![e],
            dialect: dialect.map(str::to_owned),
            ..CommandSideEffects::default()
        };
    }

    // Destroys a variable (`unset`).
    if spec.traits.contains(Traits::DESTROYS_VARIABLE) {
        // The destroyed variables are exactly the `VarWrite`-role args, resolved
        // through the registry's arg-role resolver — which skips the leading
        // options (`-nocomplain`, `--`) and yields *every* trailing name, not
        // just the first. So `unset -nocomplain x` keys `x` (not `-nocomplain`),
        // and `unset a b` keys both `a` and `b` (RUST_ISSUE_078).
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let targets =
            registry.arg_indices_for_role(command, &arg_refs, tcl_registry::ArgRole::VarWrite);
        let make_effect = |name: Option<&str>| {
            let mut e = SideEffect::new(SideEffectTarget::Variable, false, true);
            if let Some(name) = name {
                let (scope, ns) = scope_from_varname(name);
                e.scope = scope;
                e.namespace = ns;
                e.key = Some(name.to_owned());
            }
            e.dialect = dialect.map(str::to_owned);
            e
        };
        // No named target (`unset`, `unset -nocomplain`, `unset --`) is a
        // valid no-op destroy; still emit one untargeted Variable effect so the
        // command stays classified impure rather than pure.
        let effects: Vec<SideEffect> = if targets.is_empty() {
            vec![make_effect(None)]
        } else {
            targets
                .iter()
                .map(|&idx| make_effect(args.get(idx).map(String::as_str)))
                .collect()
        };
        return CommandSideEffects {
            effects,
            dialect: dialect.map(str::to_owned),
            ..CommandSideEffects::default()
        };
    }

    // Hints-first: consult the spec's structured `side_effects`
    // (`side_effect_hints`). Dialect-gated, subcommand hints
    // first. e.g. `puts`/`read` declare a `FileIo` write whose coarse
    // region is `NONE` — so a proc that only calls `puts` is impure
    // (`writes_any`) yet has no tracked effect region.
    let effective_sub = args.first().map(String::as_str);
    if let Some(effects) = dialect_side_effect_hints(registry, command, effective_sub, dialect) {
        return CommandSideEffects {
            effects,
            dialect: dialect.map(str::to_owned),
            ..CommandSideEffects::default()
        };
    }

    // Fallback: conservative unknown read+write.
    fallback_unknown_write(dialect)
}

/// Translate an interprocedural [`CalleeSummary`] into a
/// [`CommandSideEffects`].
fn classify_from_callee_summary(
    summary: &CalleeSummary,
    dialect: Option<&str>,
) -> CommandSideEffects {
    let reads = summary.effect_reads;
    let writes = summary.effect_writes;
    let commits = writes.contains(EffectRegion::RESPONSE_LIFECYCLE);
    let mut effects = Vec::new();

    if reads.contains(EffectRegion::HTTP_STATE) || writes.contains(EffectRegion::HTTP_STATE) {
        effects.push(SideEffect::new(
            SideEffectTarget::HttpHeader,
            reads.contains(EffectRegion::HTTP_STATE),
            writes.contains(EffectRegion::HTTP_STATE),
        ));
    }
    if commits {
        effects.push(SideEffect::new(
            SideEffectTarget::ResponseCommit,
            false,
            true,
        ));
    }
    if writes.contains(EffectRegion::GLOBAL_STATE) {
        let mut e = SideEffect::new(SideEffectTarget::Variable, false, true);
        e.scope = StorageScope::Global;
        effects.push(e);
    }
    if writes.contains(EffectRegion::UNKNOWN_STATE) {
        effects.push(SideEffect::new(SideEffectTarget::Unknown, true, true));
    }
    if effects.is_empty() && !summary.pure {
        effects.push(SideEffect::new(SideEffectTarget::Unknown, true, true));
    }

    CommandSideEffects {
        effects,
        pure: summary.pure,
        deterministic: summary.pure,
        dynamic_barrier: false,
        dialect: dialect.map(str::to_owned),
    }
}

/// Build a `SideEffect` for a command that assigns a variable at
/// position `idx`. Applies these heuristics:
///
/// - Read-before-write commands (`incr`, `append`, `lappend`) always
///   read and write.
/// - `set x` (single var, no value) is a read; `set x v` is a write.
/// - iRules context: `static::` targets use `ConnectionSide::Global`;
///   proc-local targets use `ConnectionSide::Both`.
fn classify_variable_assignment(
    registry: &CommandRegistry,
    command: &str,
    args: &[String],
    idx: usize,
    traits: Traits,
    dialect: Option<&str>,
) -> CommandSideEffects {
    let varname = &args[idx];
    let (scope, ns) = scope_from_varname(varname);
    let st = storage_type_for_command(registry, command, args);
    let rmw = traits.contains(Traits::READS_BEFORE_WRITE);

    // When there is only the variable name and no value, it's a read
    // (`set x`); otherwise it's a write. Read-modify-write commands
    // always write even with one arg.
    let is_write = args.len() > idx + 1 || rmw;
    let is_read = rmw || args.len() == idx + 1;

    let is_irules = matches!(dialect, Some("irules" | "f5-irules"));
    let side = if is_irules {
        match scope {
            StorageScope::Static => ConnectionSide::Global,
            StorageScope::ProcLocal => ConnectionSide::Both,
            _ => ConnectionSide::None,
        }
    } else {
        ConnectionSide::None
    };

    let mut effect = SideEffect::new(SideEffectTarget::Variable, is_read, is_write);
    effect.storage_type = st;
    effect.scope = scope;
    effect.connection_side = side;
    effect.namespace = ns;
    effect.dialect = dialect.map(str::to_owned);
    effect.key = Some(varname.clone());

    let mut effects = vec![effect];

    // A write to an interpreter special variable (`auto_path`, `tcl_precision`,
    // `env`, …) mutates the interpreter/runtime state the special-variable
    // registry records — not just the variable slot — so surface that extra
    // effect. It keeps effect analysis and dead-code elimination from treating
    // `set auto_path …` as a removable plain assignment (issue #831). The
    // registry lookup is dialect-aware, so it fires only where the variable
    // actually exists.
    if is_write {
        let base = crate::naming::normalise_var_name(varname);
        if let Some(target) =
            tcl_registry::special_vars::special_var_write_effect(base, dialect.unwrap_or(""))
        {
            let mut extra = SideEffect::new(lift_registry_target(target), false, true);
            extra.dialect = dialect.map(str::to_owned);
            effects.push(extra);
        }
    }

    CommandSideEffects {
        effects,
        dialect: dialect.map(str::to_owned),
        ..CommandSideEffects::default()
    }
}

/// Translate a registry [`RegistryTarget`] into the richer
/// compiler-side [`SideEffectTarget`]. The two enums share variant
/// names 1:1 except for the registry-only `Process` / `ChannelIo`
/// targets: `Process` collapses to
/// [`SideEffectTarget::Unknown`] (its coarse region is
/// `UNKNOWN_STATE`, matching `exec` fallback) and `ChannelIo`
/// maps to [`SideEffectTarget::FileIo`] (channel I/O is file-shaped,
/// region `NONE`).
// Each variant is mapped explicitly (1:1 by name, plus the two
// registry-only fall-throughs) so the correspondence is auditable and
// new registry targets fail to compile until mapped. Arms that share a
// body are grouped with `|`.
fn lift_registry_target(t: RegistryTarget) -> SideEffectTarget {
    use RegistryTarget as R;
    match t {
        R::Variable => SideEffectTarget::Variable,
        R::SessionTable => SideEffectTarget::SessionTable,
        R::PersistenceTable => SideEffectTarget::PersistenceTable,
        R::DataGroup => SideEffectTarget::DataGroup,
        R::HttpHeader => SideEffectTarget::HttpHeader,
        R::HttpBody => SideEffectTarget::HttpBody,
        R::HttpStatus => SideEffectTarget::HttpStatus,
        R::HttpUri => SideEffectTarget::HttpUri,
        R::HttpCookie => SideEffectTarget::HttpCookie,
        R::HttpMethod => SideEffectTarget::HttpMethod,
        R::Http2State => SideEffectTarget::Http2State,
        R::ResponseCommit => SideEffectTarget::ResponseCommit,
        R::ConnectionControl => SideEffectTarget::ConnectionControl,
        R::TcpState => SideEffectTarget::TcpState,
        R::SslState => SideEffectTarget::SslState,
        R::UdpState => SideEffectTarget::UdpState,
        R::PoolSelection => SideEffectTarget::PoolSelection,
        R::NodeSelection => SideEffectTarget::NodeSelection,
        R::SnatSelection => SideEffectTarget::SnatSelection,
        // `ChannelIo` is file-shaped, so it lifts to `FileIo` like `FileIo`.
        R::FileIo | R::ChannelIo => SideEffectTarget::FileIo,
        R::NetworkIo => SideEffectTarget::NetworkIo,
        R::LogIo => SideEffectTarget::LogIo,
        R::StreamProfile => SideEffectTarget::StreamProfile,
        R::DnsState => SideEffectTarget::DnsState,
        R::ClassificationState => SideEffectTarget::ClassificationState,
        R::Dosl7State => SideEffectTarget::Dosl7State,
        R::FlowState => SideEffectTarget::FlowState,
        R::LsnState => SideEffectTarget::LsnState,
        R::FtpState => SideEffectTarget::FtpState,
        R::IcapState => SideEffectTarget::IcapState,
        R::MessageState => SideEffectTarget::MessageState,
        R::IStats => SideEffectTarget::IStats,
        R::ApmState => SideEffectTarget::ApmState,
        R::AsmState => SideEffectTarget::AsmState,
        R::BigipConfig => SideEffectTarget::BigipConfig,
        R::ProcDefinition => SideEffectTarget::ProcDefinition,
        R::NamespaceState => SideEffectTarget::NamespaceState,
        R::InterpState => SideEffectTarget::InterpState,
        // Registry-only targets with no compiler counterpart.
        R::Process | R::Unknown => SideEffectTarget::Unknown,
    }
}

/// Translate a registry [`RegistryConnectionSide`] into the compiler's
/// [`ConnectionSide`] (1:1).
fn lift_registry_side(s: RegistryConnectionSide) -> ConnectionSide {
    match s {
        RegistryConnectionSide::None => ConnectionSide::None,
        RegistryConnectionSide::Client => ConnectionSide::Client,
        RegistryConnectionSide::Server => ConnectionSide::Server,
        RegistryConnectionSide::Both => ConnectionSide::Both,
        RegistryConnectionSide::Global => ConnectionSide::Global,
    }
}

/// Lift one registry [`RegistrySideEffect`] hint into a compiler
/// [`SideEffect`], stamping the active `dialect` onto it when a dialect
/// is supplied. The registry hint carries only
/// `target`/`reads`/`writes`/`connection_side`; the remaining fields
/// stay at their defaults.
fn lift_registry_effect(e: RegistrySideEffect, dialect: Option<&str>) -> SideEffect {
    let mut effect = SideEffect::new(lift_registry_target(e.target), e.reads, e.writes);
    effect.connection_side = lift_registry_side(e.connection_side);
    effect.dialect = dialect.map(str::to_owned);
    effect
}

/// Whether `spec` is available in the (already `irules`→`f5-irules`
/// normalised) `filter` dialect, gating on `spec.supports_dialect`.
/// When no dialect is requested every spec qualifies; when the
/// dialect string fails to parse to a known [`DialectSet`] (e.g. the
/// bare `"tcl"` family alias) only universal (`dialects = None`) specs
/// qualify — an unknown name intersects no explicit dialect set.
fn spec_in_dialect(spec: &CommandSpec, filter: Option<&str>) -> bool {
    match filter {
        None => true,
        Some(name) => match DialectSet::parse(name) {
            Some(set) => spec.supports_dialect(set),
            None => spec.dialects.is_none(),
        },
    }
}

/// Resolve the dialect-gated structured side-effect hints for a
/// command invocation.
///
/// Iterates the command's specs newest-first, skipping specs that don't
/// support the active
/// dialect. Subcommand-level hints take precedence over command-level
/// hints. Returns `None` when no qualifying spec declares any hint
/// (so the caller falls back to the conservative UNKNOWN write).
fn dialect_side_effect_hints(
    registry: &CommandRegistry,
    command: &str,
    subcommand: Option<&str>,
    dialect: Option<&str>,
) -> Option<Vec<SideEffect>> {
    // lookup_dialect is "f5-irules" when dialect == "irules", else dialect.
    let filter = match dialect {
        Some("irules") => Some("f5-irules"),
        other => other,
    };

    for spec in registry.specs(command).iter().rev() {
        if !spec_in_dialect(spec, filter) {
            continue;
        }
        if let Some(sub_name) = subcommand
            && let Some(sub) = spec.resolve_subcommand(sub_name)
            && !sub.side_effects.is_empty()
        {
            return Some(
                sub.side_effects
                    .iter()
                    .map(|e| lift_registry_effect(*e, dialect))
                    .collect(),
            );
        }
        if !spec.side_effects.is_empty() {
            return Some(
                spec.side_effects
                    .iter()
                    .map(|e| lift_registry_effect(*e, dialect))
                    .collect(),
            );
        }
    }
    None
}

fn fallback_unknown_write(dialect: Option<&str>) -> CommandSideEffects {
    CommandSideEffects {
        effects: vec![SideEffect::new(SideEffectTarget::Unknown, true, true)],
        dialect: dialect.map(str::to_owned),
        ..CommandSideEffects::default()
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

    #[test]
    fn http_uri_family_reads_http_state_region() {
        // Regression: `HTTP::uri` / `HTTP::path` / `HTTP::query` getters carry a
        // read-only `HttpUri` side effect, so they classify into the
        // `HTTP_STATE` *read* region. Previously their specs had no
        // `side_effects`, so callgraph/dataflow reported `NONE` instead of
        // `HTTP_STATE`.
        let mut reg = tcl_registry::CommandRegistry::build_default();
        reg.load_dialect(tcl_dialect::DialectSet::IRULES);
        for cmd in ["HTTP::uri", "HTTP::path", "HTTP::query"] {
            let ci = classify_side_effects(&reg, cmd, &[], None, None);
            let (reads, writes) = ci.to_effect_regions();
            assert!(
                reads.contains(EffectRegion::HTTP_STATE),
                "{cmd}: reads should include HTTP_STATE, got {reads:?}"
            );
            assert!(
                !writes.contains(EffectRegion::HTTP_STATE),
                "{cmd}: getter form must not write HTTP_STATE, got {writes:?}"
            );
        }
    }

    // -- SideEffect / CommandSideEffects --

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

    // -- scope + storage-type inference --

    #[test]
    fn scope_proc_local_for_bare_names() {
        let (s, ns) = scope_from_varname("x");
        assert_eq!(s, StorageScope::ProcLocal);
        assert!(ns.is_none());
    }

    #[test]
    fn scope_static_for_irules_prefix() {
        let (s, ns) = scope_from_varname("static::counter");
        assert_eq!(s, StorageScope::Static);
        assert!(ns.is_none());
    }

    #[test]
    fn scope_global_for_root_qualified() {
        let (s, ns) = scope_from_varname("::var");
        assert_eq!(s, StorageScope::Global);
        assert_eq!(ns.as_deref(), Some("::"));
    }

    #[test]
    fn scope_namespace_for_qualified_with_segments() {
        let (s, ns) = scope_from_varname("::foo::bar::var");
        assert_eq!(s, StorageScope::Namespace);
        assert_eq!(ns.as_deref(), Some("::foo::bar"));
    }

    #[test]
    fn scope_namespace_single_segment() {
        let (s, ns) = scope_from_varname("::foo::var");
        assert_eq!(s, StorageScope::Namespace);
        assert_eq!(ns.as_deref(), Some("::foo"));
    }

    #[test]
    fn storage_type_default_scalar_for_unknown_command() {
        let registry = CommandRegistry::build_default();
        let args = vec![];
        assert_eq!(
            storage_type_for_command(&registry, "nonexistentcmd", &args),
            StorageType::Scalar
        );
    }

    #[test]
    fn storage_type_registry_backed_list_dict_array() {
        let registry = CommandRegistry::build_default();
        let args = vec![];
        // lappend / dict / array are declared with inferred storage
        // types in the registry and should be mapped through.
        assert_eq!(
            storage_type_for_command(&registry, "lappend", &args),
            StorageType::List
        );
        assert_eq!(
            storage_type_for_command(&registry, "dict", &args),
            StorageType::Dict
        );
        assert_eq!(
            storage_type_for_command(&registry, "array", &args),
            StorageType::Array
        );
    }

    // -- classify_side_effects --

    #[test]
    fn classify_pure_expr() {
        let registry = CommandRegistry::build_default();
        let cse = classify_side_effects(&registry, "expr", &["{1 + 2}".into()], None, None);
        assert!(cse.pure);
        assert!(cse.deterministic);
        assert!(cse.effects.is_empty());
    }

    #[test]
    fn classify_set_write() {
        let registry = CommandRegistry::build_default();
        let cse = classify_side_effects(&registry, "set", &["x".into(), "42".into()], None, None);
        assert!(!cse.pure);
        assert!(cse.writes_any());
        assert!(!cse.reads_any());
        let eff = &cse.effects[0];
        assert_eq!(eff.target, SideEffectTarget::Variable);
        assert_eq!(eff.scope, StorageScope::ProcLocal);
        assert_eq!(eff.key.as_deref(), Some("x"));
    }

    #[test]
    fn classify_set_read_when_no_value() {
        let registry = CommandRegistry::build_default();
        let cse = classify_side_effects(&registry, "set", &["x".into()], None, None);
        assert!(cse.reads_any());
        assert!(!cse.writes_any());
    }

    #[test]
    fn classify_incr_is_read_modify_write() {
        let registry = CommandRegistry::build_default();
        let cse = classify_side_effects(&registry, "incr", &["i".into()], None, None);
        // incr always reads AND writes (READS_BEFORE_WRITE trait).
        assert!(cse.reads_any());
        assert!(cse.writes_any());
    }

    #[test]
    fn classify_unset_keys_the_variable_not_the_option() {
        // RUST_ISSUE_078: `unset x` destroys `x` — it is a WRITE (destroy) of
        // `x`, never a read, and its key is `x` (not modelled as an assignment).
        let registry = CommandRegistry::build_default();
        let cse = classify_side_effects(&registry, "unset", &["x".into()], None, None);
        assert!(cse.writes_any());
        assert!(!cse.reads_any(), "unset does not read its target");
        assert_eq!(cse.effects.len(), 1);
        assert_eq!(cse.effects[0].target, SideEffectTarget::Variable);
        assert_eq!(cse.effects[0].key.as_deref(), Some("x"));
    }

    #[test]
    fn classify_unset_skips_leading_options() {
        // RUST_ISSUE_078: `unset -nocomplain x` keys `x`, NOT `-nocomplain`.
        let registry = CommandRegistry::build_default();
        let cse = classify_side_effects(
            &registry,
            "unset",
            &["-nocomplain".into(), "x".into()],
            None,
            None,
        );
        let keys: Vec<&str> = cse
            .effects
            .iter()
            .filter_map(|e| e.key.as_deref())
            .collect();
        assert_eq!(keys, vec!["x"], "must key the variable, not the option");
    }

    #[test]
    fn classify_unset_keys_every_name() {
        // RUST_ISSUE_078: `unset a b c` destroys all three — not just the first.
        let registry = CommandRegistry::build_default();
        let cse = classify_side_effects(
            &registry,
            "unset",
            &["a".into(), "b".into(), "c".into()],
            None,
            None,
        );
        let mut keys: Vec<&str> = cse
            .effects
            .iter()
            .filter_map(|e| e.key.as_deref())
            .collect();
        keys.sort_unstable();
        assert_eq!(keys, vec!["a", "b", "c"]);
        assert!(cse.writes_any());
    }

    #[test]
    fn classify_global_scope_variable() {
        let registry = CommandRegistry::build_default();
        let cse = classify_side_effects(
            &registry,
            "set",
            &["::foo::bar".into(), "v".into()],
            None,
            None,
        );
        let eff = &cse.effects[0];
        assert_eq!(eff.scope, StorageScope::Namespace);
        assert_eq!(eff.namespace.as_deref(), Some("::foo"));
    }

    #[test]
    fn classify_irules_static_uses_global_connection_side() {
        let registry = CommandRegistry::build_default();
        let cse = classify_side_effects(
            &registry,
            "set",
            &["static::counter".into(), "0".into()],
            Some("irules"),
            None,
        );
        let eff = &cse.effects[0];
        assert_eq!(eff.scope, StorageScope::Static);
        assert_eq!(eff.connection_side, ConnectionSide::Global);
    }

    #[test]
    fn classify_irules_proc_local_uses_both_side() {
        let registry = CommandRegistry::build_default();
        let cse = classify_side_effects(
            &registry,
            "set",
            &["local".into(), "0".into()],
            Some("irules"),
            None,
        );
        let eff = &cse.effects[0];
        assert_eq!(eff.connection_side, ConnectionSide::Both);
    }

    #[test]
    fn classify_eval_is_dynamic_barrier() {
        let registry = CommandRegistry::build_default();
        let cse = classify_side_effects(&registry, "eval", &["script".into()], None, None);
        assert!(cse.dynamic_barrier);
        let (_, w) = cse.to_effect_regions();
        assert!(w.contains(EffectRegion::UNKNOWN_STATE));
    }

    #[test]
    fn classify_proc_is_procedure_definition() {
        let registry = CommandRegistry::build_default();
        let cse = classify_side_effects(
            &registry,
            "proc",
            &["foo".into(), "{}".into(), "{}".into()],
            None,
            None,
        );
        let eff = &cse.effects[0];
        assert_eq!(eff.target, SideEffectTarget::ProcDefinition);
        assert!(eff.writes);
        assert_eq!(eff.scope, StorageScope::Namespace);
    }

    #[test]
    fn classify_unknown_command_is_conservative() {
        let registry = CommandRegistry::build_default();
        let cse = classify_side_effects(&registry, "__nonexistent", &[], None, None);
        assert!(cse.reads_any());
        assert!(cse.writes_any());
        assert_eq!(cse.effects[0].target, SideEffectTarget::Unknown);
    }

    #[test]
    fn classify_puts_consults_structured_side_effects() {
        // `puts` carries no purity/assign/proc-def trait, so it used to
        // fall through to the conservative UNKNOWN write. It now resolves
        // via the spec's structured `side_effects` (a `FileIo` write) —
        // impure (`writes_any`) but region-free: `classify_side_effects`
        // for `puts` yields pure=False, regions=(NONE, NONE).
        let registry = CommandRegistry::build_default();
        let cse = classify_side_effects(&registry, "puts", &["hi".into()], None, None);
        assert!(!cse.pure);
        assert!(cse.writes_any());
        assert!(!cse.reads_any());
        assert_eq!(cse.effects[0].target, SideEffectTarget::FileIo);
        // The whole point: no tracked effect region (NONE, not UNKNOWN_STATE).
        let (r, w) = cse.to_effect_regions();
        assert_eq!(r, EffectRegion::NONE);
        assert_eq!(w, EffectRegion::NONE);
    }

    #[test]
    fn pure_command_mutator_subcommand_downgrades_purity() {
        // `HTTP::header` carries `Traits::PURE` + a structured `HttpHeader`
        // read+write hint + `mutator` subcommands. The getter stays pure with
        // a read-only effect; a mutator subcommand (`insert`) downgrades to
        // impure with a read+write effect. On the pure branch,
        // `HTTP::header` → (HTTP_STATE, NONE) pure; `insert` → (HTTP_STATE,
        // HTTP_STATE) impure.
        let mut registry = CommandRegistry::build_default();
        registry.load_dialect(DialectSet::IRULES);

        let getter = classify_side_effects(
            &registry,
            "HTTP::header",
            &["value".into(), "Host".into()],
            Some("f5-irules"),
            None,
        );
        assert!(getter.pure, "HTTP::header getter should stay pure");
        assert_eq!(
            getter.to_effect_regions(),
            (EffectRegion::HTTP_STATE, EffectRegion::NONE),
            "pure getter reads header state, no write",
        );

        let insert = classify_side_effects(
            &registry,
            "HTTP::header",
            &["insert".into(), "X-Foo".into(), "y".into()],
            Some("f5-irules"),
            None,
        );
        assert!(
            !insert.pure,
            "HTTP::header insert (mutator subcommand) must not be pure",
        );
        assert_eq!(
            insert.to_effect_regions(),
            (EffectRegion::HTTP_STATE, EffectRegion::HTTP_STATE),
            "header mutation reads + writes header state",
        );
    }

    #[test]
    fn classify_hints_are_dialect_gated() {
        // `log` is irules-only. Under a Tcl dialect its irules `LogIo`
        // hint must NOT apply — the spec doesn't support `tcl8.6`, so the
        // classifier falls back to the conservative UNKNOWN write
        // (dialect-mismatched specs are skipped). With no dialect requested
        // the hint applies.
        let mut registry = CommandRegistry::build_default();
        registry.load_dialect(DialectSet::IRULES);
        // Dialect-agnostic (interproc path): LogIo, region-free, impure.
        let agnostic = classify_side_effects(&registry, "log", &["hi".into()], None, None);
        assert!(!agnostic.pure);
        assert_eq!(agnostic.effects[0].target, SideEffectTarget::LogIo);
        assert_eq!(
            agnostic.to_effect_regions(),
            (EffectRegion::NONE, EffectRegion::NONE)
        );
        // Under a Tcl dialect the irules spec is gated out → UNKNOWN write.
        let tcl = classify_side_effects(&registry, "log", &["hi".into()], Some("tcl8.6"), None);
        assert!(!tcl.pure);
        assert_eq!(tcl.effects[0].target, SideEffectTarget::Unknown);
        assert_eq!(
            tcl.to_effect_regions(),
            (EffectRegion::UNKNOWN_STATE, EffectRegion::UNKNOWN_STATE)
        );
    }

    #[test]
    fn callee_summary_http_read_emits_http_header_effect() {
        let registry = CommandRegistry::build_default();
        let summary = CalleeSummary {
            effect_reads: EffectRegion::HTTP_STATE,
            effect_writes: EffectRegion::NONE,
            pure: false,
        };
        let cse = classify_side_effects(&registry, "doesnt_matter", &[], None, Some(&summary));
        assert!(cse.affects_target(SideEffectTarget::HttpHeader));
        assert!(cse.reads_target(SideEffectTarget::HttpHeader));
        assert!(!cse.writes_target(SideEffectTarget::HttpHeader));
    }

    #[test]
    fn callee_summary_response_commit_sets_write() {
        let registry = CommandRegistry::build_default();
        let summary = CalleeSummary {
            effect_reads: EffectRegion::NONE,
            effect_writes: EffectRegion::RESPONSE_LIFECYCLE | EffectRegion::HTTP_STATE,
            pure: false,
        };
        let cse = classify_side_effects(&registry, "doesnt_matter", &[], None, Some(&summary));
        assert!(cse.writes_target(SideEffectTarget::ResponseCommit));
    }

    #[test]
    fn callee_summary_pure_bypass_registry() {
        let registry = CommandRegistry::build_default();
        let summary = CalleeSummary {
            effect_reads: EffectRegion::NONE,
            effect_writes: EffectRegion::NONE,
            pure: true,
        };
        let cse = classify_side_effects(&registry, "doesnt_matter", &[], None, Some(&summary));
        assert!(cse.pure);
        assert!(cse.deterministic);
        assert!(cse.effects.is_empty());
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
