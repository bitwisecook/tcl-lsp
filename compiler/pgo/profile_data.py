# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

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
* ``span_time_us`` — ``(occurrence-kind, value)`` → total microseconds,
  derived by pairing every ``*_ENTRY`` / ``*_ENTER`` occurrence with its
  matching exit.  This mirrors the iRule timing hierarchy from the
  rule-profiler model (Event → Rule → Rule-VM → VM → Command): the
  ``RULE_ENTRY``/``RULE_EXIT`` span is what correlates to BIG-IP's
  existing per-rule CPU-cycle stats.  Convenience views
  :attr:`command_time_us` / :attr:`rule_time_us` / :attr:`event_time_us`
  slice it by level.

Caveats baked into the importers' semantics (see the rule-profiler docs):

* Core Tcl commands compiled to bytecode (``set``, ``append``, ``incr``,
  ``expr`` …) never appear as ``CMD_ENTER`` occurrences — a variable write
  surfaces as ``VAR_MOD`` instead.  Coarse command attribution therefore
  only sees genuine (uncompiled / extension) commands.
* Suspend/resume commands (``table``, ``after`` …) fire enter/exit more
  than once for a single logical call, so their counts can over-report.
* On multi-TMM systems one log interleaves occurrences from every TMM;
  timing is paired per ``(process_id, flow_id)`` so spans don't cross
  TMM/flow boundaries.
"""

from __future__ import annotations

from collections import Counter
from collections.abc import Mapping, Sequence
from dataclasses import dataclass, field

from .occurrence import Occurrence, OccurrenceKind

#: ``*_ENTRY`` / ``*_ENTER`` occurrence kind -> its matching exit kind.
#: Ordered top-down to match the iRule execution hierarchy.
_SPAN_PAIRS: dict[OccurrenceKind, OccurrenceKind] = {
    OccurrenceKind.EVENT_ENTRY: OccurrenceKind.EVENT_EXIT,
    OccurrenceKind.RULE_ENTRY: OccurrenceKind.RULE_EXIT,
    OccurrenceKind.RULE_VM_ENTRY: OccurrenceKind.RULE_VM_EXIT,
    OccurrenceKind.CMD_VM_ENTRY: OccurrenceKind.CMD_VM_EXIT,
    OccurrenceKind.CMD_ENTER: OccurrenceKind.CMD_EXIT,
}
_EXIT_TO_ENTER: dict[OccurrenceKind, OccurrenceKind] = {v: k for k, v in _SPAN_PAIRS.items()}


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
    #: (occurrence-kind value, occurrence value) -> total microseconds.
    span_time_us: Mapping[tuple[str, str], int] = field(default_factory=dict)
    #: Provenance tag — e.g. ``"f5-log"`` or ``"tclsh-trace"``.
    source: str = ""

    # -- span-timing convenience views -------------------------------------

    def _span_level(self, kind: OccurrenceKind) -> dict[str, int]:
        return {
            value: total
            for (kind_value, value), total in self.span_time_us.items()
            if kind_value == kind.value
        }

    @property
    def command_time_us(self) -> dict[str, int]:
        """command name -> total microseconds (CMD_ENTER → CMD_EXIT)."""
        return self._span_level(OccurrenceKind.CMD_ENTER)

    @property
    def rule_time_us(self) -> dict[str, int]:
        """rule name -> total microseconds (RULE_ENTRY → RULE_EXIT).

        This is the span that correlates to BIG-IP's existing per-rule
        min/max/avg CPU-cycle timing stats.
        """
        return self._span_level(OccurrenceKind.RULE_ENTRY)

    @property
    def event_time_us(self) -> dict[str, int]:
        """event name -> total microseconds (EVENT_ENTRY → EVENT_EXIT)."""
        return self._span_level(OccurrenceKind.EVENT_ENTRY)

    # -- presence / lookups ------------------------------------------------

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
        """Roll an occurrence stream up into aggregated counts + span timing."""
        lines: Counter[int] = Counter()
        commands: Counter[str] = Counter()
        events: Counter[str] = Counter()
        var_mods: Counter[str] = Counter()
        bytecode: Counter[str] = Counter()
        spans: Counter[tuple[str, str]] = Counter()

        # Per-(pid, flow) stacks of open spans, so enter/exit pairing never
        # crosses a TMM or connection boundary (multi-TMM interleaving).
        open_spans: dict[tuple[int | None, str | None], list[tuple[OccurrenceKind, str, int]]] = {}

        def _flow_key(occ: Occurrence) -> tuple[int | None, str | None]:
            ctx = occ.context
            return (ctx.process_id, ctx.flow_id) if ctx else (None, None)

        for occ in occurrences:
            if occ.kind is OccurrenceKind.CMD_ENTER:
                commands[occ.value] += 1
                if occ.line is not None:
                    lines[occ.line] += 1
            elif occ.kind is OccurrenceKind.EVENT_ENTRY:
                events[occ.value] += 1
            elif occ.kind is OccurrenceKind.VAR_MOD:
                var_mods[occ.var_name] += 1
            elif occ.kind is OccurrenceKind.BYTECODE:
                bytecode[occ.value] += 1

            # Span timing — independent of the counting above.
            if occ.timestamp_us is None:
                continue
            if occ.kind in _SPAN_PAIRS:
                open_spans.setdefault(_flow_key(occ), []).append(
                    (occ.kind, occ.value, occ.timestamp_us)
                )
            elif occ.kind in _EXIT_TO_ENTER:
                want = _EXIT_TO_ENTER[occ.kind]
                stack = open_spans.get(_flow_key(occ))
                if not stack:
                    continue
                # Match the most recent open span of the wanted enter-kind
                # (handles interleaved levels without assuming strict LIFO).
                for i in range(len(stack) - 1, -1, -1):
                    enter_kind, value, started = stack[i]
                    if enter_kind is want:
                        del stack[i]
                        delta = occ.timestamp_us - started
                        if delta >= 0:
                            spans[(enter_kind.value, value)] += delta
                        break

        return cls(
            occurrences=tuple(occurrences),
            line_counts=dict(lines),
            command_counts=dict(commands),
            event_counts=dict(events),
            var_mod_counts=dict(var_mods),
            bytecode_counts=dict(bytecode),
            span_time_us=dict(spans),
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
    spans: Counter[tuple[str, str]] = Counter()
    tags: list[str] = []
    for p in profiles:
        occurrences.extend(p.occurrences)
        lines.update(p.line_counts)
        commands.update(p.command_counts)
        events.update(p.event_counts)
        var_mods.update(p.var_mod_counts)
        bytecode.update(p.bytecode_counts)
        spans.update(p.span_time_us)
        if p.source and p.source not in tags:
            tags.append(p.source)
    return ProfileData(
        occurrences=tuple(occurrences),
        line_counts=dict(lines),
        command_counts=dict(commands),
        event_counts=dict(events),
        var_mod_counts=dict(var_mods),
        bytecode_counts=dict(bytecode),
        span_time_us=dict(spans),
        source=source if source is not None else "+".join(tags),
    )
