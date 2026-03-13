"""Command side-effect structure and classification.

This module provides a rich, structured model for understanding what a
command *does* to its environment — what it reads, what it writes, where
the data lives, what shape it has, and which dialect/connection context
applies.

The model is layered:

1. **Enums** describe the vocabulary:

   - :class:`StorageType` — data shape (scalar, list, dict, array).
   - :class:`StorageScope` — where the data lives (proc-local, namespace,
     global, F5 connection, static, session table, etc.).
   - :class:`ConnectionSide` — F5 proxy context (client, server, both,
     global/none).
   - :class:`SideEffectTarget` — the external resource affected (variable,
     HTTP header, file, socket, session table, pool selection, etc.).

2. **Dataclasses** compose those into per-invocation facts:

   - :class:`SideEffect` — one read or write to a specific target.
   - :class:`CommandSideEffects` — the complete side-effect profile for
     a command invocation (possibly multiple targets).

3. **Classification function** :func:`classify_side_effects` resolves
   registry metadata + runtime arguments into a :class:`CommandSideEffects`.

Consumers include the optimiser (kill safety, CSE), the iRules flow
checker (response-commit tracking), the taint engine, and future data-
flow analyses that need to know *what* a command touches rather than
just *whether* it is pure.
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum, IntFlag, auto
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    pass


# ---------------------------------------------------------------------------
# Enums
# ---------------------------------------------------------------------------


class StorageType(Enum):
    """Data shape of the target being read or written."""

    SCALAR = auto()
    """Simple string/integer value (Tcl default representation)."""

    LIST = auto()
    """Tcl list value."""

    DICT = auto()
    """Tcl dict value (key-value pairs)."""

    ARRAY = auto()
    """Tcl array (associative array of variables, not a value type)."""

    UNKNOWN = auto()
    """Shape cannot be determined statically."""


class StorageScope(Enum):
    """Where the data resides at runtime."""

    # --- Tcl-universal scopes ---

    PROC_LOCAL = auto()
    """Local variable inside the current procedure."""

    NAMESPACE = auto()
    """Variable in a named Tcl namespace (``namespace eval``)."""

    GLOBAL = auto()
    """Global namespace variable (``::var`` or via ``global`` command)."""

    UPVAR = auto()
    """Aliased from a caller's frame via ``upvar``/``uplevel``."""

    # --- F5 iRules-specific scopes ---

    EVENT = auto()
    """iRules event-scoped state (stable within a single event handler).

    Cannot change during execution of a single ``when`` block, but may
    differ between events on the same connection.  For example,
    ``HTTP::uri`` is stable within ``HTTP_REQUEST`` but changes on the
    next request over a keepalive connection.  Server-side transport
    properties (``IP::server_addr``) are stable within an event but may
    change if a pool reselection occurs between events.
    """

    CONNECTION = auto()
    """iRules connection-scoped state (lives for the connection lifetime).

    Immutable for the life of the TCP/UDP flow.  Client-side transport
    properties (``IP::client_addr``, ``TCP::client_port``) live here.
    Shared across ``when`` event handlers for the same connection.
    """

    STATIC = auto()
    """iRules ``static::`` variable (system-wide, survives across connections).

    Should only be written in ``RULE_INIT``.
    """

    SESSION_TABLE = auto()
    """F5 session table (``table`` command).  Keyed, with lifetime/timeout."""

    PERSISTENCE = auto()
    """F5 persistence table (``session``/``persist`` commands)."""

    DATA_GROUP = auto()
    """F5 data group / class (read-only at runtime, ``class`` command)."""

    # --- External I/O scopes ---

    FILE_SYSTEM = auto()
    """File on disk (``open``/``puts``/``read``/``close``)."""

    NETWORK_SOCKET = auto()
    """Network socket or connection (``socket``/``connect``/``send``)."""

    LOG_OUTPUT = auto()
    """Logging destination (``log``/``puts stderr``)."""

    UNKNOWN = auto()
    """Scope cannot be determined statically."""


