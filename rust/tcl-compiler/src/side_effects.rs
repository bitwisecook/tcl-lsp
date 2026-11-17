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

use tcl_registry::prelude::StorageType as RegistryStorageType;
use tcl_registry::side_effects::{
    ConnectionSide as RegistryConnectionSide, SideEffect as RegistrySideEffect,
    SideEffectTarget as RegistryTarget,
};
use tcl_registry::{CommandRegistry, Traits};

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
    /// iRules root static namespace cell, persistent within one TMM worker.
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
    /// Event-handler flow control: abandons the remaining script in the
    /// current event invocation, without touching the connection, other
    /// events, or other rules (iRules `return` inside a `when` body).
    /// Distinct from [`Self::ConnectionControl`], which terminates the
    /// connection rather than just the current handler.
    EventControl,

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
        SideEffectTarget::Variable => match scope {
            StorageScope::Global | StorageScope::Namespace | StorageScope::Static => {
                EffectRegion::GLOBAL_STATE
            }
            StorageScope::ProcLocal | StorageScope::Event | StorageScope::Connection => {
                EffectRegion::NONE
            }
            _ => EffectRegion::UNKNOWN_STATE,
        },
        // External I/O does not mutate compiler-tracked in-memory state.
        SideEffectTarget::FileIo | SideEffectTarget::NetworkIo | SideEffectTarget::LogIo => {
            EffectRegion::NONE
        }
        _ => EffectRegion::UNKNOWN_STATE,
    }
}

// Scope / storage-type inference helpers

/// A compatibility query for Tcl names in a root procedure context.
///
/// Host storage policy is applied only by the contextual classifier. A name
/// `static::x` is an ordinary Tcl namespace reference in this query.
#[must_use]
pub fn scope_from_varname(name: &str) -> (StorageScope, Option<String>) {
    if crate::naming::is_qualified(name.as_bytes()) {
        let rooted = crate::naming::qualify("::", name);
        let (namespace, _) = crate::naming::key_holder_and_tail(&rooted);
        let scope = if namespace == "::" {
            StorageScope::Global
        } else {
            StorageScope::Namespace
        };
        (scope, Some(namespace.to_owned()))
    } else {
        (StorageScope::ProcLocal, None)
    }
}

fn variable_effect_context<'a>(
    context: &'a crate::var_resolve::ResolveContext,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> std::borrow::Cow<'a, crate::var_resolve::ResolveContext> {
    // An explicit metadata profile supplies lookup policy only when the
    // retained point has not already selected its execution dialect.
    if context.invocation_dialect.is_none()
        && let Some(profile) = dialect
    {
        let mut supplied = context.clone();
        supplied.invocation_dialect = Some(tcl_registry::InvocationDialect::of_profile(profile));
        std::borrow::Cow::Owned(supplied)
    } else {
        std::borrow::Cow::Borrowed(context)
    }
}

fn resolved_variable_scope(
    name: &str,
    registry: &CommandRegistry,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    context: &crate::var_resolve::ResolveContext,
) -> (StorageScope, Option<String>, String) {
    let context = variable_effect_context(context, dialect);
    let context = context.as_ref();
    let Some((place, canonical)) = crate::var_resolve::canonical_namespace_variable_place(
        name, context, registry,
    )
    .and_then(|place| crate::var_resolve::canonical_place_key(&place).map(|key| (place, key))) else {
        let place = crate::var_resolve::resolve_place(name, context, false, registry);
        let scope = if place.dynamic {
            StorageScope::Unknown
        } else if place.ns == crate::place::LOCAL_NS {
            StorageScope::ProcLocal
        } else {
            StorageScope::Unknown
        };
        return (scope, None, name.to_owned());
    };
    let (scope, namespace) = canonical_namespace_storage_scope(&place, &canonical, dialect);
    (scope, namespace, canonical.compatibility_name())
}

