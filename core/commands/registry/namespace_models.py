"""Namespace model data types.

This module defines all data types for the unified namespace model:

- **Tier 1** (all dialects): ``NamespaceScope``, ``NamespaceTracker``
  for Tcl namespace scoping.
- **Tier 2** (iRules only): ``ProfileSpec``, ``ProtocolNamespaceSpec``,
  ``LayerStack``, ``StackModification``, ``ConnectionModel``,
  ``EventNode``, ``EventEdge``, ``EventGraph``, ``VirtualServerModel``
  for F5 protocol namespace modelling.
- **Migrated types**: ``EventProps``, ``EventRequires``, ``FlowStep``,
  ``FlowChain``, ``CommandLegality``, ``ProfileContext`` carry forward
  semantics from the old ``event_tree`` / ``event_flow_chains`` modules.

This module is deliberately free of registry imports so it can be used
by command definition files without introducing circular dependencies
(same principle as the old ``event_tree.py``).
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Literal

# Tier 1: Tcl namespace scope (all dialects)


@dataclass(frozen=True, slots=True)
class NamespaceScope:
    """A namespace scope at a point in the source.

    Attributes:
        path: Namespace path segments, e.g. ``("foo", "bar")`` for
            ``::foo::bar``.  Empty tuple means the global namespace.
        source: How this scope was entered: ``"namespace_eval"``,
            ``"proc_body"``, ``"irule"``, ``"oo_class"``, ``"global"``.
    """

    path: tuple[str, ...] = ()
    source: str = "global"

    @property
    def qualified(self) -> str:
        """Fully qualified namespace name: ``'::foo::bar'``."""
        if not self.path:
            return "::"
        return "::" + "::".join(self.path)

    def resolve(self, name: str) -> str:
        r"""Resolve a potentially unqualified name in this scope.

        Absolute names (starting with ``::``\ ) are returned as-is.
        Relative names are prefixed with this scope's qualified path.
        """
        if name.startswith("::"):
            return name
        if not self.path:
            return "::" + name
        return self.qualified + "::" + name


@dataclass(slots=True)
class NamespaceTracker:
    """Tracks namespace scope through a file's AST.

    Handles:

    - ``namespace eval ::foo { ... }`` -- pushes scope
    - proc definitions -- infers parent namespace
    - ``oo::class create`` -- pushes scope for methods
    - iRule name detection -- sets root scope for procs

    Attributes:
        _stack: Stack of active scopes (innermost last).
        _irule_name: iRule name from ``# irule:`` pragma or filename.
    """

    _stack: list[NamespaceScope] = field(default_factory=list)
    _irule_name: str | None = None

    @property
    def current(self) -> NamespaceScope:
        """The innermost active namespace scope."""
        return self._stack[-1] if self._stack else NamespaceScope((), "global")

    def push(self, scope: NamespaceScope) -> None:
        """Push a new namespace scope onto the stack."""
        self._stack.append(scope)

    def pop(self) -> None:
        """Pop the innermost scope.  No-op if the stack is empty."""
        if self._stack:
            self._stack.pop()

    def set_irule_name(self, name: str) -> None:
        """Set the iRule name (from pragma comment or filename).

        In iRules, procs defined at top level end up in
        ``::irule_name::``.  Strips partition prefix (``/Common/``)
        if present.
        """
        # Strip partition prefix: "/Common/my_irule" -> "my_irule"
        if "/" in name:
            name = name.rsplit("/", 1)[-1]
        self._irule_name = name

    @property
    def irule_namespace(self) -> str | None:
        """The iRule's implicit namespace, if known."""
        return f"::{self._irule_name}" if self._irule_name else None


# Migrated from event_tree.py