class ConnectionSide(Enum):
    """F5 proxy connection context in which a side effect occurs."""

    CLIENT = auto()
    """Client-side of the proxy (between client and BIG-IP)."""

    SERVER = auto()
    """Server-side of the proxy (between BIG-IP and pool member)."""

    BOTH = auto()
    """Both sides / the proxy itself (e.g. ``table`` which is connection-wide)."""

    GLOBAL = auto()
    """No connection context (e.g. ``RULE_INIT``, ``static::`` variables)."""

    NONE = auto()
    """Not applicable (pure Tcl command, no F5 proxy context)."""


class SideEffectTarget(Enum):
    """The category of external resource that a command touches.

    This is the *what*, not the *where* — combine with :class:`StorageScope`
    and :class:`ConnectionSide` for the full picture.
    """

    # --- Variable mutation ---

    VARIABLE = auto()
    """Tcl variable read or write (``set``, ``incr``, ``append``, etc.)."""

    # --- F5 iRules data stores ---

    SESSION_TABLE = auto()
    """Session table entry (``table set/add/lookup/delete``)."""

    PERSISTENCE_TABLE = auto()
    """Persistence record (``session add/lookup``, ``persist``)."""

    DATA_GROUP = auto()
    """Data group / class lookup (``class match/search/lookup``)."""

    # --- HTTP state ---

    HTTP_HEADER = auto()
    """HTTP header read/write (``HTTP::header``)."""

    HTTP_BODY = auto()
    """HTTP payload/body (``HTTP::payload``, ``HTTP::collect``)."""

    HTTP_STATUS = auto()
    """HTTP status code (``HTTP::status``)."""

    HTTP_URI = auto()
    """HTTP URI components (``HTTP::uri``, ``HTTP::path``, ``HTTP::query``)."""

    HTTP_COOKIE = auto()
    """HTTP cookie (``HTTP::cookie``)."""

    HTTP_METHOD = auto()
    """HTTP method (``HTTP::method``)."""

    HTTP2_STATE = auto()
    """HTTP/2 protocol state (``HTTP2::enable``, ``HTTP2::disable``, ``HTTP2::stream``, etc.)."""

    # --- Response lifecycle ---

    RESPONSE_COMMIT = auto()
    """Commits or sends an HTTP response (``HTTP::respond``, ``HTTP::redirect``)."""

    CONNECTION_CONTROL = auto()
    """Connection-level action: drop, reject, discard, forward."""

    # --- Transport / TLS ---

    TCP_STATE = auto()
    """TCP connection state (``TCP::close``, ``TCP::collect``, etc.)."""

    SSL_STATE = auto()
    """SSL/TLS state (``SSL::disable``, ``SSL::cert``, etc.)."""

    UDP_STATE = auto()
    """UDP datagram state."""

    # --- Load balancing ---

    POOL_SELECTION = auto()
    """Pool or pool member selection (``pool``, ``LB::select``)."""

    NODE_SELECTION = auto()
    """Direct node selection (``node``)."""

    SNAT_SELECTION = auto()
    """SNAT address selection (``snat``, ``snatpool``)."""

    # --- External I/O ---

    FILE_IO = auto()
    """File system read/write (``open``, ``puts``, ``read``, ``close``)."""

    NETWORK_IO = auto()
    """Network socket I/O (``socket``, ``connect``, ``send``)."""

    LOG_IO = auto()
    """Logging output (``log local0.``, ``puts stderr``)."""

    STREAM_PROFILE = auto()
    """Content rewriting via stream profile (``STREAM::``, ``REWRITE::``, ``COMPRESS::``).."""

    # --- DNS ---

    DNS_STATE = auto()
    """DNS message state (``DNS::header``, ``DNS::answer``, ``DNS::question``, etc.)."""

    # --- Classification / DoS ---

    CLASSIFICATION_STATE = auto()
    """Traffic classification state (``CLASSIFY::``, ``CLASSIFICATION::``).."""

    DOSL7_STATE = auto()
    """Layer 7 DoS protection state (``DOSL7::``).."""

    # --- Flow / connection management ---

    FLOW_STATE = auto()
    """Flow object state (``FLOW::create_related``, ``FLOW::idle_timeout``, etc.)."""

    LSN_STATE = auto()
    """Large Scale NAT state (``LSN::address``, ``LSN::persistence``, etc.)."""

    # --- Application protocols ---

    FTP_STATE = auto()
    """FTP protocol state (``FTP::enable``, ``FTP::port``, etc.)."""

    ICAP_STATE = auto()
    """ICAP protocol state (``ICAP::header``, ``ICAP::method``, etc.)."""

    MESSAGE_STATE = auto()
    """Message routing state (``MESSAGE::field``, ``MR::message``, etc.)."""

    # --- Statistics ---

    ISTATS = auto()
    """Internal statistics counters (``ISTATS::set``, ``ISTATS::incr``, etc.)."""

    # --- F5 security / policy ---

    APM_STATE = auto()
    """Access Policy Manager state (``ACCESS::session``, ``ACCESS::policy``, etc.)."""

    ASM_STATE = auto()
    """Application Security Manager state (``ASM::enable``, ``ASM::disable``, etc.)."""

    # --- F5 configuration (iApps) ---

    BIGIP_CONFIG = auto()
    """BIG-IP configuration change (iApps ``iapp::conf``, ``tmsh::`` commands, ``PROFILE::`` reads)."""

    # --- Interpreter state ---

    PROC_DEFINITION = auto()
    """Defines or removes a procedure (``proc``, ``rename``)."""

    NAMESPACE_STATE = auto()
    """Namespace creation/deletion (``namespace eval``, ``namespace delete``)."""

    INTERP_STATE = auto()
    """Interpreter-level state (``interp``, ``package``, ``load``)."""

    # --- Catch-all ---

    UNKNOWN = auto()
    """Target cannot be determined statically."""


