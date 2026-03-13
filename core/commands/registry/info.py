"""High-level event and command metadata lookup helpers.

These helpers provide stable, serialisable payloads for CLI/AI consumers
that need registry metadata without duplicating lookup logic.
"""

from __future__ import annotations

from dataclasses import dataclass

from .command_registry import REGISTRY
from .namespace_data import (
    event_multiplicity,
    event_side_label,
    events_matching,
    get_event_description,
    get_event_props,
)


@dataclass(frozen=True, slots=True)
class EventInfo:
    """Lookup result for a single iRules event."""

    event: str
    dialect: str
    known: bool
    deprecated: bool
    multiplicity: str
    description: str
    side: str
    transport: str | None
    implied_profiles: tuple[str, ...]
    valid_commands: tuple[str, ...]

    @property
    def valid_command_count(self) -> int:
        """Return the number of valid commands in this event."""
        return len(self.valid_commands)


@dataclass(frozen=True, slots=True)
class CommandInfo:
    """Lookup result for a single registry command."""

    found: bool
    command: str
    dialect: str
    summary: str
    synopsis: tuple[str, ...]
    switches: tuple[str, ...]
    valid_events: tuple[str, ...]
    valid_in_any_event: bool


def lookup_event_info(event_name: str, *, dialect: str = "f5-irules") -> EventInfo:
    """Resolve metadata for *event_name* in *dialect*."""
    name = event_name.strip().upper()

    when_values = REGISTRY.argument_values("when", 0, dialect)
    known_events = {item.value for item in when_values}
    deprecated_events = {
        item.value for item in when_values if "deprecated" in (item.detail or "").lower()
    }

    is_known = bool(name) and name in known_events
    valid_commands: tuple[str, ...] = ()
    if is_known:
        event_set = REGISTRY.commands_for_event(dialect, name)
        valid_commands = tuple(sorted(event_set.valid_commands))

    props = get_event_props(name)
    return EventInfo(
        event=name,
        dialect=dialect,
        known=is_known,
        deprecated=name in deprecated_events,
        multiplicity=event_multiplicity(name),
        description=get_event_description(name) or "",
        side=event_side_label(props) if props is not None else "unknown",
        transport=props.transport if props is not None else None,
        implied_profiles=tuple(sorted(props.implied_profiles)) if props is not None else (),
        valid_commands=valid_commands,
    )


def lookup_command_info(command_name: str, *, dialect: str) -> CommandInfo:
    """Resolve registry metadata for *command_name* in *dialect*."""
    query = command_name.strip()
    if not query:
        raise ValueError("command name is required")

    spec = REGISTRY.get(query, dialect)
    resolved_name = query

    if spec is None:
        lowered = query.lower()
        for candidate in REGISTRY.command_names(dialect):
            if candidate.lower() == lowered:
                spec = REGISTRY.get(candidate, dialect)
                resolved_name = candidate
                break

    if spec is None:
        return CommandInfo(
            found=False,
            command=query,
            dialect=dialect,
            summary="",
            synopsis=(),
            switches=(),
            valid_events=(),
            valid_in_any_event=False,
        )

    valid_events: tuple[str, ...] = ()
    valid_in_any_event = False
    if spec.event_requires is not None:
        requires = spec.event_requires
        valid_events = tuple(events_matching(spec.event_requires))
        valid_in_any_event = not (
            requires.client_side
            or requires.server_side
            or requires.transport is not None
            or bool(requires.profiles)
            or bool(requires.also_in)
            or requires.init_only
            or requires.flow
            or requires.capability is not None
        )

    hover = spec.hover
    return CommandInfo(
        found=True,
        command=resolved_name,
        dialect=dialect,
        summary=hover.summary if hover is not None else "",
        synopsis=hover.synopsis if hover is not None else (),
        switches=tuple(sorted(spec.switch_names())),
        valid_events=valid_events,
        valid_in_any_event=valid_in_any_event,
    )