@dataclass(frozen=True, slots=True)
class EventProps:
    """Properties of a single iRules event based on protocol stack position.

    Attributes:
        client_side: Client-side connection is available.
        server_side: Server-side connection is available.
        transport: ``"tcp"``, ``"udp"``, a tuple like ``("tcp", "udp")``
            for L4 events that fire on both, or ``None`` (no transport layer).
        implied_profiles: Profiles implied by this event firing
            (e.g. ``{"HTTP"}`` for ``HTTP_REQUEST``).
        flow: Whether an active traffic flow is present.  ``False`` for
            lifecycle events like ``RULE_INIT`` and ``PERSIST_DOWN``
            that fire outside of connection processing (K14320).
    """

    client_side: bool = False
    server_side: bool = False
    transport: str | tuple[str, ...] | None = None
    implied_profiles: frozenset[str] = field(default_factory=frozenset)
    flow: bool = True
    deprecated: bool = False
    hot: bool = False
    common: bool = False


@dataclass(frozen=True, slots=True)
class EventRequires:
    """What a command needs from the protocol stack to function.

    All fields default to False/None/empty, meaning "no requirement".
    A command matches an event when every non-default requirement is
    satisfied by the event's properties, OR the event name is in
    ``also_in``.

    Evaluation order (must be preserved):
    1. ``init_only`` -- short-circuits to ``event_name == "RULE_INIT"``
    2. ``also_in`` -- unconditional match if event name is in the set
    3. ``flow``, ``client_side``, ``server_side``, ``transport``,
       ``profiles``, ``capability`` -- all must pass

    Attributes:
        client_side: Command needs a client-side connection.
        server_side: Command needs a server-side connection.
        transport: Command needs a specific transport (``"tcp"`` or ``"udp"``).
        profiles: Command needs at least one of these profiles attached
            to the virtual server.
        also_in: Additional event names that are always valid regardless
            of profile/transport matching (e.g. setup events like
            ``CLIENT_ACCEPTED`` for protocol-specific commands).
        init_only: Command only works in ``RULE_INIT`` (no connection).
        flow: Command requires an active traffic flow.  Set for commands
            like ``table``, ``connect``, ``LB::select`` that cannot work
            in non-flow events (K14320).
        capability: Profile capability required (e.g. ``"sni"``,
            ``"cipher"``).  When set, at least one active profile must
            provide this capability via ``ProfileSpec.capabilities``.
    """

    client_side: bool = False
    server_side: bool = False
    transport: str | None = None
    profiles: frozenset[str] = field(default_factory=frozenset)
    also_in: frozenset[str] = field(default_factory=frozenset)
    init_only: bool = False
    flow: bool = False
    capability: str | None = None


# Migrated from event_flow_chains.py


@dataclass(frozen=True, slots=True)
class FlowStep:
    """A single step in an event flow chain.

    Attributes:
        event: The event name (e.g. ``"CLIENT_ACCEPTED"``).
        phase: Logical grouping (e.g. ``"l4_client"``, ``"tls_client"``).
            Used by AI tools for diagram formatting.  Values:
            ``init``, ``l4_client``, ``tls_client``, ``http_request``,
            ``lb``, ``l4_server``, ``tls_server``,
            ``http_request_server``, ``http_response``, ``l4_teardown``,
            ``dns_request``, ``dns_response``.
        conditional: Whether the event only fires under specific conditions.
        condition_note: Human-readable description of the condition.
    """

    event: str
    phase: str
    conditional: bool = False
    condition_note: str = ""


@dataclass(frozen=True, slots=True)
class FlowChain:
    """A complete event flow chain for a virtual server profile combination.

    Attributes:
        chain_id: Unique identifier (e.g. ``"tcp_clientssl_serverssl_http"``).
        description: Human-readable label.
        profiles: The set of BigIP profile *types* on the virtual server.
        steps: Ordered list of :class:`FlowStep` instances.
        notes: Any caveats or implementation notes.
    """

    chain_id: str
    description: str
    profiles: frozenset[str]
    steps: tuple[FlowStep, ...]
    notes: str = ""


# Tier 2: F5 protocol namespace model (iRules only)