# ---------------------------------------------------------------------------
# Coarse effect regions (legacy bridge for GVN / interprocedural)
# ---------------------------------------------------------------------------


class EffectRegion(IntFlag):
    """Abstract mutable regions used for effect invalidation.

    This is a coarse view consumed by GVN and interprocedural analysis.
    The structured :class:`SideEffect` model is authoritative; this enum
    exists solely so those passes can do fast bitwise kill checks.
    """

    NONE = 0
    HTTP_STATE = 1 << 0
    RESPONSE_LIFECYCLE = 1 << 1
    GLOBAL_STATE = 1 << 2
    UNKNOWN_STATE = 1 << 3


def _target_to_region(target: SideEffectTarget, scope: StorageScope) -> EffectRegion:
    """Map a structured target to a coarse :class:`EffectRegion`."""
    match target:
        case (
            SideEffectTarget.HTTP_HEADER
            | SideEffectTarget.HTTP_BODY
            | SideEffectTarget.HTTP_STATUS
            | SideEffectTarget.HTTP_URI
            | SideEffectTarget.HTTP_COOKIE
            | SideEffectTarget.HTTP_METHOD
            | SideEffectTarget.HTTP2_STATE
        ):
            return EffectRegion.HTTP_STATE
        case SideEffectTarget.RESPONSE_COMMIT:
            return EffectRegion.RESPONSE_LIFECYCLE | EffectRegion.HTTP_STATE
        case SideEffectTarget.VARIABLE:
            if scope in (StorageScope.GLOBAL, StorageScope.NAMESPACE):
                return EffectRegion.GLOBAL_STATE
            return EffectRegion.NONE
        case SideEffectTarget.FILE_IO | SideEffectTarget.NETWORK_IO | SideEffectTarget.LOG_IO:
            # External I/O does not mutate compiler-tracked in-memory state.
            return EffectRegion.NONE
        case _:
            return EffectRegion.UNKNOWN_STATE


# ---------------------------------------------------------------------------
# Dataclasses
# ---------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class SideEffect:
    """One discrete read or write produced by a command invocation.

    A single command may produce multiple :class:`SideEffect` instances.
    For example, ``HTTP::header replace Host "example.com"`` produces
    both a read (current header state) and a write (new header value).
    """

    target: SideEffectTarget
    """What category of resource is touched."""

    reads: bool = False
    """Whether this effect includes a read from the target."""

    writes: bool = False
    """Whether this effect includes a write to the target."""

    storage_type: StorageType = StorageType.UNKNOWN
    """Data shape of the target (scalar, list, dict, array)."""

    scope: StorageScope = StorageScope.UNKNOWN
    """Where the data resides (proc-local, global, F5 table, etc.)."""

    connection_side: ConnectionSide = ConnectionSide.NONE
    """F5 proxy context for this effect."""

    namespace: str | None = None
    """Tcl namespace or F5 protocol namespace (e.g. ``"HTTP"``).

    For Tcl variables this is the namespace path (``"::foo::bar"``).
    For F5 commands this is the protocol prefix (``"HTTP"``, ``"SSL"``).
    ``None`` when not applicable or not determinable.
    """

    dialect: str | None = None
    """Dialect this effect applies to (``"irules"``, ``"tcl"``, etc.).

    ``None`` means the effect is dialect-independent.
    """

    key: str | None = None
    """Optional key identifying the specific target.

    For variables: the variable name.
    For table/session: the key expression (if literal).
    For HTTP headers: the header name (if literal).
    ``None`` when the key is dynamic or not applicable.
    """

    subtable: str | None = None
    """F5 session table subtable name, if applicable."""


