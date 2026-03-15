"""Unified namespace registry facade.

Consolidates all namespace and event metadata behind a single lookup
interface.  Replaces the old ``EventRegistry`` from ``event_registry.py``.

Command definition files should continue to import
:class:`~.namespace_models.EventRequires` directly from
``namespace_models`` (which is deliberately free of heavy imports to
avoid circular dependencies).  Higher-level consumers (analysis,
features, compiler) should prefer this registry.
"""

from __future__ import annotations

from dataclasses import dataclass

from .namespace_data import (
    EVENT_DESCRIPTIONS,
    EVENT_PROPS,
    FLOW_CHAINS,
    MASTER_ORDER,
    MODIFICATION_SPECS,
    ONCE_PER_CONNECTION,
    PER_REQUEST,
    PROFILE_SPECS,
    PROTOCOL_NAMESPACE_SPECS,
    chain_for_profiles,
    compatible_connection_predecessors,
    compute_file_profiles,
    event_satisfies,
    event_side_label,
    event_side_label_short,
    events_after,
    events_before,
    events_matching,
    expand_profile_stack,
    get_event_detail,
    get_event_props,
    infer_profiles_from_events,
    order_events,
    order_events_for_file,
    parse_profile_directive,
    profile_stack_satisfies,
    scan_file_events,
    variable_scope_note,
)
from .namespace_models import (
    EventProps,
    EventRequires,
    FlowChain,
    LayerStack,
    ProfileContext,
    ProfileSpec,
    ProtocolNamespaceSpec,
    StackModification,
    VirtualServerModel,
    _build_layer_stack,
)