@dataclass(frozen=True, slots=True)
class ProfileSpec:
    """Metadata for one F5 profile type.

    Attributes:
        name: Profile type name (e.g. ``"HTTP"``, ``"CLIENTSSL"``).
        layer: Protocol stack layer: ``"transport"``, ``"tls"``,
            ``"application"``, ``"security"``, ``"acceleration"``,
            ``"message"``.
        side: Connection side: ``"client"``, ``"server"``, ``"both"``,
            ``"global"``.
        requires: Profile types this one requires (e.g. HTTP requires TCP).
        conflicts: Profile types this one conflicts with (mutual
            exclusivity within a layer).
        capabilities: Capabilities this profile provides (e.g.
            ``"sni"``, ``"cipher"``).  Used for fine-grained command
            validation -- see SSL persistence profile discussion.
    """

    name: str
    layer: str
    side: str
    requires: frozenset[str] = field(default_factory=frozenset)
    conflicts: frozenset[str] = field(default_factory=frozenset)
    capabilities: frozenset[str] = field(default_factory=frozenset)


@dataclass(frozen=True, slots=True)
class ProtocolNamespaceSpec:
    """An iRules protocol command namespace (e.g. HTTP::, SSL::, TCP::).

    Not a Tcl namespace -- these are command prefixes whose availability
    depends on attached profiles.

    Attributes:
        prefix: The namespace prefix (e.g. ``"HTTP"``, ``"SSL"``).
        profiles: Profiles that provide this namespace.
        layer: Protocol layer this namespace operates on.
        side: Default connection side.
        side_selectable: Whether ``clientside``/``serverside`` qualifiers
            are supported.
    """

    prefix: str
    profiles: frozenset[str]
    layer: str
    side: str
    side_selectable: bool = False


@dataclass(frozen=True, slots=True)
class LayerStack:
    """Active protocol layers for a connection at a point in time.

    Immutable -- modifications return a new instance.

    Attributes:
        transport: ``"TCP"``, ``"UDP"``, ``"FASTL4"``, or ``None``.
        tls_client: ``"CLIENTSSL"`` or ``None``.
        tls_server: ``"SERVERSSL"`` or ``None``.
        tls_shared: Shared TLS helper profiles that can coexist with
            client- or server-side TLS profiles (for example ``PERSIST``).
        application: Active application-layer profiles (e.g. ``{"HTTP"}``).
        security: Active security profiles (e.g. ``{"ASM", "ACCESS"}``).
        acceleration: Active acceleration profiles.
    """

    transport: str | None = None
    tls_client: str | None = None
    tls_server: str | None = None
    tls_shared: frozenset[str] = field(default_factory=frozenset)
    application: frozenset[str] = field(default_factory=frozenset)
    security: frozenset[str] = field(default_factory=frozenset)
    acceleration: frozenset[str] = field(default_factory=frozenset)

    @property
    def all_profiles(self) -> frozenset[str]:
        """All active profile names as a flat set."""
        profiles: set[str] = set()
        if self.transport:
            profiles.add(self.transport)
        if self.tls_client:
            profiles.add(self.tls_client)
        if self.tls_server:
            profiles.add(self.tls_server)
        profiles.update(self.tls_shared)
        profiles.update(self.application)
        profiles.update(self.security)
        profiles.update(self.acceleration)
        return frozenset(profiles)


@dataclass(frozen=True, slots=True)
class StackModification:
    """A command that changes the effective profile stack.

    Attributes:
        command: The command name (e.g. ``"SSL::disable"``).
        side: ``"clientside"``, ``"serverside"``, or ``None`` (both/resolved from arg).
        removes_profile: Profile type removed, or ``None``.
        adds_profile: Profile type added (rare), or ``None``.
        requires_before_event: Must happen before this event, or ``None``.
    """

    command: str
    side: str | None = None
    removes_profile: str | None = None
    adds_profile: str | None = None
    requires_before_event: str | None = None


@dataclass(frozen=True, slots=True)
class ConnectionModel:
    """Models a connection's profile state across its lifecycle.

    Frozen -- ``apply_modification()`` returns a *new* ``ConnectionModel``
    rather than mutating in place.

    Attributes:
        initial_profiles: From virtual server configuration.
        current_stack: Active protocol stack at the current event.
        modifications: Ordered list of applied modifications.
    """

    initial_profiles: frozenset[str]
    current_stack: LayerStack
    modifications: tuple[StackModification, ...] = ()

    def apply_modification(self, mod: StackModification) -> ConnectionModel:
        """Return a new ``ConnectionModel`` with *mod* applied.

        The current stack is updated to reflect the profile change.
        """
        profiles = set(self.current_stack.all_profiles)
        if mod.removes_profile and mod.removes_profile in profiles:
            profiles.discard(mod.removes_profile)
        if mod.adds_profile:
            profiles.add(mod.adds_profile)
        # Rebuild stack from modified profile set.
        new_stack = _build_layer_stack(frozenset(profiles))
        return ConnectionModel(
            initial_profiles=self.initial_profiles,
            current_stack=new_stack,
            modifications=(*self.modifications, mod),
        )