@dataclass(frozen=True, slots=True)
class CommandSideEffects:
    """Complete side-effect profile for one command invocation.

    This is the top-level result of :func:`classify_side_effects`.
    It wraps zero or more individual :class:`SideEffect` instances plus
    summary flags for quick consumer queries.
    """

    effects: tuple[SideEffect, ...] = ()
    """Individual side effects produced by this invocation."""

    pure: bool = False
    """No observable side effects (reads from immutable state are OK)."""

    deterministic: bool = False
    """Same inputs always produce the same outputs."""

    dynamic_barrier: bool = False
    """Contains ``eval``/``uplevel``/``call`` — effects are unknowable."""

    dialect: str | None = None
    """Dialect context in which this classification was made."""

    @property
    def reads_any(self) -> bool:
        """Whether any effect includes a read."""
        return any(e.reads for e in self.effects)

    @property
    def writes_any(self) -> bool:
        """Whether any effect includes a write."""
        return any(e.writes for e in self.effects)

    @property
    def targets(self) -> frozenset[SideEffectTarget]:
        """Unique set of targets touched by this invocation."""
        return frozenset(e.target for e in self.effects)

    @property
    def write_targets(self) -> frozenset[SideEffectTarget]:
        """Targets that are written to."""
        return frozenset(e.target for e in self.effects if e.writes)

    @property
    def read_targets(self) -> frozenset[SideEffectTarget]:
        """Targets that are read from."""
        return frozenset(e.target for e in self.effects if e.reads)

    @property
    def scopes(self) -> frozenset[StorageScope]:
        """Unique set of storage scopes touched."""
        return frozenset(e.scope for e in self.effects)

    def affects_target(self, target: SideEffectTarget) -> bool:
        """Check if this invocation touches *target* (read or write)."""
        return any(e.target is target for e in self.effects)

    def writes_target(self, target: SideEffectTarget) -> bool:
        """Check if this invocation writes to *target*."""
        return any(e.target is target and e.writes for e in self.effects)

    def reads_target(self, target: SideEffectTarget) -> bool:
        """Check if this invocation reads from *target*."""
        return any(e.target is target and e.reads for e in self.effects)

    def effects_in_scope(self, scope: StorageScope) -> tuple[SideEffect, ...]:
        """Return effects restricted to a specific storage scope."""
        return tuple(e for e in self.effects if e.scope is scope)

    def effects_on_side(self, side: ConnectionSide) -> tuple[SideEffect, ...]:
        """Return effects restricted to a specific connection side."""
        return tuple(e for e in self.effects if e.connection_side is side)

    def to_effect_regions(self) -> tuple[EffectRegion, EffectRegion]:
        """Map structured effects to coarse :class:`EffectRegion` bitflags.

        Returns ``(reads, writes)`` suitable for GVN / interprocedural kill
        checks.
        """
        reads = EffectRegion.NONE
        writes = EffectRegion.NONE
        for e in self.effects:
            region = _target_to_region(e.target, e.scope)
            if e.reads:
                reads |= region
            if e.writes:
                writes |= region
        if self.dynamic_barrier:
            writes |= EffectRegion.UNKNOWN_STATE
        return reads, writes


# ---------------------------------------------------------------------------
# Classification
# ---------------------------------------------------------------------------

# Pure results cached for reuse.
_PURE = CommandSideEffects(pure=True, deterministic=True)
_UNKNOWN_WRITE = CommandSideEffects(
    effects=(SideEffect(target=SideEffectTarget.UNKNOWN, writes=True),),
)
_DYNAMIC_BARRIER = CommandSideEffects(dynamic_barrier=True)