@dataclass(slots=True)
class NamespaceRegistry:
    """Lookup facade over iRules namespace and event metadata.

    Replaces ``EventRegistry``.  Built once at module load time from
    the data tables in ``namespace_data.py``.
    """

    _props: dict[str, EventProps]
    _descriptions: dict[str, str]
    _master_index: dict[str, int]
    _once_per_connection: frozenset[str]
    _per_request: frozenset[str]
    _flow_chains: dict[str, FlowChain]
    _profile_specs: dict[str, ProfileSpec]
    _protocol_namespace_specs: dict[str, ProtocolNamespaceSpec]
    _modification_specs: dict[str, StackModification]
    _known_irules_namespaces: frozenset[str]

    # Derived caches built at construction time.
    _all_names: frozenset[str]

    @classmethod
    def build_default(cls) -> NamespaceRegistry:
        """Construct the registry from the module-level data tables."""
        from .command_registry import REGISTRY

        known_irules_namespaces = frozenset(
            name.split("::", 1)[0] for name in REGISTRY.command_names("f5-irules") if "::" in name
        )
        master_index = {evt: idx for idx, (evt, _gates) in enumerate(MASTER_ORDER)}
        return cls(
            _props=EVENT_PROPS,
            _descriptions=EVENT_DESCRIPTIONS,
            _master_index=master_index,
            _once_per_connection=ONCE_PER_CONNECTION,
            _per_request=PER_REQUEST,
            _flow_chains=FLOW_CHAINS,
            _profile_specs=PROFILE_SPECS,
            _protocol_namespace_specs=PROTOCOL_NAMESPACE_SPECS,
            _modification_specs=MODIFICATION_SPECS,
            _known_irules_namespaces=known_irules_namespaces,
            _all_names=frozenset(EVENT_PROPS),
        )

    # Property queries

    def get_props(self, name: str) -> EventProps | None:
        """Look up event properties, returning ``None`` for unknown events."""
        return get_event_props(name)

    def is_known(self, name: str) -> bool:
        """Return ``True`` if *name* is a registered event."""
        return name in self._props

    def is_flow_event(self, name: str) -> bool:
        """Return ``True`` if *name* is a flow event (has active traffic flow).

        Returns ``True`` for unknown events (safe default).
        """
        props = self._props.get(name)
        if props is None:
            return True
        return props.flow

    @property
    def flow_events(self) -> frozenset[str]:
        """Return all event names that have an active traffic flow."""
        return frozenset(name for name, props in self._props.items() if props.flow)

    def all_event_names(self) -> frozenset[str]:
        """Return all registered event names."""
        return self._all_names

    def events_matching(self, requires: EventRequires) -> list[str]:
        """Return sorted event names whose properties satisfy *requires*."""
        return events_matching(requires)

    def event_satisfies(
        self,
        event: EventProps,
        requires: EventRequires,
        event_name: str | None = None,
    ) -> bool:
        """Return True if *event* properties satisfy the command *requires*."""
        return event_satisfies(event, requires, event_name)

    # Description queries

    def get_description(self, name: str) -> str | None:
        """Return the human-readable description for *name*."""
        return self._descriptions.get(name)

    def get_detail(self, name: str) -> str:
        """Build a short detail string for completion items."""
        return get_event_detail(name)

    def side_label(self, name: str) -> str:
        """Return ``"client-side"``, ``"server-side"``, etc. for *name*."""
        props = self._props.get(name)
        if props is None:
            return "global"
        return event_side_label(props)

    def side_label_short(self, name: str) -> str:
        """Short form of :meth:`side_label` for completion detail strings."""
        props = self._props.get(name)
        if props is None:
            return "global"
        return event_side_label_short(props)

    # Flow ordering queries

    def event_index(self, name: str) -> int | None:
        """Return the position of *name* in the master firing order."""
        return self._master_index.get(name)

    def order_events(self, file_events: frozenset[str]) -> list[str]:
        """Return *file_events* sorted into canonical firing order."""
        return order_events(file_events)

    def events_before(self, name: str, file_events: frozenset[str]) -> list[str]:
        """Return events from *file_events* that fire before *name*."""
        return events_before(name, file_events)

    def events_after(self, name: str, file_events: frozenset[str]) -> list[str]:
        """Return events from *file_events* that fire after *name*."""
        return events_after(name, file_events)

    def is_per_request(self, name: str) -> bool:
        """Return ``True`` if *name* can fire multiple times per connection."""
        return name in self._per_request

    def is_once_per_connection(self, name: str) -> bool:
        """Return ``True`` if *name* fires at most once per connection."""
        return name in self._once_per_connection

    def event_multiplicity(self, name: str) -> str:
        """Return ``"init"``, ``"once_per_connection"``, ``"per_request"``, or ``"unknown"``."""
        from core.commands.registry.namespace_data import event_multiplicity

        return event_multiplicity(name)

    def variable_scope_note(self, set_event: str, read_event: str) -> str | None:
        """Return a scoping concern note, or ``None`` if safe."""
        return variable_scope_note(set_event, read_event)

    def chain_for_profiles(self, profiles: frozenset[str]) -> FlowChain | None:
        """Find the best-matching flow chain for *profiles*."""
        return chain_for_profiles(profiles)

    def compatible_connection_predecessors(self, event: str) -> frozenset[str]:
        """Return ``ONCE_PER_CONNECTION`` events preceding *event*."""
        return compatible_connection_predecessors(event)

    # File-level helpers

    def scan_file_events(self, source: str) -> frozenset[str]:
        """Return event names from all ``when EVENT`` blocks in *source*."""
        return scan_file_events(source)

    def compute_file_profiles(self, source: str) -> frozenset[str]:
        """Compute effective profiles for a file (directive + inferred)."""
        return compute_file_profiles(source)

    def expand_profile_stack(self, profiles: frozenset[str]) -> frozenset[str]:
        """Expand *profiles* with all transitive parent requirements."""
        return expand_profile_stack(profiles)

    def profile_stack_satisfies(
        self,
        required_profiles: frozenset[str],
        active_profiles: frozenset[str],
    ) -> bool:
        """Return True if *active_profiles* satisfies any required profile stack."""
        return profile_stack_satisfies(required_profiles, active_profiles)

    def parse_profile_directive(self, source: str) -> frozenset[str]:
        """Scan leading comments for ``# profiles: ...`` directive."""
        return parse_profile_directive(source)

    def infer_profiles_from_events(self, event_names: frozenset[str]) -> frozenset[str]:
        """Infer profiles from the events present in a file."""
        return infer_profiles_from_events(event_names)

    def order_events_for_file(self, source: str) -> list[str]:
        """Scan ``when`` blocks and return events in firing order."""
        return order_events_for_file(source)

    # Profile / namespace queries (Tier 2)

    def get_profile_spec(self, name: str) -> ProfileSpec | None:
        """Look up a profile spec by name."""
        return self._profile_specs.get(name)

    def get_protocol_namespace(self, prefix: str) -> ProtocolNamespaceSpec | None:
        """Look up a protocol namespace spec by prefix."""
        spec = self._protocol_namespace_specs.get(prefix)
        if spec is not None:
            return spec
        # Fallback: treat any known iRules namespace prefix as a generic
        # application-layer namespace to avoid hard-failing lookups.
        if prefix in self._known_irules_namespaces:
            return ProtocolNamespaceSpec(
                prefix,
                profiles=frozenset(),
                layer="application",
                side="both",
            )
        return None

    def get_modification_spec(self, command: str) -> StackModification | None:
        """Look up a stack modification spec by command name."""
        return self._modification_specs.get(command)

    def build_layer_stack(self, profiles: frozenset[str]) -> LayerStack:
        """Build a ``LayerStack`` from a flat set of profile names."""
        return _build_layer_stack(profiles)

    def build_virtual_server_model(self, profiles: frozenset[str]) -> VirtualServerModel:
        """Build a ``VirtualServerModel`` for the given profile set.

        Computes the layer stack and determines which events can fire.
        """
        upper = frozenset(p.upper() for p in profiles)
        expanded = expand_profile_stack(upper)
        stack = _build_layer_stack(expanded)
        # An event can fire if any gate profile stack is present in the
        # active profile stack (empty gate = always fires).
        valid: set[str] = set()
        for evt, gates in MASTER_ORDER:
            if not gates or profile_stack_satisfies(gates, expanded):
                valid.add(evt)
        return VirtualServerModel(
            profiles=expanded,
            layer_stack=stack,
            valid_events=frozenset(valid),
        )

    def build_profile_context(self, source: str) -> ProfileContext:
        """Build a ``ProfileContext`` for a source file."""
        explicit = parse_profile_directive(source)
        events = scan_file_events(source)
        implied = infer_profiles_from_events(events)
        combined = expand_profile_stack(implied | explicit)
        return ProfileContext(
            implied=implied,
            explicit=explicit,
            combined=combined,
        )

    # Backward-compatible aliases
    # These exist only during migration. Once all consumers are migrated
    # to NamespaceRegistry, old EventRegistry method names map here.

    def non_flow_events(self) -> frozenset[str]:
        """Return all event names that are non-flow (K14320).

        Derived query — positive form is ``flow_events`` property.
        """
        return frozenset(name for name, props in self._props.items() if not props.flow)


NAMESPACE_REGISTRY = NamespaceRegistry.build_default()