@dataclass(frozen=True, slots=True)
class EventNode:
    """A node in the event graph.

    Attributes:
        event: Event name (e.g. ``"HTTP_REQUEST"``).
        side: Connection side (``"client"``, ``"server"``, ``"both"``,
            ``"global"``).
        profiles: Profile gates -- profiles that must be active for this
            event to fire.
        layer: Protocol layer that triggers this event.
        multiplicity: ``"once_per_connection"``, ``"per_request"``, or
            ``"unknown"``.
    """

    event: str
    side: str
    profiles: frozenset[str]
    layer: str
    multiplicity: str = "unknown"


@dataclass(frozen=True, slots=True)
class EventEdge:
    """An edge in the event graph (temporal ordering).

    Attributes:
        source: Preceding event name.
        target: Following event name.
        conditional: Whether the transition is conditional.
        condition: Human-readable description of the condition.
    """

    source: str
    target: str
    conditional: bool = False
    condition: str = ""


@dataclass(slots=True)
class EventGraph:
    """The complete event flow graph for a profile combination.

    Attributes:
        nodes: Event nodes keyed by event name.
        edges: Temporal ordering edges.
        profiles: The virtual server profiles this graph covers.
    """

    nodes: dict[str, EventNode]
    edges: list[EventEdge]
    profiles: frozenset[str]

    def reachable_after(
        self,
        event: str,
        modifications: list[StackModification] | None = None,
    ) -> frozenset[str]:
        """Events reachable after *event*, considering *modifications*."""
        # Build adjacency from edges.
        adj: dict[str, list[str]] = {}
        for edge in self.edges:
            adj.setdefault(edge.source, []).append(edge.target)

        # BFS from event.
        visited: set[str] = set()
        queue = list(adj.get(event, []))
        while queue:
            node = queue.pop(0)
            if node in visited:
                continue
            visited.add(node)
            queue.extend(adj.get(node, []))

        # Filter out events whose profile gates are removed by modifications.
        if modifications:
            removed = frozenset(m.removes_profile for m in modifications if m.removes_profile)
            visited = {
                e for e in visited if e in self.nodes and not (self.nodes[e].profiles & removed)
            }

        return frozenset(visited)

    def valid_namespaces_at(
        self,
        event: str,
        modifications: list[StackModification] | None = None,
    ) -> frozenset[str]:
        """Protocol command namespaces valid at *event*.

        Determined by the profile gates of the event node and any
        modifications applied before it.
        """
        node = self.nodes.get(event)
        if node is None:
            return frozenset()
        active = set(node.profiles)
        if modifications:
            for mod in modifications:
                if mod.removes_profile:
                    active.discard(mod.removes_profile)
                if mod.adds_profile:
                    active.add(mod.adds_profile)
        return frozenset(active)


@dataclass(frozen=True, slots=True)
class VirtualServerModel:
    """Computed model for a virtual server's profile and event configuration.

    Attributes:
        profiles: Attached profiles.
        layer_stack: Initial protocol stack.
        valid_events: Events that can fire with these profiles.
    """

    profiles: frozenset[str]
    layer_stack: LayerStack
    valid_events: frozenset[str]


# Query result types


@dataclass(frozen=True, slots=True)
class CommandLegality:
    """Result of querying whether a command is legal in a given context.

    Attributes:
        status: ``"allowed"``, ``"disallowed"``, or ``"conditional"``.
        condition: Human-readable condition string when ``status`` is
            ``"conditional"`` or ``"disallowed"``.
    """

    status: Literal["allowed", "disallowed", "conditional"]
    condition: str | None = None