def _scope_from_varname(name: str) -> tuple[StorageScope, str | None]:
    """Infer storage scope and namespace from a variable name prefix."""
    if name.startswith("static::"):
        return StorageScope.STATIC, None
    if name.startswith("::"):
        parts = name.split("::")
        # "::foo::bar::var" → namespace "::foo::bar"
        ns_parts = [p for p in parts[:-1] if p]
        ns = "::" + "::".join(ns_parts) if ns_parts else "::"
        return StorageScope.GLOBAL if ns == "::" else StorageScope.NAMESPACE, ns
    return StorageScope.PROC_LOCAL, None


def _storage_type_for_command(command: str, args: tuple[str, ...]) -> StorageType:
    """Heuristic storage type from command name."""
    if command in ("dict", "dict set", "dict append", "dict lappend", "dict unset"):
        return StorageType.DICT
    if command in ("lappend", "linsert", "lset", "lsort", "lreplace", "lrepeat"):
        return StorageType.LIST
    if command == "array" or (args and command == "array"):
        return StorageType.ARRAY
    return StorageType.SCALAR


def _extract_protocol_namespace(command: str) -> str | None:
    """Extract the F5 protocol namespace prefix from a command name."""
    if "::" in command:
        prefix = command.split("::")[0]
        if prefix and prefix[0].isupper():
            return prefix
    return None


