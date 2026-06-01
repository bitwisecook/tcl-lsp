"""Aggregated profile data for profile-guided optimisation.

:class:`ProfileData` is the query-friendly rollup of an
:mod:`~compiler.pgo.occurrence` stream — the form the PGO analyser reads.
It keeps the raw ``occurrences`` (lossless, so nothing the importers
captured is thrown away) alongside derived counts and timing.

Aggregation dimensions, chosen so the model covers **both** C Tcl and
iRules telemetry:

* ``line_counts`` — 1-based line → count.  **Precise**; from a tclsh
  ``enterstep`` capture (``info frame`` lines) — the signal the branch
  reorderer prefers.
* ``command_counts`` — command name → count.  **Coarse**; the fallback
  when only F5 command-level data is present.
* ``event_counts`` — iRule event name → entry count.
* ``var_mod_counts`` — variable name → write count.
* ``bytecode_counts`` — opcode → execution count (unifies F5
  ``RP_CMD_BYTECODE`` with a C Tcl ``instructionCount`` dump).
* ``command_time_us`` — command name → total microseconds, derived by
  pairing ``CMD_ENTER``/``CMD_EXIT`` per flow (reserved for future
  hot-path / hot-proc work; not used by the branch reorderer).

Importers may either hand in an occurrence stream (use
:meth:`ProfileData.from_occurrences`) or, for already-aggregated sources
like a C Tcl ``evalstats`` dump, populate the count maps directly.
"""

from __future__ import annotations

from collections import Counter
from collections.abc import Mapping, Sequence
from dataclasses import dataclass, field

from .occurrence import Occurrence, OccurrenceKind


@dataclass(frozen=True, slots=True)
class ProfileData:
    """Aggregated execution counts for one profiled run (or merge of runs)."""

    #: Raw normalised stream (lossless; may be empty for aggregate-only sources).
    occurrences: tuple[Occurrence, ...] = ()
    #: 1-based source line -> execution count (precise; tclsh capture).
    line_counts: Mapping[int, int] = field(default_factory=dict)
    #: command name -> invocation count (coarse; F5 log / C Tcl trace).
    command_counts: Mapping[str, int] = field(default_factory=dict)
    #: iRule event name -> entry count.
    event_counts: Mapping[str, int] = field(default_factory=dict)
    #: variable name -> write count.
    var_mod_counts: Mapping[str, int] = field(default_factory=dict)
    #: bytecode opcode -> execution count.
    bytecode_counts: Mapping[str, int] = field(default_factory=dict)
    #: command name -> total time in microseconds (enter/exit deltas).
    command_time_us: Mapping[str, int] = field(default_factory=dict)
    #: Provenance tag — e.g. ``"f5-log"`` or ``"tclsh-trace"``.
    source: str = ""

    @property
    def has_line_data(self) -> bool:
        """True when precise per-line counts are available."""
        return bool(self.line_counts)

    @property
    def has_command_data(self) -> bool:
        """True when coarse per-command counts are available."""
        return bool(self.command_counts)

    @property
    def is_empty(self) -> bool:
        """True when no usable count signal of any grade is present."""
        return not (
            self.line_counts
            or self.command_counts
            or self.event_counts
            or self.var_mod_counts
            or self.bytecode_counts
        )

    def line_count(self, line: int) -> int:
        """Execution count for *line* (1-based), or ``0`` if unseen."""
        return self.line_counts.get(line, 0)

    def command_count(self, name: str) -> int:
        """Invocation count for command *name*, or ``0`` if unseen."""
        return self.command_counts.get(name, 0)

    @classmethod
    def from_occurrences(
        cls,
        occurrences: Sequence[Occurrence],
        *,
        source: str = "",
    ) -> ProfileData:
        """Roll an occurrence stream up into aggregated counts + timing."""
        lines: Counter[int] = Counter()
        commands: Counter[str] = Counter()
        events: Counter[str] = Counter()
        var_mods: Counter[str] = Counter()
        bytecode: Counter[str] = Counter()
        times: Counter[str] = Counter()

        # Per-flow enter stacks for enter/exit timing pairing.  Keyed by
        # flow id (None collapses to a single shared stack for plain Tcl).
        open_calls: dict[str | None, list[tuple[str, int]]] = {}

        for occ in occurrences:
            if occ.kind is OccurrenceKind.CMD_ENTER:
                commands[occ.value] += 1
                if occ.line is not None:
                    lines[occ.line] += 1
                if occ.timestamp_us is not None:
                    flow = occ.context.flow_id if occ.context else None
                    open_calls.setdefault(flow, []).append((occ.value, occ.timestamp_us))
            elif occ.kind is OccurrenceKind.CMD_EXIT:
                if occ.timestamp_us is not None:
                    flow = occ.context.flow_id if occ.context else None
                    stack = open_calls.get(flow)
                    if stack:
                        name, started = stack.pop()
                        delta = occ.timestamp_us - started
                        if delta >= 0:
                            times[name] += delta
            elif occ.kind is OccurrenceKind.EVENT_ENTRY:
                events[occ.value] += 1
            elif occ.kind is OccurrenceKind.VAR_MOD:
                var_mods[occ.var_name] += 1
            elif occ.kind is OccurrenceKind.BYTECODE:
                bytecode[occ.value] += 1

        return cls(
            occurrences=tuple(occurrences),
            line_counts=dict(lines),
            command_counts=dict(commands),
            event_counts=dict(events),
            var_mod_counts=dict(var_mods),
            bytecode_counts=dict(bytecode),
            command_time_us=dict(times),
            source=source,
        )


def merge(profiles: Sequence[ProfileData], *, source: str | None = None) -> ProfileData:
    """Sum several :class:`ProfileData` into one (counts add elementwise).

    Useful for combining captures across multiple runs or traffic samples.
    The raw ``occurrences`` are concatenated so the merge stays lossless.
    """
    occurrences: list[Occurrence] = []
    lines: Counter[int] = Counter()
    commands: Counter[str] = Counter()
    events: Counter[str] = Counter()
    var_mods: Counter[str] = Counter()
    bytecode: Counter[str] = Counter()
    times: Counter[str] = Counter()
    tags: list[str] = []
    for p in profiles:
        occurrences.extend(p.occurrences)
        lines.update(p.line_counts)
        commands.update(p.command_counts)
        events.update(p.event_counts)
        var_mods.update(p.var_mod_counts)
        bytecode.update(p.bytecode_counts)
        times.update(p.command_time_us)
        if p.source and p.source not in tags:
            tags.append(p.source)
    return ProfileData(
        occurrences=tuple(occurrences),
        line_counts=dict(lines),
        command_counts=dict(commands),
        event_counts=dict(events),
        var_mod_counts=dict(var_mods),
        bytecode_counts=dict(bytecode),
        command_time_us=dict(times),
        source=source if source is not None else "+".join(tags),
    )