@dataclass(frozen=True, slots=True)
class ProfileContext:
    """Profile information for a file, combining inference and directives.

    Attributes:
        implied: Profiles inferred from ``when EVENT`` blocks.
        explicit: Profiles from ``# profiles: ...`` directive.
        combined: Union of *implied* and *explicit*.
    """

    implied: frozenset[str]
    explicit: frozenset[str]
    combined: frozenset[str]


# Internal helpers

# Known profile-to-layer mapping.  Used by _build_layer_stack() to
# reconstruct a LayerStack from a flat profile set.  This table is
# intentionally minimal -- it covers the profiles that appear in the
# existing EVENT_PROPS table.  New profiles can be added as needed.

_PROFILE_LAYERS: dict[str, str] = {
    # Transport
    "TCP": "transport",
    "UDP": "transport",
    "FASTL4": "transport",
    "SCTP": "transport",
    # TLS
    "CLIENTSSL": "tls_client",
    "SERVERSSL": "tls_server",
    "SSL_PERSISTENCE": "tls_shared",
    # Application
    "HTTP": "application",
    "FASTHTTP": "application",
    "HTTP2": "application",
    "HTTP_PROXY_CONNECT": "application",
    "DNS": "application",
    "SIP": "application",
    "SIPROUTER": "application",
    "SIPSESSION": "application",
    "FIX": "application",
    "DIAMETER": "application",
    "DIAMETERSESSION": "application",
    "DIAMETER_ENDPOINT": "application",
    "MQTT": "application",
    "RTSP": "application",
    "GENERICMSG": "application",
    "MR": "application",
    "GTP": "application",
    "RADIUS": "application",
    "RADIUS_AAA": "application",
    "PCP": "application",
    "SOCKS": "application",
    "TDS": "application",
    "MSSQL": "application",
    "IVS_ENTRY": "application",
    # Security
    "ASM": "security",
    "ACCESS": "security",
    "BOTDEFENSE": "security",
    "ANTIFRAUD": "security",
    "DOSL7": "security",
    "AUTH": "security",
    "ECA": "security",
    "PROTOCOL_INSPECTION": "security",
    "IPS": "security",
    # Acceleration / content
    "STREAM": "acceleration",
    "WEBACCELERATION": "acceleration",
    "CACHE": "acceleration",
    "CATEGORY": "acceleration",
    "CLASSIFICATION": "acceleration",
    "HTML": "acceleration",
    "REWRITE": "acceleration",
    "QOE": "acceleration",
    "XML": "acceleration",
    "JSON": "acceleration",
    "SSE": "acceleration",
    "WS": "acceleration",
    "ICAP": "acceleration",
    "REQUESTADAPT": "acceleration",
    "RESPONSEADAPT": "acceleration",
    # Miscellaneous
    "PEM": "security",
    "AVR": "acceleration",
    "NAME": "application",
    "FLOW": "application",
    "TAP": "security",
    "CONNECTOR": "application",
    "L7CHECK": "application",
    "PERSIST": "tls_shared",
    "LSN": "application",
    "DATAGRAM": "application",
}


def _build_layer_stack(profiles: frozenset[str]) -> LayerStack:
    """Build a ``LayerStack`` from a flat set of profile names."""
    transport: str | None = None
    tls_client: str | None = None
    tls_server: str | None = None
    tls_shared: set[str] = set()
    application: set[str] = set()
    security: set[str] = set()
    acceleration: set[str] = set()

    for p in profiles:
        layer = _PROFILE_LAYERS.get(p)
        if layer == "transport":
            transport = p
        elif layer == "tls_client":
            tls_client = p
        elif layer == "tls_server":
            tls_server = p
        elif layer == "tls_shared":
            tls_shared.add(p)
        elif layer == "application":
            application.add(p)
        elif layer == "security":
            security.add(p)
        elif layer == "acceleration":
            acceleration.add(p)

    return LayerStack(
        transport=transport,
        tls_client=tls_client,
        tls_server=tls_server,
        tls_shared=frozenset(tls_shared),
        application=frozenset(application),
        security=frozenset(security),
        acceleration=frozenset(acceleration),
    )