def classify_side_effects(
    command: str,
    args: tuple[str, ...] = (),
    *,
    dialect: str | None = None,
    subcommand: str | None = None,
    callee_summary: object | None = None,
) -> CommandSideEffects:
    """Classify the side effects of a command invocation.

    This function combines registry metadata with argument inspection to
    produce a structured :class:`CommandSideEffects` describing what the
    command reads and writes, in what scope, and on which connection side.

    Parameters:
        command: The command name (e.g. ``"set"``, ``"HTTP::header"``,
            ``"table"``).
        args: Arguments to the command (after the command word itself).
        dialect: Active dialect (``"irules"``, ``"tcl"``, ``"iapps"``,
            etc.).  ``None`` means unknown/any.
        subcommand: Explicit subcommand override.  When ``None``, inferred
            from ``args[0]`` for commands that use subcommands.
        callee_summary: Interprocedural procedure summary for internal
            calls.  When provided, ``effect_reads``/``effect_writes``
            (:class:`EffectRegion`) and ``pure`` are read from it,
            bypassing registry-based classification.

    Returns:
        A :class:`CommandSideEffects` instance describing all side effects.
    """
    # --- Interprocedural summary shortcut ---
    if callee_summary is not None:
        reads = EffectRegion(int(getattr(callee_summary, "effect_reads", 0)))
        writes = EffectRegion(int(getattr(callee_summary, "effect_writes", 0)))
        is_pure = bool(getattr(callee_summary, "pure", False))
        commits = bool(writes & EffectRegion.RESPONSE_LIFECYCLE)
        effects: list[SideEffect] = []
        if bool(reads & EffectRegion.HTTP_STATE) or bool(writes & EffectRegion.HTTP_STATE):
            effects.append(
                SideEffect(
                    target=SideEffectTarget.HTTP_HEADER,
                    reads=bool(reads & EffectRegion.HTTP_STATE),
                    writes=bool(writes & EffectRegion.HTTP_STATE),
                )
            )
        if commits:
            effects.append(
                SideEffect(
                    target=SideEffectTarget.RESPONSE_COMMIT,
                    writes=True,
                )
            )
        if bool(writes & EffectRegion.GLOBAL_STATE):
            effects.append(
                SideEffect(
                    target=SideEffectTarget.VARIABLE,
                    writes=True,
                    scope=StorageScope.GLOBAL,
                )
            )
        if bool(writes & EffectRegion.UNKNOWN_STATE):
            effects.append(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN,
                    reads=True,
                    writes=True,
                )
            )
        if not effects and not is_pure:
            effects.append(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN,
                    reads=True,
                    writes=True,
                )
            )
        return CommandSideEffects(
            effects=tuple(effects),
            pure=is_pure,
            deterministic=is_pure,
            dialect=dialect,
        )

    # Lazy import to avoid circular dependencies at module level.
    from ..commands.registry import REGISTRY

    lookup_dialect = "f5-irules" if dialect == "irules" else dialect
    spec = REGISTRY.get(command, lookup_dialect) or REGISTRY.get_any(command)

    # --- Dynamic barriers are unknowable ---
    if REGISTRY.is_dynamic_barrier(command):
        return CommandSideEffects(
            effects=(SideEffect(target=SideEffectTarget.UNKNOWN, reads=True, writes=True),),
            dynamic_barrier=True,
            dialect=dialect,
        )

    # --- Resolve subcommand ---
    effective_sub = subcommand or (args[0] if args else None)
    sub_spec = None
    if spec is not None and effective_sub is not None:
        sub_spec = spec.subcommands.get(effective_sub)

    # --- Look up registry hints ---
    # Hints provide *target* and *connection_side* (the "what" and "where").
    # Heuristics below provide *runtime shape*: key, scope, storage_type,
    # pure flag, arity-dependent read/write.  The two are complementary.
    hints = REGISTRY.side_effect_hints(command, effective_sub, dialect)
    _hint = hints[0] if hints else None

    # --- Special-case: expr is pure when braced (the common/recommended case) ---
    if command == "expr":
        return CommandSideEffects(pure=True, deterministic=True, dialect=dialect)

    # --- Form-aware resolution ---
    # When a command or subcommand has structured forms (getter/setter),
    # resolve the matching form by arity and use its purity/mutator/hints.
    resolved_form = None
    if sub_spec is not None and sub_spec.forms:
        # Subcommand has forms — resolve with args after the subcommand word.
        sub_args = tuple(args[1:]) if args else ()
        resolved_form = sub_spec.resolve_form(sub_args)
    elif spec is not None and len(spec.forms) > 1:
        resolved_form = spec.resolve_form(tuple(args))
        if command in {"HTTP::uri", "HTTP::path", "HTTP::query"} and args == ("-normalized",):
            getter_form = spec.resolve_form(())
            if getter_form is not None:
                resolved_form = getter_form

    if resolved_form is not None and resolved_form.side_effect_hints:
        _hint = resolved_form.side_effect_hints[0]
        hints = resolved_form.side_effect_hints

    # --- Check purity first ---
    is_pure = REGISTRY.is_pure(command)
    if not is_pure and sub_spec is not None:
        is_pure = sub_spec.pure

    # Form-level purity/mutator overrides command and subcommand levels.
    if resolved_form is not None:
        if resolved_form.pure:
            is_pure = True
        if resolved_form.mutator:
            is_pure = False
    else:
        # Mutator subcommands override command-level purity.
        # e.g. HTTP::header is pure, but HTTP::header replace is a mutator.
        if is_pure and sub_spec is not None and getattr(sub_spec, "mutator", False):
            is_pure = False

    if is_pure:
        # Pure commands may still read state (e.g. HTTP::uri with no args).
        protocol_ns = _extract_protocol_namespace(command)
        if protocol_ns:
            return CommandSideEffects(
                effects=(
                    SideEffect(
                        target=_hint.target if _hint else SideEffectTarget.UNKNOWN,
                        reads=True,
                        namespace=protocol_ns,
                        connection_side=_hint.connection_side if _hint else ConnectionSide.BOTH,
                        dialect=dialect,
                    ),
                ),
                pure=True,
                deterministic=True,
                dialect=dialect,
            )
        if _hint:
            # Subcommand marked pure but has a hint — include as read-only.
            return CommandSideEffects(
                effects=(
                    SideEffect(
                        target=_hint.target,
                        reads=True,
                        writes=False,
                        scope=_hint.scope,
                        connection_side=_hint.connection_side,
                        dialect=dialect,
                    ),
                ),
                pure=True,
                deterministic=True,
                dialect=dialect,
            )
        return CommandSideEffects(pure=True, deterministic=True, dialect=dialect)

    # --- Variable-assigning commands ---
    if spec is not None and spec.assigns_variable_at is not None:
        idx = spec.assigns_variable_at
        if idx < len(args):
            varname = args[idx]
            scope, ns = _scope_from_varname(varname)
            st = _storage_type_for_command(command, args)
            # For set with 1 arg it's a read, with 2 it's a write.
            is_write = len(args) > idx + 1 or command not in ("set",)
            is_read = command == "set" and len(args) == idx + 1
            side = ConnectionSide.NONE
            if dialect in ("irules", "f5-irules") and scope is StorageScope.STATIC:
                side = ConnectionSide.GLOBAL
            elif dialect in ("irules", "f5-irules") and scope is StorageScope.PROC_LOCAL:
                side = ConnectionSide.BOTH
            return CommandSideEffects(
                effects=(
                    SideEffect(
                        target=SideEffectTarget.VARIABLE,
                        reads=is_read or command in ("incr", "append", "lappend"),
                        writes=is_write,
                        storage_type=st,
                        scope=scope,
                        connection_side=side,
                        namespace=ns,
                        dialect=dialect,
                        key=varname,
                    ),
                ),
                dialect=dialect,
            )
        # Variable name is dynamic — conservative.
        return CommandSideEffects(
            effects=(SideEffect(target=SideEffectTarget.VARIABLE, reads=True, writes=True),),
            dialect=dialect,
        )

    # --- Protocol namespace commands (HTTP::, SSL::, TCP::, etc.) ---
    protocol_ns = _extract_protocol_namespace(command)
    if protocol_ns:
        return _classify_protocol_ns_command(
            command, args, effective_sub, sub_spec, protocol_ns, dialect, _hint
        )

    # --- Procedure definition ---
    if spec is not None and spec.defines_procedure:
        return CommandSideEffects(
            effects=(
                SideEffect(
                    target=SideEffectTarget.PROC_DEFINITION,
                    writes=True,
                    scope=StorageScope.NAMESPACE,
                    dialect=dialect,
                ),
            ),
            dialect=dialect,
        )

    # --- Hints-first: use structured registry hints when available ---
    if hints:
        return CommandSideEffects(effects=hints, dialect=dialect)

    # --- Fallback: conservative unknown ---
    return CommandSideEffects(
        effects=(SideEffect(target=SideEffectTarget.UNKNOWN, reads=True, writes=True),),
        dialect=dialect,
    )