/// Classify a resolved namespace place; display supplies neither cell nor worker ownership.
#[must_use]
pub(crate) fn namespace_storage_scope_for_place(
    place: &crate::place::Place,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> StorageScope {
    let Some(canonical) = crate::var_resolve::canonical_place_key(place) else {
        return StorageScope::Unknown;
    };
    canonical_namespace_storage_scope(place, &canonical, dialect).0
}

fn canonical_namespace_storage_scope(
    place: &crate::place::Place,
    canonical: &crate::var_resolve::VariableCellKey,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> (StorageScope, Option<String>) {
    let namespace = if let Some(identity) = canonical.namespace_identity() {
        let root = identity
            .exact_native_path()
            .is_some_and(tcl_core_types::ByteNamespacePath::is_root);
        let worker = place.cell.as_ref().is_some_and(|cell| {
            cell.storage_domain == Some(tcl_registry::f5::VariableStorageDomain::WorkerNamespace)
        });
        Some((root, identity.display(), worker))
    } else {
        match canonical.root() {
            crate::var_resolve::VariableCellKey::Authored(_) => {
                match place.cell.as_ref().map(|cell| &cell.owner) {
                    Some(crate::place::CellOwner::Namespace(namespace)) => {
                        // This authored classification supplies no native TMM owner.
                        let worker = crate::taint::is_irules_dialect(dialect)
                            && tcl_registry::f5::namespace_storage_domain(
                                tcl_registry::f5::BigIpExecutionContext::TmmIRule,
                                namespace,
                            ) == tcl_registry::f5::VariableStorageDomain::WorkerNamespace;
                        Some((namespace == "::", Some(namespace.clone()), worker))
                    }
                    None => {
                        // A resolved legacy symbolic place has no native cell.
                        let namespace = &place.ns;
                        let worker = crate::taint::is_irules_dialect(dialect)
                            && tcl_registry::f5::namespace_storage_domain(
                                tcl_registry::f5::BigIpExecutionContext::TmmIRule,
                                namespace,
                            ) == tcl_registry::f5::VariableStorageDomain::WorkerNamespace;
                        Some((namespace == "::", Some(namespace.clone()), worker))
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    };
    let Some((root, namespace, worker)) = namespace else {
        return (StorageScope::Unknown, None);
    };
    let scope = if worker {
        StorageScope::Static
    } else if root {
        StorageScope::Global
    } else {
        StorageScope::Namespace
    };
    (scope, namespace)
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
    /// For Tcl variables this is optional presentation of the retained
    /// namespace path (`"::foo::bar"`), never a namespace identity.
    /// For F5 commands this is the protocol prefix (`"HTTP"`,
    /// `"SSL"`). `None` when not applicable or not determinable.
    pub namespace: Option<String>,
    /// Dialect this effect applies to (`"irules"`, `"tcl"`, …).
    ///
    /// `None` means the effect is dialect-independent.
    pub dialect: Option<String>,
    /// Optional key identifying the specific target.
    ///
    /// For variables: a compatibility label, not a cell-equality proof.
    /// For `table` / `session`:
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
/// facts consulted today are derived purity, `PURE_EVALUATION`,
/// `EVALUATES_CODE`, `CREATES_BARRIER`, `DEFINES_PROCEDURE`, and
/// `DESTROYS_VARIABLE`, plus the spec-level `assigns_variable_at`
/// field, and the spec's structured `side_effects`. A pure command
/// with a mutator subcommand
/// (`HTTP::header insert …`) is downgraded to impure and a still-pure
/// command surfaces its hint read-only. Full arity-based form
/// resolution and the per-subcommand protocol-namespace write modelling
/// are not implemented.
#[must_use]
pub fn classify_side_effects(
    registry: &CommandRegistry,
    command: &str,
    args: &[String],
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    callee_summary: Option<&CalleeSummary>,
) -> CommandSideEffects {
    let mut context = crate::var_resolve::ResolveContext::default();
    if crate::taint::is_irules_dialect(dialect) {
        context.known_namespaces.extend(
            tcl_registry::f5::runtime_namespaces(tcl_registry::f5::BigIpExecutionContext::TmmIRule)
                .iter()
                .map(|namespace| (*namespace).to_owned()),
        );
    }
    classify_side_effects_with_context(registry, command, args, dialect, callee_summary, &context)
}

/// Classify effects using the variable bindings in force at this invocation.
#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "sequential registry-trait dispatch; splitting hurts readability"
)]
pub fn classify_side_effects_with_context(
    registry: &CommandRegistry,
    command: &str,
    args: &[String],
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    callee_summary: Option<&CalleeSummary>,
    context: &crate::var_resolve::ResolveContext,
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
            dialect: dialect.map(|profile| profile.name.to_owned()),
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
            dialect: dialect.map(|profile| profile.name.to_owned()),
            ..CommandSideEffects::default()
        };
    }

    // Commands pure at the command level. Two refinements on the pure
    // branch:
    //   1. A *mutator* subcommand downgrades purity — e.g. `HTTP::header`
    //      is pure but `HTTP::header insert …` mutates header state, so
    //      it must fall through to the structured-hints classification
    //      below (which yields the read+write effect). Without this a
    //      header mutation reads as pure with no effects.
    //   2. A still-pure command may carry a *read-only* hint — `HTTP::header`
    //      (getter) reads header state. Return the structured hint
    //      with `reads=true, writes=false` by forcing the
    //      spec's hints read-only.
    // With no dialect, the registry's own release answers.
    let query = dialect
        .or_else(|| registry.profile())
        .map(tcl_dialect::DialectProfile::surface_query);
    if spec.is_pure_at(query.as_ref()) {
        let effective_sub = args.first().map(String::as_str);
        let sub_is_mutator = effective_sub
            .and_then(|s| spec.resolve_subcommand(s))
            .is_some_and(|sc| sc.mutator);
        if !sub_is_mutator {
            let effects = dialect_side_effect_hints(registry, command, effective_sub, dialect)
                .map_or_default(|hints| {
                    hints
                        .into_iter()
                        .map(|mut e| {
                            e.reads = true;
                            e.writes = false;
                            e
                        })
                        .collect::<Vec<_>>()
                });
            return CommandSideEffects {
                pure: true,
                effects,
                deterministic: true,
                dialect: dialect.map(|profile| profile.name.to_owned()),
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
        && sub.is_pure_at(spec, query.as_ref())
        && !sub.mutator
    {
        return CommandSideEffects {
            pure: true,
            deterministic: true,
            dialect: dialect.map(|profile| profile.name.to_owned()),
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
                context,
            );
        }
        return CommandSideEffects {
            effects: vec![SideEffect::new(SideEffectTarget::Variable, true, true)],
            dialect: dialect.map(|profile| profile.name.to_owned()),
            ..CommandSideEffects::default()
        };
    }

    // Procedure definition (`proc`).
    if spec.traits.contains(Traits::DEFINES_PROCEDURE) {
        let mut e = SideEffect::new(SideEffectTarget::ProcDefinition, false, true);
        e.scope = StorageScope::Namespace;
        e.dialect = dialect.map(|profile| profile.name.to_owned());
        return CommandSideEffects {
            effects: vec![e],
            dialect: dialect.map(|profile| profile.name.to_owned()),
            ..CommandSideEffects::default()
        };
    }

    // Destroys a variable (`unset`).
    if spec.traits.contains(Traits::DESTROYS_VARIABLE) {
        // The destroyed variables are exactly the `VarWrite`-role args, resolved
        // through the registry's arg-role resolver — which skips the leading
        // options (`-nocomplain`, `--`) and yields *every* trailing name, not
        // just the first. So `unset -nocomplain x` keys `x` (not `-nocomplain`),
        // and `unset a b` keys both `a` and `b`.
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let targets =
            registry.arg_indices_for_role(command, &arg_refs, tcl_registry::ArgRole::VarWrite);
        let make_effect = |name: Option<&str>| {
            let mut e = SideEffect::new(SideEffectTarget::Variable, false, true);
            if let Some(name) = name {
                let (scope, namespace, key) =
                    resolved_variable_scope(name, registry, dialect, context);
                e.scope = scope;
                e.namespace = namespace;
                e.key = Some(key);
            }
            e.dialect = dialect.map(|profile| profile.name.to_owned());
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
            dialect: dialect.map(|profile| profile.name.to_owned()),
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
            dialect: dialect.map(|profile| profile.name.to_owned()),
            ..CommandSideEffects::default()
        };
    }

    // Fallback: conservative unknown read+write.
    fallback_unknown_write(dialect)
}

/// Classify the side effects of a command invocation against one
/// document's command surface — the catalogue plus the document's own
/// `# tcl-lsp: stub` declarations.
///
/// A name the document declares answers from its declaration (nearest
/// wins): the facts a stub's flags state, read in the order
/// [`classify_side_effects`] reads a catalogue spec's — see
/// [`classify_declared`]. Every other name, and every call with a
/// `callee_summary`, is [`classify_side_effects`] over the catalogue.
#[must_use]
pub fn classify_side_effects_in(
    surface: &tcl_registry::model::DocumentCommandSurface<'_>,
    command: &str,
    args: &[String],
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    callee_summary: Option<&CalleeSummary>,
) -> CommandSideEffects {
    if callee_summary.is_none() && surface.declares(command) {
        return classify_declared(surface, command, dialect);
    }
    classify_side_effects(surface.commands(), command, args, dialect, callee_summary)
}

/// The side effects of a command the document declares, read off the facts
/// its declaration states in the order [`classify_side_effects`] reads a
/// catalogue spec's: an eval-like barrier trait first, then purity (its
/// stated effects surfaced read-only, as a pure catalogue command's hints
/// are), then the stated effects, and the conservative unknown write for a
/// declaration that states none — so a stub with no flags classifies exactly
/// as an undeclared command does.
///
/// The traits and effects come from
/// [`DocumentCommandSurface`](tcl_registry::model::DocumentCommandSurface),
/// so a declaration that redeclares a shipped command keeps that command's
/// security traits and side effects beneath its own (invariant I6).
fn classify_declared(
    surface: &tcl_registry::model::DocumentCommandSurface<'_>,
    command: &str,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> CommandSideEffects {
    let traits = surface.traits(command).unwrap_or_default();
    if traits.intersects(Traits::EVALUATES_CODE | Traits::CREATES_BARRIER) {
        return CommandSideEffects {
            effects: vec![SideEffect::new(SideEffectTarget::Unknown, true, true)],
            dynamic_barrier: true,
            dialect: dialect.map(|profile| profile.name.to_owned()),
            ..CommandSideEffects::default()
        };
    }
    let stated = surface.side_effects(command).unwrap_or_default();
    let query = dialect.map(tcl_dialect::DialectProfile::surface_query);
    if surface.is_pure_at(command, query.as_ref()) {
        return CommandSideEffects {
            effects: stated
                .iter()
                .map(|effect| {
                    let mut effect = lift_registry_effect(*effect, dialect);
                    effect.reads = true;
                    effect.writes = false;
                    effect
                })
                .collect(),
            pure: true,
            deterministic: true,
            dialect: dialect.map(|profile| profile.name.to_owned()),
            ..CommandSideEffects::default()
        };
    }
    if stated.is_empty() {
        return fallback_unknown_write(dialect);
    }
    CommandSideEffects {
        effects: stated
            .iter()
            .map(|effect| lift_registry_effect(*effect, dialect))
            .collect(),
        dialect: dialect.map(|profile| profile.name.to_owned()),
        ..CommandSideEffects::default()
    }
}

/// Translate an interprocedural [`CalleeSummary`] into a
/// [`CommandSideEffects`].
fn classify_from_callee_summary(
    summary: &CalleeSummary,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
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
        dialect: dialect.map(|profile| profile.name.to_owned()),
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
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    context: &crate::var_resolve::ResolveContext,
) -> CommandSideEffects {
    let context = variable_effect_context(context, dialect);
    let context = context.as_ref();
    let varname = &args[idx];
    let (scope, ns, key) = resolved_variable_scope(varname, registry, dialect, context);
    let st = storage_type_for_command(registry, command, args);
    let rmw = traits.contains(Traits::READS_BEFORE_WRITE);

    // When there is only the variable name and no value, it's a read
    // (`set x`); otherwise it's a write. Read-modify-write commands
    // always write even with one arg.
    let is_write = args.len() > idx + 1 || rmw;
    let is_read = rmw || args.len() == idx + 1;

    let is_irules = crate::taint::is_irules_dialect(dialect);
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
    effect.dialect = dialect.map(|profile| profile.name.to_owned());
    effect.key = Some(key);

    let mut effects = vec![effect];

    // Only the resolved root namespace cell can carry the registry's extra
    // interpreter effect. A source alias retains that owner; a same-named
    // procedure local or named namespace cell does not.
    if is_write {
        let place = crate::var_resolve::resolve_place(varname, context, false, registry);
        if let Some(target) = crate::var_resolve::root_namespace_variable_simple_name(&place)
            .and_then(|simple| {
                tcl_registry::special_vars::special_var_write_effect(
                    simple,
                    Some(tcl_registry::special_vars::surface_query_for_profile(
                        dialect,
                    )),
                )
            })
        {
            let mut extra = SideEffect::new(lift_registry_target(target), false, true);
            extra.dialect = dialect.map(|profile| profile.name.to_owned());
            effects.push(extra);
        }
    }

    CommandSideEffects {
        effects,
        dialect: dialect.map(|profile| profile.name.to_owned()),
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
        R::EventControl => SideEffectTarget::EventControl,
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
fn lift_registry_effect(
    e: RegistrySideEffect,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> SideEffect {
    let mut effect = SideEffect::new(lift_registry_target(e.target), e.reads, e.writes);
    effect.connection_side = lift_registry_side(e.connection_side);
    effect.dialect = dialect.map(|profile| profile.name.to_owned());
    effect
}

/// Coarse regions of already selected normal-handler effects. This May
/// projection establishes neither successful entry nor purity and does not
/// replace the invocation's unknown or callback obligations.
pub(crate) fn normal_handler_effect_regions(
    footprint: &tcl_registry::world_effect::EffectFootprint,
) -> (EffectRegion, EffectRegion) {
    let mut reads = EffectRegion::NONE;
    let mut writes = EffectRegion::NONE;
    for effect in &footprint.legacy().side_effects {
        let region = target_to_region(lift_registry_target(effect.target), StorageScope::Unknown);
        if effect.reads {
            reads |= region;
        }
        if effect.writes {
            writes |= region;
        }
    }
    (reads, writes)
}

/// Resolve the dialect-gated structured side-effect hints for a
/// command invocation, through the registry model's one hint-selection
/// primitive (centralisation C7 —
/// [`tcl_registry::model::side_effect_hints_in_context`]): specs
/// newest-first, filtered by the resolved context's availability,
/// subcommand-level hints before command-level ones. `None` when no
/// qualifying spec declares any hint (so the caller falls back to the
/// conservative UNKNOWN write). A `None` dialect selects with no
/// availability filter, as before.
fn dialect_side_effect_hints(
    registry: &CommandRegistry,
    command: &str,
    subcommand: Option<&str>,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> Option<Vec<SideEffect>> {
    let generation = dialect.map(crate::environment_ingress::context_for_profile);
    let hints = tcl_registry::model::side_effect_hints_in_context(
        registry,
        generation
            .as_deref()
            .map(tcl_registry::model::ContextRegistry::context),
        command,
        subcommand,
    )?;
    Some(
        hints
            .iter()
            .map(|e| lift_registry_effect(*e, dialect))
            .collect(),
    )
}

fn fallback_unknown_write(
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> CommandSideEffects {
    CommandSideEffects {
        effects: vec![SideEffect::new(SideEffectTarget::Unknown, true, true)],
        dialect: dialect.map(|profile| profile.name.to_owned()),
        ..CommandSideEffects::default()
    }
}

#[cfg(test)]
mod tests;