# ---------------------------------------------------------------------------
# Protocol namespace helper
# ---------------------------------------------------------------------------


def _classify_protocol_ns_command(
    command: str,
    args: tuple[str, ...],
    effective_sub: str | None,
    sub_spec: object | None,
    protocol_ns: str,
    dialect: str | None,
    hint: SideEffect | None = None,
) -> CommandSideEffects:
    """Classify an F5 protocol namespace command (e.g. ``HTTP::header``).

    *hint* provides target/connection_side from registry metadata.
    Subcommand mutator flags can upgrade ``is_write`` but never downgrade
    hint values.
    """
    from ..commands.registry import REGISTRY

    if hint:
        target = hint.target
        side = hint.connection_side
        is_read = hint.reads
        is_write = hint.writes
    else:
        # No hint — conservative fallback for unknown protocol NS commands.
        target = SideEffectTarget.UNKNOWN
        side = ConnectionSide.BOTH
        is_read = True
        is_write = False

    # Subcommand mutator flags can upgrade writes.
    # HTTP::uri/path/query treat "-normalized" as getter mode even though
    # these commands also have 1-arg setter forms.
    if command in {"HTTP::uri", "HTTP::path", "HTTP::query"} and args == ("-normalized",):
        is_read = True
        is_write = False

    subs = REGISTRY.mutator_subcommands(command)
    if subs is not None and effective_sub:
        if effective_sub in subs:
            is_write = True
        else:
            # Known non-mutator subcommand — narrow to read-only.
            is_read = True
            is_write = False

    # Extract header/cookie name if available for HTTP commands.
    suffix = command.split("::", 1)[1] if "::" in command else ""
    key: str | None = None
    if protocol_ns == "HTTP" and suffix in ("header", "cookie") and len(args) >= 2:
        candidate = args[1] if effective_sub else (args[0] if args else None)
        if candidate and not candidate.startswith("$") and not candidate.startswith("["):
            key = candidate

    return CommandSideEffects(
        effects=(
            SideEffect(
                target=target,
                reads=is_read,
                writes=is_write,
                namespace=protocol_ns,
                connection_side=side,
                dialect=dialect,
                key=key,
            ),
        ),
        dialect=dialect,
    )
